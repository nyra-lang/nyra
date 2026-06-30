mod match_or;
mod async_desugar;
mod async_for_in;
mod async_state_machine;
mod future_async;
mod future_await;
mod future_structs;
mod arrows;
mod clone;
mod coerce;
mod main_argv;
mod nullish;
mod ownership_prefix;
mod struct_ctors;
mod struct_serde;
mod trait_objects;
mod vec_nested;
mod vec_pod;
mod vec_reloc;
mod serde_traits;
mod try_op;

pub use coerce::coerce_auto_borrow;
pub use clone::synthesize_clone_impls;
pub use serde_traits::synthesize_serde_trait_impls;
pub use struct_serde::synthesize_struct_json_helpers;
pub use vec_nested::synthesize_vec_nested_helpers;
pub use vec_pod::synthesize_vec_pod_helpers;
pub use vec_reloc::synthesize_vec_reloc_helpers;
pub use try_op::desugar_try;

use typecheck::TypeChecker;

use std::collections::HashMap;

use ast::*;
use arrows::desugar_arrows;
use match_or::desugar_match_or_patterns;
use nullish::{desugar_nullish, infer_nullish_option_types};

fn substitute_param(expr: &Expression, param: &str, arg: &Expression) -> Expression {
    match expr {
        Expression::Variable { name, span } if name == param => arg.clone(),
        Expression::Binary(b) => Expression::Binary(Box::new(BinaryExpr {
            left: substitute_param(&b.left, param, arg),
            op: b.op,
            right: substitute_param(&b.right, param, arg),
            span: b.span.clone(),
        })),
        Expression::Unary(u) => Expression::Unary(Box::new(UnaryExpr {
            op: u.op,
            operand: substitute_param(&u.operand, param, arg),
            span: u.span.clone(),
        })),
        Expression::Grouped(inner) => {
            Expression::Grouped(Box::new(substitute_param(inner, param, arg)))
        }
        Expression::Call(c) => Expression::Call(CallExpr {
            args: c
                .args
                .iter()
                .map(|a| substitute_param(a, param, arg))
                .collect(),
            ..c.clone()
        }),
        other => other.clone(),
    }
}

fn expand_macro_body(body: &Expression, params: &[String], args: &[Expression]) -> Expression {
    let mut out = body.clone();
    for (p, a) in params.iter().zip(args.iter()) {
        out = substitute_param(&out, p, a);
    }
    out
}

fn expand_expr(expr: &Expression, macros: &HashMap<String, MacroDef>) -> Expression {
    match expr {
        Expression::Call(c) if macros.contains_key(&c.callee) => {
            let m = &macros[&c.callee];
            expand_macro_body(&m.body, &m.params, &c.args)
        }
        Expression::Binary(b) => Expression::Binary(Box::new(BinaryExpr {
            left: expand_expr(&b.left, macros),
            op: b.op,
            right: expand_expr(&b.right, macros),
            span: b.span.clone(),
        })),
        Expression::Unary(u) => Expression::Unary(Box::new(UnaryExpr {
            op: u.op,
            operand: expand_expr(&u.operand, macros),
            span: u.span.clone(),
        })),
        Expression::Grouped(inner) => Expression::Grouped(Box::new(expand_expr(inner, macros))),
        Expression::Call(c) => Expression::Call(CallExpr {
            args: c.args.iter().map(|a| expand_expr(a, macros)).collect(),
            ..c.clone()
        }),
        Expression::TemplateLiteral(t) => Expression::TemplateLiteral(TemplateLiteralExpr {
            parts: t
                .parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Static(s) => TemplatePart::Static(s.clone()),
                    TemplatePart::Interpolation(e) => {
                        TemplatePart::Interpolation(Box::new(expand_expr(e, macros)))
                    }
                })
                .collect(),
            span: t.span.clone(),
        }),
        other => other.clone(),
    }
}

