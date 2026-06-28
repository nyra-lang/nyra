//! Compile-time evaluation for `comptime` modules and `#[comptime]` functions.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::ControlFlow;

use ast::{Block, Expression, ForKind, Function, MatchPayloadPattern, MatchPattern, Program, Statement, UnaryOp};
use errors::{ErrorKind, NyraError, Span};

use crate::{const_value_to_expr, const_value_to_expr_typed, eval_const_expr, ConstValue};

const MAX_COMPTIME_STEPS: usize = 4_194_304;
const MAX_COMPTIME_DEPTH: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopAction {
    Proceed,
    BreakLoop,
    ContinueLoop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockMode {
    Function,
    Statement,
    ValueBlock,
}

type BlockEval = ControlFlow<ConstValue, LoopAction>;

/// Nyra `i32` arithmetic wraps; comptime stores ints as `i64` but must match runtime width.
fn comptime_wrap_i32(n: i64) -> i64 {
    n as i32 as i64
}

/// Variable bindings and mutability for comptime interpretation.
#[derive(Clone, Debug)]
struct ComptimeFrame {
    values: HashMap<String, ConstValue>,
    mutables: HashSet<String>,
}

impl ComptimeFrame {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            mutables: HashSet::new(),
        }
    }

    fn from_env(env: &HashMap<String, ConstValue>) -> Self {
        Self {
            values: env.clone(),
            mutables: HashSet::new(),
        }
    }
}

fn root_binding_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable { name, .. } => Some(name.clone()),
        Expression::Index(ix) => root_binding_name(&ix.object),
        Expression::FieldAccess(f) => root_binding_name(&f.object),
        _ => None,
    }
}

fn comptime_len(value: &ConstValue, span: Span) -> Result<ConstValue, NyraError> {
    let n = match value {
        ConstValue::Array(a) => a.len(),
        ConstValue::String(s) => s.len(),
        _ => {
            return Err(comptime_error(
                span,
                "comptime `.len()` requires an array or string",
            ));
        }
    };
    Ok(ConstValue::Int(n as i64))
}

fn comptime_assign(
    target: &Expression,
    value: ConstValue,
    frame: &mut ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
    span: Span,
) -> Result<(), NyraError> {
    match target {
        Expression::Variable { name, span: var_span } => {
            if !frame.values.contains_key(name) {
                return Err(comptime_error(
                    var_span.clone(),
                    format!("cannot assign unknown comptime variable `{name}`"),
                ));
            }
            if !frame.mutables.contains(name) {
                return Err(comptime_error(
                    var_span.clone(),
                    format!("cannot assign to immutable `{name}` in comptime (use `let mut`)"),
                ));
            }
            frame.values.insert(name.clone(), value);
            Ok(())
        }
        Expression::Index(ix) => {
            let root = root_binding_name(target).ok_or_else(|| {
                comptime_error(
                    span.clone(),
                    "comptime index assignment requires a mutable array variable",
                )
            })?;
            if !frame.mutables.contains(&root) {
                return Err(comptime_error(
                    span.clone(),
                    format!("cannot mutate `{root}` in comptime (use `let mut`)"),
                ));
            }
            let idx_val = eval_comptime_expr(&ix.index, frame, functions, depth)?;
            let idx = match idx_val {
                ConstValue::Int(i) if i >= 0 => i as usize,
                _ => {
                    return Err(comptime_error(
                        ix.span.clone(),
                        "comptime array index must be a non-negative integer",
                    ));
                }
            };
            let mut arr = match frame.values.get(&root) {
                Some(ConstValue::Array(items)) => items.clone(),
                Some(_) => {
                    return Err(comptime_error(
                        ix.span.clone(),
                        "comptime index assignment requires an array",
                    ));
                }
                None => {
                    return Err(comptime_error(
                        ix.span.clone(),
                        format!("unknown comptime variable `{root}`"),
                    ));
                }
            };
            if idx >= arr.len() {
                return Err(comptime_error(
                    ix.span.clone(),
                    "comptime array index out of bounds",
                ));
            }
            arr[idx] = value;
            frame.values.insert(root, ConstValue::Array(arr));
            Ok(())
        }
        Expression::FieldAccess(fa) => {
            let root = root_binding_name(target).ok_or_else(|| {
                comptime_error(
                    span.clone(),
                    "comptime field assignment requires a mutable struct variable",
                )
            })?;
            if !frame.mutables.contains(&root) {
                return Err(comptime_error(
                    span.clone(),
                    format!("cannot mutate `{root}` in comptime (use `let mut`)"),
                ));
            }
            let current = frame.values.get(&root).cloned().ok_or_else(|| {
                comptime_error(
                    fa.span.clone(),
                    format!("unknown comptime variable `{root}`"),
                )
            })?;
            let ConstValue::Struct { name, mut fields } = current else {
                return Err(comptime_error(
                    fa.span.clone(),
                    "comptime field assignment requires a struct",
                ));
            };
            fields.insert(fa.field.clone(), value);
            frame.values.insert(root, ConstValue::Struct { name, fields });
            Ok(())
        }
        _ => Err(comptime_error(
            span,
            "unsupported comptime assignment target",
        )),
    }
}

pub fn finalize_comptime_module(program: &mut Program) -> Vec<NyraError> {
    if !program.comptime {
        return vec![];
    }
    let mut errors = validate_comptime_module(program);
    if !errors.is_empty() {
        return errors;
    }
    errors.extend(monomorph::monomorphize_program(program));
    if !errors.is_empty() {
        return errors;
    }
    let functions: HashMap<String, Function> = program
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    let mut frame = ComptimeFrame::new();
    for c in &mut program.consts {
        match eval_comptime_expr(&c.value, &frame, &functions, 0) {
            Ok(v) => {
                frame.values.insert(c.name.clone(), v.clone());
                c.value = const_value_to_expr_typed(&v, c.ty.as_ref());
            }
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        strip_comptime_artifacts(program);
    }
    errors
}

pub fn strip_comptime_artifacts(program: &mut Program) {
    program.functions.clear();
    program.externs.clear();
    program.impls.clear();
    program.trait_impls.clear();
    program.macros.clear();
    program.export_instances.clear();
    program.consts.retain(|c| c.public);
    program.structs.retain(|s| s.public);
    program.enums.retain(|e| e.public);
}

/// Fold `#[comptime]` calls and `comptime { }` blocks in normal (non-`comptime` file) programs.
/// Comptime-marked functions are removed from the program unless still referenced at runtime.
pub fn fold_attributed_comptime_functions(program: &mut Program) -> Vec<NyraError> {
    if program.comptime {
        return vec![];
    }

    let comptime_fns: HashMap<String, Function> = program
        .functions
        .iter()
        .filter(|f| f.comptime)
        .map(|f| (f.name.clone(), f.clone()))
        .collect();

    let mut errors = Vec::new();
    if !comptime_fns.is_empty() {
        for f in comptime_fns.values() {
            if f.name == "main" {
                errors.push(comptime_error(
                    f.span.clone(),
                    "`#[comptime]` cannot be applied to `main`",
                ));
                continue;
            }
            errors.extend(validate_comptime_function(f));
        }
        if !errors.is_empty() {
            return errors;
        }
    }

    fold_comptime_exprs_in_program(program, &comptime_fns);

    if comptime_fns.is_empty() {
        return errors;
    }

    let comptime_names: std::collections::HashSet<String> =
        comptime_fns.keys().cloned().collect();

    for f in &program.functions {
        if f.comptime {
            continue;
        }
        check_runtime_comptime_calls_in_block(&f.body, &comptime_names, &mut errors);
    }
    for imp in &program.impls {
        for m in &imp.methods {
            if m.comptime {
                continue;
            }
            check_runtime_comptime_calls_in_block(&m.body, &comptime_names, &mut errors);
        }
    }

    if errors.is_empty() {
        program.functions.retain(|f| !f.comptime);
    }

    errors
}

fn fold_comptime_exprs_in_program(
    program: &mut Program,
    comptime_fns: &HashMap<String, Function>,
) {
    let _ = comptime_fns;
    let all_fns: HashMap<String, Function> = program
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.clone()))
        .collect();

    let mut module_consts = HashMap::new();
    for c in &program.consts {
        if let Some(v) = eval_const_expr(&c.value, &module_consts) {
            module_consts.insert(c.name.clone(), v);
        }
    }

    for c in &mut program.consts {
        fold_comptime_in_expr(&mut c.value, &all_fns, &module_consts);
        if let Ok(v) = eval_comptime_expr(&c.value, &ComptimeFrame::from_env(&module_consts), &all_fns, 0) {
            module_consts.insert(c.name.clone(), v.clone());
            c.value = const_value_to_expr_typed(&v, c.ty.as_ref());
        } else if let Some(v) = eval_const_expr(&c.value, &module_consts) {
            module_consts.insert(c.name.clone(), v.clone());
            c.value = const_value_to_expr_typed(&v, c.ty.as_ref());
        }
    }

    for f in &mut program.functions {
        let mut env = module_consts.clone();
        fold_comptime_in_block(&mut f.body, &all_fns, &mut env);
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            let mut env = module_consts.clone();
            fold_comptime_in_block(&mut m.body, &all_fns, &mut env);
        }
    }
}

