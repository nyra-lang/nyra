//! Inferred type hints and parameter-name hints (rust-analyzer style).

use ast::{
    expr_span, for_each_expr_in_block, Expression, Function, Program, TraitImpl,
};
use typecheck::{type_pretty, TypeChecker};

#[derive(Debug, Clone)]
pub struct InlayHintInfo {
    pub line: u32,
    pub character: u32,
    pub label: String,
    pub kind: InlayHintKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    Type,
    Parameter,
}

pub fn collect_inlay_hints(checker: &TypeChecker, program: &Program) -> Vec<InlayHintInfo> {
    let mut hints: Vec<InlayHintInfo> = checker
        .inferred_bindings
        .iter()
        .filter(|b| b.ty != typecheck::Type::Unknown)
        .map(|b| InlayHintInfo {
            line: (b.span.start.line.saturating_sub(1)) as u32,
            character: (b.span.end.column.saturating_sub(1)) as u32,
            label: format!(": {}", type_pretty(&b.ty)),
            kind: InlayHintKind::Type,
        })
        .collect();

    hints.extend(collect_parameter_hints(program));
    hints.sort_by(|a, b| {
        a.line
            .cmp(&b.line)
            .then(a.character.cmp(&b.character))
            .then(a.kind.cmp_rank().cmp(&b.kind.cmp_rank()))
    });
    hints
}

impl InlayHintKind {
    fn cmp_rank(self) -> u8 {
        match self {
            InlayHintKind::Parameter => 0,
            InlayHintKind::Type => 1,
        }
    }
}

fn collect_parameter_hints(program: &Program) -> Vec<InlayHintInfo> {
    let mut fn_params: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for f in &program.functions {
        fn_params.insert(f.name.clone(), param_names(f));
    }
    for imp in &program.impls {
        for m in &imp.methods {
            fn_params.insert(m.name.clone(), param_names(m));
            // Unqualified method name for `obj.method(...)` — prefer Type_method when present.
            fn_params
                .entry(format!("{}_{}", imp.type_name, m.name))
                .or_insert_with(|| param_names(m));
        }
    }
    for ti in &program.trait_impls {
        register_trait_impl_params(ti, &mut fn_params);
    }

    let mut out = Vec::new();
    for f in &program.functions {
        for_each_expr_in_block(&f.body, &mut |e| collect_call_param_hints(e, &fn_params, &mut out));
    }
    for imp in &program.impls {
        for m in &imp.methods {
            for_each_expr_in_block(&m.body, &mut |e| {
                collect_call_param_hints(e, &fn_params, &mut out)
            });
        }
    }
    for ti in &program.trait_impls {
        for m in &ti.methods {
            for_each_expr_in_block(&m.body, &mut |e| {
                collect_call_param_hints(e, &fn_params, &mut out)
            });
        }
    }
    out
}

fn register_trait_impl_params(
    ti: &TraitImpl,
    fn_params: &mut std::collections::HashMap<String, Vec<String>>,
) {
    for m in &ti.methods {
        fn_params.insert(m.name.clone(), param_names(m));
        fn_params.insert(
            format!("{}_{}_{}", ti.trait_name, ti.type_name, m.name),
            param_names(m),
        );
    }
}

fn param_names(f: &Function) -> Vec<String> {
    f.params
        .iter()
        .filter(|p| p.name != "self")
        .map(|p| p.name.clone())
        .collect()
}

fn collect_call_param_hints(
    expr: &Expression,
    fn_params: &std::collections::HashMap<String, Vec<String>>,
    out: &mut Vec<InlayHintInfo>,
) {
    match expr {
        Expression::Call(c) => {
            if let Some(names) = fn_params.get(&c.callee) {
                push_arg_hints(&c.args, names, /*skip_receiver*/ false, out);
            }
            for a in &c.args {
                collect_call_param_hints(a, fn_params, out);
            }
        }
        Expression::MethodCall(m) => {
            // Prefer Type_method from receiver if we can resolve; fall back to bare method name.
            if let Some(names) = fn_params.get(&m.method) {
                // Method params in `param_names` already skip `self`.
                push_arg_hints(&m.args, names, false, out);
            }
            collect_call_param_hints(&m.object, fn_params, out);
            for a in &m.args {
                collect_call_param_hints(a, fn_params, out);
            }
        }
        Expression::Binary(b) => {
            collect_call_param_hints(&b.left, fn_params, out);
            collect_call_param_hints(&b.right, fn_params, out);
        }
        Expression::Unary(u) => collect_call_param_hints(&u.operand, fn_params, out),
        Expression::Grouped(e) | Expression::Await(e) => {
            collect_call_param_hints(e, fn_params, out)
        }
        Expression::FieldAccess(f) => collect_call_param_hints(&f.object, fn_params, out),
        Expression::Index(i) => {
            collect_call_param_hints(&i.object, fn_params, out);
            collect_call_param_hints(&i.index, fn_params, out);
        }
        Expression::StructLiteral(s) => {
            for (_, v) in &s.fields {
                collect_call_param_hints(v, fn_params, out);
            }
        }
        Expression::ArrayLiteral(a) => {
            for e in &a.elems {
                collect_call_param_hints(e, fn_params, out);
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_call_param_hints(e, fn_params, out);
            }
        }
        Expression::Cast(c) => collect_call_param_hints(&c.expr, fn_params, out),
        Expression::If(i) => {
            collect_call_param_hints(&i.condition, fn_params, out);
            for_each_expr_in_block(&i.then_block, &mut |e| {
                collect_call_param_hints(e, fn_params, out)
            });
            for_each_expr_in_block(&i.else_block, &mut |e| {
                collect_call_param_hints(e, fn_params, out)
            });
        }
        Expression::Match(m) => {
            collect_call_param_hints(&m.scrutinee, fn_params, out);
            for arm in &m.arms {
                for_each_expr_in_block(&arm.body, &mut |e| {
                    collect_call_param_hints(e, fn_params, out)
                });
            }
        }
        _ => {}
    }
}

fn push_arg_hints(
    args: &[Expression],
    names: &[String],
    _skip_receiver: bool,
    out: &mut Vec<InlayHintInfo>,
) {
    for (i, arg) in args.iter().enumerate() {
        let Some(name) = names.get(i) else {
            break;
        };
        if name.starts_with('_') || name.is_empty() {
            continue;
        }
        // Skip when the argument is already a same-named variable (`foo(x)` with param `x`).
        if let Expression::Variable { name: var, .. } = arg {
            if var == name {
                continue;
            }
        }
        let sp = expr_span(arg);
        if sp.start.line == 0 {
            continue;
        }
        out.push(InlayHintInfo {
            line: (sp.start.line.saturating_sub(1)) as u32,
            character: (sp.start.column.saturating_sub(1)) as u32,
            label: format!("{name}:"),
            kind: InlayHintKind::Parameter,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentAnalysis;

    #[test]
    fn type_and_param_hints() {
        let src = r#"
fn add(a: i32, b: i32) -> i32 { return a + b }
fn main() {
    let x = 1
    let _ = add(1, 2)
}
"#;
        let a = DocumentAnalysis::analyze(src, "t.ny");
        assert!(a.inlay_hints.iter().any(|h| h.label.contains("i32")));
        assert!(a
            .inlay_hints
            .iter()
            .any(|h| h.kind == InlayHintKind::Parameter && h.label.starts_with("a:")));
    }
}
