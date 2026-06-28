//! Infer generic type arguments at call sites (e.g. `id(7)` → `id<i32>(7)`).

use std::collections::HashMap;

use ast::*;

fn infer_expr_ann(expr: &Expression, env: &HashMap<String, TypeAnnotation>) -> Option<TypeAnnotation> {
    match expr {
        Expression::Literal(Literal::Int(_)) => Some(TypeAnnotation::Integer(ast::IntKind::I32)),
        Expression::Literal(Literal::IntKind(_, k)) => Some(TypeAnnotation::Integer(*k)),
        Expression::Literal(Literal::Float(_, _)) => Some(TypeAnnotation::F64),
        Expression::Literal(Literal::Char(_)) => Some(TypeAnnotation::Char),
        Expression::Literal(Literal::Bool(_)) => Some(TypeAnnotation::Bool),
        Expression::Literal(Literal::String(_)) => Some(TypeAnnotation::String),
        Expression::Variable { name, .. } => env.get(name).cloned(),
        Expression::Unary(u) => match u.op {
            UnaryOp::Ref => infer_expr_ann(&u.operand, env).map(|inner| TypeAnnotation::Ref {
                inner: Box::new(inner),
                mutable: false,
                lifetime: None,
            }),
            UnaryOp::RefMut => infer_expr_ann(&u.operand, env).map(|inner| TypeAnnotation::Ref {
                inner: Box::new(inner),
                mutable: true,
                lifetime: None,
            }),
            _ => None,
        },
        Expression::Grouped(inner) => infer_expr_ann(inner, env),
        Expression::StructLiteral(s) => Some(TypeAnnotation::Struct(s.name.clone())),
        Expression::ArrayLiteral(al) if al.spreads.is_empty() && !al.elems.is_empty() => {
            infer_expr_ann(&al.elems[0], env).map(|elem| TypeAnnotation::Array {
                elem: Box::new(elem),
                len: Some(al.elems.len()),
            })
        }
        _ => None,
    }
}

fn type_param_from_param(pt: &TypeAnnotation, type_params: &[String]) -> Option<String> {
    match pt {
        TypeAnnotation::Generic(n) | TypeAnnotation::Struct(n) if type_params.contains(n) => {
            Some(n.clone())
        }
        _ => None,
    }
}

fn unify_generic_args(
    type_params: &[String],
    param_types: &[TypeAnnotation],
    arg_anns: &[TypeAnnotation],
) -> Option<Vec<TypeAnnotation>> {
    let mut map = HashMap::new();
    for (pt, arg) in param_types.iter().zip(arg_anns.iter()) {
        if let Some(tp) = type_param_from_param(pt, type_params) {
            if let Some(prev) = map.get(&tp) {
                if prev != arg {
                    return None;
                }
            } else {
                map.insert(tp, arg.clone());
            }
        }
    }
    if map.len() == type_params.len() {
        return Some(type_params.iter().map(|n| map.get(n).cloned().unwrap()).collect());
    }
    if type_params.len() == 1 && param_types.len() == arg_anns.len() {
        let pt = param_types.first()?;
        let arg = arg_anns.first()?;
        if type_param_from_param(pt, type_params).is_some() {
            return Some(vec![arg.clone()]);
        }
    }
    None
}

fn infer_call_type_args(
    callee: &str,
    args: &[Expression],
    env: &HashMap<String, TypeAnnotation>,
    generics: &HashMap<String, Function>,
) -> Option<Vec<TypeAnnotation>> {
    let func = generics.get(callee)?;
    if func.type_params.is_empty() {
        return None;
    }
    let arg_anns: Vec<TypeAnnotation> = args
        .iter()
        .filter_map(|a| infer_expr_ann(a, env))
        .collect();
    if arg_anns.len() != args.len() {
        return None;
    }
    unify_generic_args(
        &func.type_params,
        &func.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
        &arg_anns,
    )
}