fn expand_stmt(stmt: &Statement, macros: &HashMap<String, MacroDef>) -> Statement {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let mut s = stmt.clone();
            if let Statement::Let(ref mut x) | Statement::Const(ref mut x) = s {
                x.value = expand_expr(&l.value, macros);
            }
            s
        }
        Statement::Assign(a) => Statement::Assign(AssignStmt {
            target: expand_expr(&a.target, macros),
            value: expand_expr(&a.value, macros),
            span: a.span.clone(),
        }),
        Statement::Return(r) => Statement::Return(ReturnStmt {
            value: r.value.as_ref().map(|v| expand_expr(v, macros)),
        }),
        Statement::Expression(e) => Statement::Expression(expand_expr(e, macros)),
        Statement::Print(p) => Statement::Print(p.clone().map_expressions(|a| expand_expr(&a, macros))),
        Statement::Defer(e) => Statement::Defer(expand_expr(e, macros)),
            Statement::If(i) => {
            let mut then_block = i.then_block.clone();
            expand_block_stmts(&mut then_block.statements, macros);
            let else_block = i.else_block.as_ref().map(|b| {
                let mut eb = b.clone();
                expand_block_stmts(&mut eb.statements, macros);
                eb
            });
            Statement::If(IfStmt {
                condition: expand_expr(&i.condition, macros),
                then_block,
                else_block,
            })
        }
        Statement::While(w) => {
            let mut body = w.body.clone();
            expand_block_stmts(&mut body.statements, macros);
            Statement::While(WhileStmt {
                condition: expand_expr(&w.condition, macros),
                body,
            })
        }
        Statement::For(f) => {
            let mut nf = f.clone();
            expand_block_stmts(&mut nf.body.statements, macros);
            nf.map_exprs_mut(|e| *e = expand_expr(e, macros));
            Statement::For(nf)
        }
        Statement::Spawn(body) => {
            let mut b = body.clone();
            expand_block_stmts(&mut b.statements, macros);
            Statement::Spawn(b)
        }
        Statement::Benchmark(body) => {
            let mut b = body.clone();
            expand_block_stmts(&mut b.statements, macros);
            Statement::Benchmark(b)
        }
        Statement::Unsafe(body) => {
            let mut b = body.clone();
            expand_block_stmts(&mut b.statements, macros);
            Statement::Unsafe(b)
        }
        Statement::Asm { .. } => stmt.clone(),
        other => other.clone(),
    }
}

fn expand_block_stmts(stmts: &mut [Statement], macros: &HashMap<String, MacroDef>) {
    for stmt in stmts.iter_mut() {
        *stmt = expand_stmt(stmt, macros);
    }
}

fn expand_function_body(body: &mut Block, macros: &HashMap<String, MacroDef>) {
    expand_block_stmts(&mut body.statements, macros);
}

pub fn expand_program(program: &mut Program) {
    desugar_arrows(program);
    desugar_match_or_patterns(program);
    main_argv::desugar_main_argv(program);
    ownership_prefix::desugar_clone_prefix(program);
    struct_ctors::desugar_struct_constructors(program);
    desugar_match_or_patterns(program);
    trait_objects::synthesize_trait_object_structs(program);
    future_structs::synthesize_future_structs(program);
    clone::synthesize_clone_impls(program);
    infer_nullish_option_types(program);
    desugar_nullish(program);
    let map: HashMap<String, MacroDef> = program
        .macros
        .iter()
        .cloned()
        .map(|m| (m.name.clone(), m))
        .collect();
    for f in &mut program.functions {
        expand_function_body(&mut f.body, &map);
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            expand_function_body(&mut m.body, &map);
        }
    }
    for ti in &mut program.trait_impls {
        for m in &mut ti.methods {
            expand_function_body(&mut m.body, &map);
        }
    }
}

