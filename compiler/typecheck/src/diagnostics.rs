use std::collections::HashSet;

use ast::{Program, TypeAnnotation};
use errors::{
    did_you_mean, ErrorKind, NyraError, Span, E002_UNDEFINED_NAME, E003_TYPE_MISMATCH,
    E004_CANNOT_INFER, E005_UNKNOWN_STRUCT, E006_IMMUTABLE_ASSIGN, E007_WRONG_ARITY,
    E008_WRONG_ARG_TYPE, E009_INVALID_ASSIGN_TARGET, E013_UNDEFINED_FUNCTION, E014_UNKNOWN_FIELD, E015_OPERATOR_MISMATCH,
    E016_UNSAFE_REQUIRED, E017_NOT_CALLABLE, E018_UNKNOWN_METHOD, E019_BOOL_CONDITION,
    E020_CONTROL_FLOW, E021_PLATFORM_UNSUPPORTED, E022_RETURN_MISMATCH, E023_MATCH, E024_FOR_IN,
    E025_DESTRUCTURE, E026_BLOCK_VALUE, E027_INTEGER_RANGE, E031_ARRAY, E032_ENUM, E033_CAST,
    E034_FFI, E036_SEND_SYNC, E037_PARALLEL,
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

pub fn undefined_function(
    checker: &mut TypeChecker,
    name: &str,
    sp: Span,
    env: &TypeEnv,
) {
    let fn_names: Vec<&str> = checker
        .global_names
        .iter()
        .filter(|n| env.functions.contains_key(*n) || checker.env.functions.contains_key(*n))
        .map(String::as_str)
        .collect();
    let mut err = NyraError::coded(
        E013_UNDEFINED_FUNCTION,
        ErrorKind::NameResolution,
        sp,
        format!("undefined function `{name}`"),
    )
    .label(format!("`{name}` is not declared in scope"));
    if let Some(suggestion) = did_you_mean(name, fn_names, 3) {
        err = err.help(format!("did you mean `{suggestion}`?"));
    }
    checker.errors.push(err);
}

pub fn not_callable(checker: &mut TypeChecker, callee: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E017_NOT_CALLABLE,
            ErrorKind::Type,
            sp,
            format!("`{callee}` is not callable"),
        )
        .label("only functions and function pointers can be called")
        .help("check that the callee name is correct and refers to a function"),
    );
}

pub fn unknown_method(checker: &mut TypeChecker, method: &str, sp: Span) {
    let err = NyraError::coded(
        E018_UNKNOWN_METHOD,
        ErrorKind::NameResolution,
        sp,
        format!("unknown method `{method}`"),
    )
    .label(format!("no method named `{method}` on this receiver"));
    checker.errors.push(err);
}

pub fn operator_mismatch(
    checker: &mut TypeChecker,
    op: &str,
    detail: impl Into<String>,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E015_OPERATOR_MISMATCH,
            ErrorKind::Type,
            sp,
            format!("type mismatch in `{op}` operation"),
        )
        .note(detail.into()),
    );
}

pub fn arithmetic_mismatch(checker: &mut TypeChecker, sp: Span) {
    operator_mismatch(
        checker,
        "arithmetic",
        "arithmetic operators require compatible numeric operands (i32, f32, f64, or char)",
        sp,
    );
}

pub fn comparison_mismatch(checker: &mut TypeChecker, sp: Span) {
    operator_mismatch(
        checker,
        "comparison",
        "comparison operators require operands of the same comparable type",
        sp,
    );
}

pub fn bool_operand_required(checker: &mut TypeChecker, op: &str, sp: Span) {
    operator_mismatch(
        checker,
        op,
        format!("`{op}` requires bool operands"),
        sp,
    );
}

pub fn bitwise_requires_integer(checker: &mut TypeChecker, sp: Span) {
    operator_mismatch(
        checker,
        "bitwise",
        "bitwise operators require integer operands",
        sp,
    );
}

pub fn unsafe_required(checker: &mut TypeChecker, operation: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E016_UNSAFE_REQUIRED,
            ErrorKind::Type,
            sp,
            format!("`{operation}` requires unsafe"),
        )
        .label("this operation can violate memory safety")
        .help("wrap the operation in an `unsafe` block or function"),
    );
}