fn validate_comptime_module(program: &Program) -> Vec<NyraError> {
    let mut errors = Vec::new();
    if !program.externs.is_empty() {
        errors.push(comptime_error(
            Span::default(),
            "comptime modules cannot declare `extern` functions",
        ));
    }
    for f in &program.functions {
        if f.name == "main" {
            errors.push(comptime_error(
                f.span.clone(),
                "comptime modules cannot define `main`",
            )
            .help("use a regular `.ny` entry file and `import` this comptime module"));
        }
        errors.extend(validate_comptime_function(f));
    }
    errors
}

fn validate_comptime_function(f: &Function) -> Vec<NyraError> {
    let mut errors = Vec::new();
    if f.is_async {
        errors.push(comptime_error(
            f.span.clone(),
            format!("comptime function `{}` cannot be `async`", f.name),
        ));
    }
    if f.is_test {
        errors.push(comptime_error(
            f.span.clone(),
            format!("comptime function `{}` cannot be a `test`", f.name),
        ));
    }
    if f.exported {
        errors.push(comptime_error(
            f.span.clone(),
            format!("comptime function `{}` cannot be `export fn`", f.name),
        )
        .help("export `pub const` values instead"));
    }
    walk_block_forbidden(&f.body, &f.name, &mut errors);
    errors
}

fn comptime_error(span: Span, message: impl Into<String>) -> NyraError {
    NyraError::new(ErrorKind::ConstEval, span, message)
}

fn walk_block_forbidden(block: &Block, fn_name: &str, errors: &mut Vec<NyraError>) {
    for stmt in &block.statements {
        walk_stmt_forbidden(stmt, fn_name, errors);
    }
}

fn walk_stmt_forbidden(stmt: &Statement, fn_name: &str, errors: &mut Vec<NyraError>) {
    match stmt {
        Statement::Print(p) => {
            errors.push(comptime_error(
                p.args.first().map(expr_span).unwrap_or_default(),
                format!("comptime function `{fn_name}` cannot use `print`"),
            ));
        }
        Statement::Spawn(b) => {
            errors.push(comptime_error(
                b.statements
                    .first()
                    .map(stmt_span)
                    .unwrap_or_default(),
                format!("comptime function `{fn_name}` cannot use `spawn`"),
            ));
        }
        Statement::Defer(e) => {
            errors.push(comptime_error(
                expr_span(e),
                format!("comptime function `{fn_name}` cannot use `defer`"),
            ));
        }
        Statement::Unsafe(b) => {
            errors.push(comptime_error(
                b.statements
                    .first()
                    .map(stmt_span)
                    .unwrap_or_default(),
                format!("comptime function `{fn_name}` cannot use `unsafe`"),
            ));
        }
        Statement::Asm { span, .. } => {
            errors.push(comptime_error(
                span.clone(),
                format!("comptime function `{fn_name}` cannot use inline `asm`"),
            ));
        }
        Statement::Benchmark(b) => walk_block_forbidden(b, fn_name, errors),
        Statement::Expression(e) => walk_expr_forbidden(e, fn_name, errors),
        Statement::If(i) => {
            walk_block_forbidden(&i.then_block, fn_name, errors);
            if let Some(el) = &i.else_block {
                walk_block_forbidden(el, fn_name, errors);
            }
        }
        Statement::While(w) => walk_block_forbidden(&w.body, fn_name, errors),
        Statement::For(f) => {
            if f.parallel.is_some() {
                errors.push(comptime_error(
                    stmt_span(stmt),
                    format!("comptime function `{fn_name}` cannot use `parallel for`"),
                ));
            }
            walk_block_forbidden(&f.body, fn_name, errors);
        }
        Statement::Break { .. } | Statement::Continue { .. } => {}
        _ => {}
    }
}

fn walk_expr_forbidden(expr: &Expression, fn_name: &str, errors: &mut Vec<NyraError>) {
    match expr {
        Expression::ComptimeBlock { body, .. } => walk_block_forbidden(body, fn_name, errors),
        Expression::Binary(b) => {
            walk_expr_forbidden(&b.left, fn_name, errors);
            walk_expr_forbidden(&b.right, fn_name, errors);
        }
        Expression::Unary(u) => walk_expr_forbidden(&u.operand, fn_name, errors),
        Expression::Call(c) => {
            for a in &c.args {
                walk_expr_forbidden(a, fn_name, errors);
            }
        }
        Expression::If(i) => {
            walk_expr_forbidden(&i.condition, fn_name, errors);
            walk_expr_forbidden(&i.then_expr, fn_name, errors);
            walk_expr_forbidden(&i.else_expr, fn_name, errors);
        }
        Expression::Grouped(g) => walk_expr_forbidden(g, fn_name, errors),
        Expression::Index(ix) => {
            walk_expr_forbidden(&ix.object, fn_name, errors);
            walk_expr_forbidden(&ix.index, fn_name, errors);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                walk_expr_forbidden(e, fn_name, errors);
            }
        }
        Expression::FieldAccess(f) => walk_expr_forbidden(&f.object, fn_name, errors),
        Expression::MethodCall(mc) => {
            walk_expr_forbidden(&mc.object, fn_name, errors);
            for a in &mc.args {
                walk_expr_forbidden(a, fn_name, errors);
            }
        }
        Expression::Match(m) => {
            walk_expr_forbidden(&m.scrutinee, fn_name, errors);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    walk_expr_forbidden(g, fn_name, errors);
                }
                walk_expr_forbidden(&arm.body, fn_name, errors);
            }
        }
        Expression::Cast(c) => walk_expr_forbidden(&c.expr, fn_name, errors),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                walk_expr_forbidden(e, fn_name, errors);
            }
        }
        Expression::Await(e) => walk_expr_forbidden(e, fn_name, errors),
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let ast::TemplatePart::Interpolation(e) = part {
                    walk_expr_forbidden(e, fn_name, errors);
                }
            }
        }
        Expression::ArrowFn(a) => match &a.body {
            ast::ArrowBody::Expr(e) => walk_expr_forbidden(e, fn_name, errors),
            ast::ArrowBody::Block(b) => walk_block_forbidden(b, fn_name, errors),
        },
        Expression::StructLiteral(s) => {
            for (_, e) in &s.fields {
                walk_expr_forbidden(e, fn_name, errors);
            }
            for e in &s.spreads {
                walk_expr_forbidden(e, fn_name, errors);
            }
        }
        Expression::ArrayRepeat { element, count_expr, .. } => {
            walk_expr_forbidden(element, fn_name, errors);
            if let Some(ce) = count_expr {
                walk_expr_forbidden(ce, fn_name, errors);
            }
        }
        Expression::Literal(_) | Expression::Variable { .. } | Expression::EnumVariant(_)
        | Expression::Invalid => {}
    }
}

