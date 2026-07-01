use std::collections::HashSet;

use ast::Program;
use errors::{
    did_you_mean, ErrorKind, NyraError, Span, E002_UNDEFINED_NAME, E003_TYPE_MISMATCH,
    E004_CANNOT_INFER, E005_UNKNOWN_STRUCT, E006_IMMUTABLE_ASSIGN, E007_WRONG_ARITY,
    E008_WRONG_ARG_TYPE,
};

use crate::{Type, TypeChecker, TypeEnv};

pub fn register_program_names(checker: &mut TypeChecker, program: &Program) {
    let mut names = HashSet::new();
    for f in &program.functions {
        names.insert(f.name.clone());
    }
    for s in &program.structs {
        names.insert(s.name.clone());
    }
    for e in &program.enums {
        names.insert(e.name.clone());
    }
    for c in &program.consts {
        names.insert(c.name.clone());
    }
    for (name, _) in &checker.env.functions {
        names.insert(name.clone());
    }
    checker.global_names = names.into_iter().collect();
    checker.global_names.sort();
}

pub fn candidate_names(checker: &TypeChecker, env: &TypeEnv) -> Vec<String> {
    let mut names: HashSet<String> = checker.global_names.iter().cloned().collect();
    for k in env.variables.keys() {
        names.insert(k.clone());
    }
    for k in env.functions.keys() {
        names.insert(k.clone());
    }
    let mut v: Vec<_> = names.into_iter().collect();
    v.sort();
    v
}

pub fn undefined_name(checker: &mut TypeChecker, name: &str, sp: Span, env: &TypeEnv) {
    let cands: Vec<String> = candidate_names(checker, env);
    let cands_ref: Vec<&str> = cands.iter().map(String::as_str).collect();
    let mut err = NyraError::coded(
        E002_UNDEFINED_NAME,
        ErrorKind::NameResolution,
        sp,
        format!("undefined variable `{name}`"),
    )
    .label(format!("`{name}` is not in scope"));
    if let Some(suggestion) = did_you_mean(name, cands_ref, 3) {
        err = err.help(format!("did you mean `{suggestion}`?"));
    }
    checker.errors.push(err);
}

pub fn type_mismatch_var(
    checker: &mut TypeChecker,
    name: &str,
    expected: &str,
    got: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E003_TYPE_MISMATCH,
            ErrorKind::Type,
            sp,
            format!("type mismatch: variable `{name}` expected {expected} but found {got}"),
        )
        .help(format!("change the type annotation or the value assigned to `{name}`")),
    );
}

pub fn cannot_infer(checker: &mut TypeChecker, name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E004_CANNOT_INFER,
            ErrorKind::Type,
            sp,
            format!("Nyra could not infer the type of `{name}`"),
        )
        .label("add an explicit type annotation — types are optional, but required when inference cannot decide")
        .help(format!("example: `let {name}: i32 = ...` or `let {name}: string = ...`")),
    );
}

pub fn cannot_infer_param(
    checker: &mut TypeChecker,
    param_name: &str,
    func_name: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E004_CANNOT_INFER,
            ErrorKind::Type,
            sp,
            format!(
                "Nyra could not infer the type of parameter `{param_name}` in `{func_name}`"
            ),
        )
        .label("add an explicit parameter type — the only time Nyra requires a type annotation")
        .help(format!(
            "example: `fn {func_name}({param_name}: string)` or `fn {func_name}({param_name}: StrVec)`"
        )),
    );
}

pub fn cannot_infer_return(checker: &mut TypeChecker, func_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E004_CANNOT_INFER,
            ErrorKind::Type,
            sp,
            format!("Nyra could not infer the return type of `{func_name}`"),
        )
        .label("add an explicit return type after the parameter list")
        .help(format!("example: `fn {func_name}() -> i32 {{ ... }}` or `fn {func_name}() -> StrVec {{ ... }}`")),
    );
}

