use ast::{ExternFn, Function, Program, TypeAnnotation};
use errors::{ErrorKind, NyraError, Span};
use types::Type;

use crate::TypeChecker;

const ABI_POLICY: &str = "See docs/abi-policy.md for allowed FFI boundary types.";

const ABI_ALLOWED_MSG: &str =
    "Allowed: i8–i128, u8–u128, isize, usize, f32, f64, bool, string, ptr, void, enum tags, [T; N], tuples, repr(C) structs, fn callbacks, and generic type params on export templates.";

impl TypeChecker {
    pub(crate) fn check_export_instances(&mut self, program: &Program) {
        for inst in &program.export_instances {
            let Some(func) = program.functions.iter().find(|f| f.name == inst.fn_name) else {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    Span::default(),
                    format!(
                        "export inst `{}` refers to unknown function",
                        inst.fn_name
                    ),
                ));
                continue;
            };
            if !func.exported {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!("export inst `{}` requires an exported function", inst.fn_name),
                ));
            }
            if func.type_params.is_empty() {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!(
                        "export inst `{}` is only valid for generic exported functions",
                        inst.fn_name
                    ),
                ));
            } else if inst.type_args.len() != func.type_params.len() {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!(
                        "export inst `{}` expects {} type argument(s), got {}",
                        inst.fn_name,
                        func.type_params.len(),
                        inst.type_args.len()
                    ),
                ));
            } else {
                for arg in &inst.type_args {
                    self.check_abi_type_ann(
                        arg,
                        &func.span,
                        &format!("export inst `{}` type argument", inst.fn_name),
                        false,
                    );
                }
            }
        }
        self.check_generic_exports_have_instances(program);
    }

    fn check_generic_exports_have_instances(&mut self, program: &Program) {
        for func in &program.functions {
            if !func.exported || func.type_params.is_empty() {
                continue;
            }
            let has_inst = program
                .export_instances
                .iter()
                .any(|inst| inst.fn_name == func.name);
            if !has_inst {
                self.errors.push(
                    NyraError::new(
                        ErrorKind::Type,
                        func.span.clone(),
                        format!(
                            "export fn `{}` is generic; add at least one `export inst {}<...>` for the FFI boundary",
                            func.name, func.name
                        ),
                    )
                    .note("Generic export templates are not linkable until monomorphized with `export inst`."),
                );
            }
        }
    }

    pub(crate) fn check_extern_fn_abi(&mut self, ext: &ExternFn) {
        let span = Span::default();
        for p in &ext.params {
            self.check_abi_type_ann(&p.ty, &span, &format!("extern fn `{}` parameter", ext.name), false);
        }
        if let Some(ret) = &ext.return_type {
            self.check_abi_type_ann(ret, &span, &format!("extern fn `{}` return type", ext.name), false);
        }
    }

    pub(crate) fn check_export_fn_abi(&mut self, func: &Function) {
        if !func.exported {
            return;
        }
        if !func.lifetime_params.is_empty() {
            self.errors.push(
                NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!(
                        "export fn `{}` cannot have lifetime parameters",
                        func.name
                    ),
                )
                .note(ABI_POLICY),
            );
        }

        let generic_template = !func.type_params.is_empty();
        if generic_template {
            self.check_export_generic_template_abi(func);
            return;
        }

        if func.is_async {
            self.check_export_async_return(func);
        }

        for p in &func.params {
            self.check_abi_type_ann(
                &p.ty,
                &func.span,
                &format!("export fn `{}` parameter `{}`", func.name, p.name),
                false,
            );
        }
        if let Some(ret) = &func.return_type {
            if !func.is_async {
                self.check_abi_type_ann(
                    ret,
                    &func.span,
                    &format!("export fn `{}` return type", func.name),
                    false,
                );
            }
        }
    }

    fn check_export_generic_template_abi(&mut self, func: &Function) {
        if func.is_async {
            self.errors.push(
                NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!("export fn `{}` cannot be both async and generic", func.name),
                )
                .note("Monomorph with `export inst name<i32>` and call the sync instance from the host."),
            );
        }
        for p in &func.params {
            self.check_abi_type_ann_for_generic(
                &p.ty,
                &func.span,
                &format!("export fn `{}` parameter `{}`", func.name, p.name),
                &func.type_params,
            );
        }
        if let Some(ret) = &func.return_type {
            self.check_abi_type_ann_for_generic(
                ret,
                &func.span,
                &format!("export fn `{}` return type", func.name),
                &func.type_params,
            );
        }
    }

    fn check_export_async_return(&mut self, func: &Function) {
        if let Some(ret) = &func.return_type {
            match ret {
                TypeAnnotation::Integer(ast::IntKind::I32) | TypeAnnotation::Void => {}
                _ => {
                    self.errors.push(
                        NyraError::new(
                            ErrorKind::Type,
                            func.span.clone(),
                            format!(
                                "export async fn `{}` must return i32 or void at the FFI boundary (got `{ret:?}`)",
                                func.name,
                                ret = ret
                            ),
                        )
                        .note(
                            "Async exports return an i32 promise handle; the host completes/awaits via async_poll / await.",
                        ),
                    );
                }
            }
        }
    }

    fn check_abi_type_ann_for_generic(
        &mut self,
        ann: &TypeAnnotation,
        span: &Span,
        context: &str,
        type_params: &[String],
    ) {
        if !self.abi_type_ann_allowed_generic(ann, type_params) {
            self.errors.push(
                NyraError::new(
                    ErrorKind::Type,
                    span.clone(),
                    format!(
                        "{context} uses type `{ann:?}` which is not allowed at the FFI boundary"
                    ),
                )
                .note(format!("{ABI_ALLOWED_MSG} {ABI_POLICY}")),
            );
        }
    }

    fn check_abi_type_ann(&mut self, ann: &TypeAnnotation, span: &Span, context: &str, allow_generic: bool) {
        if !self.abi_type_ann_allowed(ann, allow_generic, &[]) {
            self.errors.push(
                NyraError::new(
                    ErrorKind::Type,
                    span.clone(),
                    format!(
                        "{context} uses type `{ann:?}` which is not allowed at the FFI boundary"
                    ),
                )
                .note(format!("{ABI_ALLOWED_MSG} {ABI_POLICY}")),
            );
        }
    }

    fn abi_type_ann_allowed_generic(&self, ann: &TypeAnnotation, type_params: &[String]) -> bool {
        self.abi_type_ann_allowed(ann, true, type_params)
    }

    fn abi_type_ann_allowed(
        &self,
        ann: &TypeAnnotation,
        allow_generic: bool,
        type_params: &[String],
    ) -> bool {
        match ann {
        TypeAnnotation::Integer(_)
        | TypeAnnotation::F32
        | TypeAnnotation::F64
        | TypeAnnotation::Char
            | TypeAnnotation::Bool
            | TypeAnnotation::String
            | TypeAnnotation::Bytes
            | TypeAnnotation::VecStr
            |             TypeAnnotation::Ptr
            | TypeAnnotation::RawPtr { .. }
            | TypeAnnotation::Void => true,
            TypeAnnotation::Enum(name) => self.enums.contains_key(name),
            TypeAnnotation::Struct(name) => {
                if allow_generic && type_params.iter().any(|p| p == name) {
                    return true;
                }
                if self.enums.contains_key(name) {
                    return true;
                }
                self.structs
                    .get(name)
                    .map(|s| {
                        s.repr_c
                            && s.field_order.iter().all(|f| {
                                s.fields.get(f).is_some_and(|ty| {
                                    self.abi_type_allowed(ty, allow_generic, type_params)
                                })
                            })
                    })
                    .unwrap_or(false)
            }
            TypeAnnotation::Array { elem, len } => {
                len.is_some() && self.abi_type_ann_allowed(elem, allow_generic, type_params)
            }
            TypeAnnotation::Tuple(elems) => elems
                .iter()
                .all(|e| self.abi_type_ann_allowed(e, allow_generic, type_params)),
            TypeAnnotation::FnPtr {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .all(|p| self.abi_type_ann_allowed(p, allow_generic, type_params))
                    && return_type
                        .as_ref()
                        .map(|r| self.abi_type_ann_allowed(r, allow_generic, type_params))
                        .unwrap_or(true)
            }
            TypeAnnotation::Generic(name) if allow_generic && type_params.iter().any(|p| p == name) => {
                true
            }
            TypeAnnotation::Generic(_) if allow_generic => true,
            TypeAnnotation::Generic(_) => false,
            TypeAnnotation::Applied { base, args } => {
                let name = Self::mangle_applied_type(base, args);
                self.structs
                    .get(&name)
                    .map(|s| {
                        s.repr_c
                            && s.field_order.iter().all(|f| {
                                s.fields.get(f).is_some_and(|ty| {
                                    self.abi_type_allowed(ty, allow_generic, type_params)
                                })
                            })
                    })
                    .unwrap_or(false)
            }
            TypeAnnotation::Ref { inner, mutable: false, .. } => {
                self.abi_type_ann_allowed(inner, allow_generic, type_params)
            }
            TypeAnnotation::Ref { .. }
            | TypeAnnotation::Lifetime(_)
            | TypeAnnotation::ForAll { .. }
            |             TypeAnnotation::DynTrait { .. } => false,
            TypeAnnotation::Simd { .. } => true,
            // RawPtr allowed at boundary (lowers to opaque ptr)
        }
    }

    fn mangle_applied_type(base: &str, args: &[TypeAnnotation]) -> String {
        let suffix: String = args
            .iter()
            .map(|a| Type::from(a.clone()))
            .map(|t| match t {
                Type::Integer(k) => k.name().into(),
                Type::F32 => "f32".into(),
                Type::F64 => "f64".into(),
                Type::Char => "char".into(),
                Type::Bool => "bool".into(),
                Type::String => "string".into(),
                Type::Struct(n) => n,
                Type::Enum(n) => n,
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join("_");
        format!("{base}__{suffix}")
    }

    fn abi_type_allowed(&self, ty: &Type, allow_generic: bool, type_params: &[String]) -> bool {
        match ty {
            Type::Integer(_)
            | Type::F32 | Type::F64
            | Type::Char
            | Type::Bool
            | Type::String
            | Type::Bytes
            | Type::Ptr
            | Type::RawPtr { .. }
            | Type::Void => true,
            Type::Enum(name) => self.enums.contains_key(name),
            Type::Struct(name) => {
                if allow_generic && type_params.iter().any(|p| p == name) {
                    return true;
                }
                if self.enums.contains_key(name) {
                    return true;
                }
                self.structs
                    .get(name)
                    .map(|s| {
                        s.repr_c
                            && s.field_order.iter().all(|f| {
                                s.fields.get(f).is_some_and(|t| {
                                    self.abi_type_allowed(t, allow_generic, type_params)
                                })
                            })
                    })
                    .unwrap_or(false)
            }
            Type::Array { elem, len } => {
                len.is_some() && self.abi_type_allowed(elem, allow_generic, type_params)
            }
            Type::Tuple { elems } => elems
                .iter()
                .all(|e| self.abi_type_allowed(e, allow_generic, type_params)),
            Type::FnPtr {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .all(|p| self.abi_type_allowed(p, allow_generic, type_params))
                    && return_type
                        .as_ref()
                        .map(|r| self.abi_type_allowed(r, allow_generic, type_params))
                        .unwrap_or(true)
            }
            Type::Generic(name) if allow_generic && type_params.iter().any(|p| p == name) => true,
            Type::Generic(_) if allow_generic => true,
            Type::Generic(_) => false,
            Type::Simd { .. } => true,
            Type::Union(name) => self
                .unions
                .get(name)
                .map(|u| {
                    u.repr_c
                        && u.field_order.iter().all(|f| {
                            u.fields.get(f).is_some_and(|t| {
                                self.abi_type_allowed(t, allow_generic, type_params)
                            })
                        })
                })
                .unwrap_or(false),
            Type::Ref { inner, mutable: false, .. } => {
                self.abi_type_allowed(inner, allow_generic, type_params)
            }
            Type::Ref { .. }
            | Type::ForAll { .. }
            | Type::Handle
            | Type::VecStr
            | Type::Unknown => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::Lexer;
    use parser::Parser;

    fn typecheck_src(src: &str) -> Vec<NyraError> {
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (program, pe) = Parser::new(tokens).parse();
        assert!(pe.is_empty(), "{pe:?}");
        let f = &program.functions[0];
        assert!(f.exported, "expected exported fn, got {:?}", f);
        let mut tc = TypeChecker::new();
        tc.check_program(&program);
        tc.errors
    }

    #[test]
    fn rejects_generic_export_without_export_inst() {
        let errs = typecheck_src(
            r#"
export fn id<T>(x: T) -> T {
    return x
}
"#,
        );
        assert!(
            errs.iter().any(|e| e.message.contains("export inst")),
            "{errs:?}"
        );
    }

    #[test]
    fn rejects_export_without_repr_c() {
        let errs = typecheck_src(
            r#"
struct Point {
    x: i32
}
export fn get_x(p: Point) -> i32 {
    return p.x
}
"#,
        );
        assert!(
            errs.iter().any(|e| e.message.contains("FFI boundary")),
            "{errs:?}"
        );
    }

    #[test]
    fn accepts_export_async_i32() {
        let errs = typecheck_src(
            r#"
export async fn work() -> i32 {
    return 1
}
"#,
        );
        assert!(
            !errs.iter().any(|e| e.message.contains("cannot be async")),
            "{errs:?}"
        );
    }

    #[test]
    fn accepts_export_enum_param() {
        let errs = typecheck_src(
            r#"
enum Color {
    Red
    Blue
}
export fn tag(c: Color) -> i32 {
    return 0
}
"#,
        );
        assert!(
            errs.iter().all(|e| !e.message.contains("FFI boundary")),
            "{errs:?}"
        );
    }
}