pub fn unknown_struct_field(
    checker: &mut TypeChecker,
    struct_name: &str,
    field: &str,
    known_fields: &[String],
    sp: Span,
) {
    let known_refs: Vec<&str> = known_fields.iter().map(|s| s.as_str()).collect();
    let mut err = NyraError::coded(
        E014_UNKNOWN_FIELD,
        ErrorKind::Type,
        sp,
        format!("struct `{struct_name}` has no field `{field}`"),
    )
    .label(format!("`{field}` is not a field of `{struct_name}`"));
    if let Some(suggestion) = did_you_mean(field, known_refs, 3) {
        err = err.help(format!("did you mean `{suggestion}`?"));
    }
    checker.errors.push(err);
}

pub fn unknown_union_field(
    checker: &mut TypeChecker,
    union_name: &str,
    field: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E014_UNKNOWN_FIELD,
            ErrorKind::Type,
            sp,
            format!("union `{union_name}` has no field `{field}`"),
        )
        .label(format!("`{field}` is not a field of `{union_name}`")),
    );
}

pub fn unknown_literal_field(
    checker: &mut TypeChecker,
    type_name: &str,
    field: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E014_UNKNOWN_FIELD,
            ErrorKind::Type,
            sp,
            format!("unknown field `{field}` on `{type_name}`"),
        )
        .help("check the field name against the struct or union definition"),
    );
}

pub fn type_mismatch(
    checker: &mut TypeChecker,
    context: &str,
    expected: &Type,
    got: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E003_TYPE_MISMATCH,
            ErrorKind::Type,
            sp,
            format!(
                "type mismatch {context}: expected {}, got {}",
                type_pretty(expected),
                type_pretty(got),
            ),
        )
        .help("adjust the expression or add an explicit type annotation"),
    );
}

pub fn bool_condition_required(checker: &mut TypeChecker, context: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E019_BOOL_CONDITION,
            ErrorKind::Type,
            sp,
            format!("`{context}` condition must be bool"),
        )
        .help("use a comparison or logical expression that evaluates to `true` or `false`"),
    );
}

pub fn branch_type_mismatch(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E003_TYPE_MISMATCH,
            ErrorKind::Type,
            sp,
            "if expression branches must have the same type",
        )
        .help("make both branches return the same type, or add an explicit annotation"),
    );
}

pub fn block_must_produce_value(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E026_BLOCK_VALUE,
            ErrorKind::Type,
            sp,
            "block must produce a value",
        )
        .label("use a trailing expression or `return` with a value")
        .help("example: `{ let x = 1; x }` or `{ return 42 }`"),
    );
}

pub fn return_type_mismatch(
    checker: &mut TypeChecker,
    expected: &Type,
    got: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E022_RETURN_MISMATCH,
            ErrorKind::Type,
            sp,
            format!(
                "return type mismatch: expected {}, got {}",
                type_pretty(expected),
                type_pretty(got),
            ),
        )
        .help("change the returned value or update the function's return type"),
    );
}

pub fn break_outside_loop(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E020_CONTROL_FLOW,
            ErrorKind::Type,
            sp,
            "`break` is only valid inside `while` or `for`",
        ),
    );
}

pub fn continue_outside_loop(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E020_CONTROL_FLOW,
            ErrorKind::Type,
            sp,
            "`continue` is only valid inside `while` or `for`",
        ),
    );
}

pub fn no_std_unavailable(checker: &mut TypeChecker, feature: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E021_PLATFORM_UNSUPPORTED,
            ErrorKind::Type,
            sp,
            format!("`{feature}` is not available in no_std programs"),
        )
        .note("use extern I/O or UART for embedded targets")
        .help("remove `#![no_std]` or provide your own I/O backend"),
    );
}

pub fn platform_unavailable(checker: &mut TypeChecker, feature: &str, platform: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E021_PLATFORM_UNSUPPORTED,
            ErrorKind::Type,
            sp,
            format!("`{feature}` is not available on {platform} targets"),
        ),
    );
}

pub fn integer_out_of_range(
    checker: &mut TypeChecker,
    value: i64,
    ty: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E027_INTEGER_RANGE,
            ErrorKind::Type,
            sp,
            format!(
                "integer literal {value} is out of range for type {}",
                type_pretty(ty),
            ),
        )
        .help("use a smaller literal or a wider integer type"),
    );
}

pub fn destructure_not_mutable(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E025_DESTRUCTURE,
            ErrorKind::Type,
            sp,
            "destructuring `let` cannot be mutable",
        )
        .help("use `let (a, b) = tuple` without `mut`"),
    );
}

