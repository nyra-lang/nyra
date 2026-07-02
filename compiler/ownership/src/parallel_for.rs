//! Static checks for `parallel for` (independent iterations, no `break`, no loop-carried writes).
use std::collections::HashSet;

use ast::*;
use errors::{NyraError, Span};

use crate::diag;

pub fn block_has_break(block: &Block) -> bool {
    block.statements.iter().any(stmt_has_break)
}

fn stmt_has_break(stmt: &Statement) -> bool {
    match stmt {
        Statement::Break { .. } | Statement::Continue { .. } => true,
        Statement::If(i) => {
            block_has_break(&i.then_block)
                || i.else_block.as_ref().is_some_and(|b| block_has_break(b))
        }
        Statement::While(w) => block_has_break(&w.body),
        Statement::For(f) => block_has_break(&f.body),
        Statement::Spawn(s) => block_has_break(&s.body),
        Statement::Unsafe(b) | Statement::Benchmark(b) => block_has_break(b),
        _ => false,
    }
}

fn assign_target_name(target: &Expression) -> Option<&str> {
    match target {
        Expression::Variable { name, .. } => Some(name),
        _ => None,
    }
}

pub fn collect_assigned_in_block(block: &Block) -> HashSet<String> {
    let mut out = HashSet::new();
    for s in &block.statements {
        match s {
            Statement::Assign(a) => {
                if let Some(n) = assign_target_name(&a.target) {
                    out.insert(n.to_string());
                }
            }
            Statement::If(i) => {
                out.extend(collect_assigned_in_block(&i.then_block));
                if let Some(el) = &i.else_block {
                    out.extend(collect_assigned_in_block(el));
                }
            }
            Statement::While(w) => out.extend(collect_assigned_in_block(&w.body)),
            Statement::For(f) => out.extend(collect_assigned_in_block(&f.body)),
            _ => {}
        }
    }
    out
}

pub fn check_parallel_for_body(
    body: &Block,
    loop_var: &str,
    outer_names: &HashSet<String>,
    span: Span,
    errors: &mut Vec<NyraError>,
) {
    if block_has_break(body) {
        errors.push(diag::parallel_for_no_break_continue(span.clone()));
    }
    for name in collect_assigned_in_block(body) {
        if name == loop_var {
            continue;
        }
        if outer_names.contains(&name) {
            errors.push(diag::parallel_for_mutates_outer(&name, span.clone()));
        }
    }
}
