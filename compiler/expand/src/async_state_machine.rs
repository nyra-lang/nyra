//! Cooperative async state-machine desugar for `async fn` with `await`.
//!
//! Bodies with `await` compile to poll states (`async_poll` + `runtime_executor_tick`).
//! Supports top-level `await`, `if`/`while`/`for`, and `await` inside `spawn`/`unsafe`.

use ast::*;
use errors::Span;

use super::future_async::{
    await_handle_expr, await_result_zero_expr, bind_let_type, future_complete_callee,
    future_struct_literal, future_struct_name_from_ann, poll_kind_for_expr, poll_kind_for_return_ann,
    PollKind,
};
use std::collections::HashMap;
use types::Type;
use typecheck::TypeChecker;

const STATE_VAR: &str = "__nyra_async_state";
const HANDLE_PREFIX: &str = "__nyra_async_h_";
const COMPLETE_EXIT: i32 = -1;

fn expr_has_await(expr: &Expression) -> bool {
    match expr {
        Expression::Await(_) => true,
        Expression::Binary(b) => expr_has_await(&b.left) || expr_has_await(&b.right),
        Expression::Unary(u) => expr_has_await(&u.operand),
        Expression::Grouped(g) => expr_has_await(g),
        Expression::If(i) => {
            expr_has_await(&i.condition)
                || block_has_await(&i.then_block)
                || block_has_await(&i.else_block)
        }
        Expression::Match(m) => {
            expr_has_await(&m.scrutinee)
                || m.arms.iter().any(|a| {
                    a.guard.as_ref().is_some_and(expr_has_await)
                        || block_has_await(&a.body)
                })
        }
        Expression::Call(c) => c.args.iter().any(expr_has_await),
        Expression::MethodCall(m) => {
            expr_has_await(&m.object) || m.args.iter().any(expr_has_await)
        }
        Expression::FieldAccess(f) => expr_has_await(&f.object),
        Expression::Index(ix) => expr_has_await(&ix.object) || expr_has_await(&ix.index),
        Expression::StructLiteral(s) => {
            s.spreads.iter().any(expr_has_await)
                || s.fields.iter().any(|(_, e)| expr_has_await(e))
        }
        Expression::ArrayLiteral(al) => al.all_exprs().any(expr_has_await),
        Expression::TupleLiteral(elems) => elems.iter().any(expr_has_await),
        Expression::ArrayRepeat { element, .. } => expr_has_await(element),
        Expression::TemplateLiteral(t) => t.parts.iter().any(|p| {
            matches!(p, TemplatePart::Interpolation(e) if expr_has_await(e))
        }),
        Expression::Cast(c) => expr_has_await(&c.expr),
        Expression::ArrowFn(a) => match &a.body {
            ArrowBody::Expr(e) => expr_has_await(e),
            ArrowBody::Block(b) => block_has_await(b),
        },
        _ => false,
    }
}

fn block_has_await(block: &Block) -> bool {
    block.statements.iter().any(stmt_has_await)
}

fn stmt_has_await(stmt: &Statement) -> bool {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => expr_has_await(&l.value),
        Statement::Assign(a) => expr_has_await(&a.target) || expr_has_await(&a.value),
        Statement::Return(r) => r.value.as_ref().is_some_and(expr_has_await),
        Statement::Expression(e) | Statement::Defer(e) => expr_has_await(e),
        Statement::If(i) => {
            expr_has_await(&i.condition)
                || block_has_await(&i.then_block)
                || i.else_block.as_ref().is_some_and(block_has_await)
        }
        Statement::While(w) => expr_has_await(&w.condition) || block_has_await(&w.body),
        Statement::For(f) => {
            (match &f.kind {
                ForKind::Range { start, end } => expr_has_await(start) || expr_has_await(end),
                ForKind::Iterable { iterable } => expr_has_await(iterable),
            }) || block_has_await(&f.body)
        }
        Statement::Spawn(s) => block_has_await(&s.body),
        Statement::Unsafe(b) | Statement::Benchmark(b) => block_has_await(b),
        _ => false,
    }
}