pub fn destructure_length_mismatch(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E025_DESTRUCTURE,
            ErrorKind::Type,
            sp,
            "destructure pattern length must match tuple length",
        ),
    );
}

pub fn destructure_requires_tuple(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E025_DESTRUCTURE,
            ErrorKind::Type,
            sp,
            "destructure requires tuple value",
        )
        .help("example: `let (x, y) = (1, 2)`"),
    );
}

pub fn for_range_requires_integer(checker: &mut TypeChecker, which: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E024_FOR_IN,
            ErrorKind::Type,
            sp,
            format!("`for` range {which} must be an integer"),
        )
        .help("example: `for i in 0..10 { ... }`"),
    );
}

pub fn for_in_requires_iterable(checker: &mut TypeChecker, got: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E024_FOR_IN,
            ErrorKind::Type,
            sp,
            format!(
                "`for-in` requires array, string, or `vec_str`, got {}",
                type_pretty(got),
            ),
        )
        .note("fixed-size arrays use `for x in arr { ... }`")
        .help("strings iterate by `char`; split results iterate by `string`"),
    );
}

pub fn for_in_requires_fixed_array(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E024_FOR_IN,
            ErrorKind::Type,
            sp,
            "`for-in` requires a fixed-size array",
        )
        .help("use `[T; N]` syntax, e.g. `[i32; 4]`"),
    );
}

pub fn for_parallel_progress_conflict(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E024_FOR_IN,
            ErrorKind::Type,
            sp,
            "`parallel for` and `progress for` cannot be combined",
        ),
    );
}

pub fn progress_label_must_be_string(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E003_TYPE_MISMATCH,
            ErrorKind::Type,
            sp,
            "progress label must be `string`",
        ),
    );
}

pub fn parallel_threads_must_be_integer(
    checker: &mut TypeChecker,
    field: &str,
    got: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E003_TYPE_MISMATCH,
            ErrorKind::Type,
            sp,
            format!("parallel {field} must be an integer, got {}", type_pretty(got)),
        ),
    );
}

pub fn parallel_search_predicate_must_be_bool(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E037_PARALLEL,
            ErrorKind::Type,
            sp,
            "`parallel any/find/all for` body must be a `bool` predicate",
        )
        .note("use a trailing expression such as `i > 10` or `arr[i] == target`"),
    );
}

pub fn invalid_assign_target(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E009_INVALID_ASSIGN_TARGET,
            ErrorKind::Type,
            sp,
            "invalid assignment target",
        )
        .help("only variables, fields, indices, and dereferences can be assigned to"),
    );
}

pub fn field_assign_requires_struct(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E009_INVALID_ASSIGN_TARGET,
            ErrorKind::Type,
            sp,
            "field assignment requires struct receiver",
        ),
    );
}

pub fn index_assign_requires_array(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E009_INVALID_ASSIGN_TARGET,
            ErrorKind::Type,
            sp,
            "index assignment requires array value",
        ),
    );
}

pub fn deref_store_requires_unsafe(checker: &mut TypeChecker, sp: Span) {
    unsafe_required(checker, "writing through raw pointer", sp);
}

pub fn deref_store_invalid_target(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E009_INVALID_ASSIGN_TARGET,
            ErrorKind::Type,
            sp,
            "assignment through deref requires `*T`, `ptr`, or `&mut T` in unsafe",
        ),
    );
}

pub fn unknown_match_variant(
    checker: &mut TypeChecker,
    variant: &str,
    enum_name: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E023_MATCH,
            ErrorKind::Type,
            sp,
            format!("unknown variant `{variant}` for enum `{enum_name}`"),
        )
        .help(format!("check the variants declared on `enum {enum_name}`")),
    );
}

pub fn match_enum_mismatch(
    checker: &mut TypeChecker,
    pattern_enum: &str,
    scrutinee_enum: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E023_MATCH,
            ErrorKind::Type,
            sp,
            format!("pattern enum `{pattern_enum}` does not match scrutinee `{scrutinee_enum}`"),
        ),
    );
}

pub fn match_non_exhaustive(
    checker: &mut TypeChecker,
    missing: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E023_MATCH,
            ErrorKind::Type,
            sp,
            format!("non-exhaustive match: missing variant `{missing}`"),
        )
        .help("add a match arm for this variant, or use `_` as a wildcard"),
    );
}

