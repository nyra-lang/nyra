use std::collections::{HashMap, HashSet};

use ast::*;
use errors::{ErrorKind, NyraError, Span};
use crate::context::OwnershipCtx;
use types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadSafety {
    pub send: bool,
    pub sync: bool,
}

struct Checker<'a> {
    ctx: &'a OwnershipCtx,
    visiting: HashSet<String>,
}

#[derive(Clone, Copy)]
enum SafetyMode {
    /// Trust explicit `Send` / `Sync` markers on structs.
    TrustExplicit,
    /// Compute from fields only (for validating explicit markers).
    FieldsOnly,
}

impl<'a> Checker<'a> {
    fn new(ctx: &'a OwnershipCtx) -> Self {
        Self {
            ctx,
            visiting: HashSet::new(),
        }
    }

    fn of_type(&mut self, ty: &Type) -> ThreadSafety {
        self.of_type_mode(ty, SafetyMode::TrustExplicit)
    }

    fn of_type_mode(&mut self, ty: &Type, mode: SafetyMode) -> ThreadSafety {
        match ty {
            Type::Integer(_)
            | Type::F32
            | Type::F64
            | Type::Char
            | Type::Bool
            | Type::Void
            | Type::Enum(_)
            | Type::Handle
            | Type::VecStr
            | Type::Ptr => ThreadSafety {
                send: true,
                sync: true,
            },
            Type::RawPtr { .. } => ThreadSafety {
                send: false,
                sync: false,
            },
            Type::String | Type::Bytes => ThreadSafety {
                send: true,
                sync: true,
            },
            Type::Simd { .. } | Type::Union(_) => ThreadSafety {
                send: true,
                sync: true,
            },
            Type::Struct(name) => self.of_struct_mode(name, mode),
            Type::Array { elem, .. } => self.of_type_mode(elem, mode),
            Type::Tuple { elems } => {
                let mut send = true;
                let mut sync = true;
                for e in elems {
                    let ts = self.of_type_mode(e, mode);
                    send &= ts.send;
                    sync &= ts.sync;
                }
                ThreadSafety { send, sync }
            }
            Type::Ref { inner, mutable, .. } => {
                let inner_ts = self.of_type_mode(inner, mode);
                if *mutable {
                    ThreadSafety {
                        send: inner_ts.send,
                        sync: inner_ts.send,
                    }
                } else {
                    ThreadSafety {
                        send: inner_ts.sync,
                        sync: true,
                    }
                }
            }
            Type::ForAll { inner, .. } => self.of_type_mode(inner, mode),
            Type::FnPtr { .. } => ThreadSafety {
                send: true,
                sync: true,
            },
            Type::Generic(_) | Type::Unknown => ThreadSafety {
                send: false,
                sync: false,
            },
        }
    }

    fn of_struct(&mut self, name: &str) -> ThreadSafety {
        self.of_struct_mode(name, SafetyMode::TrustExplicit)
    }

    fn of_struct_mode(&mut self, name: &str, mode: SafetyMode) -> ThreadSafety {
        if matches!(mode, SafetyMode::TrustExplicit) {
            if let Some(meta) = self.ctx.structs.get(name) {
                if meta.explicit_send || meta.explicit_sync {
                    return ThreadSafety {
                        send: meta.explicit_send,
                        sync: meta.explicit_sync,
                    };
                }
            }
        }
        self.derived_of_struct(name)
    }

    /// Field-derived Send/Sync only — ignores explicit `Send` / `Sync` struct markers.
    fn derived_of_struct(&mut self, name: &str) -> ThreadSafety {
        if name.starts_with("Arc_") || name == "Arc" {
            return ThreadSafety {
                send: true,
                sync: true,
            };
        }
        if !self.visiting.insert(name.to_string()) {
            return ThreadSafety {
                send: false,
                sync: false,
            };
        }
        let Some(meta) = self.ctx.structs.get(name) else {
            self.visiting.remove(name);
            return ThreadSafety {
                send: false,
                sync: false,
            };
        };
        let mut send = true;
        let mut sync = true;
        for ty in meta.info.fields.values() {
            let ts = self.of_type_mode(ty, SafetyMode::FieldsOnly);
            send &= ts.send;
            sync &= ts.sync;
        }
        self.visiting.remove(name);
        ThreadSafety { send, sync }
    }
}

pub fn check_program(program: &Program, ctx: &OwnershipCtx, errors: &mut Vec<NyraError>) {
    for s in &program.structs {
        check_struct_attrs(s, ctx, errors);
    }
}

