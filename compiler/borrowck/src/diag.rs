use std::collections::{HashMap, HashSet};

use ast::*;
use errors::{
    ErrorKind, NyraError, Span, E011_USE_WHILE_BORROWED, E012_USE_AFTER_MOVE, E028_BORROW_ACTIVE,
    E029_MOVE_WHILE_BORROWED, E030_MANUAL_FREE,
};
use types::Type;

#[derive(Debug, Clone)]
pub struct MoveOrigin {
    pub callee: Option<String>,
    pub call_span: Span,
}

#[derive(Debug, Clone)]
pub struct DiagCtx {
    pub fn_params: HashMap<String, Vec<(String, Type)>>,
    pub clone_structs: HashSet<String>,
}

impl DiagCtx {
    pub fn from_program(program: &Program) -> Self {
        let mut fn_params = HashMap::new();
        let mut insert_fn = |name: String, params: &[Param]| {
            if params.is_empty() {
                fn_params.insert(name, vec![]);
            } else {
                fn_params.insert(
                    name,
                    params
                        .iter()
                        .map(|p| (p.name.clone(), Type::from(p.ty.clone())))
                        .collect(),
                );
            }
        };
        for f in &program.functions {
            if f.type_params.is_empty() {
                insert_fn(f.name.clone(), &f.params);
            }
        }
        for imp in &program.impls {
            for m in &imp.methods {
                if m.type_params.is_empty() {
                    insert_fn(m.name.clone(), &m.params);
                }
            }
        }
        for ti in &program.trait_impls {
            for m in &ti.methods {
                insert_fn(m.name.clone(), &m.params);
            }
        }
        let mut clone_structs = HashSet::new();
        for ti in &program.trait_impls {
            if ti.trait_name == "Clone" {
                clone_structs.insert(ti.type_name.clone());
            }
        }
        Self {
            fn_params,
            clone_structs,
        }
    }
}

pub fn type_label(ty: &Type) -> String {
    match ty {
        Type::Integer(k) => k.name().into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::Char => "char".into(),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Bytes => "bytes".into(),
        Type::Void => "void".into(),
        Type::Ptr => "ptr".into(),
        Type::Handle => "handle".into(),
        Type::JoinHandle => "JoinHandle".into(),
        Type::VecStr => "vec_str".into(),
        Type::Struct(n) => n.clone(),
        Type::Union(n) => format!("union {n}"),
        Type::Enum(n) => n.clone(),
        Type::Simd { elem, lanes } => format!("{}x{lanes}", type_label(elem)),
        Type::Ref { inner, mutable, .. } => {
            if *mutable {
                format!("&mut {}", type_label(inner))
            } else {
                format!("&{}", type_label(inner))
            }
        }
        Type::RawPtr { inner } => format!("*{}", type_label(inner)),
        Type::Array { elem, len } => {
            if let Some(n) = len {
                format!("[{}; {n}]", type_label(elem))
            } else {
                format!("[{}]", type_label(elem))
            }
        }
        Type::Tuple { elems } => {
            let inner = elems.iter().map(type_label).collect::<Vec<_>>().join(", ");
            format!("({inner})")
        }
        Type::Generic(n) => n.clone(),
        Type::Unknown => "_".into(),
        Type::ForAll { inner, .. } => type_label(inner),
        Type::FnPtr { .. } => "fn".into(),
    }
}

fn param_expects_ref(ty: &Type) -> bool {
    matches!(ty, Type::Ref { mutable: false, .. })
}

fn type_is_clone(ty: &Type, diag: &DiagCtx) -> bool {
    match ty {
        Type::String | Type::Bytes => true,
        Type::Struct(n) => diag.clone_structs.contains(n),
        _ => false,
    }
}

fn callee_param_note(callee: &str, diag: &DiagCtx) -> Option<String> {
    let params = diag.fn_params.get(callee)?;
    let (pname, pty) = params.first()?;
    let sig = format!("fn {callee}({pname}: {})", type_label(pty));
    if param_expects_ref(pty) {
        Some(format!("`{callee}` accepts a borrow:\n  {sig}"))
    } else {
        Some(format!("`{callee}` expects ownership:\n  {sig}"))
    }
}