fn loop_control_outside_loop(span: Span, kw: &str) -> NyraError {
    comptime_error(span, format!("`{kw}` outside loop in comptime evaluation"))
}

fn stmt_span(stmt: &Statement) -> Span {
    ast::stmt_span(stmt)
}

fn expr_span(expr: &Expression) -> Span {
    ast::expr_span(expr)
}

fn eval_comptime_expr(
    expr: &Expression,
    frame: &ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
) -> Result<ConstValue, NyraError> {
    if depth > MAX_COMPTIME_DEPTH {
        return Err(comptime_error(
            expr_span(expr),
            "comptime evaluation exceeded maximum call depth",
        ));
    }
    if let Some(v) = eval_const_expr(expr, &frame.values) {
        return Ok(v);
    }
    match expr {
        Expression::Variable { name, span } => frame.values.get(name).cloned().ok_or_else(|| {
            comptime_error(span.clone(), format!("unknown comptime variable `{name}`"))
        }),
        Expression::Binary(b) => {
            let l = eval_comptime_expr(&b.left, frame, functions, depth)?;
            let r = eval_comptime_expr(&b.right, frame, functions, depth)?;
            apply_binary(b.op, l, r).ok_or_else(|| {
                comptime_error(
                    b.span.clone(),
                    "invalid comptime binary operation",
                )
            })
        }
        Expression::Unary(u) => {
            let v = eval_comptime_expr(&u.operand, frame, functions, depth)?;
            apply_unary(u.op, v).ok_or_else(|| {
                comptime_error(u.span.clone(), "invalid comptime unary operation")
            })
        }
        Expression::Call(c) => {
            let f = lookup_comptime_function(c, functions)?;
            let mut args = Vec::with_capacity(c.args.len());
            for arg in &c.args {
                args.push(eval_comptime_expr(arg, frame, functions, depth)?);
            }
            eval_comptime_function(f, &args, functions, depth + 1, frame)
        }
        Expression::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() && !mc.optional => {
            let obj = eval_comptime_expr(&mc.object, frame, functions, depth)?;
            comptime_len(&obj, mc.span.clone())
        }
        Expression::ArrayLiteral(al) => {
            let mut out = Vec::new();
            for spread in &al.spreads {
                match eval_comptime_expr(spread, frame, functions, depth)? {
                    ConstValue::Array(part) => out.extend(part),
                    _ => {
                        return Err(comptime_error(
                            al.span.clone(),
                            "comptime array spread requires an array value",
                        ));
                    }
                }
            }
            for e in &al.elems {
                out.push(eval_comptime_expr(e, frame, functions, depth)?);
            }
            Ok(ConstValue::Array(out))
        }
        Expression::ArrayRepeat {
            element,
            count,
            count_from,
            count_expr,
            span,
            ..
        } => {
            let elem = eval_comptime_expr(element, frame, functions, depth)?;
            let n = if let Some(ce) = count_expr {
                match eval_comptime_expr(ce, frame, functions, depth)? {
                    ConstValue::Int(n) if n >= 0 => n as usize,
                    _ => {
                        return Err(comptime_error(
                            span.clone(),
                            "comptime array repeat count must be a non-negative integer",
                        ));
                    }
                }
            } else if let Some(name) = count_from {
                match frame.values.get(name) {
                    Some(ConstValue::Int(n)) if *n >= 0 => *n as usize,
                    _ => {
                        return Err(comptime_error(
                            span.clone(),
                            format!("comptime array repeat count `{name}` must be a non-negative integer"),
                        ));
                    }
                }
            } else {
                *count
            };
            if n > 65_536 {
                return Err(comptime_error(
                    span.clone(),
                    "comptime array repeat count must be at most 65536",
                ));
            }
            Ok(ConstValue::Array(vec![elem; n]))
        }
        Expression::Index(ix) => {
            let obj = eval_comptime_expr(&ix.object, frame, functions, depth)?;
            let idx = eval_comptime_expr(&ix.index, frame, functions, depth)?;
            match (obj, idx) {
                (ConstValue::Array(items), ConstValue::Int(i))
                    if i >= 0 && (i as usize) < items.len() =>
                {
                    Ok(items[i as usize].clone())
                }
                (ConstValue::Array(_), ConstValue::Int(_)) => Err(comptime_error(
                    ix.span.clone(),
                    "comptime array index out of bounds",
                )),
                _ => Err(comptime_error(
                    ix.span.clone(),
                    "comptime index requires a fixed array and integer index",
                )),
            }
        }
        Expression::If(i) => {
            let cond = eval_comptime_expr(&i.condition, frame, functions, depth)?;
            match cond {
                ConstValue::Bool(true) => {
                    eval_comptime_expr(&i.then_expr, frame, functions, depth)
                }
                ConstValue::Bool(false) => {
                    eval_comptime_expr(&i.else_expr, frame, functions, depth)
                }
                _ => Err(comptime_error(
                    i.span.clone(),
                    "comptime if condition must be bool",
                )),
            }
        }
        Expression::Grouped(inner) => eval_comptime_expr(inner, frame, functions, depth),
        Expression::ComptimeBlock { body, span } => {
            let mut block_frame = frame.clone();
            eval_comptime_value_block(&body, &mut block_frame, functions, depth, span.clone())
        }
        Expression::EnumVariant(ev) => {
            let enum_name = ev.enum_name.clone().ok_or_else(|| {
                comptime_error(
                    ev.span.clone(),
                    "comptime enum variant requires a type prefix (e.g. `Color.Red`)",
                )
            })?;
            let payloads: Result<Vec<ConstValue>, _> = ev
                .args
                .iter()
                .map(|arg| eval_comptime_expr(arg, frame, functions, depth))
                .collect();
            let payloads = payloads?;
            let payload = match payloads.len() {
                0 => None,
                1 => Some(Box::new(payloads.into_iter().next().unwrap())),
                _ => Some(Box::new(ConstValue::Tuple(payloads))),
            };
            Ok(ConstValue::Enum {
                enum_name,
                variant: ev.variant.clone(),
                payload,
            })
        }
        Expression::StructLiteral(s) => eval_comptime_struct_literal(s, frame, functions, depth),
        Expression::TupleLiteral(elems) => {
            let mut out = Vec::with_capacity(elems.len());
            for e in elems {
                out.push(eval_comptime_expr(e, frame, functions, depth)?);
            }
            Ok(ConstValue::Tuple(out))
        }
        Expression::FieldAccess(f) => {
            let obj = eval_comptime_expr(&f.object, frame, functions, depth)?;
            comptime_field_access(&obj, &f.field, f.span.clone())
        }
        Expression::Match(m) => eval_comptime_match(m, frame, functions, depth),
        other => Err(comptime_error(
            expr_span(other),
            "expression is not evaluable at comptime",
        )
        .help("comptime supports integers, bools, strings, arrays, structs, enums, tuples, match, `.len()`, pure function calls, and `if` expressions")),
    }
}