/// Post-typecheck async pipeline: for-in desugar, state-machine retry, blocking fallback.
pub fn finish_async_desugar(program: &mut Program, checker: &TypeChecker) {
    async_for_in::desugar_async_for_in_loops(program, checker);
    async_state_machine::desugar_async_state_machines(program, checker);
    async_desugar::desugar_async_functions(program);
    future_async::patch_desugared_async_returns(program);
    future_await::desugar_future_await(program, checker);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast::{Expression, Literal, Statement, UnaryOp};
    use coerce::coerce_auto_borrow;

    fn parse_and_expand(src: &str) -> Program {
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        expand_program(&mut program);
        program
    }

    #[test]
    fn expands_macro_call_in_expression() {
        let src = r#"macro twice(x) { x + x }
fn main() {
    let n = twice(5)
    print(n)
}"#;
        let program = parse_and_expand(src);
        let main = &program.functions[0];
        let let_stmt = main.body.statements.iter().find_map(|s| match s {
            Statement::Let(l) if l.name == "n" => Some(l),
            _ => None,
        });
        let value = let_stmt.expect("let n").value.clone();
        match value {
            Expression::Binary(b) => {
                assert!(matches!(b.left, Expression::Literal(Literal::Int(5))));
                assert!(matches!(b.right, Expression::Literal(Literal::Int(5))));
            }
            other => panic!("expected binary add after macro expand, got {other:?}"),
        }
    }

    #[test]
    fn expands_two_param_macro() {
        let src = r#"macro add(a, b) { a + b }
fn main() {
    let n = add(2, 3)
    print(n)
}"#;
        let program = parse_and_expand(src);
        let main = &program.functions[0];
        let let_stmt = main.body.statements.iter().find_map(|s| match s {
            Statement::Let(l) if l.name == "n" => Some(l),
            _ => None,
        });
        match &let_stmt.expect("let n").value {
            Expression::Binary(b) => {
                assert!(matches!(b.left, Expression::Literal(Literal::Int(2))));
                assert!(matches!(b.right, Expression::Literal(Literal::Int(3))));
            }
            other => panic!("expected add(2,3), got {other:?}"),
        }
    }

    #[test]
    fn expands_macro_in_if_block() {
        let src = r#"macro inc(x) { x + 1 }
fn main() {
    if true {
        let n = inc(4)
        print(n)
    }
}"#;
        let program = parse_and_expand(src);
        let main = &program.functions[0];
        let if_stmt = match &main.body.statements[0] {
            Statement::If(i) => i,
            other => panic!("expected if, got {other:?}"),
        };
        let let_stmt = if_stmt.then_block.statements.iter().find_map(|s| match s {
            Statement::Let(l) if l.name == "n" => Some(l),
            _ => None,
        });
        match &let_stmt.expect("let n").value {
            Expression::Binary(b) => {
                assert!(matches!(b.left, Expression::Literal(Literal::Int(4))));
            }
            other => panic!("expected inc(4), got {other:?}"),
        }
    }

    #[test]
    fn leaves_non_macro_calls_unchanged() {
        let src = r#"fn main() { print(1) }"#;
        let program = parse_and_expand(src);
        assert!(matches!(
            program.functions[0].body.statements[0],
            Statement::Print(_)
        ));
    }

    #[test]
    fn substring_reuses_binding_via_auto_borrow() {
        let src = r#"extern fn substring(s: &string, start: i32, len: i32) -> string
fn main() {
    let s = "abcdef"
    let a = substring(s, 0, 2)
    let b = substring(s, 2, 2)
    print(a)
    print(b)
}"#;
        let mut program = parse_and_expand(src);
        monomorph::monomorphize_program(&mut program);
        coerce_auto_borrow(&mut program);
        let main = &program.functions[0];
        let substring_calls: Vec<_> = main
            .body
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::Let(l) => Some(&l.value),
                _ => None,
            })
            .filter(|v| matches!(v, Expression::Call(c) if c.callee == "substring"))
            .collect();
        assert_eq!(substring_calls.len(), 2);
        for call in substring_calls {
            if let Expression::Call(c) = call {
                assert!(
                    matches!(
                        &c.args[0],
                        Expression::Unary(u) if u.op == UnaryOp::Ref
                    ),
                    "substring should auto-borrow source binding"
                );
            }
        }
    }

    #[test]
    fn desugars_arrow_fn_to_hoisted_function() {
        let src = r#"fn main() {
    let f = (x: i32) => x + 1
    print(f(41))
}"#;
        let program = parse_and_expand(src);
        let arrow_fn = program.functions.iter().find(|f| f.name.starts_with("__arrow_"));
        assert!(arrow_fn.is_some(), "expected hoisted __arrow_* function");
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let let_stmt = main.body.statements.iter().find_map(|s| match s {
            Statement::Let(l) if l.name == "f" => Some(l),
            _ => None,
        });
        assert!(matches!(
            let_stmt.expect("let f").value,
            Expression::Variable { ref name, .. } if name.starts_with("__arrow_")
        ));
    }
}

#[cfg(test)]
mod async_is_async_tests {
    use super::*;
    #[test]
    fn give_keeps_is_async_after_expand() {
        let src = r#"async fn give() -> i32 { return 7 }"#;
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        assert!(program.functions[0].is_async);
        expand_program(&mut program);
        assert!(program.functions[0].is_async, "expand stripped is_async");
    }
}
