//! Auto-borrow coercion: pass owned bindings as `&T` / `&mut T` when the callee expects references.

use std::collections::HashMap;

use ast::*;
use errors::Span;

fn param_is_ref(ty: &TypeAnnotation) -> Option<bool> {
    match ty {
        TypeAnnotation::Ref { mutable, .. } => Some(*mutable),
        _ => None,
    }
}

fn make_ref_expr(operand: Expression, mutable: bool, span: Span) -> Expression {
    Expression::Unary(Box::new(UnaryExpr {
        op: if mutable { UnaryOp::RefMut } else { UnaryOp::Ref },
        operand,
        span,
    }))
}

fn coerce_call_args(
    callee: &str,
    args: &mut [Expression],
    sigs: &HashMap<String, Function>,
) {
    let Some(func) = sigs.get(callee) else {
        return;
    };
    for (arg, param) in args.iter_mut().zip(func.params.iter()) {
        let Some(mut_ref) = param_is_ref(&param.ty) else {
            continue;
        };
        if matches!(
            &*arg,
            Expression::Unary(u) if u.op == UnaryOp::Move || u.op == UnaryOp::Clone
        ) {
            continue;
        }
        if let Expression::Variable { name, span } = &*arg {
            let name = name.clone();
            let span = span.clone();
            *arg = make_ref_expr(Expression::Variable { name, span }, mut_ref, expr_span(arg));
        }
    }
}

fn coerce_expr_tree(expr: &mut Expression, sigs: &HashMap<String, Function>) {
    match expr {
        Expression::Call(c) => {
            for a in &mut c.args {
                coerce_expr_tree(a, sigs);
            }
            coerce_call_args(&c.callee, &mut c.args, sigs);
        }
        Expression::Binary(b) => {
            coerce_expr_tree(&mut b.left, sigs);
            coerce_expr_tree(&mut b.right, sigs);
        }
        Expression::Unary(u) => coerce_expr_tree(&mut u.operand, sigs),
        Expression::Grouped(g) => coerce_expr_tree(g, sigs),
        Expression::If(i) => {
            coerce_expr_tree(&mut i.condition, sigs);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| coerce_expr_tree(e, sigs));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| coerce_expr_tree(e, sigs));
        }
        Expression::Match(m) => {
            coerce_expr_tree(&mut m.scrutinee, sigs);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    coerce_expr_tree(g, sigs);
                }
                for_each_expr_in_block_mut(&mut arm.body, &mut |e| coerce_expr_tree(e, sigs));
            }
        }
        Expression::Await(e) => coerce_expr_tree(e, sigs),
        Expression::MethodCall(mc) => {
            coerce_expr_tree(&mut mc.object, sigs);
            for a in &mut mc.args {
                coerce_expr_tree(a, sigs);
            }
        }
        Expression::FieldAccess(f) => coerce_expr_tree(&mut f.object, sigs),
        Expression::Index(ix) => {
            coerce_expr_tree(&mut ix.object, sigs);
            coerce_expr_tree(&mut ix.index, sigs);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                coerce_expr_tree(e, sigs);
            }
        }
        Expression::ArrayRepeat { element, .. } => coerce_expr_tree(element, sigs),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                coerce_expr_tree(e, sigs);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                coerce_expr_tree(spread, sigs);
            }
            for (_, e) in &mut s.fields {
                coerce_expr_tree(e, sigs);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    coerce_expr_tree(e, sigs);
                }
            }
        }
        Expression::Cast(c) => coerce_expr_tree(&mut c.expr, sigs),
        _ => {}
    }
}

fn coerce_stmt(stmt: &mut Statement, sigs: &HashMap<String, Function>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => coerce_expr_tree(&mut l.value, sigs),
        Statement::Assign(a) => {
            coerce_expr_tree(&mut a.target, sigs);
            coerce_expr_tree(&mut a.value, sigs);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                coerce_expr_tree(v, sigs);
            }
        }
        Statement::If(i) => {
            coerce_expr_tree(&mut i.condition, sigs);
            for s in &mut i.then_block.statements {
                coerce_stmt(s, sigs);
            }
            if let Some(e) = &mut i.else_block {
                for s in &mut e.statements {
                    coerce_stmt(s, sigs);
                }
            }
        }
        Statement::While(w) => {
            coerce_expr_tree(&mut w.condition, sigs);
            for s in &mut w.body.statements {
                coerce_stmt(s, sigs);
            }
        }
        Statement::For(f) => {
            f.map_exprs_mut(|e| coerce_expr_tree(e, sigs));
            for s in &mut f.body.statements {
                coerce_stmt(s, sigs);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => coerce_expr_tree(e, sigs),
        Statement::Print(p) => {
            for a in &mut p.args {
                coerce_expr_tree(a, sigs);
            }
            if let Some(c) = &mut p.color {
                coerce_expr_tree(c, sigs);
            }
        }
        Statement::Spawn(b) => {
            for s in &mut b.statements {
                coerce_stmt(s, sigs);
            }
        }
        Statement::Benchmark(b) => {
            for s in &mut b.statements {
                coerce_stmt(s, sigs);
            }
        }
        Statement::Unsafe(b) => {
            for s in &mut b.statements {
                coerce_stmt(s, sigs);
            }
        }
        _ => {}
    }
}

fn build_sig_map(program: &Program) -> HashMap<String, Function> {
    let mut map = HashMap::new();
    for f in &program.functions {
        if f.type_params.is_empty() {
            map.insert(f.name.clone(), f.clone());
        }
    }
    for imp in &program.impls {
        for m in &imp.methods {
            map.insert(m.name.clone(), m.clone());
        }
    }
    for ti in &program.trait_impls {
        for m in &ti.methods {
            map.insert(m.name.clone(), m.clone());
        }
    }
    map
}

/// Rewrite call sites so owned values are passed as references when the callee expects `&T` / `&mut T`.
pub fn coerce_auto_borrow(program: &mut Program) {
    let sigs = build_sig_map(program);
    for f in &mut program.functions {
        if !f.type_params.is_empty() {
            continue;
        }
        for stmt in &mut f.body.statements {
            coerce_stmt(stmt, &sigs);
        }
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            for stmt in &mut m.body.statements {
                coerce_stmt(stmt, &sigs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::Lexer;
    use parser::Parser;

    #[test]
    fn coerces_owned_binding_to_ref_param() {
        let src = r#"struct User {
    name: string
    age: i32
}
fn save(u: &User) -> void {
    print(u.age)
}
fn main() {
    let user = User { name: "Ahmed" age: 25 }
    save(user)
    print(user.age)
}"#;
        let (tokens, _) = Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        crate::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        coerce_auto_borrow(&mut program);
        const_eval::fold_program_consts(&mut program);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let call = match &main.body.statements[1] {
            Statement::Expression(Expression::Call(c)) => c,
            other => panic!("expected call stmt, got {other:?}"),
        };
        assert!(matches!(
            &call.args[0],
            Expression::Unary(u) if u.op == UnaryOp::Ref
        ));
        let mut tc = typecheck::TypeChecker::new();
        tc.check_program(&program);
        assert!(tc.errors.is_empty(), "{:?}", tc.errors);
    }
}