fn eval_comptime_struct_literal(
    s: &ast::StructLiteralExpr,
    frame: &ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
) -> Result<ConstValue, NyraError> {
    let mut fields = BTreeMap::new();
    for spread in &s.spreads {
        match eval_comptime_expr(spread, frame, functions, depth)? {
            ConstValue::Struct { fields: spread_fields, .. } => {
                fields.extend(spread_fields);
            }
            _ => {
                return Err(comptime_error(
                    s.span.clone(),
                    "comptime struct spread requires a struct value",
                ));
            }
        }
    }
    for (name, expr) in &s.fields {
        fields.insert(
            name.clone(),
            eval_comptime_expr(expr, frame, functions, depth)?,
        );
    }
    Ok(ConstValue::Struct {
        name: s.name.clone(),
        fields,
    })
}

fn comptime_field_access(
    obj: &ConstValue,
    field: &str,
    span: Span,
) -> Result<ConstValue, NyraError> {
    match obj {
        ConstValue::Struct { fields, .. } => fields.get(field).cloned().ok_or_else(|| {
            comptime_error(
                span,
                format!("comptime struct has no field `{field}`"),
            )
        }),
        _ => Err(comptime_error(
            span,
            "comptime field access requires a struct value",
        )),
    }
}

fn eval_comptime_match(
    m: &ast::MatchExpr,
    frame: &ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
) -> Result<ConstValue, NyraError> {
    let scrutinee = eval_comptime_expr(&m.scrutinee, frame, functions, depth)?;
    for arm in &m.arms {
        let mut arm_frame = frame.clone();
        if !comptime_pattern_matches(
            &arm.pattern,
            &scrutinee,
            &mut arm_frame.values,
            m.span.clone(),
        )? {
            continue;
        }
        if let Some(guard) = &arm.guard {
            match eval_comptime_expr(guard, &arm_frame, functions, depth)? {
                ConstValue::Bool(true) => {}
                ConstValue::Bool(false) => continue,
                _ => {
                    return Err(comptime_error(
                        m.span.clone(),
                        "comptime match guard must be bool",
                    ));
                }
            }
        }
        return eval_comptime_expr(&arm.body, &arm_frame, functions, depth);
    }
    Err(comptime_error(
        m.span.clone(),
        "non-exhaustive comptime match",
    ))
}

fn comptime_pattern_matches(
    pattern: &MatchPattern,
    scrutinee: &ConstValue,
    env: &mut HashMap<String, ConstValue>,
    span: Span,
) -> Result<bool, NyraError> {
    match pattern {
        MatchPattern::Wildcard => Ok(true),
        MatchPattern::Or(_) => Err(comptime_error(
            span,
            "internal error: or-pattern should be desugared before comptime eval",
        )),
        MatchPattern::Literal(lit) => match scrutinee {
            ConstValue::String(s) => Ok(s == lit),
            _ => Ok(false),
        },
        MatchPattern::Struct(pat_name, field_pats) => {
            comptime_struct_pattern_matches(pat_name, field_pats, scrutinee, env, span)
        }
        MatchPattern::Tuple(binds) => {
            comptime_tuple_pattern_matches(binds, scrutinee, env, span)
        }
        MatchPattern::Variant(name) => comptime_variant_pattern_matches(name, scrutinee, env, None),
        MatchPattern::Qualified(en, v) => {
            comptime_enum_variant_matches(en, v, scrutinee, env, None, span)
        }
        MatchPattern::QualifiedBind(en, v, payload) => {
            comptime_enum_variant_matches(en, v, scrutinee, env, Some(payload), span)
        }
    }
}

fn comptime_variant_pattern_matches(
    name: &str,
    scrutinee: &ConstValue,
    env: &mut HashMap<String, ConstValue>,
    enum_name: Option<&str>,
) -> Result<bool, NyraError> {
    match scrutinee {
        ConstValue::Enum {
            enum_name: en,
            variant,
            payload: _,
        } => {
            if enum_name.is_some_and(|expected| expected != en) {
                return Ok(false);
            }
            Ok(variant == name)
        }
        ConstValue::Bool(b) => {
            let matches = (name == "true" && *b) || (name == "false" && !*b);
            if matches {
                env.insert(name.to_string(), scrutinee.clone());
            }
            Ok(matches)
        }
        ConstValue::Int(n) => {
            if let Ok(lit) = name.parse::<i64>() {
                Ok(lit == *n)
            } else {
                env.insert(name.to_string(), ConstValue::Int(*n));
                Ok(true)
            }
        }
        ConstValue::String(s) => {
            if name == "_" {
                Ok(false)
            } else {
                env.insert(name.to_string(), ConstValue::String(s.clone()));
                Ok(true)
            }
        }
        _ => Ok(false),
    }
}

fn comptime_struct_pattern_matches(
    pat_name: &str,
    field_pats: &[ast::StructMatchField],
    scrutinee: &ConstValue,
    env: &mut HashMap<String, ConstValue>,
    span: Span,
) -> Result<bool, NyraError> {
    let ConstValue::Struct { name, fields } = scrutinee else {
        return Ok(false);
    };
    if !pat_name.is_empty() && pat_name != name {
        return Ok(false);
    }
    for field_pat in field_pats {
        let bind = field_pat
            .bind
            .as_deref()
            .unwrap_or(field_pat.field.as_str());
        if bind == "_" {
            continue;
        }
        let Some(value) = fields.get(&field_pat.field) else {
            return Ok(false);
        };
        env.insert(bind.to_string(), value.clone());
    }
    let _ = span;
    Ok(true)
}

fn comptime_tuple_pattern_matches(
    binds: &[MatchPayloadPattern],
    scrutinee: &ConstValue,
    env: &mut HashMap<String, ConstValue>,
    span: Span,
) -> Result<bool, NyraError> {
    let ConstValue::Tuple(elems) = scrutinee else {
        return Ok(false);
    };
    if binds.len() != elems.len() {
        return Ok(false);
    }
    for (pat, value) in binds.iter().zip(elems.iter()) {
        comptime_bind_payload(pat, Some(value), env, span.clone())?;
    }
    Ok(true)
}

fn comptime_enum_variant_matches(
    enum_name: &str,
    variant: &str,
    scrutinee: &ConstValue,
    env: &mut HashMap<String, ConstValue>,
    payload_pat: Option<&MatchPayloadPattern>,
    span: Span,
) -> Result<bool, NyraError> {
    let ConstValue::Enum {
        enum_name: en,
        variant: v,
        payload,
    } = scrutinee
    else {
        return Ok(false);
    };
    if en != enum_name || v != variant {
        return Ok(false);
    }
    if let Some(pat) = payload_pat {
        comptime_bind_payload(pat, payload.as_deref(), env, span)?;
    }
    Ok(true)
}

fn comptime_bind_payload(
    pat: &MatchPayloadPattern,
    payload: Option<&ConstValue>,
    env: &mut HashMap<String, ConstValue>,
    span: Span,
) -> Result<(), NyraError> {
    match pat {
        MatchPayloadPattern::Wildcard => Ok(()),
        MatchPayloadPattern::Bind(name) => {
            let Some(value) = payload else {
                return Err(comptime_error(
                    span,
                    format!("expected payload for bind `{name}` in comptime match"),
                ));
            };
            env.insert(name.clone(), value.clone());
            Ok(())
        }
        MatchPayloadPattern::Nested(inner) => {
            let Some(value) = payload else {
                return Err(comptime_error(span.clone(), "expected nested payload in comptime match"));
            };
            comptime_pattern_matches(inner, value, env, span.clone()).and_then(|ok| {
                if ok {
                    Ok(())
                } else {
                    Err(comptime_error(span, "nested comptime match pattern mismatch"))
                }
            })
        }
    }
}

fn lookup_comptime_function<'a>(
    call: &ast::CallExpr,
    functions: &'a HashMap<String, Function>,
) -> Result<&'a Function, NyraError> {
    let name = if !call.type_args.is_empty() {
        monomorph::mangle_inst(&call.callee, &call.type_args)
    } else {
        call.callee.clone()
    };
    functions.get(&name).ok_or_else(|| {
        comptime_error(
            call.span.clone(),
            format!("unknown comptime function `{name}`"),
        )
    })
}