fn check_struct_attrs(s: &StructDef, ctx: &OwnershipCtx, errors: &mut Vec<NyraError>) {
    let derived = derived_thread_safety_of_struct(&s.name, ctx);
    if s.attrs.send && !derived.send {
        errors.push(NyraError::new(
            ErrorKind::BorrowCheck,
            Span::default(),
            format!(
                "struct '{}' is marked Send but field types are not all Send",
                s.name
            ),
        ));
    }
    if s.attrs.sync && !derived.sync {
        errors.push(NyraError::new(
            ErrorKind::BorrowCheck,
            Span::default(),
            format!(
                "struct '{}' is marked Sync but field types are not all Sync",
                s.name
            ),
        ));
    }
}

pub fn thread_safety_of(ty: &Type, ctx: &OwnershipCtx) -> ThreadSafety {
    Checker::new(ctx).of_type(ty)
}

pub fn thread_safety_of_struct(name: &str, ctx: &OwnershipCtx) -> ThreadSafety {
    Checker::new(ctx).of_struct(name)
}

pub fn derived_thread_safety_of_struct(name: &str, ctx: &OwnershipCtx) -> ThreadSafety {
    Checker::new(ctx).derived_of_struct(name)
}

pub fn is_send(ty: &Type, ctx: &OwnershipCtx) -> bool {
    thread_safety_of(ty, ctx).send
}

pub fn is_sync(ty: &Type, ctx: &OwnershipCtx) -> bool {
    thread_safety_of(ty, ctx).sync
}

pub fn check_spawn_captures(
    body: &Block,
    outer_vars: &HashMap<String, Type>,
    span: Span,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
) {
    check_thread_captures(body, outer_vars, span, ctx, errors, "spawn");
}

pub fn check_parallel_for_captures(
    body: &Block,
    outer_vars: &HashMap<String, Type>,
    span: Span,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
) {
    check_thread_captures(body, outer_vars, span, ctx, errors, "parallel for");
}

fn check_thread_captures(
    body: &Block,
    outer_vars: &HashMap<String, Type>,
    span: Span,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
    context: &str,
) {
    use crate::nll::collect_captures;

    let declared: HashSet<String> = outer_vars.keys().cloned().collect();
    for name in collect_captures(body, &declared) {
        let Some(ty) = outer_vars.get(&name) else {
            continue;
        };
        if !is_send(ty, ctx) {
            errors.push(
                NyraError::new(
                    ErrorKind::BorrowCheck,
                    span.clone(),
                    format!("cannot use {context}: captured value '{name}' is not Send"),
                )
                .note("Only Send types may cross thread boundaries"),
            );
        }
        if matches!(ty, Type::Ref { mutable: false, .. }) {
            let inner = ref_inner(ty);
            if !is_sync(&inner, ctx) {
                errors.push(
                    NyraError::new(
                        ErrorKind::BorrowCheck,
                        span.clone(),
                        format!(
                            "cannot use {context}: shared reference '{name}' requires Sync inner type"
                        ),
                    )
                    .note("Immutable references captured across threads need T: Sync"),
                );
            }
        }
        if matches!(ty, Type::Ref { mutable: true, .. }) {
            let inner = ref_inner(ty);
            if !is_send(&inner, ctx) {
                errors.push(
                    NyraError::new(
                        ErrorKind::BorrowCheck,
                        span.clone(),
                        format!(
                            "cannot use {context}: mutable reference '{name}' requires Send inner type"
                        ),
                    )
                    .note("Mutable references captured across threads need T: Send"),
                );
            }
        }
    }
}

fn ref_inner(ty: &Type) -> Type {
    match ty {
        Type::Ref { inner, .. } => *inner.clone(),
        other => other.clone(),
    }
}

/// Reject reference captures in sync closures (no Send requirement).
pub fn check_sync_closure_captures(
    arrow: &ArrowFnExpr,
    outer_vars: &HashMap<String, Type>,
    span: Span,
    _ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
) {
    use crate::nll::collect_arrow_captures;

    let outer_names: HashSet<String> = outer_vars.keys().cloned().collect();
    for name in collect_arrow_captures(arrow, &outer_names) {
        let Some(ty) = outer_vars.get(&name) else {
            continue;
        };
        if matches!(ty, Type::Ref { .. }) {
            errors.push(
                NyraError::new(
                    ErrorKind::BorrowCheck,
                    span.clone(),
                    "cannot capture reference in closure; use owned value or copy type".to_string(),
                )
                .note(format!("Captured variable '{name}' has reference type")),
            );
        }
    }
}