fn rewrite_expr_calls(
    expr: &mut Expression,
    env: &HashMap<String, TypeAnnotation>,
    generics: &HashMap<String, Function>,
) {
    match expr {
        Expression::Call(c) if c.type_args.is_empty() => {
            for a in &mut c.args {
                rewrite_expr_calls(a, env, generics);
            }
            if let Some(args) = infer_call_type_args(&c.callee, &c.args, env, generics) {
                c.type_args = args;
            }
        }
        Expression::Call(c) => {
            for a in &mut c.args {
                rewrite_expr_calls(a, env, generics);
            }
        }
        Expression::Binary(b) => {
            rewrite_expr_calls(&mut b.left, env, generics);
            rewrite_expr_calls(&mut b.right, env, generics);
        }
        Expression::Unary(u) => rewrite_expr_calls(&mut u.operand, env, generics),
        Expression::Grouped(g) => rewrite_expr_calls(g, env, generics),
        Expression::If(i) => {
            rewrite_expr_calls(&mut i.condition, env, generics);
            rewrite_expr_calls(&mut i.then_expr, env, generics);
            rewrite_expr_calls(&mut i.else_expr, env, generics);
        }
        Expression::Match(m) => {
            rewrite_expr_calls(&mut m.scrutinee, env, generics);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    rewrite_expr_calls(g, env, generics);
                }
                rewrite_expr_calls(&mut arm.body, env, generics);
            }
        }
        Expression::Await(e) => rewrite_expr_calls(e, env, generics),
        Expression::MethodCall(mc) => {
            rewrite_expr_calls(&mut mc.object, env, generics);
            for a in &mut mc.args {
                rewrite_expr_calls(a, env, generics);
            }
        }
        Expression::FieldAccess(f) => rewrite_expr_calls(&mut f.object, env, generics),
        Expression::Index(ix) => {
            rewrite_expr_calls(&mut ix.object, env, generics);
            rewrite_expr_calls(&mut ix.index, env, generics);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                rewrite_expr_calls(e, env, generics);
            }
        }
        Expression::ArrayRepeat { element, .. } => rewrite_expr_calls(element, env, generics),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                rewrite_expr_calls(e, env, generics);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                rewrite_expr_calls(spread, env, generics);
            }
            for (_, e) in &mut s.fields {
                rewrite_expr_calls(e, env, generics);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    rewrite_expr_calls(e, env, generics);
                }
            }
        }
        Expression::Cast(c) => rewrite_expr_calls(&mut c.expr, env, generics),
        _ => {}
    }
}

fn stmt_extends_env(stmt: &Statement, env: &mut HashMap<String, TypeAnnotation>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            if let Some(ty) = &l.ty {
                env.insert(l.name.clone(), ty.clone());
            } else if let Some(ann) = infer_expr_ann(&l.value, env) {
                env.insert(l.name.clone(), ann);
            }
        }
        _ => {}
    }
}