fn apply_binary(op: ast::BinaryOp, l: ConstValue, r: ConstValue) -> Option<ConstValue> {
    use ast::BinaryOp;
    match (op, l, r) {
        (BinaryOp::Add, ConstValue::Int(a), ConstValue::Int(b)) => {
            Some(ConstValue::Int(comptime_wrap_i32(a.wrapping_add(b))))
        }
        (BinaryOp::Add, ConstValue::String(a), ConstValue::String(b)) => {
            Some(ConstValue::String(format!("{a}{b}")))
        }
        (BinaryOp::Sub, ConstValue::Int(a), ConstValue::Int(b)) => {
            Some(ConstValue::Int(comptime_wrap_i32(a.wrapping_sub(b))))
        }
        (BinaryOp::Mul, ConstValue::Int(a), ConstValue::Int(b)) => {
            Some(ConstValue::Int(comptime_wrap_i32(a.wrapping_mul(b))))
        }
        (BinaryOp::Div, ConstValue::Int(a), ConstValue::Int(b)) if b != 0 => {
            Some(ConstValue::Int(comptime_wrap_i32(a / b)))
        }
        (BinaryOp::Mod, ConstValue::Int(a), ConstValue::Int(b)) if b != 0 => {
            Some(ConstValue::Int(comptime_wrap_i32(a % b)))
        }
        (BinaryOp::Eq, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Bool(a == b)),
        (BinaryOp::Eq, ConstValue::String(a), ConstValue::String(b)) => {
            Some(ConstValue::Bool(a == b))
        }
        (BinaryOp::Ne, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Bool(a != b)),
        (BinaryOp::Ne, ConstValue::String(a), ConstValue::String(b)) => {
            Some(ConstValue::Bool(a != b))
        }
        (BinaryOp::Lt, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Bool(a < b)),
        (BinaryOp::Gt, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Bool(a > b)),
        (BinaryOp::Le, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Bool(a <= b)),
        (BinaryOp::Ge, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Bool(a >= b)),
        (BinaryOp::And, ConstValue::Bool(a), ConstValue::Bool(b)) => Some(ConstValue::Bool(a && b)),
        (BinaryOp::Or, ConstValue::Bool(a), ConstValue::Bool(b)) => Some(ConstValue::Bool(a || b)),
        (BinaryOp::BitOr, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a | b)),
        (BinaryOp::BitAnd, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a & b)),
        (BinaryOp::BitXor, ConstValue::Int(a), ConstValue::Int(b)) => Some(ConstValue::Int(a ^ b)),
        (BinaryOp::Shl, ConstValue::Int(a), ConstValue::Int(b)) if (0..64).contains(&b) => {
            Some(ConstValue::Int(a.wrapping_shl(b as u32)))
        }
        (BinaryOp::Shr, ConstValue::Int(a), ConstValue::Int(b)) if (0..64).contains(&b) => {
            Some(ConstValue::Int(a.wrapping_shr(b as u32)))
        }
        _ => None,
    }
}

fn apply_unary(op: UnaryOp, v: ConstValue) -> Option<ConstValue> {
    match (op, v) {
        (UnaryOp::Neg, ConstValue::Int(n)) => Some(ConstValue::Int(comptime_wrap_i32(-n))),
        (UnaryOp::Not, ConstValue::Bool(b)) => Some(ConstValue::Bool(!b)),
        _ => None,
    }
}

fn eval_comptime_function(
    f: &Function,
    args: &[ConstValue],
    functions: &HashMap<String, Function>,
    depth: usize,
    globals: &ComptimeFrame,
) -> Result<ConstValue, NyraError> {
    if f.params.len() != args.len() {
        return Err(comptime_error(
            f.span.clone(),
            format!(
                "comptime function `{}` expected {} arguments, got {}",
                f.name,
                f.params.len(),
                args.len()
            ),
        ));
    }
    let mut frame = globals.clone();
    frame.mutables.clear();
    for (p, v) in f.params.iter().zip(args.iter()) {
        frame.values.insert(p.name.clone(), v.clone());
        if p.mutable {
            frame.mutables.insert(p.name.clone());
        }
    }
    eval_comptime_block(&f.body, &mut frame, functions, depth, true)
}

fn eval_comptime_block(
    block: &Block,
    frame: &mut ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
    must_return: bool,
) -> Result<ConstValue, NyraError> {
    let mut steps = 0usize;
    let mut last_value = None;
    match eval_comptime_block_inner(
        block,
        frame,
        functions,
        depth,
        &mut steps,
        BlockMode::Function,
        &mut last_value,
    )? {
        ControlFlow::Break(v) => Ok(v),
        ControlFlow::Continue(LoopAction::Proceed) if must_return => Err(comptime_error(
            Span::default(),
            "comptime function fell through without `return`",
        )),
        ControlFlow::Continue(LoopAction::Proceed) => Err(comptime_error(
            Span::default(),
            "comptime block missing value",
        )),
        ControlFlow::Continue(LoopAction::BreakLoop) => {
            Err(loop_control_outside_loop(Span::default(), "break"))
        }
        ControlFlow::Continue(LoopAction::ContinueLoop) => {
            Err(loop_control_outside_loop(Span::default(), "continue"))
        }
    }
}

fn eval_comptime_value_block(
    block: &Block,
    frame: &mut ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
    span: Span,
) -> Result<ConstValue, NyraError> {
    let mut steps = 0usize;
    let mut last_value = None;
    match eval_comptime_block_inner(
        block,
        frame,
        functions,
        depth,
        &mut steps,
        BlockMode::ValueBlock,
        &mut last_value,
    )? {
        ControlFlow::Break(v) => Ok(v),
        ControlFlow::Continue(LoopAction::Proceed) => last_value.ok_or_else(|| {
            comptime_error(
                span,
                "comptime block must produce a value (use a trailing expression or `return`)",
            )
        }),
        ControlFlow::Continue(LoopAction::BreakLoop) => {
            Err(loop_control_outside_loop(span, "break"))
        }
        ControlFlow::Continue(LoopAction::ContinueLoop) => {
            Err(loop_control_outside_loop(span, "continue"))
        }
    }
}