pub fn match_guard_must_be_bool(checker: &mut TypeChecker, sp: Span) {
    bool_condition_required(checker, "match guard", sp);
}

pub fn match_unsupported_pattern(checker: &mut TypeChecker, detail: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E023_MATCH,
            ErrorKind::Type,
            sp,
            detail.to_string(),
        ),
    );
}

pub fn builtin_wrong_arity(
    checker: &mut TypeChecker,
    callee: &str,
    expected: usize,
    got: usize,
    sp: Span,
) {
    wrong_arity(checker, callee, expected, got, sp);
}

pub fn builtin_wrong_arity_range(
    checker: &mut TypeChecker,
    callee: &str,
    min: usize,
    max: usize,
    got: usize,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E007_WRONG_ARITY,
            ErrorKind::Type,
            sp,
            format!("`{callee}` expects {min} to {max} arguments, found {got}"),
        )
        .label("wrong number of arguments"),
    );
}

pub fn builtin_arg_type(checker: &mut TypeChecker, callee: &str, detail: impl Into<String>, sp: Span) {
    wrong_arg_type(checker, callee, detail, sp);
}

pub fn string_op_invalid(checker: &mut TypeChecker, op: &str, sp: Span) {
    operator_mismatch(
        checker,
        op,
        format!("`{op}` is not valid on `string` values (only `+` for concatenation)"),
        sp,
    );
}

pub fn fn_ptr_wrong_arity(
    checker: &mut TypeChecker,
    name: &str,
    expected: usize,
    got: usize,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E007_WRONG_ARITY,
            ErrorKind::Type,
            sp,
            format!("function pointer `{name}` expects {expected} arguments, found {got}"),
        ),
    );
}

pub fn fn_ptr_arg_mismatch(checker: &mut TypeChecker, name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!("argument type mismatch calling function pointer `{name}`"),
        ),
    );
}

pub fn tuple_missing_index(checker: &mut TypeChecker, idx: usize, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            format!("tuple has no field index {idx}"),
        ),
    );
}

pub fn tuple_index_not_number(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "tuple field index must be a number",
        ),
    );
}

pub fn field_access_invalid_receiver(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E009_INVALID_ASSIGN_TARGET,
            ErrorKind::Type,
            sp,
            "field access requires struct or tuple value",
        ),
    );
}

pub fn struct_spread_requires_struct(
    checker: &mut TypeChecker,
    struct_name: &str,
    got: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            format!(
                "struct spread `..expr` on `{struct_name}` requires a struct value, got {}",
                type_pretty(got),
            ),
        ),
    );
}

pub fn struct_field_spread_mismatch(
    checker: &mut TypeChecker,
    field: &str,
    src: &str,
    dst: &str,
    got: &Type,
    expected: &Type,
    sp: Span,
) {
    type_mismatch(
        checker,
        &format!("for field `{field}` spread from `{src}` onto `{dst}`"),
        expected,
        got,
        sp,
    );
}

pub fn struct_field_not_set(checker: &mut TypeChecker, field: &str, struct_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E014_UNKNOWN_FIELD,
            ErrorKind::Type,
            sp,
            format!("field `{field}` on `{struct_name}` not set explicitly or via struct spread"),
        )
        .help(format!("set `{field}: ...` in the `{struct_name}` literal")),
    );
}

pub fn duplicate_struct_field(checker: &mut TypeChecker, field: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E014_UNKNOWN_FIELD,
            ErrorKind::Type,
            sp,
            format!("duplicate field `{field}` in struct literal"),
        ),
    );
}

pub fn union_construct_requires_unsafe(checker: &mut TypeChecker, name: &str, sp: Span) {
    unsafe_required(checker, &format!("constructing union `{name}`"), sp);
}

pub fn array_index_must_be_i32(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "array index must be `i32`",
        ),
    );
}

pub fn index_requires_array_or_bytes(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "index requires array or bytes value",
        ),
    );
}

pub fn array_homogeneous_elements(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "array elements must have the same type",
        ),
    );
}

pub fn array_spread_homogeneous(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "array spread elements must have the same type",
        ),
    );
}

pub fn array_spread_invalid_source(checker: &mut TypeChecker, got: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            format!(
                "array spread `...expr` requires an array or struct value, got {}",
                type_pretty(got),
            ),
        ),
    );
}