fn wrap_spawn_block(kind: SpawnKind, body: Block) -> Statement {
    Statement::Spawn(SpawnStmt { kind, body })
}

fn lower_nested_async_block(
    builder: &mut CfgBuilder,
    block: &Block,
    exit: i32,
    wrap: impl FnOnce(Block) -> Statement,
    checker: &TypeChecker,
) -> Option<Statement> {
    let mut nested = CfgBuilder::new(
        builder.handle.clone(),
        builder.span.clone(),
        builder.complete_fn.clone(),
        builder.result_kind,
    );
    nested.finish_promise = false;
    nested.local_types = builder.local_types.clone();
    let (entry, _) = nested.lower_block(block, exit, checker)?;
    let hoisted = std::mem::take(&mut nested.hoisted);
    let inner = nested.build_spawn_body(entry);
    builder.hoisted.extend(hoisted);
    Some(wrap(inner))
}

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

fn expr_int(n: i64, span: Span) -> Expression {
    Expression::Literal(Literal::Int(n))
}

fn stmt_expr(expr: Expression) -> Statement {
    Statement::Expression(expr)
}

fn stmt_let(name: &str, value: Expression, span: Span) -> Statement {
    stmt_let_mut(name, value, span, false)
}

fn stmt_let_mut(name: &str, value: Expression, span: Span, mutable: bool) -> Statement {
    Statement::Let(LetStmt {
        name: name.into(),
        mutable,
        destructure: vec![],
        span,
        ty: None,
        value,
    })
}

fn stmt_if(condition: Expression, then_block: Block, else_block: Option<Block>, span: Span) -> Statement {
    Statement::If(IfStmt {
        condition,
        then_block,
        else_block,
    })
}

fn stmt_while(condition: Expression, body: Block) -> Statement {
    Statement::While(WhileStmt { condition, body })
}

fn stmt_assign(name: &str, value: Expression, span: Span) -> Statement {
    Statement::Assign(AssignStmt {
        target: expr_var(name, span.clone()),
        span,
        value,
    })
}

fn build_if_chain(states: &[(Expression, Block)], span: Span) -> Statement {
    let (cond, body) = &states[0];
    if states.len() == 1 {
        return Statement::If(IfStmt {
            condition: cond.clone(),
            then_block: body.clone(),
            else_block: None,
        });
    }
    let else_block = Block {
        statements: vec![build_if_chain(&states[1..], span)],
    };
    Statement::If(IfStmt {
        condition: cond.clone(),
        then_block: body.clone(),
        else_block: Some(else_block),
    })
}

/// Strip `await` wrapper; returns inner expression.
fn unwrap_await(expr: &Expression) -> Expression {
    match expr {
        Expression::Await(inner) => *inner.clone(),
        other => other.clone(),
    }
}