fn eval_comptime_block_inner(
    block: &Block,
    frame: &mut ComptimeFrame,
    functions: &HashMap<String, Function>,
    depth: usize,
    steps: &mut usize,
    mode: BlockMode,
    last_value: &mut Option<ConstValue>,
) -> Result<BlockEval, NyraError> {
    for stmt in &block.statements {
        *steps += 1;
        if *steps > MAX_COMPTIME_STEPS {
            return Err(comptime_error(
                stmt_span(stmt),
                "comptime evaluation exceeded step limit",
            ));
        }
        match stmt {
            Statement::Let(l) => {
                let v = eval_comptime_expr(&l.value, frame, functions, depth)?;
                frame.values.insert(l.name.clone(), v);
                if l.mutable {
                    frame.mutables.insert(l.name.clone());
                }
            }
            Statement::Const(l) => {
                let v = eval_comptime_expr(&l.value, frame, functions, depth)?;
                frame.values.insert(l.name.clone(), v);
            }
            Statement::Assign(a) => {
                let v = eval_comptime_expr(&a.value, frame, functions, depth)?;
                comptime_assign(&a.target, v, frame, functions, depth, a.span.clone())?;
            }
            Statement::Return(r) => {
                return match &r.value {
                    Some(v) => eval_comptime_expr(v, frame, functions, depth).map(ControlFlow::Break),
                    None => Err(comptime_error(
                        Span::default(),
                        "comptime return requires a value",
                    )),
                };
            }
            Statement::Break { .. } => {
                return Ok(ControlFlow::Continue(LoopAction::BreakLoop));
            }
            Statement::Continue { .. } => {
                return Ok(ControlFlow::Continue(LoopAction::ContinueLoop));
            }
            Statement::If(i) => {
                let cond = eval_comptime_expr(&i.condition, frame, functions, depth)?;
                let branch = match cond {
                    ConstValue::Bool(true) => Some(&i.then_block),
                    ConstValue::Bool(false) => i.else_block.as_ref(),
                    _ => {
                        return Err(comptime_error(
                            Span::default(),
                            "comptime if condition must be bool",
                        ));
                    }
                };
                if let Some(branch) = branch {
                    match eval_comptime_block_inner(
                        branch, frame, functions, depth, steps, mode, last_value,
                    )? {
                        ControlFlow::Break(v) => return Ok(ControlFlow::Break(v)),
                        ControlFlow::Continue(
                            action @ (LoopAction::BreakLoop | LoopAction::ContinueLoop),
                        ) => return Ok(ControlFlow::Continue(action)),
                        ControlFlow::Continue(LoopAction::Proceed) => {}
                    }
                }
            }
            Statement::While(w) => {
                loop {
                    let cond = eval_comptime_expr(&w.condition, frame, functions, depth)?;
                    match cond {
                        ConstValue::Bool(true) => {}
                        ConstValue::Bool(false) => break,
                        _ => {
                            return Err(comptime_error(
                                stmt_span(stmt),
                                "comptime while condition must be bool",
                            ));
                        }
                    }
                    match eval_comptime_block_inner(
                        &w.body,
                        frame,
                        functions,
                        depth,
                        steps,
                        BlockMode::Statement,
                        last_value,
                    )? {
                        ControlFlow::Break(v) => return Ok(ControlFlow::Break(v)),
                        ControlFlow::Continue(LoopAction::BreakLoop) => break,
                        ControlFlow::Continue(LoopAction::ContinueLoop) => continue,
                        ControlFlow::Continue(LoopAction::Proceed) => {}
                    }
                }
            }
            Statement::For(f) => {
                if f.parallel.is_some() || f.progress.is_some() {
                    return Err(comptime_error(
                        stmt_span(stmt),
                        "comptime `for` loops cannot use parallel/progress modifiers",
                    ));
                }
                match &f.kind {
                    ForKind::Range { start, end } => {
                        let start_v = eval_comptime_expr(start, frame, functions, depth)?;
                        let end_v = eval_comptime_expr(end, frame, functions, depth)?;
                        let (start_i, end_i) = match (start_v, end_v) {
                            (ConstValue::Int(s), ConstValue::Int(e)) => (s, e),
                            _ => {
                                return Err(comptime_error(
                                    stmt_span(stmt),
                                    "comptime range bounds must be integers",
                                ));
                            }
                        };
                        let mut i = start_i;
                        while i < end_i {
                            frame.values.insert(f.var.clone(), ConstValue::Int(i));
                            match eval_comptime_block_inner(
                                &f.body, frame, functions, depth, steps,
                                BlockMode::Statement, last_value,
                            )? {
                                ControlFlow::Break(v) => return Ok(ControlFlow::Break(v)),
                                ControlFlow::Continue(LoopAction::BreakLoop) => break,
                                ControlFlow::Continue(LoopAction::ContinueLoop) => {
                                    i += 1;
                                    continue;
                                }
                                ControlFlow::Continue(LoopAction::Proceed) => {}
                            }
                            i += 1;
                        }
                    }
                    ForKind::Iterable { iterable } => {
                        let values = eval_comptime_expr(iterable, frame, functions, depth)?;
                        let items = match values {
                            ConstValue::Array(items) => items,
                            _ => {
                                return Err(comptime_error(
                                    stmt_span(stmt),
                                    "comptime `for x in expr` requires a fixed array value",
                                ));
                            }
                        };
                        for item in items {
                            frame.values.insert(f.var.clone(), item);
                            match eval_comptime_block_inner(
                                &f.body, frame, functions, depth, steps,
                                BlockMode::Statement, last_value,
                            )? {
                                ControlFlow::Break(v) => return Ok(ControlFlow::Break(v)),
                                ControlFlow::Continue(LoopAction::BreakLoop) => break,
                                ControlFlow::Continue(LoopAction::ContinueLoop) => continue,
                                ControlFlow::Continue(LoopAction::Proceed) => {}
                            }
                        }
                    }
                }
            }
            Statement::Expression(e) => {
                let v = eval_comptime_expr(e, frame, functions, depth)?;
                if mode == BlockMode::ValueBlock {
                    *last_value = Some(v);
                }
            }
            Statement::Print(_)
            | Statement::Spawn(_)
            | Statement::Defer(_)
            | Statement::Unsafe(_)
            | Statement::Asm { .. }
            | Statement::Benchmark(_) => {
                return Err(comptime_error(
                    stmt_span(stmt),
                    "forbidden statement in comptime evaluation",
                ));
            }
            _ => {
                return Err(comptime_error(
                    stmt_span(stmt),
                    "unsupported statement in comptime evaluation",
                ));
            }
        }
    }
    Ok(ControlFlow::Continue(LoopAction::Proceed))
}

fn fold_comptime_in_block(
    block: &mut Block,
    functions: &HashMap<String, Function>,
    env: &mut HashMap<String, ConstValue>,
) {
    for stmt in &mut block.statements {
        fold_comptime_in_stmt(stmt, functions, env);
    }
}

fn fold_comptime_in_stmt(
    stmt: &mut Statement,
    functions: &HashMap<String, Function>,
    env: &mut HashMap<String, ConstValue>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            fold_comptime_in_expr(&mut l.value, functions, env);
            if let Ok(v) = eval_comptime_expr(&l.value, &ComptimeFrame::from_env(env), functions, 0) {
                if !l.mutable {
                    env.insert(l.name.clone(), v);
                }
            }
        }
        Statement::Assign(a) => {
            fold_comptime_in_expr(&mut a.target, functions, env);
            fold_comptime_in_expr(&mut a.value, functions, env);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                fold_comptime_in_expr(v, functions, env);
            }
        }
        Statement::If(i) => {
            fold_comptime_in_expr(&mut i.condition, functions, env);
            fold_comptime_in_block(&mut i.then_block, functions, env);
            if let Some(el) = &mut i.else_block {
                fold_comptime_in_block(el, functions, env);
            }
        }
        Statement::While(w) => {
            fold_comptime_in_expr(&mut w.condition, functions, env);
            fold_comptime_in_block(&mut w.body, functions, env);
        }
        Statement::Break { .. } | Statement::Continue { .. } => {}
        Statement::For(f) => {
            f.map_exprs_mut(|e| fold_comptime_in_expr(e, functions, env));
            fold_comptime_in_block(&mut f.body, functions, env);
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            fold_comptime_in_expr(e, functions, env);
        }
        Statement::Print(p) => {
            for a in &mut p.args {
                fold_comptime_in_expr(a, functions, env);
            }
            if let Some(c) = &mut p.color {
                fold_comptime_in_expr(c, functions, env);
            }
        }
        Statement::Spawn(b) | Statement::Benchmark(b) | Statement::Unsafe(b) => {
            fold_comptime_in_block(b, functions, env);
        }
        _ => {}
    }
}

