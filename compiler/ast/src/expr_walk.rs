//! Walk expressions inside statement blocks (if-expression branches, etc.).

use crate::*;

pub fn for_each_expr_in_block<'a>(block: &'a Block, f: &mut impl FnMut(&'a Expression)) {
    for stmt in &block.statements {
        for_each_expr_in_stmt(stmt, f);
    }
}

pub fn for_each_expr_in_block_mut(block: &mut Block, f: &mut impl FnMut(&mut Expression)) {
    for stmt in &mut block.statements {
        for_each_expr_in_stmt_mut(stmt, f);
    }
}

fn for_each_expr_in_stmt<'a>(stmt: &'a Statement, f: &mut impl FnMut(&'a Expression)) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => f(&l.value),
        Statement::Assign(a) => {
            f(&a.target);
            f(&a.value);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                f(v);
            }
        }
        Statement::If(i) => {
            f(&i.condition);
            for_each_expr_in_block(&i.then_block, f);
            if let Some(el) = &i.else_block {
                for_each_expr_in_block(el, f);
            }
        }
        Statement::While(w) => {
            f(&w.condition);
            for_each_expr_in_block(&w.body, f);
        }
        Statement::For(fo) => {
            match &fo.kind {
                ForKind::Range { start, end } => {
                    f(start);
                    f(end);
                }
                ForKind::Iterable { iterable } => f(iterable),
            }
            for_each_expr_in_block(&fo.body, f);
        }
        Statement::Expression(e) | Statement::Defer(e) => f(e),
        Statement::Print(p) => {
            for a in &p.args {
                f(a);
            }
            if let Some(c) = &p.color {
                f(c);
            }
        }
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for_each_expr_in_block(b, f);
        }
        Statement::Asm { .. } | Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn for_each_expr_in_stmt_mut(stmt: &mut Statement, f: &mut impl FnMut(&mut Expression)) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => f(&mut l.value),
        Statement::Assign(a) => {
            f(&mut a.target);
            f(&mut a.value);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                f(v);
            }
        }
        Statement::If(i) => {
            f(&mut i.condition);
            for_each_expr_in_block_mut(&mut i.then_block, f);
            if let Some(el) = &mut i.else_block {
                for_each_expr_in_block_mut(el, f);
            }
        }
        Statement::While(w) => {
            f(&mut w.condition);
            for_each_expr_in_block_mut(&mut w.body, f);
        }
        Statement::For(fo) => {
            match &mut fo.kind {
                ForKind::Range { start, end } => {
                    f(start);
                    f(end);
                }
                ForKind::Iterable { iterable } => f(iterable),
            }
            for_each_expr_in_block_mut(&mut fo.body, f);
        }
        Statement::Expression(e) | Statement::Defer(e) => f(e),
        Statement::Print(p) => {
            for a in &mut p.args {
                f(a);
            }
            if let Some(c) = &mut p.color {
                f(c);
            }
        }
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for_each_expr_in_block_mut(b, f);
        }
        Statement::Asm { .. } | Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}