pub fn conflicting_param_types(
    checker: &mut TypeChecker,
    param_name: &str,
    func_name: &str,
    types: &[Type],
    sp: Span,
) {
    let kinds: Vec<String> = types.iter().map(type_pretty).collect();
    checker.errors.push(
        NyraError::coded(
            E004_CANNOT_INFER,
            ErrorKind::Type,
            sp,
            format!(
                "conflicting inferred types for parameter `{param_name}` in `{func_name}`: {}",
                kinds.join(", ")
            ),
        )
        .label("add an explicit type annotation to disambiguate")
        .help(format!("example: `fn {func_name}({param_name}: string)`")),
    );
}

pub fn unknown_struct(checker: &mut TypeChecker, name: &str, sp: Span, env: &TypeEnv) {
    let struct_names: Vec<&str> = checker
        .global_names
        .iter()
        .filter(|n| checker.structs.contains_key(*n))
        .map(String::as_str)
        .collect();
    let mut err = NyraError::coded(
        E005_UNKNOWN_STRUCT,
        ErrorKind::Type,
        sp,
        format!("unknown struct `{name}`"),
    )
    .help(format!("declare it first: `struct {name} {{ ... }}`"));
    if let Some(suggestion) = did_you_mean(name, struct_names, 3) {
        err = err.help(format!("did you mean `{suggestion}`?"));
    }
    let _ = env;
    checker.errors.push(err);
}

pub fn immutable_assign(checker: &mut TypeChecker, name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E006_IMMUTABLE_ASSIGN,
            ErrorKind::Type,
            sp,
            format!("cannot assign to immutable variable `{name}`"),
        )
        .help(format!("use `let mut {name} = ...` to make it mutable")),
    );
}

pub fn wrong_arity(
    checker: &mut TypeChecker,
    callee: &str,
    expected: usize,
    got: usize,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E007_WRONG_ARITY,
            ErrorKind::Type,
            sp,
            format!("function `{callee}` expects {expected} arguments, found {got}"),
        )
        .label("wrong number of arguments"),
    );
}

pub fn wrong_arg_type(checker: &mut TypeChecker, callee: &str, detail: impl Into<String>, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!("argument type mismatch in call to `{callee}`"),
        )
        .note(detail.into()),
    );
}

pub fn type_pretty(ty: &Type) -> String {
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
        Type::VecStr => "vec_str".into(),
        Type::Struct(n) => n.clone(),
        Type::Union(n) => format!("union {n}"),
        Type::Enum(n) => n.clone(),
        Type::Simd { elem, lanes } => format!("{}x{lanes}", type_pretty(elem)),
        Type::Ref {
            inner,
            mutable,
            lifetime,
        } => {
            let lt = lifetime
                .as_deref()
                .map(|l| format!("{l} "))
                .unwrap_or_default();
            if *mutable {
                format!("&mut {lt}{}", type_pretty(inner))
            } else {
                format!("&{lt}{}", type_pretty(inner))
            }
        }
        Type::RawPtr { inner } => format!("*{}", type_pretty(inner)),
        Type::Array { elem, len } => {
            if let Some(n) = len {
                format!("[{}; {n}]", type_pretty(elem))
            } else {
                format!("[{}]", type_pretty(elem))
            }
        }
        Type::Tuple { elems } => {
            let inner = elems.iter().map(type_pretty).collect::<Vec<_>>().join(", ");
            format!("({inner})")
        }
        Type::Generic(n) => n.clone(),
        Type::Unknown => "_".into(),
        Type::ForAll { inner, .. } => type_pretty(inner),
        Type::FnPtr { .. } => "fn".into(),
    }
}

pub fn invalid_print_arg(checker: &mut TypeChecker, ty: &Type, sp: Span) {
    if let Type::Ref { inner, mutable, .. } = ty {
        let ref_ty = type_pretty(ty);
        let mut err = NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            "cannot pass a reference to `print`",
        )
        .note(format!(
            "expected string, i32, f32, f64, char, bool, or a fixed array of those — found `{ref_ty}`"
        ));
        if *mutable {
            err = err.help("dereference: `print(*name)`");
        } else {
            err = err.help("dereference: `print(*name)`");
            err = err.help("end any active borrows before printing the value");
        }
        let _ = inner;
        checker.errors.push(err);
        return;
    }
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            "argument type mismatch in call to `print`",
        )
        .note(format!(
            "expected string, i32, f32, f64, char, bool, or a fixed array of those — found `{}`",
            type_pretty(ty)
        )),
    );
}