fn fold_comptime_in_expr(
    expr: &mut Expression,
    functions: &HashMap<String, Function>,
    env: &HashMap<String, ConstValue>,
) {
    match expr {
        Expression::Binary(b) => {
            fold_comptime_in_expr(&mut b.left, functions, env);
            fold_comptime_in_expr(&mut b.right, functions, env);
        }
        Expression::Unary(u) => fold_comptime_in_expr(&mut u.operand, functions, env),
        Expression::Call(c) => {
            for a in &mut c.args {
                fold_comptime_in_expr(a, functions, env);
            }
        }
        Expression::If(i) => {
            fold_comptime_in_expr(&mut i.condition, functions, env);
            fold_comptime_in_expr(&mut i.then_expr, functions, env);
            fold_comptime_in_expr(&mut i.else_expr, functions, env);
        }
        Expression::Grouped(g) => fold_comptime_in_expr(g, functions, env),
        Expression::EnumVariant(ev) => {
            for arg in &mut ev.args {
                fold_comptime_in_expr(arg, functions, env);
            }
        }
        Expression::Index(ix) => {
            fold_comptime_in_expr(&mut ix.object, functions, env);
            fold_comptime_in_expr(&mut ix.index, functions, env);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                fold_comptime_in_expr(e, functions, env);
            }
        }
        Expression::FieldAccess(f) => fold_comptime_in_expr(&mut f.object, functions, env),
        Expression::MethodCall(mc) => {
            fold_comptime_in_expr(&mut mc.object, functions, env);
            for a in &mut mc.args {
                fold_comptime_in_expr(a, functions, env);
            }
        }
        Expression::Match(m) => {
            fold_comptime_in_expr(&mut m.scrutinee, functions, env);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    fold_comptime_in_expr(g, functions, env);
                }
                fold_comptime_in_expr(&mut arm.body, functions, env);
            }
        }
        Expression::Cast(c) => fold_comptime_in_expr(&mut c.expr, functions, env),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                fold_comptime_in_expr(e, functions, env);
            }
        }
        Expression::Await(e) => fold_comptime_in_expr(e, functions, env),
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let ast::TemplatePart::Interpolation(e) = part {
                    fold_comptime_in_expr(e, functions, env);
                }
            }
        }
        Expression::ArrowFn(a) => match &mut a.body {
            ast::ArrowBody::Expr(e) => fold_comptime_in_expr(e, functions, env),
            ast::ArrowBody::Block(b) => fold_comptime_in_block(b, functions, &mut env.clone()),
        },
        Expression::ComptimeBlock { body, .. } => {
            fold_comptime_in_block(body, functions, &mut env.clone());
        }
        Expression::StructLiteral(s) => {
            for (_, e) in &mut s.fields {
                fold_comptime_in_expr(e, functions, env);
            }
            for e in &mut s.spreads {
                fold_comptime_in_expr(e, functions, env);
            }
        }
        Expression::ArrayRepeat { element, count_expr, .. } => {
            fold_comptime_in_expr(element, functions, env);
            if let Some(ce) = count_expr {
                fold_comptime_in_expr(ce, functions, env);
            }
        }
        _ => {}
    }
    if let Ok(v) = eval_comptime_expr(expr, &ComptimeFrame::from_env(env), functions, 0) {
        *expr = const_value_to_expr(&v);
    }
}

fn check_runtime_comptime_calls_in_block(
    block: &Block,
    comptime_names: &std::collections::HashSet<String>,
    errors: &mut Vec<NyraError>,
) {
    for stmt in &block.statements {
        check_runtime_comptime_calls_in_stmt(stmt, comptime_names, errors);
    }
}

fn check_runtime_comptime_calls_in_stmt(
    stmt: &Statement,
    comptime_names: &std::collections::HashSet<String>,
    errors: &mut Vec<NyraError>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            check_runtime_comptime_calls_in_expr(&l.value, comptime_names, errors);
        }
        Statement::Assign(a) => {
            check_runtime_comptime_calls_in_expr(&a.target, comptime_names, errors);
            check_runtime_comptime_calls_in_expr(&a.value, comptime_names, errors);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                check_runtime_comptime_calls_in_expr(v, comptime_names, errors);
            }
        }
        Statement::If(i) => {
            check_runtime_comptime_calls_in_expr(&i.condition, comptime_names, errors);
            check_runtime_comptime_calls_in_block(&i.then_block, comptime_names, errors);
            if let Some(el) = &i.else_block {
                check_runtime_comptime_calls_in_block(el, comptime_names, errors);
            }
        }
        Statement::While(w) => {
            check_runtime_comptime_calls_in_expr(&w.condition, comptime_names, errors);
            check_runtime_comptime_calls_in_block(&w.body, comptime_names, errors);
        }
        Statement::For(f) => {
            f.for_each_expr(|e| check_runtime_comptime_calls_in_expr(e, comptime_names, errors));
            check_runtime_comptime_calls_in_block(&f.body, comptime_names, errors);
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            check_runtime_comptime_calls_in_expr(e, comptime_names, errors);
        }
        Statement::Print(p) => {
            for a in &p.args {
                check_runtime_comptime_calls_in_expr(a, comptime_names, errors);
            }
            if let Some(c) = &p.color {
                check_runtime_comptime_calls_in_expr(c, comptime_names, errors);
            }
        }
        Statement::Spawn(b) | Statement::Benchmark(b) | Statement::Unsafe(b) => {
            check_runtime_comptime_calls_in_block(b, comptime_names, errors);
        }
        _ => {}
    }
}

