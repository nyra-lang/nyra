use ast::*;
use errors::Span;
use types::Type;

use crate::TypeChecker;
use crate::TypeEnv;
use crate::VarInfo;
use crate::diagnostics;

pub fn array_method_borrows_receiver(method: &str) -> bool {
    matches!(method, "length" | "len" | "sort" | "sort_by")
}

impl TypeChecker {
    fn validate_sort_by_cmp(&mut self, cmp_ty: &Type, elem: &Type, sp: &Span) {
        match cmp_ty {
            Type::FnPtr {
                params,
                return_type,
                ..
            } => {
                if params.len() != 2 {
                    diagnostics::sort_by_wrong_arity(self, params.len(), sp.clone());
                    return;
                }
                if params[0] != *elem || params[1] != *elem {
                    diagnostics::sort_by_param_mismatch(self, elem, sp.clone());
                }
                let ret = return_type
                    .as_ref()
                    .map(|t| t.as_ref())
                    .unwrap_or(&Type::Void);
                if *ret != Type::Integer(ast::IntKind::I32) {
                    diagnostics::sort_by_return_mismatch(self, sp.clone());
                }
            }
            _ => {
                diagnostics::sort_by_expects_fn(self, sp.clone());
            }
        }
    }

    fn check_sort_by_arrow(
        &mut self,
        a: &ArrowFnExpr,
        elem: &Type,
        env: &TypeEnv,
        sp: &Span,
    ) -> Type {
        if a.params.len() != 2 {
            diagnostics::sort_by_wrong_arity(self, a.params.len(), sp.clone());
        }
        let mut inner = TypeEnv {
            variables: env.variables.clone(),
            functions: env.functions.clone(),
        };
        for p in &a.params {
            if p.destructure.is_empty() {
                inner.variables.insert(
                    p.name.clone(),
                    VarInfo {
                        ty: elem.clone(),
                        mutable: p.mutable,
                    },
                );
            }
        }
        let ret_ty = match &a.body {
            ArrowBody::Expr(e) => self.check_expr(e, &mut inner),
            ArrowBody::Block(b) => {
                self.check_block(b, &mut inner, &Type::Unknown);
                let mut ret = Type::Integer(ast::IntKind::I32);
                for stmt in &b.statements {
                    if let Statement::Return(r) = stmt {
                        ret = if let Some(v) = &r.value {
                            self.check_expr(v, &mut inner)
                        } else {
                            Type::Void
                        };
                    }
                }
                ret
            }
        };
        if ret_ty != Type::Integer(ast::IntKind::I32) && ret_ty != Type::Unknown {
            diagnostics::sort_by_return_mismatch(self, sp.clone());
        }
        Type::FnPtr {
            lifetime_params: vec![],
            params: vec![elem.clone(), elem.clone()],
            return_type: Some(Box::new(Type::Integer(ast::IntKind::I32))),
        }
    }

    pub(super) fn check_array_method(
        &mut self,
        mc: &MethodCallExpr,
        obj_ty: &Type,
        env: &mut TypeEnv,
        sp: &Span,
    ) -> Option<Type> {
        let Type::Array { elem, len } = obj_ty else {
            return None;
        };
        let Some(n) = *len else {
            if mc.method == "length" || mc.method == "len" || mc.method == "sort" || mc.method == "sort_by"
            {
                diagnostics::array_method_requires_fixed(self, &mc.method, sp.clone());
            }
            return None;
        };

        match mc.method.as_str() {
            "length" | "len" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".{}", mc.method), 0, mc.args.len(), sp.clone());
                }
                Some(Type::Integer(ast::IntKind::I32))
            }
            "sort" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, ".sort", 0, mc.args.len(), sp.clone());
                }
                if !matches!(elem.as_ref(), Type::Integer(ast::IntKind::I32) | Type::F32 | Type::F64) {
                    diagnostics::array_sort_unsupported_elem(self, elem, sp.clone());
                    return Some(Type::Unknown);
                }
                Some(Type::Array {
                    elem: elem.clone(),
                    len: Some(n),
                })
            }
            "sort_by" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, ".sort_by", 1, mc.args.len(), sp.clone());
                } else if let Expression::ArrowFn(a) = &mc.args[0] {
                    self.check_sort_by_arrow(a, elem, env, sp);
                } else {
                    let cmp_ty = self.check_expr(&mc.args[0], env);
                    self.validate_sort_by_cmp(&cmp_ty, elem, sp);
                }
                Some(Type::Array {
                    elem: elem.clone(),
                    len: Some(n),
                })
            }
            _ => None,
        }
    }
}