/// Lift nested `await` expressions into preceding `let` bindings so the CFG
/// lowerer only sees statement-level `let x = await e` / bare `await e`.
fn hoist_awaits_in_expr(
    expr: Expression,
    counter: &mut usize,
    out: &mut Vec<Statement>,
    span: &Span,
) -> Expression {
    match expr {
        Expression::Await(inner) => {
            let inner = hoist_awaits_in_expr(*inner, counter, out, span);
            let name = format!("__nyra_hoist_aw_{counter}");
            *counter += 1;
            out.push(stmt_let(
                &name,
                Expression::Await(Box::new(inner)),
                span.clone(),
            ));
            expr_var(&name, span.clone())
        }
        Expression::Binary(mut b) => {
            b.left = hoist_awaits_in_expr(b.left, counter, out, span);
            b.right = hoist_awaits_in_expr(b.right, counter, out, span);
            Expression::Binary(b)
        }
        Expression::Unary(mut u) => {
            u.operand = hoist_awaits_in_expr(u.operand, counter, out, span);
            Expression::Unary(u)
        }
        Expression::Grouped(g) => {
            Expression::Grouped(Box::new(hoist_awaits_in_expr(*g, counter, out, span)))
        }
        Expression::Call(mut c) => {
            c.args = c
                .args
                .into_iter()
                .map(|a| hoist_awaits_in_expr(a, counter, out, span))
                .collect();
            Expression::Call(c)
        }
        Expression::MethodCall(mut m) => {
            m.object = hoist_awaits_in_expr(m.object, counter, out, span);
            m.args = m
                .args
                .into_iter()
                .map(|a| hoist_awaits_in_expr(a, counter, out, span))
                .collect();
            Expression::MethodCall(m)
        }
        Expression::FieldAccess(mut f) => {
            f.object = hoist_awaits_in_expr(f.object, counter, out, span);
            Expression::FieldAccess(f)
        }
        Expression::Index(mut ix) => {
            ix.object = hoist_awaits_in_expr(ix.object, counter, out, span);
            ix.index = hoist_awaits_in_expr(ix.index, counter, out, span);
            Expression::Index(ix)
        }
        Expression::Cast(mut c) => {
            c.expr = hoist_awaits_in_expr(c.expr, counter, out, span);
            Expression::Cast(c)
        }
        Expression::StructLiteral(mut s) => {
            s.spreads = s
                .spreads
                .into_iter()
                .map(|e| hoist_awaits_in_expr(e, counter, out, span))
                .collect();
            s.fields = s
                .fields
                .into_iter()
                .map(|(n, e)| (n, hoist_awaits_in_expr(e, counter, out, span)))
                .collect();
            Expression::StructLiteral(s)
        }
        Expression::TupleLiteral(elems) => Expression::TupleLiteral(
            elems
                .into_iter()
                .map(|e| hoist_awaits_in_expr(e, counter, out, span))
                .collect(),
        ),
        Expression::ArrayLiteral(mut al) => {
            for e in al.spreads.iter_mut().chain(al.elems.iter_mut()) {
                let taken = std::mem::replace(e, Expression::Literal(Literal::Int(0)));
                *e = hoist_awaits_in_expr(taken, counter, out, span);
            }
            Expression::ArrayLiteral(al)
        }
        Expression::ArrayRepeat {
            element,
            count,
            count_from,
            count_expr,
            span: arr_span,
        } => Expression::ArrayRepeat {
            element: Box::new(hoist_awaits_in_expr(*element, counter, out, span)),
            count,
            count_from,
            count_expr: count_expr.map(|e| Box::new(hoist_awaits_in_expr(*e, counter, out, span))),
            span: arr_span,
        },
        other => other,
    }
}

fn hoist_nested_awaits_in_block(block: &mut Block, counter: &mut usize, span: &Span) {
    let mut rewritten = Vec::with_capacity(block.statements.len());
    for stmt in std::mem::take(&mut block.statements) {
        match stmt {
            Statement::Let(mut l) if expr_has_await(&l.value) && !matches!(l.value, Expression::Await(_)) => {
                let mut prefix = Vec::new();
                l.value = hoist_awaits_in_expr(l.value, counter, &mut prefix, span);
                rewritten.extend(prefix);
                rewritten.push(Statement::Let(l));
            }
            Statement::Const(mut l) if expr_has_await(&l.value) && !matches!(l.value, Expression::Await(_)) => {
                let mut prefix = Vec::new();
                l.value = hoist_awaits_in_expr(l.value, counter, &mut prefix, span);
                rewritten.extend(prefix);
                rewritten.push(Statement::Const(l));
            }
            Statement::Assign(mut a) if expr_has_await(&a.value) => {
                let mut prefix = Vec::new();
                a.value = hoist_awaits_in_expr(a.value, counter, &mut prefix, span);
                rewritten.extend(prefix);
                rewritten.push(Statement::Assign(a));
            }
            Statement::Expression(e) if expr_has_await(&e) && !matches!(e, Expression::Await(_)) => {
                let mut prefix = Vec::new();
                let e = hoist_awaits_in_expr(e, counter, &mut prefix, span);
                rewritten.extend(prefix);
                rewritten.push(Statement::Expression(e));
            }
            Statement::Return(mut r) => {
                if let Some(v) = r.value.take() {
                    if expr_has_await(&v) && !matches!(v, Expression::Await(_)) {
                        let mut prefix = Vec::new();
                        r.value = Some(hoist_awaits_in_expr(v, counter, &mut prefix, span));
                        rewritten.extend(prefix);
                    } else {
                        r.value = Some(v);
                    }
                }
                rewritten.push(Statement::Return(r));
            }
            Statement::If(mut i) => {
                if expr_has_await(&i.condition) {
                    let mut prefix = Vec::new();
                    i.condition = hoist_awaits_in_expr(i.condition, counter, &mut prefix, span);
                    rewritten.extend(prefix);
                }
                hoist_nested_awaits_in_block(&mut i.then_block, counter, span);
                if let Some(eb) = i.else_block.as_mut() {
                    hoist_nested_awaits_in_block(eb, counter, span);
                }
                rewritten.push(Statement::If(i));
            }
            Statement::While(mut w) => {
                hoist_nested_awaits_in_block(&mut w.body, counter, span);
                rewritten.push(Statement::While(w));
            }
            Statement::For(mut f) => {
                hoist_nested_awaits_in_block(&mut f.body, counter, span);
                rewritten.push(Statement::For(f));
            }
            Statement::Spawn(mut s) => {
                hoist_nested_awaits_in_block(&mut s.body, counter, span);
                rewritten.push(Statement::Spawn(s));
            }
            Statement::Unsafe(mut b) => {
                hoist_nested_awaits_in_block(&mut b, counter, span);
                rewritten.push(Statement::Unsafe(b));
            }
            other => rewritten.push(other),
        }
    }
    block.statements = rewritten;
}