fn check_runtime_comptime_calls_in_expr(
    expr: &Expression,
    comptime_names: &std::collections::HashSet<String>,
    errors: &mut Vec<NyraError>,
) {
    match expr {
        Expression::Call(c) => {
            if comptime_names.contains(&c.callee) {
                errors.push(
                    comptime_error(
                        c.span.clone(),
                        format!(
                            "`#[comptime]` function `{}` cannot be called at runtime",
                            c.callee
                        ),
                    )
                    .help(
                        "pass compile-time literals/`const` values only, or move logic to a comptime module",
                    ),
                );
            }
            for a in &c.args {
                check_runtime_comptime_calls_in_expr(a, comptime_names, errors);
            }
        }
        Expression::Binary(b) => {
            check_runtime_comptime_calls_in_expr(&b.left, comptime_names, errors);
            check_runtime_comptime_calls_in_expr(&b.right, comptime_names, errors);
        }
        Expression::Unary(u) => {
            check_runtime_comptime_calls_in_expr(&u.operand, comptime_names, errors);
        }
        Expression::Grouped(g) => check_runtime_comptime_calls_in_expr(g, comptime_names, errors),
        Expression::If(i) => {
            check_runtime_comptime_calls_in_expr(&i.condition, comptime_names, errors);
            check_runtime_comptime_calls_in_expr(&i.then_expr, comptime_names, errors);
            check_runtime_comptime_calls_in_expr(&i.else_expr, comptime_names, errors);
        }
        Expression::Index(ix) => {
            check_runtime_comptime_calls_in_expr(&ix.object, comptime_names, errors);
            check_runtime_comptime_calls_in_expr(&ix.index, comptime_names, errors);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                check_runtime_comptime_calls_in_expr(e, comptime_names, errors);
            }
        }
        Expression::FieldAccess(f) => {
            check_runtime_comptime_calls_in_expr(&f.object, comptime_names, errors);
        }
        Expression::MethodCall(mc) => {
            check_runtime_comptime_calls_in_expr(&mc.object, comptime_names, errors);
            for a in &mc.args {
                check_runtime_comptime_calls_in_expr(a, comptime_names, errors);
            }
        }
        Expression::Match(m) => {
            check_runtime_comptime_calls_in_expr(&m.scrutinee, comptime_names, errors);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    check_runtime_comptime_calls_in_expr(g, comptime_names, errors);
                }
                check_runtime_comptime_calls_in_expr(&arm.body, comptime_names, errors);
            }
        }
        Expression::Cast(c) => check_runtime_comptime_calls_in_expr(&c.expr, comptime_names, errors),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast::Literal;

    fn parse(src: &str) -> Program {
        let (tokens, _) = lexer::Lexer::new(src, "test.ny").tokenize();
        let (program, errs) = parser::Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        program
    }

    #[test]
    fn comptime_module_folds_const_call() {
        let mut program = parse(
            r#"comptime

fn mix(n) {
    return n * 3
}

pub const ANSWER = mix(14)
"#,
        );
        assert!(program.comptime);
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(program.functions.is_empty());
        assert_eq!(program.consts.len(), 1);
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(42))
        ));
    }

    #[test]
    fn comptime_rejects_main() {
        let mut program = parse(
            r#"comptime
fn main() {
    return 0
}
"#,
        );
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.iter().any(|e| e.message.contains("main")));
    }

    #[test]
    fn comptime_for_loop_accumulator() {
        let mut program = parse(
            r#"comptime

fn sum_to(n) {
    let mut acc = 0
    for i in 0..n {
        acc = acc + i
    }
    return acc
}

const TOTAL = sum_to(5)
"#,
        );
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(10))
        ));
    }

    #[test]
    fn comptime_for_in_array() {
        let mut program = parse(
            r#"comptime

fn sum_array(values) {
    let mut acc = 0
    for x in values {
        acc = acc + x
    }
    return acc
}

const VALUES = [1, 2, 3, 4]
const TOTAL = sum_array(VALUES)
"#,
        );
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts.iter().find(|c| c.name == "TOTAL").unwrap().value,
            Expression::Literal(Literal::Int(10))
        ));
    }

    #[test]
    fn comptime_generic_call() {
        let mut program = parse(
            r#"comptime

fn id<T>(x: T) -> T {
    return x
}

const DOUBLED = id(21) + id(21)
"#,
        );
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(42))
        ));
    }

    #[test]
    fn comptime_array_spread() {
        let mut program = parse(
            r#"comptime

fn append_one(values) {
    return [...values, 99]
}

const TAIL = append_one([1, 2])
"#,
        );
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        match &program.consts[0].value {
            Expression::ArrayLiteral(al) => {
                assert_eq!(al.elems.len(), 3);
            }
            other => panic!("expected array literal, got {other:?}"),
        }
    }

    #[test]
    fn comptime_fn_attr_folds_in_normal_file() {
        let mut program = parse(
            r#"#[comptime]
fn mix(n) {
    return n * 3
}

const SEED = mix(14)

fn main() {
    return 0
}
"#,
        );
        assert!(!program.comptime);
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(!program.functions.iter().any(|f| f.name == "mix"));
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(42))
        ));
    }

    #[test]
    fn comptime_fn_attr_preserves_declared_int_kind() {
        use ast::IntKind;

        let mut program = parse(
            r#"#[comptime]
fn mix(n: i64) -> i64 {
    return n * 3
}

const SEED: i64 = mix(14)
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::IntKind(42, IntKind::I64))
        ));
    }

    #[test]
    fn comptime_block_while() {
        let mut program = parse(
            r#"const N = comptime {
    let mut acc = 0
    let mut i = 0
    while i < 4 {
        acc = acc + i
        i = i + 1
    }
    acc
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(6))
        ));
    }

    #[test]
    fn comptime_break_in_for() {
        let mut program = parse(
            r#"const N = comptime {
    let mut acc = 0
    for i in 0..100 {
        if i == 5 {
            break
        }
        acc = acc + i
    }
    acc
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(10))
        ));
    }

    #[test]
    fn comptime_continue_in_for() {
        let mut program = parse(
            r#"const N = comptime {
    let mut acc = 0
    for i in 0..4 {
        if i % 2 == 0 {
            continue
        }
        acc = acc + i
    }
    acc
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(4))
        ));
    }

    #[test]
    fn comptime_match_enum() {
        let mut program = parse(
            r#"enum Status { Ok, Err }

const N = comptime {
    let s = Status.Ok
    match s {
        Status.Ok => 1
        Status.Err => 2
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(1))
        ));
    }

    #[test]
    fn comptime_match_int_guard() {
        let mut program = parse(
            r#"const N = comptime {
    let n = 7
    match n {
        _ if n < 5 => 1
        _ if n < 10 => 2
        _ => 3
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(2))
        ));
    }

    #[test]
    fn comptime_struct_literal_and_match() {
        let mut program = parse(
            r#"struct Point {
    x: i32
    y: i32
}

const N = comptime {
    let p = Point { x: 3, y: 4 }
    match p {
        Point { x, y } => x + y
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(7))
        ));
    }

    #[test]
    fn comptime_enum_payload_match() {
        let mut program = parse(
            r#"enum Opt { None, Some(i32) }

const N = comptime {
    let o = Opt.Some(42)
    match o {
        Opt.None => 0
        Opt.Some(x) => x
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(42))
        ));
    }

    #[test]
    fn comptime_tuple_match() {
        let mut program = parse(
            r#"const N = comptime {
    let pair = (10, 20)
    match pair {
        (a, b) => a + b
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(30))
        ));
    }

    #[test]
    fn comptime_string_concat_and_match() {
        let mut program = parse(
            r#"const LABEL = comptime {
    let a = "hello"
    let b = " world"
    a + b
}

const CODE = comptime {
    let s = "GET"
    match s {
        "GET" => 1
        "POST" => 2
        _ => 0
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        let label = program.consts.iter().find(|c| c.name == "LABEL").unwrap();
        assert!(matches!(
            label.value,
            Expression::Literal(Literal::String(ref s)) if s == "hello world"
        ));
        let code = program.consts.iter().find(|c| c.name == "CODE").unwrap();
        assert!(matches!(code.value, Expression::Literal(Literal::Int(1))));
    }

    #[test]
    fn comptime_len_and_array_assign() {
        let mut program = parse(
            r#"const LEN = comptime {
    let t = [1, 2, 3, 4]
    t.len()
}

const TABLE = comptime {
    let mut table = [0; 4]
    let mut i = 0
    while i < 4 {
        table[i] = i * i
        i = i + 1
    }
    table
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts.iter().find(|c| c.name == "LEN").unwrap().value,
            Expression::Literal(Literal::Int(4))
        ));
        let table = program.consts.iter().find(|c| c.name == "TABLE").unwrap();
        assert!(matches!(table.value, Expression::ArrayLiteral(_)));
    }

    #[test]
    fn comptime_int_literal_match() {
        let mut program = parse(
            r#"const N = comptime {
    let n = 7
    match n {
        5 => 1
        7 => 2
        _ => 3
    }
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(2))
        ));
    }

    #[test]
    fn comptime_module_keeps_pub_types() {
        let mut program = parse(
            r#"comptime

pub struct Point {
    x: i32
    y: i32
}

fn origin() {
    return Point { x: 0, y: 0 }
}

pub const ZERO = origin()
"#,
        );
        let errors = finalize_comptime_module(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(program.structs.len(), 1);
        assert!(program.structs[0].public);
        assert!(program.functions.is_empty());
    }

    #[test]
    fn comptime_block_calls_plain_helper_fn() {
        let mut program = parse(
            r#"fn mix(n) {
    return n * 2
}

const R = comptime {
    mix(21)
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(42))
        ));
    }

    #[test]
    fn comptime_block_while_calls_plain_helper_fn() {
        let mut program = parse(
            r#"fn mix(n) {
    return n * 2
}

const R = comptime {
    let mut acc = 0
    let mut i = 0
    while i < 3 {
        acc = acc + mix(i)
        i = i + 1
    }
    acc
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(6))
        ));
    }

    #[test]
    fn comptime_helper_reads_module_const() {
        let mut program = parse(
            r#"const MOD = 1000000007

fn mix(n) {
    return (n * 100003) % MOD
}

const R = comptime {
    mix(10)
}
"#,
        );
        let errors = fold_attributed_comptime_functions(&mut program);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            program.consts[1].value,
            Expression::Literal(Literal::Int(1_000_030))
        ));
    }
}