pub fn enum_variant_wrong_arity(
    checker: &mut TypeChecker,
    enum_name: &str,
    variant: &str,
    expected: usize,
    got: usize,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E032_ENUM,
            ErrorKind::Type,
            sp,
            format!("variant `{enum_name}.{variant}` expects {expected} arguments, found {got}"),
        ),
    );
}

pub fn enum_variant_payload_mismatch(
    checker: &mut TypeChecker,
    enum_name: &str,
    variant: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E032_ENUM,
            ErrorKind::Type,
            sp,
            format!("variant payload type mismatch for `{enum_name}.{variant}`"),
        ),
    );
}

pub fn unsupported_method_on_type(
    checker: &mut TypeChecker,
    method: &str,
    ty: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E018_UNKNOWN_METHOD,
            ErrorKind::Type,
            sp,
            format!("type `{}` does not support `.{method}()`", type_pretty(ty)),
        ),
    );
}

pub fn method_expects_no_args(checker: &mut TypeChecker, method: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E007_WRONG_ARITY,
            ErrorKind::Type,
            sp,
            format!("`.{method}()` expects no arguments"),
        ),
    );
}

pub fn trait_method_not_found(
    checker: &mut TypeChecker,
    trait_name: &str,
    method: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E018_UNKNOWN_METHOD,
            ErrorKind::Type,
            sp,
            format!("trait `{trait_name}` has no method `{method}`"),
        ),
    );
}

pub fn method_receiver_requires_struct(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E018_UNKNOWN_METHOD,
            ErrorKind::Type,
            sp,
            "method call requires struct receiver",
        ),
    );
}

pub fn method_arg_mismatch(checker: &mut TypeChecker, method: &str, sp: Span) {
    wrong_arg_type(checker, method, format!("method `{method}` argument mismatch"), sp);
}

pub fn unary_requires_i32(checker: &mut TypeChecker, sp: Span) {
    operator_mismatch(checker, "unary `-`", "`unary -` requires `i32` operand", sp);
}

pub fn unary_requires_bool(checker: &mut TypeChecker, sp: Span) {
    operator_mismatch(checker, "unary `!`", "`unary !` requires `bool` operand", sp);
}

pub fn deref_requires_ref_or_ptr(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E009_INVALID_ASSIGN_TARGET,
            ErrorKind::Type,
            sp,
            "deref requires reference or raw pointer",
        ),
    );
}

pub fn await_wrong_type(checker: &mut TypeChecker, got: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E033_CAST,
            ErrorKind::Type,
            sp,
            format!(
                "`await` expects `Future` handle (`i32`) or `Future<T>`, got {}",
                type_pretty(got),
            ),
        ),
    );
}

pub fn template_interpolation_invalid(checker: &mut TypeChecker, ty: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!(
                "template interpolation must be string, i32, f32, f64, char, or bool — found {}",
                type_pretty(ty),
            ),
        ),
    );
}

pub fn invalid_cast(checker: &mut TypeChecker, from: &Type, to: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E033_CAST,
            ErrorKind::Type,
            sp,
            format!(
                "invalid cast from `{}` to `{}`",
                type_pretty(from),
                type_pretty(to),
            ),
        ),
    );
}

pub fn trait_not_implemented(
    checker: &mut TypeChecker,
    concrete: &str,
    trait_name: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E033_CAST,
            ErrorKind::Type,
            sp,
            format!("type `{concrete}` does not implement trait `{trait_name}`"),
        )
        .help(format!("add `impl {trait_name} for {concrete}`")),
    );
}

pub fn trait_object_cast_requires_struct(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E033_CAST,
            ErrorKind::Type,
            sp,
            "trait object cast requires a concrete struct value",
        ),
    );
}

pub fn send_bound_required(
    checker: &mut TypeChecker,
    concrete: &str,
    trait_name: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E036_SEND_SYNC,
            ErrorKind::Type,
            sp,
            format!("type `{concrete}` is not `Send`; cannot cast to `dyn {trait_name} + Send`"),
        )
        .note("raw pointers and types with non-Send fields cannot cross thread boundaries"),
    );
}

pub fn sync_bound_required(
    checker: &mut TypeChecker,
    concrete: &str,
    trait_name: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E036_SEND_SYNC,
            ErrorKind::Type,
            sp,
            format!("type `{concrete}` is not `Sync`; cannot cast to `dyn {trait_name} + Sync`"),
        )
        .note("shared references across threads require all fields to be Sync"),
    );
}

