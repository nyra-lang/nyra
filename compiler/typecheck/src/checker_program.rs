//! Module-level checking: structs, enums, imports, and top-level items.
use std::collections::HashMap;

use ast::*;
use ast::expr_span;
use errors::{ErrorKind, NyraError};

use super::{FunctionSignature, TypeChecker, TypeEnv, VarInfo};
use super::diagnostics;
use types::{self, EnumInfo, StructInfo, UnionInfo, Type};

impl TypeChecker {
    pub fn check_program(&mut self, program: &Program) {
        self.no_std = program.no_std || self.no_std;
        self.register_builtins();
        self.register_date_builtin();
        diagnostics::register_program_names(self, program);
        for e in &program.enums {
            if !e.type_params.is_empty() {
                continue;
            }
            let mut variant_infos = Vec::new();
            for v in &e.variants {
                let fields: Vec<Type> = v
                    .fields
                    .iter()
                    .map(|f| self.type_from_ann(f))
                    .collect();
                variant_infos.push(types::EnumVariantInfo {
                    name: v.name.clone(),
                    fields,
                });
            }
            self.enums.insert(
                e.name.clone(),
                EnumInfo {
                    variants: variant_infos,
                },
            );
        }
        self.finalize_enum_registry(program);
        self.trait_methods.clear();
        self.trait_impl_pairs.clear();
        for t in &program.traits {
            self.trait_methods
                .insert(t.name.clone(), t.methods.clone());
        }
        for ti in &program.trait_impls {
            self.trait_impl_pairs
                .push((ti.type_name.clone(), ti.trait_name.clone()));
        }
        self.register_trait_dispatch_sigs(program);
        for s in &program.structs {
            let mut fields = HashMap::new();
            let mut field_anns = HashMap::new();
            let mut field_order = Vec::new();
            for f in &s.fields {
                fields.insert(f.name.clone(), self.type_from_ann(&f.ty));
                field_anns.insert(f.name.clone(), f.ty.clone());
                field_order.push(f.name.clone());
            }
            self.structs.insert(
                s.name.clone(),
                StructInfo {
                    fields,
                    field_anns,
                    field_order,
                    repr_c: s.attrs.repr_c,
                    align: s.attrs.align,
                    packed: s.attrs.packed,
                },
            );
        }
        for u in &program.unions {
            let mut fields = HashMap::new();
            let mut field_anns = HashMap::new();
            let mut field_order = Vec::new();
            for f in &u.fields {
                fields.insert(f.name.clone(), self.type_from_ann(&f.ty));
                field_anns.insert(f.name.clone(), f.ty.clone());
                field_order.push(f.name.clone());
            }
            self.unions.insert(
                u.name.clone(),
                UnionInfo {
                    fields,
                    field_anns,
                    field_order,
                    repr_c: u.attrs.repr_c,
                    align: u.attrs.align,
                    packed: u.attrs.packed,
                },
            );
        }
        for t in &program.traits {
            for m in &t.methods {
                let params: Vec<Type> = m.params.iter().map(|p| self.type_from_ann(&p.ty)).collect();
                let ret = m
                    .return_type
                    .clone()
                    .map(|a| self.type_from_ann(&a))
                    .unwrap_or(Type::Void);
                self.env.functions.insert(
                    format!("{}_{}", t.name, m.name),
                    FunctionSignature {
                        params,
                        return_type: ret,
                    },
                );
            }
        }
        for ti in &program.trait_impls {
            for m in &ti.methods {
                let params: Vec<Type> = m
                    .params
                    .iter()
                    .map(|p| self.type_from_ann(&p.ty))
                    .collect();
                let ret = m
                    .return_type
                    .clone()
                    .map(|a| self.type_from_ann(&a))
                    .unwrap_or(Type::Void);
                self.env.functions.insert(m.name.clone(), FunctionSignature {
                    params,
                    return_type: ret,
                });
            }
        }
        for ext in &program.externs {
            self.check_extern_fn_abi(ext);
            let param_types: Vec<Type> = ext
                .params
                .iter()
                .map(|p| self.type_from_ann(&p.ty))
                .collect();
            let ret = ext
                .return_type
                .clone()
                .map(|t| self.type_from_ann(&t))
                .unwrap_or(Type::Void);
            self.env.functions.insert(
                ext.name.clone(),
                FunctionSignature {
                    params: param_types,
                    return_type: ret,
                },
            );
        }
        for c in &program.consts {
            let value_ty = self.check_expr(&c.value, &mut self.env.clone());
            let declared = c.ty.clone().map(Type::from);
            if let Some(ref d) = declared {
                if value_ty != *d && value_ty != Type::Unknown && *d != Type::Unknown {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        expr_span(&c.value),
                        format!(
                            "Const '{}' type mismatch: expected {:?}, got {:?}",
                            c.name, d, value_ty
                        ),
                    ));
                }
            }
            let var_ty = declared.unwrap_or(value_ty);
            self.env.variables.insert(
                c.name.clone(),
                VarInfo {
                    ty: var_ty,
                    mutable: false,
                },
            );
        }
        self.check_export_instances(program);
        self.program_for_inference = Some(program as *const Program);
        self.register_all_fn_sigs(program);
        for func in &program.functions {
            if types::is_math_intrinsic_fn(&func.name)
                || types::is_simd_intrinsic_fn(&func.name)
                || types::is_layout_intrinsic_fn(&func.name)
            {
                continue;
            }
            self.check_function(func);
        }
        for imp in &program.impls {
            for method in &imp.methods {
                self.check_function(method);
            }
        }
        for ti in &program.trait_impls {
            for method in &ti.methods {
                self.check_function(method);
            }
        }
    }

    fn needs_inference_registration(func: &Function) -> bool {
        if func
            .params
            .iter()
            .any(|p| matches!(&p.ty, TypeAnnotation::Generic(n) if n == "_"))
        {
            return true;
        }
        func.return_type.is_none()
            && !func.is_async
            && !func.name.starts_with("__arrow_")
            && func.name != "main"
            && !func.is_test
            && !func.name.ends_with("_drop")
    }

    fn register_all_fn_sigs(&mut self, program: &Program) {
        for imp in &program.impls {
            for method in &imp.methods {
                if !Self::needs_inference_registration(method) {
                    self.register_fn_sig(method);
                }
            }
        }
        for ti in &program.trait_impls {
            for method in &ti.methods {
                if !Self::needs_inference_registration(method) {
                    self.register_fn_sig(method);
                }
            }
        }
        for func in &program.functions {
            if !Self::needs_inference_registration(func) {
                self.register_fn_sig(func);
            }
        }
        for _ in 0..5 {
            for imp in &program.impls {
                for method in &imp.methods {
                    if Self::needs_inference_registration(method) {
                        self.register_fn_sig(method);
                    }
                }
            }
            for func in &program.functions {
                if Self::needs_inference_registration(func) {
                    self.register_fn_sig(func);
                }
            }
            for ti in &program.trait_impls {
                for method in &ti.methods {
                    if Self::needs_inference_registration(method) {
                        self.register_fn_sig(method);
                    }
                }
            }
        }
    }

    pub(super) fn register_fn_sig(&mut self, func: &Function) {
        self.signature_inference = true;
        let params = self.resolve_inferred_param_anns(func);
        let param_types: Vec<Type> = params.iter().map(|p| self.type_from_ann(p)).collect();
        let ret = self.function_return_type_with_params(func, &param_types);
        self.env.functions.insert(
            func.name.clone(),
            FunctionSignature {
                params: param_types,
                return_type: ret,
            },
        );
        self.signature_inference = false;
    }

    /// Write inferred parameter and return types onto the AST so codegen sees them.
    pub fn apply_inferred_signatures(&self, program: &mut Program) {
        for func in &mut program.functions {
            Self::apply_inferred_signature_to_func(func, &self.env);
        }
        for imp in &mut program.impls {
            for method in &mut imp.methods {
                Self::apply_inferred_signature_to_func(method, &self.env);
            }
        }
        for ti in &mut program.trait_impls {
            for method in &mut ti.methods {
                Self::apply_inferred_signature_to_func(method, &self.env);
            }
        }
    }

    /// Back-compat alias for callers that only propagated return types.
    pub fn apply_inferred_return_types(&self, program: &mut Program) {
        self.apply_inferred_signatures(program);
    }

    /// Best-effort type hint for an expression after `check_program` (used by async for-in desugar).
    pub fn expression_type_hint(&self, expr: &Expression) -> Option<Type> {
        self.expr_type_hint(expr)
    }

    fn apply_inferred_signature_to_func(func: &mut Function, env: &TypeEnv) {
        let Some(sig) = env.functions.get(&func.name) else {
            return;
        };
        for (param, inferred_ty) in func.params.iter_mut().zip(sig.params.iter()) {
            if matches!(&param.ty, TypeAnnotation::Generic(n) if n == "_")
                && !matches!(inferred_ty, Type::Unknown | Type::Generic(_))
            {
                param.ty = Self::type_to_ann(inferred_ty);
            }
        }
        if func.return_type.is_some() || func.is_async {
            return;
        }
        if func.name == "main" || func.is_test || func.name.ends_with("_drop") {
            return;
        }
        let ret = &sig.return_type;
        if matches!(ret, Type::Unknown | Type::Generic(_)) {
            return;
        }
        func.return_type = Some(Self::type_to_ann(ret));
    }
}