pub fn use_after_move_error(
    name: &str,
    use_span: Span,
    origin: &MoveOrigin,
    var_ty: &Type,
    diag: &DiagCtx,
) -> NyraError {
    let message = if let Some(callee) = &origin.callee {
        format!("`{name}` was moved into `{callee}()`")
    } else {
        format!("`{name}` was moved")
    };

    let mut err = NyraError::coded(
        E012_USE_AFTER_MOVE,
        ErrorKind::BorrowCheck,
        use_span,
        message,
    )
    .label_span(
        origin.call_span.clone(),
        if let Some(callee) = &origin.callee {
            format!("`{name}` moved into `{callee}()` here")
        } else {
            format!("`{name}` moved here")
        },
    );

    if let Some(callee) = &origin.callee {
        if let Some(note) = callee_param_note(callee, diag) {
            err = err.note(note);
        }
        if let Some(params) = diag.fn_params.get(callee) {
            if let Some((_pname, param_ty)) = params.first() {
                if param_expects_ref(param_ty) {
                    err = err.note(format!(
                        "borrow instead:\n  {callee}(&{name})   // or: {callee}({name}) — auto-borrow applies"
                    ));
                } else                 if type_is_clone(var_ty, diag) {
                    err = err.note(format!(
                        "keep using `{name}`:\n  {callee}(clone {name})"
                    ));
                    err = err.note(format!(
                        "if you intended to transfer ownership:\n  {callee}(move {name})"
                    ));
                } else {
                    err = err.note(format!(
                        "if you intended to transfer ownership:\n  {callee}(move {name})"
                    ));
                }
            }
        }
    } else if type_is_clone(var_ty, diag) {
        err = err.note(format!(
            "keep using `{name}`:\n  let copy = clone {name}"
        ));
    }

    err
}

pub fn use_moved_value_error(name: &str, sp: Span, origin: Option<&MoveOrigin>, diag: &DiagCtx, var_ty: &Type) -> NyraError {
    if let Some(origin) = origin {
        return use_after_move_error(name, sp, origin, var_ty, diag);
    }
    NyraError::coded(
        E012_USE_AFTER_MOVE,
        ErrorKind::BorrowCheck,
        sp,
        format!("use of moved value `{name}`"),
    )
}

pub fn move_while_borrowed(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E029_MOVE_WHILE_BORROWED,
        ErrorKind::BorrowCheck,
        sp,
        format!("cannot move `{name}` while it is borrowed"),
    )
    .help(format!("drop the borrow before moving, or clone: `clone {name}`"))
}

pub fn borrow_active_error(message: &str, sp: Span, note: &str) -> NyraError {
    NyraError::coded(E028_BORROW_ACTIVE, ErrorKind::BorrowCheck, sp, message)
        .note(note)
}

pub fn manual_free_warning(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E030_MANUAL_FREE,
        ErrorKind::BorrowCheck,
        sp,
        format!("manual `free({name})` on owned value; Nyra auto-drops at scope end (double-free risk)"),
    )
    .note("remove `free` unless this is FFI escape hatch code")
}

pub fn cannot_borrow_moved(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E012_USE_AFTER_MOVE,
        ErrorKind::BorrowCheck,
        sp,
        format!("cannot borrow moved value `{name}`"),
    )
}

pub fn cannot_borrow_mut_alias(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E011_USE_WHILE_BORROWED,
        ErrorKind::BorrowCheck,
        sp,
        format!("cannot borrow `{name}` as mutable (`&mut` aliasing rule)"),
    )
    .help("only one active `&mut` borrow is allowed at a time")
}

pub fn cannot_borrow_while_mut_borrowed(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E011_USE_WHILE_BORROWED,
        ErrorKind::BorrowCheck,
        sp,
        format!("cannot borrow `{name}` while it is mutably borrowed"),
    )
    .help("finish using the mutable borrow before creating another borrow")
}

pub fn record_move_origin(
    _name: &str,
    _span: Span,
    callee: Option<&str>,
    call_span: Span,
    _explicit: bool,
) -> MoveOrigin {
    MoveOrigin {
        callee: callee.map(str::to_string),
        call_span,
    }
}

/// Operand passed to a call that may move ownership.
pub fn move_candidate<'a>(arg: &'a Expression) -> Option<(&'a str, bool)> {
    match arg {
        Expression::Unary(u) if u.op == UnaryOp::Move => {
            binding_name(&u.operand).map(|n| (n, true))
        }
        Expression::Unary(u) if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) => None,
        Expression::MethodCall(mc) if mc.method == "clone" => None,
        Expression::Variable { name, .. } => Some((name.as_str(), false)),
        _ => None,
    }
}

fn binding_name(expr: &Expression) -> Option<&str> {
    ast::binding_name(expr)
}
