//! Desugar `async fn` into spawn + promise handle (non-blocking call sites).

use ast::*;
use errors::Span;

use super::future_async::{future_complete_callee, future_struct_literal, future_struct_name_from_ann};

const HANDLE_PREFIX: &str = "__nyra_async_h_";

fn expr_call(callee: &str, args: Vec<Expression>, span: Span) -> Expression {
    Expression::Call(CallExpr {
        callee: callee.into(),
        type_args: vec![],
        args,
        span,
    })
}

fn expr_var(name: &str, span: Span) -> Expression {
    Expression::Variable {
        name: name.into(),
        span,
    }
}

fn expr_int(n: i32, _span: Span) -> Expression {
    Expression::Literal(Literal::Int(n as i64))
}

fn rewrite_block_returns(block: &mut Block, handle: &str, complete_fn: &str) {
    for stmt in &mut block.statements {
        *stmt = rewrite_stmt_returns(stmt, handle, complete_fn);
    }
}

fn rewrite_stmt_returns(stmt: &Statement, handle: &str, complete_fn: &str) -> Statement {
    match stmt {
        Statement::Return(r) => {
            let span = stmt_span(stmt);
            let value = r
                .value
                .clone()
                .unwrap_or_else(|| expr_int(0, span.clone()));
            Statement::Expression(expr_call(
                complete_fn,
                vec![expr_var(handle, span.clone()), value],
                span,
            ))
        }
        Statement::If(i) => {
            let mut then_block = i.then_block.clone();
            rewrite_block_returns(&mut then_block, handle, complete_fn);
            let else_block = i.else_block.as_ref().map(|b| {
                let mut eb = b.clone();
                rewrite_block_returns(&mut eb, handle, complete_fn);
                eb
            });
            Statement::If(IfStmt {
                condition: i.condition.clone(),
                then_block,
                else_block,
            })
        }
        Statement::While(w) => {
            let mut body = w.body.clone();
            rewrite_block_returns(&mut body, handle, complete_fn);
            Statement::While(WhileStmt {
                condition: w.condition.clone(),
                body,
            })
        }
        Statement::For(f) => {
            let mut nf = f.clone();
            rewrite_block_returns(&mut nf.body, handle, complete_fn);
            Statement::For(nf)
        }
        Statement::Spawn(s) => {
            let mut inner = s.body.clone();
            rewrite_block_returns(&mut inner, handle, complete_fn);
            Statement::Spawn(SpawnStmt {
                kind: s.kind,
                body: inner,
            })
        }
        Statement::Unsafe(b) | Statement::Benchmark(b) => {
            let mut inner = b.clone();
            rewrite_block_returns(&mut inner, handle, complete_fn);
            match stmt {
                Statement::Unsafe(_) => Statement::Unsafe(inner),
                _ => Statement::Benchmark(inner),
            }
        }
        other => other.clone(),
    }
}

fn desugar_one_async(func: &mut Function) {
    if !func.is_async || !func.type_params.is_empty() {
        return;
    }
    let span = func.span.clone();
    let handle = format!("{HANDLE_PREFIX}{}", func.name);
    let complete_fn = func
        .return_type
        .as_ref()
        .map(|ann| future_complete_callee(ann))
        .unwrap_or("async_promise_complete");
    let future_struct = func
        .return_type
        .as_ref()
        .and_then(future_struct_name_from_ann)
        .unwrap_or("Future_i32");

    let mut inner = func.body.clone();
    rewrite_block_returns(&mut inner, &handle, complete_fn);
    ensure_spawn_completes(&mut inner, &handle, complete_fn);
    let return_value = if func.exported {
        expr_var(&handle, span.clone())
    } else {
        future_struct_literal(&handle, future_struct, span.clone())
    };

    func.is_async = false;
    func.body = Block {
        statements: vec![
            Statement::Let(LetStmt {
                name: handle.clone(),
                mutable: false,
                destructure: vec![],
                span: span.clone(),
                ty: None,
                value: expr_call("async_promise_new", vec![], span.clone()),
            }),
            Statement::Spawn(SpawnStmt {
                kind: SpawnKind::Task,
                body: inner,
            }),
            Statement::Return(ReturnStmt {
                value: Some(return_value),
            }),
        ],
    };
}

fn ensure_spawn_completes(block: &mut Block, handle: &str, complete_fn: &str) {
    if block.statements.last().is_some_and(|s| matches!(s,
        Statement::Expression(Expression::Call(c)) if c.callee == complete_fn || c.callee == "async_promise_complete"
    )) {
        return;
    }
    let span = Span::default();
    block.statements.push(Statement::Expression(expr_call(
        complete_fn,
        vec![expr_var(handle, span.clone()), expr_int(0, span.clone())],
        span,
    )));
}

pub fn desugar_async_functions(program: &mut Program) {
    for f in &mut program.functions {
        desugar_one_async(f);
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            desugar_one_async(m);
        }
    }
    for ti in &mut program.trait_impls {
        for m in &mut ti.methods {
            desugar_one_async(m);
        }
    }
}
