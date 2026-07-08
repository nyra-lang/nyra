//! Trait object dispatch signatures and static trait method resolution.

use ast::*;
use errors::Span;

use super::{FunctionSignature, TypeChecker, TypeEnv};
use super::diagnostics;
use types::Type;

impl TypeChecker {
    pub(super) fn register_trait_dispatch_sigs(&mut self, program: &Program) {
        for trait_def in &program.traits {
            if trait_def.name == "Drop" || trait_def.name == "Clone" {
                continue;
            }
            let dyn_name = format!("Dyn_{}", trait_def.name);
            for ti in &program.trait_impls {
                if ti.trait_name != trait_def.name {
                    continue;
                }
                let box_fn = format!("{}_dyn_{}", trait_def.name, ti.type_name);
                let dyn_name = format!("Dyn_{}", trait_def.name);
                self.env.functions.insert(
                    box_fn,
                    FunctionSignature {
                        params: vec![Type::Struct(ti.type_name.clone())],
                        return_type: Type::Struct(dyn_name),
                    },
                );
            }
            for method in &trait_def.methods {
                let dispatch = format!("__dyn_{}_{}", trait_def.name, method.name);
                let mut params = vec![Type::Struct(dyn_name.clone())];
                for p in method.params.iter().skip(1) {
                    params.push(self.type_from_ann(&p.ty));
                }
                let return_type = method
                    .return_type
                    .clone()
                    .map(|a| self.type_from_ann(&a))
                    .unwrap_or(Type::Void);
                self.env.functions.insert(
                    dispatch,
                    FunctionSignature {
                        params,
                        return_type,
                    },
                );
            }
            self.env.functions.insert(
                format!("__dyn_{}_drop", trait_def.name),
                FunctionSignature {
                    params: vec![Type::Struct(dyn_name.clone())],
                    return_type: Type::Void,
                },
            );
        }
    }

    pub(super) fn trait_impl_exists(&self, trait_name: &str, type_name: &str) -> bool {
        self.trait_impl_pairs
            .iter()
            .any(|(ty, tr)| ty == type_name && tr == trait_name)
    }

    pub(super) fn dyn_trait_name(type_name: &str) -> Option<&str> {
        type_name.strip_prefix("Dyn_")
    }

    pub fn resolve_method_name(&self, type_name: &str, method: &str) -> String {
        if let Some(trait_name) = Self::dyn_trait_name(type_name) {
            return format!("__dyn_{trait_name}_{method}");
        }
        let plain = format!("{type_name}_{method}");
        if self.env.functions.contains_key(&plain) {
            return plain;
        }
        let suffix = format!("_{method}");
        for (concrete, trait_name) in &self.trait_impl_pairs {
            if concrete == type_name {
                let mangled = format!("{trait_name}_{type_name}_{method}");
                if self.env.functions.contains_key(&mangled) {
                    return mangled;
                }
                if mangled.ends_with(&suffix) && self.env.functions.contains_key(&mangled) {
                    return mangled;
                }
            }
        }
        plain
    }

    pub(super) fn trait_has_method(&self, trait_name: &str, method: &str) -> bool {
        self.trait_methods
            .get(trait_name)
            .is_some_and(|ms| ms.iter().any(|m| m.name == method))
    }

    /// Method call on a generic parameter with trait bounds (e.g. `x.hello()` when `T: Greet`).
    pub(super) fn check_generic_bound_method(
        &mut self,
        mc: &MethodCallExpr,
        type_param: &str,
        env: &mut TypeEnv,
        sp: &Span,
    ) -> Option<Type> {
        let bounds = self.current_type_param_bounds.get(type_param)?;
        for trait_name in bounds {
            if !self.trait_has_method(trait_name, &mc.method) {
                continue;
            }
            let Some(sig) = self
                .trait_methods
                .get(trait_name)
                .and_then(|methods| methods.iter().find(|m| m.name == mc.method))
                .cloned()
            else {
                continue;
            };
            let expected_args = sig.params.len().saturating_sub(1);
            if mc.args.len() != expected_args {
                diagnostics::wrong_arity(
                    self,
                    &format!("{}::{}", trait_name, mc.method),
                    expected_args,
                    mc.args.len(),
                    sp.clone(),
                );
            }
            for (arg, p) in mc.args.iter().zip(sig.params.iter().skip(1)) {
                let at = self.check_expr(arg, env);
                let expected = self.type_from_ann(&p.ty);
                if at != expected && at != Type::Unknown && expected != Type::Unknown {
                    diagnostics::method_arg_mismatch(self, &mc.method, sp.clone());
                }
            }
            return sig
                .return_type
                .map(|a| self.type_from_ann(&a))
                .or(Some(Type::Void));
        }
        None
    }
}