struct CfgBuilder {
    states: Vec<(i32, Vec<Statement>)>,
    hoisted: Vec<Statement>,
    next_id: i32,
    poll_counter: i32,
    span: Span,
    handle: String,
    complete_fn: String,
    result_kind: PollKind,
    local_types: HashMap<String, Type>,
    /// When false (nested `spawn`/`unsafe` poll loops), exit without completing the outer promise.
    finish_promise: bool,
}

impl CfgBuilder {
    fn new(handle: String, span: Span, complete_fn: String, result_kind: PollKind) -> Self {
        Self {
            states: Vec::new(),
            hoisted: Vec::new(),
            next_id: 0,
            poll_counter: 0,
            span,
            handle,
            complete_fn,
            result_kind,
            local_types: HashMap::new(),
            finish_promise: true,
        }
    }

    fn track_let(&mut self, name: &str, value: &Expression, checker: &TypeChecker) {
        bind_let_type(name, value, checker, &mut self.local_types);
    }

    fn alloc_state(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn push_state(&mut self, id: i32, mut stmts: Vec<Statement>) {
        if stmts.is_empty() {
            return;
        }
        if let Some((last_id, prev)) = self.states.last_mut() {
            if *last_id == id {
                prev.append(&mut stmts);
                return;
            }
        }
        self.states.push((id, stmts));
    }

    fn goto_state(&self, target: i32) -> Statement {
        stmt_assign(STATE_VAR, expr_int(target as i64, self.span.clone()), self.span.clone())
    }

    fn finish(&self, value: Expression) -> Vec<Statement> {
        if self.finish_promise {
            self.complete(value)
        } else {
            vec![self.goto_state(COMPLETE_EXIT)]
        }
    }

    fn complete(&self, value: Expression) -> Vec<Statement> {
        vec![
            stmt_expr(expr_call(
                &self.complete_fn,
                vec![expr_var(&self.handle, self.span.clone()), value],
                self.span.clone(),
            )),
            self.goto_state(COMPLETE_EXIT),
        ]
    }

    fn emit_await(
        &mut self,
        state_id: i32,
        inner: Expression,
        bind: Option<&str>,
        next_state: i32,
        checker: &TypeChecker,
    ) {
        let hname = format!("__nyra_await_h_{}", self.poll_counter);
        self.poll_counter += 1;
        let poll_kind = poll_kind_for_expr(&inner, checker, &self.local_types);
        let handle_expr = await_handle_expr(&inner, checker, &self.local_types, self.span.clone());
        let poll_state = self.alloc_state();
        self.hoisted.push(stmt_let_mut(
            &hname,
            expr_int(0, self.span.clone()),
            self.span.clone(),
            true,
        ));
        self.push_state(
            state_id,
            vec![
                stmt_assign(&hname, handle_expr, self.span.clone()),
                self.goto_state(poll_state),
            ],
        );
        let stmts = build_poll_state(
            poll_state,
            next_state,
            &hname,
            bind,
            self.span.clone(),
            poll_kind,
        );
        self.push_state(poll_state, stmts);
    }

    fn lower_block(
        &mut self,
        block: &Block,
        exit: i32,
        checker: &TypeChecker,
    ) -> Option<(i32, bool)> {
        let entry = self.alloc_state();
        let mut cur = entry;
        let mut prefix: Vec<Statement> = Vec::new();

        let flush = |prefix: &mut Vec<Statement>| -> Vec<Statement> {
            if prefix.is_empty() {
                Vec::new()
            } else {
                std::mem::take(prefix)
            }
        };

        for stmt in &block.statements {
            match stmt {
                Statement::If(i)
                    if block_has_await(&i.then_block)
                        || i.else_block.as_ref().is_some_and(block_has_await) =>
                {
                    self.push_state(cur, flush(&mut prefix));
                    let merge = self.alloc_state();
                    let (then_e, then_ret) = self.lower_block(&i.then_block, merge, checker)?;
                    let (else_e, else_ret) = match &i.else_block {
                        Some(eb) => {
                            let (e, r) = self.lower_block(eb, merge, checker)?;
                            (e, r)
                        }
                        None => (merge, false),
                    };
                    self.push_state(
                        cur,
                        vec![stmt_if(
                            i.condition.clone(),
                            Block {
                                statements: vec![self.goto_state(then_e)],
                            },
                            Some(Block {
                                statements: vec![self.goto_state(else_e)],
                            }),
                            self.span.clone(),
                        )],
                    );
                    if then_ret && else_ret {
                        return Some((entry, true));
                    }
                    cur = merge;
                }
                Statement::While(w) if block_has_await(&w.body) => {
                    self.hoisted.extend(flush(&mut prefix));
                    let loop_head = self.alloc_state();
                    let after = self.alloc_state();
                    self.push_state(cur, vec![self.goto_state(loop_head)]);
                    let body_e = self.lower_block(&w.body, loop_head, checker)?.0;
                    self.push_state(
                        loop_head,
                        vec![stmt_if(
                            w.condition.clone(),
                            Block {
                                statements: vec![self.goto_state(body_e)],
                            },
                            Some(Block {
                                statements: vec![self.goto_state(after)],
                            }),
                            self.span.clone(),
                        )],
                    );
                    cur = after;
                }
                Statement::For(f)
                    if matches!(f.kind, ForKind::Iterable { .. }) && block_has_await(&f.body) =>
                {
                    return None;
                }
                Statement::For(f)
                    if matches!(f.kind, ForKind::Range { .. })
                        && f.parallel.is_none()
                        && f.progress.is_none()
                        && block_has_await(&f.body) =>
                {
                    // Hoist setup lets (e.g. `let __nyra_iter_n = arr` from for-in desugar) outside
                    // the poll `while` — same as `while` with await; otherwise loop env restore
                    // drops bindings after the first poll tick.
                    self.hoisted.extend(flush(&mut prefix));
                    let ForKind::Range { start, end } = &f.kind else {
                        return None;
                    };
                    let loop_head = self.alloc_state();
                    let after = self.alloc_state();
                    let inc_state = self.alloc_state();
                    self.hoisted.push(stmt_let_mut(
                        &f.var,
                        start.clone(),
                        self.span.clone(),
                        true,
                    ));
                    self.push_state(cur, vec![self.goto_state(loop_head)]);
                    let body_e = self.lower_block(&f.body, inc_state, checker)?.0;
                    self.push_state(
                        inc_state,
                        vec![
                            stmt_assign(
                                &f.var,
                                Expression::Binary(Box::new(BinaryExpr {
                                    left: expr_var(&f.var, self.span.clone()),
                                    op: BinaryOp::Add,
                                    right: expr_int(1, self.span.clone()),
                                    span: self.span.clone(),
                                })),
                                self.span.clone(),
                            ),
                            self.goto_state(loop_head),
                        ],
                    );
                    self.push_state(
                        loop_head,
                        vec![stmt_if(
                            Expression::Binary(Box::new(BinaryExpr {
                                left: expr_var(&f.var, self.span.clone()),
                                op: BinaryOp::Lt,
                                right: end.clone(),
                                span: self.span.clone(),
                            })),
                            Block {
                                statements: vec![self.goto_state(body_e)],
                            },
                            Some(Block {
                                statements: vec![self.goto_state(after)],
                            }),
                            self.span.clone(),
                        )],
                    );
                    cur = after;
                }
                Statement::Spawn(sp) if block_has_await(&sp.body) => {
                    self.push_state(cur, flush(&mut prefix));
                    let after = self.alloc_state();
                    let kind = sp.kind;
                    let wrapped = lower_nested_async_block(
                        self,
                        &sp.body,
                        COMPLETE_EXIT,
                        |inner| wrap_spawn_block(kind, inner),
                        checker,
                    )?;
                    self.push_state(cur, vec![wrapped, self.goto_state(after)]);
                    cur = after;
                }
                Statement::Unsafe(b) if block_has_await(b) => {
                    self.push_state(cur, flush(&mut prefix));
                    let after = self.alloc_state();
                    let wrapped =
                        lower_nested_async_block(self, b, COMPLETE_EXIT, Statement::Unsafe, checker)?;
                    self.push_state(cur, vec![wrapped, self.goto_state(after)]);
                    cur = after;
                }
                Statement::Let(l) if matches!(&l.value, Expression::Await(_)) => {
                    self.push_state(cur, flush(&mut prefix));
                    let inner = unwrap_await(&l.value);
                    let next = self.alloc_state();
                    let bind = if l.name == "_" {
                        None
                    } else {
                        Some(l.name.as_str())
                    };
                    self.emit_await(cur, inner, bind, next, checker);
                    cur = next;
                }
                Statement::Return(r) => {
                    self.push_state(cur, flush(&mut prefix));
                    if let Some(Expression::Await(inner)) = &r.value {
                        let next = self.alloc_state();
                        self.emit_await(cur, *inner.clone(), None, next, checker);
                        self.push_state(next, self.finish(expr_var("__nyra_await_result", self.span.clone())));
                    } else {
                        let ret = r
                            .value
                            .clone()
                            .unwrap_or_else(|| expr_int(0, self.span.clone()));
                        self.push_state(cur, self.finish(ret));
                    }
                    return Some((entry, true));
                }
                Statement::Expression(e) if expr_has_await(e) => {
                    if let Expression::Await(inner) = e {
                        self.push_state(cur, flush(&mut prefix));
                        let next = self.alloc_state();
                        self.emit_await(cur, *inner.clone(), None, next, checker);
                        cur = next;
                    } else {
                        return None;
                    }
                }
                other => {
                    if let Statement::Let(l) | Statement::Const(l) = stmt {
                        self.track_let(&l.name, &l.value, checker);
                    }
                    prefix.push(other.clone());
                }
            }
        }

        self.push_state(cur, flush(&mut prefix));
        if cur != exit {
            if exit == COMPLETE_EXIT {
                self.push_state(cur, self.finish(await_result_zero_expr(self.result_kind, self.span.clone())));
            } else {
                self.push_state(cur, vec![self.goto_state(exit)]);
            }
        }
        Some((entry, false))
    }

    fn build_spawn_body(self, entry: i32) -> Block {
        let states: Vec<(Expression, Block)> = self
            .states
            .into_iter()
            .map(|(id, stmts)| {
                (
                    Expression::Binary(Box::new(BinaryExpr {
                        left: expr_var(STATE_VAR, self.span.clone()),
                        op: BinaryOp::Eq,
                        right: expr_int(id as i64, self.span.clone()),
                        span: self.span.clone(),
                    })),
                    Block { statements: stmts },
                )
            })
            .collect();
        let running = Expression::Binary(Box::new(BinaryExpr {
            left: expr_var(STATE_VAR, self.span.clone()),
            op: BinaryOp::Ge,
            right: expr_int(0, self.span.clone()),
            span: self.span.clone(),
        }));
        Block {
            statements: {
                let mut top = self.hoisted;
                top.extend([
                stmt_let_mut(
                    STATE_VAR,
                    expr_int(entry as i64, self.span.clone()),
                    self.span.clone(),
                    true,
                ),
                stmt_let_mut(
                    "__nyra_await_result",
                    await_result_zero_expr(self.result_kind, self.span.clone()),
                    self.span.clone(),
                    true,
                ),
                stmt_while(
                    running,
                    Block {
                        statements: vec![build_if_chain(&states, self.span.clone())],
                    },
                ),
                ]);
                top
            },
        }
    }
}

fn build_poll_state(
    state_id: i32,
    next_state: i32,
    handle_var: &str,
    bind_to: Option<&str>,
    span: Span,
    kind: PollKind,
) -> Vec<Statement> {
    match kind {
        PollKind::String => build_string_poll_state(state_id, next_state, handle_var, bind_to, span),
        PollKind::Bool => build_scalar_poll_state(
            state_id,
            next_state,
            handle_var,
            bind_to,
            span,
            "async_poll_bool",
            PollKind::Bool,
        ),
        PollKind::I32 => build_scalar_poll_state(
            state_id,
            next_state,
            handle_var,
            bind_to,
            span,
            "async_poll",
            PollKind::I32,
        ),
    }
}

fn poll_wait_or_advance(
    state_id: i32,
    next_state: i32,
    not_ready: Expression,
    span: Span,
    ready_stmts: Vec<Statement>,
) -> Statement {
    let mut on_ready = ready_stmts;
    on_ready.push(stmt_assign(
        STATE_VAR,
        expr_int(next_state as i64, span.clone()),
        span.clone(),
    ));
    stmt_if(
        not_ready,
        Block {
            statements: vec![
                stmt_expr(expr_call(
                    "runtime_executor_tick",
                    vec![expr_int(10, span.clone())],
                    span.clone(),
                )),
                stmt_assign(
                    STATE_VAR,
                    expr_int(state_id as i64, span.clone()),
                    span.clone(),
                ),
            ],
        },
        Some(Block {
            statements: on_ready,
        }),
        span,
    )
}

fn assign_poll_result(
    poll_var: &str,
    bind_to: Option<&str>,
    span: Span,
    kind: PollKind,
) -> Vec<Statement> {
    let raw = expr_var(poll_var, span.clone());
    let value = if kind == PollKind::Bool {
        Expression::Binary(Box::new(BinaryExpr {
            left: raw,
            op: BinaryOp::Ne,
            right: expr_int(0, span.clone()),
            span: span.clone(),
        }))
    } else {
        raw
    };
    if let Some(name) = bind_to.filter(|n| *n != "_") {
        vec![stmt_assign(name, value, span)]
    } else if bind_to.is_none() {
        vec![stmt_assign("__nyra_await_result", value, span)]
    } else {
        vec![]
    }
}

fn build_scalar_poll_state(
    state_id: i32,
    next_state: i32,
    handle_var: &str,
    bind_to: Option<&str>,
    span: Span,
    poll_callee: &str,
    kind: PollKind,
) -> Vec<Statement> {
    let poll = expr_call(
        poll_callee,
        vec![expr_var(handle_var, span.clone())],
        span.clone(),
    );
    let poll_var = format!("__nyra_poll_{state_id}");
    let mut stmts = vec![stmt_let(&poll_var, poll, span.clone())];
    let not_ready = Expression::Binary(Box::new(BinaryExpr {
        left: expr_var(&poll_var, span.clone()),
        op: BinaryOp::Lt,
        right: expr_int(0, span.clone()),
        span: span.clone(),
    }));
    let ready = assign_poll_result(&poll_var, bind_to, span.clone(), kind);
    stmts.push(poll_wait_or_advance(
        state_id,
        next_state,
        not_ready,
        span.clone(),
        ready,
    ));
    stmts
}

fn build_string_poll_state(
    state_id: i32,
    next_state: i32,
    handle_var: &str,
    bind_to: Option<&str>,
    span: Span,
) -> Vec<Statement> {
    let done_var = format!("__nyra_poll_done_{state_id}");
    let mut stmts = vec![stmt_let(
        &done_var,
        expr_call(
            "async_future_done",
            vec![expr_var(handle_var, span.clone())],
            span.clone(),
        ),
        span.clone(),
    )];
    let not_ready = Expression::Binary(Box::new(BinaryExpr {
        left: expr_var(&done_var, span.clone()),
        op: BinaryOp::Eq,
        right: expr_int(0, span.clone()),
        span: span.clone(),
    }));
    let value = expr_call(
        "async_await_ptr",
        vec![expr_var(handle_var, span.clone())],
        span.clone(),
    );
    let mut ready = Vec::new();
    if let Some(name) = bind_to.filter(|n| *n != "_") {
        ready.push(stmt_assign(name, value, span.clone()));
    } else if bind_to.is_none() {
        ready.push(stmt_assign("__nyra_await_result", value, span.clone()));
    }
    stmts.push(poll_wait_or_advance(
        state_id,
        next_state,
        not_ready,
        span.clone(),
        ready,
    ));
    stmts
}

pub fn try_desugar_state_machine(func: &mut Function, checker: &TypeChecker) -> bool {
    if !func.is_async || !func.type_params.is_empty() {
        return false;
    }
    if !block_has_await(&func.body) {
        return false;
    }
    let span = func.span.clone();
    let mut hoist_counter = 0usize;
    hoist_nested_awaits_in_block(&mut func.body, &mut hoist_counter, &span);
    let handle = format!("{HANDLE_PREFIX}{}", func.name);
    let complete_fn = func
        .return_type
        .as_ref()
        .map(|ann| future_complete_callee(ann).to_string())
        .unwrap_or_else(|| "async_promise_complete".to_string());
    let future_struct = func
        .return_type
        .as_ref()
        .and_then(future_struct_name_from_ann)
        .unwrap_or("Future_i32");
    let return_value = if func.exported {
        expr_var(&handle, span.clone())
    } else {
        future_struct_literal(&handle, future_struct, span.clone())
    };
    let result_kind = func
        .return_type
        .as_ref()
        .map(poll_kind_for_return_ann)
        .unwrap_or(PollKind::I32);
    let mut builder = CfgBuilder::new(handle.clone(), span.clone(), complete_fn, result_kind);
    if let Some(sig) = checker.env.functions.get(&func.name) {
        for (param, ty) in func.params.iter().zip(sig.params.iter()) {
            builder.local_types.insert(param.name.clone(), ty.clone());
        }
    } else {
        for param in &func.params {
            builder
                .local_types
                .insert(param.name.clone(), checker.type_from_ann(&param.ty));
        }
    }
    let Some((entry, _)) = builder.lower_block(&func.body, COMPLETE_EXIT, checker) else {
        return false;
    };
    let inner = builder.build_spawn_body(entry);

    func.is_async = false;
    func.body = Block {
        statements: vec![
            stmt_let(
                &handle,
                expr_call("async_promise_new", vec![], span.clone()),
                span.clone(),
            ),
            Statement::Spawn(SpawnStmt {
                kind: SpawnKind::Task,
                body: inner,
            }),
            Statement::Return(ReturnStmt {
                value: Some(return_value),
            }),
        ],
    };
    true
}

pub fn desugar_async_state_machines(program: &mut Program, checker: &TypeChecker) {
    for f in &mut program.functions {
        try_desugar_state_machine(f, checker);
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            try_desugar_state_machine(m, checker);
        }
    }
    for ti in &mut program.trait_impls {
        for m in &mut ti.methods {
            try_desugar_state_machine(m, checker);
        }
    }
}