pub fn comptime_must_produce_value(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E026_BLOCK_VALUE,
            ErrorKind::ConstEval,
            sp,
            "comptime block must produce a value at compile time",
        ),
    );
}

pub fn io_arg_not_printable(checker: &mut TypeChecker, callee: &str, ty: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!(
                "`{callee}` argument must be a printable scalar or fixed array — found {}",
                type_pretty(ty),
            ),
        ),
    );
}

pub fn print_color_must_be_string(checker: &mut TypeChecker, ty: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!(
                "`print` color must be a string or color name — found {}",
                type_pretty(ty),
            ),
        ),
    );
}

pub fn array_method_requires_fixed(checker: &mut TypeChecker, method: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            format!("`.{method}()` requires a fixed-size array"),
        ),
    );
}

pub fn array_sort_unsupported_elem(checker: &mut TypeChecker, elem: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            format!(
                "`.sort()` on arrays supports `i32` and `f64` elements — found {}",
                type_pretty(elem),
            ),
        ),
    );
}

pub fn sort_by_wrong_arity(checker: &mut TypeChecker, got: usize, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E007_WRONG_ARITY,
            ErrorKind::Type,
            sp,
            format!("`.sort_by()` comparator expects 2 parameters, found {got}"),
        ),
    );
}

pub fn sort_by_param_mismatch(checker: &mut TypeChecker, elem: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!(
                "`.sort_by()` comparator must be `fn({}, {}) -> i32`",
                type_pretty(elem),
                type_pretty(elem),
            ),
        ),
    );
}

pub fn sort_by_return_mismatch(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E022_RETURN_MISMATCH,
            ErrorKind::Type,
            sp,
            "`.sort_by()` comparator must return `i32` (`<0`, `0`, or `>0`)",
        ),
    );
}

pub fn sort_by_expects_fn(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            "`.sort_by()` expects a comparator `fn(element, element) -> i32`",
        ),
    );
}

pub fn unknown_dyn_auto_trait(
    checker: &mut TypeChecker,
    bound: &str,
    trait_name: &str,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E033_CAST,
            ErrorKind::Type,
            sp,
            format!(
                "unknown auto trait bound `{bound}` on `dyn {trait_name}` (supported: `Send`, `Sync`)"
            ),
        ),
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
        Type::JoinHandle => "JoinHandle".into(),
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

const ABI_POLICY: &str = "See docs/abi-policy.md for allowed FFI boundary types.";
const ABI_ALLOWED_MSG: &str =
    "Allowed: i8–i128, u8–u128, isize, usize, f32, f64, bool, string, ptr, void, enum tags, [T; N], tuples, repr(C) structs, fn callbacks, and generic type params on export templates.";

pub fn const_type_mismatch(
    checker: &mut TypeChecker,
    name: &str,
    expected: &Type,
    got: &Type,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E003_TYPE_MISMATCH,
            ErrorKind::Type,
            sp,
            format!(
                "const `{name}` type mismatch: expected {}, got {}",
                type_pretty(expected),
                type_pretty(got),
            ),
        ),
    );
}

pub fn function_incompatible_returns(checker: &mut TypeChecker, name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E022_RETURN_MISMATCH,
            ErrorKind::Type,
            sp,
            format!(
                "function `{name}` has incompatible return types; add an explicit return type",
            ),
        )
        .note("example: `fn run() -> i32 { return 1 }`"),
    );
}

pub fn function_missing_return(checker: &mut TypeChecker, name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E022_RETURN_MISMATCH,
            ErrorKind::Type,
            sp,
            format!(
                "function `{name}` is missing a return value; add `return` or declare `-> void`",
            ),
        )
        .note("example: `fn run() -> void { print(1) }`"),
    );
}

pub fn stack_buffer_return_forbidden(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E022_RETURN_MISMATCH,
            ErrorKind::Type,
            sp,
            "references to `StackBuffer` cannot be returned from functions (stack-only allocation)",
        ),
    );
}

pub fn layout_intrinsic_no_args(checker: &mut TypeChecker, callee: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!("`{callee}` takes no runtime arguments (use type parameter)"),
        ),
    );
}

pub fn layout_intrinsic_requires_type_arg(checker: &mut TypeChecker, callee: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E004_CANNOT_INFER,
            ErrorKind::Type,
            sp,
            format!("`{callee}` requires a type argument (e.g. `{callee}<i32>()`)"),
        ),
    );
}