fn rewrite_stmt_calls(
    stmt: &mut Statement,
    env: &mut HashMap<String, TypeAnnotation>,
    generics: &HashMap<String, Function>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            rewrite_expr_calls(&mut l.value, env, generics);
            stmt_extends_env(stmt, env);
        }
        Statement::Assign(a) => {
            rewrite_expr_calls(&mut a.target, env, generics);
            rewrite_expr_calls(&mut a.value, env, generics);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                rewrite_expr_calls(v, env, generics);
            }
        }
        Statement::If(i) => {
            rewrite_expr_calls(&mut i.condition, env, generics);
            let mut then_env = env.clone();
            for s in &mut i.then_block.statements {
                rewrite_stmt_calls(s, &mut then_env, generics);
            }
            if let Some(e) = &mut i.else_block {
                let mut else_env = env.clone();
                for s in &mut e.statements {
                    rewrite_stmt_calls(s, &mut else_env, generics);
                }
            }
        }
        Statement::While(w) => {
            rewrite_expr_calls(&mut w.condition, env, generics);
            let mut body_env = env.clone();
            for s in &mut w.body.statements {
                rewrite_stmt_calls(s, &mut body_env, generics);
            }
        }
        Statement::For(f) => {
            f.map_exprs_mut(|e| rewrite_expr_calls(e, env, generics));
            let mut body_env = env.clone();
            for s in &mut f.body.statements {
                rewrite_stmt_calls(s, &mut body_env, generics);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => rewrite_expr_calls(e, env, generics),
        Statement::Print(p) => {
            for a in &mut p.args {
                rewrite_expr_calls(a, env, generics);
            }
            if let Some(c) = &mut p.color {
                rewrite_expr_calls(c, env, generics);
            }
        }
        Statement::Spawn(b) => {
            let mut body_env = env.clone();
            for s in &mut b.statements {
                rewrite_stmt_calls(s, &mut body_env, generics);
            }
        }
        Statement::Benchmark(b) => {
            for s in &mut b.statements {
                rewrite_stmt_calls(s, env, generics);
            }
        }
        Statement::Unsafe(b) => {
            for s in &mut b.statements {
                rewrite_stmt_calls(s, env, generics);
            }
        }
        _ => {}
    }
}

pub fn infer_generic_call_sites(program: &mut Program) {
    let generics: HashMap<String, Function> = program
        .functions
        .iter()
        .filter(|f| !f.type_params.is_empty())
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    if generics.is_empty() {
        return;
    }
    let mut const_env = HashMap::new();
    for c in &mut program.consts {
        rewrite_expr_calls(&mut c.value, &const_env, &generics);
        if let Some(ty) = &c.ty {
            const_env.insert(c.name.clone(), ty.clone());
        } else if let Some(ann) = infer_expr_ann(&c.value, &const_env) {
            const_env.insert(c.name.clone(), ann);
        }
    }
    for f in &mut program.functions {
        let mut env = HashMap::new();
        for p in &f.params {
            env.insert(p.name.clone(), p.ty.clone());
        }
        for stmt in &mut f.body.statements {
            rewrite_stmt_calls(stmt, &mut env, &generics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::Lexer;
    use parser::Parser;

    #[test]
    fn infers_trait_bound_call_with_struct_arg() {
        let src = r#"trait Add {
    fn add(self, other: i32) -> i32
}
struct Counter {
    value: i32
}
impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}
fn sum_one<T: Add>(x: T) -> i32 {
    return x.add(1)
}
test fn test_trait_bound_generic_call() {
    let c = Counter { value: 10 }
    assert_eq(sum_one(c), 11)
}"#;
        let (tokens, _) = Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        infer_generic_call_sites(&mut program);
        let test_fn = program
            .functions
            .iter()
            .find(|f| f.name == "test_trait_bound_generic_call")
            .unwrap();
        let assert_call = test_fn.body.statements.iter().find_map(|s| {
            if let Statement::Expression(Expression::Call(c)) = s {
                Some(c)
            } else {
                None
            }
        }).expect("assert_eq call");
        let sum_call = assert_call
            .args
            .first()
            .and_then(|e| {
                if let Expression::Call(c) = e {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("sum_one nested in assert_eq");
        assert_eq!(sum_call.callee, "sum_one");
        assert_eq!(
            sum_call.type_args,
            vec![TypeAnnotation::Struct("Counter".into())]
        );
    }

    #[test]
    fn sets_type_args_on_inferred_id_call() {
        let src = r#"fn id<T>(x: T) -> T { return x }
fn main() { print(id(7)) }"#;
        let (tokens, _) = Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        infer_generic_call_sites(&mut program);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let call = match &main.body.statements[0] {
            Statement::Print(p) => match p.args.first() {
                Some(Expression::Call(c)) => c,
                other => panic!("expected call, got {other:?}"),
            },
            other => panic!("expected print, got {other:?}"),
        };
        assert_eq!(call.callee, "id");
        assert_eq!(call.type_args, vec![TypeAnnotation::Integer(ast::IntKind::I32)]);
    }
}