pub fn string_method_arg_must_be_string(checker: &mut TypeChecker, method: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!("`.{method}()` argument must be `string`"),
        ),
    );
}

pub fn string_replacen_count_must_be_i32(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            "`.replacen()` count argument must be `i32`",
        ),
    );
}

pub fn object_spread_requires_struct(checker: &mut TypeChecker, got: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            format!(
                "object spread `...expr` requires a struct value, got {}",
                type_pretty(got),
            ),
        ),
    );
}

pub fn duplicate_object_field(checker: &mut TypeChecker, field: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E014_UNKNOWN_FIELD,
            ErrorKind::Type,
            sp,
            format!("duplicate field `{field}` in object literal"),
        ),
    );
}

pub fn object_field_cannot_infer(checker: &mut TypeChecker, field: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E004_CANNOT_INFER,
            ErrorKind::Type,
            sp,
            format!(
                "cannot infer type of field `{field}` in object literal; add a struct declaration or type annotation",
            ),
        ),
    );
}

pub fn object_field_override_mismatch(
    checker: &mut TypeChecker,
    field: &str,
    got: &Type,
    expected: &Type,
    sp: Span,
) {
    type_mismatch(
        checker,
        &format!("for field `{field}` override from spread"),
        expected,
        got,
        sp,
    );
}

pub fn object_literal_empty(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "object literal must have at least one field or spread",
        ),
    );
}

pub fn bytes_index_requires_bytes(checker: &mut TypeChecker, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E031_ARRAY,
            ErrorKind::Type,
            sp,
            "indexing requires `bytes` value",
        ),
    );
}

pub fn ffi_export_inst_unknown_fn(checker: &mut TypeChecker, inst_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!("export inst `{inst_name}` refers to unknown function"),
        ),
    );
}

pub fn ffi_export_inst_requires_exported(checker: &mut TypeChecker, inst_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!("export inst `{inst_name}` requires an exported function"),
        ),
    );
}

pub fn ffi_export_inst_generic_only(checker: &mut TypeChecker, inst_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!("export inst `{inst_name}` is only valid for generic exported functions"),
        ),
    );
}

pub fn ffi_export_inst_wrong_type_args(
    checker: &mut TypeChecker,
    inst_name: &str,
    expected: usize,
    got: usize,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!(
                "export inst `{inst_name}` expects {expected} type argument(s), found {got}",
            ),
        ),
    );
}

pub fn ffi_generic_export_needs_inst(checker: &mut TypeChecker, fn_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!(
                "export fn `{fn_name}` is generic; add at least one `export inst {fn_name}<...>` for the FFI boundary",
            ),
        )
        .note("generic export templates are not linkable until monomorphized with `export inst`"),
    );
}

pub fn ffi_export_has_lifetime_params(checker: &mut TypeChecker, fn_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!("export fn `{fn_name}` cannot have lifetime parameters"),
        )
        .note(ABI_POLICY),
    );
}

pub fn ffi_export_async_and_generic(checker: &mut TypeChecker, fn_name: &str, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!("export fn `{fn_name}` cannot be both async and generic"),
        )
        .note("monomorph with `export inst name<i32>` and call the sync instance from the host"),
    );
}

pub fn ffi_export_async_return_invalid(
    checker: &mut TypeChecker,
    fn_name: &str,
    ret: &TypeAnnotation,
    sp: Span,
) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!(
                "export async fn `{fn_name}` must return `i32` or `void` at the FFI boundary (got `{ret:?}`)",
            ),
        )
        .note("async exports return an i32 promise handle; the host completes/awaits via async_poll / await"),
    );
}

pub fn ffi_type_not_allowed(checker: &mut TypeChecker, context: &str, ann: &TypeAnnotation, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E034_FFI,
            ErrorKind::Type,
            sp,
            format!(
                "{context} uses type `{ann:?}` which is not allowed at the FFI boundary",
            ),
        )
        .note(format!("{ABI_ALLOWED_MSG} {ABI_POLICY}")),
    );
}

pub fn print_template_interpolation_invalid(checker: &mut TypeChecker, ty: &Type, sp: Span) {
    checker.errors.push(
        NyraError::coded(
            E008_WRONG_ARG_TYPE,
            ErrorKind::Type,
            sp,
            format!(
                "template interpolation must be a printable scalar or fixed array — found {}",
                type_pretty(ty),
            ),
        ),
    );
}
