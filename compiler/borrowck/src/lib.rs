use std::collections::{HashMap, HashSet};

use ast::*;
use ast::{expr_span, variable_name};
use errors::{ErrorKind, NyraError, Span, E010_BORROW_WHILE_ASSIGNED, E011_USE_WHILE_BORROWED};
use ownership::{
    check_parallel_for_captures, check_send_sync_program, check_spawn_captures,
    check_sync_closure_captures, collect_arrow_captures, compute_last_uses, arrow_has_captures,
    check_copy_attrs, OwnershipCtx, OwnershipKind,
};
use types::Type;

mod diag;
mod inspect;
pub use inspect::{
    analyze_ownership_verbose, inspect_binding, BindingInspectReport, BindingStatus,
    InspectQuery, InspectRole, OwnershipVerbosePlan,
};

use diag::{
    borrow_active_error, cannot_borrow_moved, cannot_borrow_mut_alias,
    cannot_borrow_while_mut_borrowed, manual_free_warning, move_while_borrowed, record_move_origin,
    use_after_move_error, use_moved_value_error, DiagCtx, MoveOrigin, move_candidate,
};

fn builtin_method_borrows_receiver(method: &str) -> bool {
    // All `String_*` stdlib helpers take `&string`, so UFCS calls
    // (`name.String_toUpperCase()`) borrow rather than move the receiver.
    if method.starts_with("String_") {
        return true;
    }
    matches!(
        method,
        "clone" | "length" | "len" | "split" | "trim" | "contains" | "starts_with"
            | "ends_with" | "replace" | "replacen" | "to_upper" | "to_lower" | "sort"
            | "sort_by" | "strip_suffix"
            // Case-conversion string builtins — all take `&string`.
            | "to_snake_case" | "to_lowercase" | "to_titlecase" | "to_capitalize"
            | "to_camel_case" | "to_kebab_case" | "to_pascal_case"
            | "to_screaming_snake_case" | "to_train_case" | "to_dot_case"
            | "includes" | "strip_prefix" | "index" | "is_empty" | "last_index" | "repeat" | "trim_end" | "trim_start" | "splitn" | "count" | "fields" | "pad_end" | "pad_start" | "split_once" | "compare" | "equal_fold" | "index_byte" | "last_index_byte" | "after_sep" | "char_at" | "pop" | "push_char" | "strip_ansi" | "substring" | "before_sep" | "collapse_ws" | "is_ascii" | "common_prefix_len" | "is_alnum" | "is_alpha" | "is_digit" | "pad_center" | "reverse" | "escape_json" | "split_after" | "truncate" 
    )
}

/// Borrow / move checking with Copy vs Move ownership and non-lexical lifetimes (NLL).
pub fn check_program(program: &Program, ctx: &OwnershipCtx, errors: &mut Vec<NyraError>) {
    let mut inspect = None;
    check_program_with_collector(program, ctx, errors, &mut inspect);
}

/// Run borrow checking and optionally capture an ownership snapshot for `nyra inspect`.
pub fn check_program_inspect(
    program: &Program,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
    query: Option<&InspectQuery>,
) -> Option<BindingInspectReport> {
    let mut collector = query.cloned().map(inspect::InspectCollector::for_query);
    let mut inspect_opt: Option<&mut inspect::InspectCollector> = collector.as_mut();
    check_program_with_collector(program, ctx, errors, &mut inspect_opt);
    collector.and_then(|c| c.query_result)
}

pub(crate) fn check_program_with_collector(
    program: &Program,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
    inspect: &mut Option<&mut inspect::InspectCollector>,
) {
    let diag = DiagCtx::from_program(program);
    check_copy_attrs(program, ctx, errors);
    check_send_sync_program(program, ctx, errors);
    for func in &program.functions {
        if !func.type_params.is_empty() {
            continue;
        }
        if let Some(col) = inspect.as_deref_mut() {
            col.set_func(&func.name);
        }
        check_block(
            &func.body,
            &mut State::new(ctx),
            ctx,
            &diag,
            errors,
            inspect,
        );
    }
    for imp in &program.impls {
        for method in &imp.methods {
            if let Some(col) = inspect.as_deref_mut() {
                col.set_func(&method.name);
            }
            check_block(
                &method.body,
                &mut State::new(ctx),
                ctx,
                &diag,
                errors,
                inspect,
            );
        }
    }
}

#[derive(Debug, Clone)]
struct ActiveBorrow {
    source: String,
    borrower: Option<String>,
    mutable: bool,
    /// Borrow is active through this statement index (inclusive).
    expires_after: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    moved: HashMap<String, MoveOrigin>,
    /// `let dest = src` move: dest → src (provenance).
    move_from: HashMap<String, String>,
    /// `let dest = src` move: src → dest (forward edge).
    move_to: HashMap<String, String>,
    mutable: HashSet<String>,
    active_borrows: Vec<ActiveBorrow>,
    borrowed_imm: HashSet<String>,
    borrowed_mut: HashSet<String>,
    var_types: HashMap<String, Type>,
    ref_sources: HashMap<String, String>,
    manually_freed: HashSet<String>,
    unsafe_depth: u32,
}

impl State {
    fn new(_ctx: &OwnershipCtx) -> Self {
        Self {
            moved: HashMap::new(),
            move_from: HashMap::new(),
            move_to: HashMap::new(),
            mutable: HashSet::new(),
            active_borrows: Vec::new(),
            borrowed_imm: HashSet::new(),
            borrowed_mut: HashSet::new(),
            var_types: HashMap::new(),
            ref_sources: HashMap::new(),
            manually_freed: HashSet::new(),
            unsafe_depth: 0,
        }
    }

    fn in_unsafe(&self) -> bool {
        self.unsafe_depth > 0
    }

    fn fork(&self) -> Self {
        self.clone()
    }

    fn merge_branches(&mut self, a: &Self, b: &Self) {
        for (k, v) in &a.moved {
            if b.moved.contains_key(k) {
                self.moved.insert(k.clone(), v.clone());
            }
        }
    }

    fn clear_borrows(&mut self) {
        self.active_borrows.clear();
        self.borrowed_imm.clear();
        self.borrowed_mut.clear();
    }

    fn rebuild_borrow_sets(&mut self) {
        self.borrowed_imm.clear();
        self.borrowed_mut.clear();
        for b in &self.active_borrows {
            if b.mutable {
                self.borrowed_mut.insert(b.source.clone());
            } else {
                self.borrowed_imm.insert(b.source.clone());
            }
        }
    }

    fn expire_borrows_before(&mut self, stmt_idx: usize) {
        self.active_borrows
            .retain(|b| b.expires_after >= stmt_idx);
        self.rebuild_borrow_sets();
    }

    fn add_borrow(
        &mut self,
        source: &str,
        mutable: bool,
        expires_after: usize,
        borrower: Option<&str>,
    ) {
        self.active_borrows.retain(|b| b.source != source);
        self.active_borrows.push(ActiveBorrow {
            source: source.to_string(),
            borrower: borrower.map(str::to_string),
            mutable,
            expires_after,
        });
        self.rebuild_borrow_sets();
    }

    fn type_of(&self, name: &str, _ctx: &OwnershipCtx) -> Type {
        self.var_types
            .get(name)
            .cloned()
            .unwrap_or(Type::Unknown)
    }

    fn ownership_of_var(&self, name: &str, ctx: &OwnershipCtx) -> OwnershipKind {
        ctx.kind_of(&self.type_of(name, ctx))
    }

    fn outer_vars(&self) -> HashMap<String, Type> {
        self.var_types.clone()
    }
}

fn check_block(
    block: &Block,
    state: &mut State,
    ctx: &OwnershipCtx,
    diag: &DiagCtx,
    errors: &mut Vec<NyraError>,
    inspect: &mut Option<&mut inspect::InspectCollector>,
) {
    let saved_imm = state.borrowed_imm.clone();
    let saved_mut = state.borrowed_mut.clone();
    let saved_active = state.active_borrows.clone();
    let last_uses = compute_last_uses(block);
    let block_len = block.statements.len().saturating_sub(1);

    for (idx, stmt) in block.statements.iter().enumerate() {
        state.expire_borrows_before(idx);
        check_statement(stmt, idx, &last_uses, block_len, state, ctx, diag, errors, inspect);
        if let Some(col) = inspect.as_deref_mut() {
            col.on_after_stmt(stmt, idx, block, state, ctx);
        }
    }

    if let Some(col) = inspect.as_deref_mut() {
        col.on_block_exit(block, state, ctx);
    }

    state.borrowed_imm = saved_imm;
    state.borrowed_mut = saved_mut;
    state.active_borrows = saved_active;
    state.rebuild_borrow_sets();
}

fn check_statement(
    stmt: &Statement,
    stmt_idx: usize,
    last_uses: &HashMap<String, usize>,
    block_len: usize,
    state: &mut State,
    ctx: &OwnershipCtx,
    diag: &DiagCtx,
    errors: &mut Vec<NyraError>,
    inspect: &mut Option<&mut inspect::InspectCollector>,
) {
    if state.in_unsafe() {
        check_statement_unsafe(stmt, state, ctx);
        return;
    }
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            clear_provenance_for(state, &l.name);
            if uses_moved(&l.value, state, ctx, diag, errors) {
                return;
            }
            let is_ref_binding = matches!(
                &l.value,
                Expression::Unary(u)
                    if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut)
                        && matches!(&u.operand, Expression::Variable { .. })
            );
            if !is_ref_binding {
                register_borrows_from_expr(&l.value, stmt_idx, state, errors);
            }
            if let Expression::ArrowFn(arrow) = &l.value {
                if !state.borrowed_mut.is_empty() || !state.borrowed_imm.is_empty() {
                    errors.push(borrow_active_error(
                        "closure while references are active",
                        expr_span(&l.value),
                        "finish using borrows before creating a capturing closure",
                    ));
                }
                let outer = state.outer_vars();
                check_sync_closure_captures(arrow, &outer, arrow.span.clone(), ctx, errors);
                mark_arrow_capture_moves(arrow, &outer, state, ctx);
            }
            let ty = l
                .ty
                .clone()
                .map(Type::from)
                .unwrap_or_else(|| infer_let_type(&l.value, state, ctx));
            state.var_types.insert(l.name.clone(), ty);
            if l.mutable {
                state.mutable.insert(l.name.clone());
            }
            if should_move_binding(&l.value, state, ctx) {
                if let Expression::Variable { name: src, .. } = &l.value {
                    state
                        .move_from
                        .insert(l.name.clone(), src.clone());
                    state.move_to.insert(src.clone(), l.name.clone());
                }
                mark_moved_from_expr(&l.value, state, ctx, None, None);
            }
            register_ref_binding_borrow(&l.name, &l.value, stmt_idx, last_uses, block_len, state);
        }
        Statement::Assign(a) => {
            if let Some(name) = variable_name(&a.target) {
                if state.borrowed_imm.contains(name) || state.borrowed_mut.contains(name) {
                    let kind = if state.borrowed_mut.contains(name) {
                        "mutably"
                    } else {
                        "immutably"
                    };
                    errors.push(
                        NyraError::coded(
                            E010_BORROW_WHILE_ASSIGNED,
                            ErrorKind::BorrowCheck,
                            a.span.clone(),
                            format!("cannot assign to `{name}` because it is borrowed"),
                        )
                        .label(format!("`{name}` is {kind} borrowed here"))
                        .help(format!(
                            "drop the borrow before assigning, or clone the value: `let copy = clone {name}`"
                        )),
                    );
                }
            }
            if uses_moved(&a.value, state, ctx, diag, errors) {
                return;
            }
            if let Some(v) = variable_name(&a.value) {
                if state.moved.contains_key(v) {
                    errors.push(use_moved_value_error(
                        v,
                        a.span.clone(),
                        state.moved.get(v),
                        diag,
                        &state.type_of(v, ctx),
                    ));
                }
            }
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                let _ = uses_moved(v, state, ctx, diag, errors);
                register_borrows_from_expr(v, stmt_idx, state, errors);
                if let Some(name) = variable_name(v) {
                    if state.ownership_of_var(name, ctx).is_move() {
                        state.moved.insert(
                            name.to_string(),
                            record_move_origin(name, expr_span(v), None, expr_span(v), false),
                        );
                    }
                }
            }
        }
        Statement::If(i) => {
            let _ = uses_moved(&i.condition, state, ctx, diag, errors);
            register_borrows_from_expr(&i.condition, stmt_idx, state, errors);
            // Condition borrows (e.g. strcmp(&a, &b)) end before branch bodies.
            state.clear_borrows();
            let mut then_s = state.fork();
            check_block(&i.then_block, &mut then_s, ctx, diag, errors, inspect);
            if let Some(e) = &i.else_block {
                let mut else_s = state.fork();
                check_block(e, &mut else_s, ctx, diag, errors, inspect);
                state.merge_branches(&then_s, &else_s);
            } else {
                state.merge_branches(&then_s, &state.fork());
            }
            state.clear_borrows();
        }
        Statement::While(w) => {
            let _ = uses_moved(&w.condition, state, ctx, diag, errors);
            register_borrows_from_expr(&w.condition, stmt_idx, state, errors);
            state.clear_borrows();
            let mut loop_s = state.fork();
            check_block(&w.body, &mut loop_s, ctx, diag, errors, inspect);
            state.clear_borrows();
        }
        Statement::For(f) => {
            f.for_each_expr(|e| {
                let _ = uses_moved(e, state, ctx, diag, errors);
                register_borrows_from_expr(e, stmt_idx, state, errors);
            });
            if f.parallel.is_some() {
                if !state.borrowed_mut.is_empty() || !state.borrowed_imm.is_empty() {
                    errors.push(borrow_active_error(
                        "`parallel for` while references are active",
                        Span::default(),
                        "finish using borrows before `parallel for`",
                    ));
                }
                let outer = state.outer_vars();
                check_parallel_for_captures(&f.body, &outer, Span::default(), ctx, errors);
            }
            let mut loop_s = state.fork();
            check_block(&f.body, &mut loop_s, ctx, diag, errors, inspect);
            state.clear_borrows();
        }
        Statement::Print(p) => {
            for arg in &p.args {
                if let Expression::Call(c) = arg {
                    check_nyra_free_call(c, state, ctx, errors);
                }
                check_expr_moves(arg, stmt_idx, state, ctx, diag, errors);
            }
            if let Some(c) = &p.color {
                check_expr_moves(c, stmt_idx, state, ctx, diag, errors);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            if let Expression::Call(c) = e {
                check_nyra_free_call(c, state, ctx, errors);
            }
            check_expr_moves(e, stmt_idx, state, ctx, diag, errors);
        }
        Statement::Spawn(sp) => {
            if !state.borrowed_mut.is_empty() || !state.borrowed_imm.is_empty() {
                errors.push(borrow_active_error(
                    "spawn while references are active",
                    Span::default(),
                    "finish using borrows before spawn",
                ));
            }
            let outer = state.outer_vars();
            check_spawn_captures(&sp.body, &outer, Span::default(), ctx, errors);
            let declared: std::collections::HashSet<String> = outer.keys().cloned().collect();
            for name in ownership::collect_captures(&sp.body, &declared) {
                if ctx.kind_of(outer.get(&name).unwrap_or(&Type::Unknown)).is_move() {
                    state.moved.insert(
                        name.clone(),
                        record_move_origin(&name, Span::default(), None, Span::default(), false),
                    );
                }
            }
            check_block(&sp.body, state, ctx, diag, errors, inspect);
            state.clear_borrows();
        }
        Statement::Benchmark(body) => {
            check_block(body, state, ctx, diag, errors, inspect);
        }
        Statement::Unsafe(body) => {
            state.unsafe_depth += 1;
            check_block(body, state, ctx, diag, errors, inspect);
            state.unsafe_depth -= 1;
        }
        Statement::Asm { .. } => {}
        Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn check_statement_unsafe(stmt: &Statement, state: &mut State, ctx: &OwnershipCtx) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let ty = l
                .ty
                .clone()
                .map(Type::from)
                .unwrap_or_else(|| infer_let_type(&l.value, state, ctx));
            state.var_types.insert(l.name.clone(), ty);
            if l.mutable {
                state.mutable.insert(l.name.clone());
            }
        }
        Statement::Assign(a) => {
            let _ = variable_name(&a.target);
        }
        Statement::Unsafe(body) => {
            state.unsafe_depth += 1;
            for s in &body.statements {
                check_statement_unsafe(s, state, ctx);
            }
            state.unsafe_depth -= 1;
        }
        _ => {}
    }
}

/// Drop stale move/ref edges when a binding is shadowed by a new `let`.
fn clear_provenance_for(state: &mut State, name: &str) {
    state.move_from.remove(name);
    state.move_to.remove(name);
    state.ref_sources.remove(name);
    state.move_from.retain(|dest, _| dest != name);
    let parents: Vec<String> = state
        .move_to
        .iter()
        .filter(|(_, dest)| dest.as_str() == name)
        .map(|(src, _)| src.clone())
        .collect();
    for src in parents {
        state.move_to.remove(&src);
    }
}

fn register_ref_binding_borrow(
    binding: &str,
    value: &Expression,
    _stmt_idx: usize,
    last_uses: &HashMap<String, usize>,
    block_len: usize,
    state: &mut State,
) {
    let Expression::Unary(u) = value else {
        return;
    };
    if !matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
        return;
    }
    let Expression::Variable { name: source, .. } = &u.operand else {
        return;
    };
    let mutable = u.op == UnaryOp::RefMut;
    let expires = last_uses.get(binding).copied().unwrap_or(block_len);
    state.ref_sources.insert(binding.to_string(), source.clone());
    state.add_borrow(source, mutable, expires, Some(binding));
}

fn check_nyra_free_call(
    call: &CallExpr,
    state: &mut State,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
) {
    if call.callee != "free" {
        return;
    }
    let Some(Expression::Variable { name, .. }) = call.args.first() else {
        return;
    };
    state.manually_freed.insert(name.clone());
    if state.ownership_of_var(name, ctx).is_move() && !state.moved.contains_key(name) {
        errors.push(manual_free_warning(name, call.span.clone()));
    }
}

fn mark_arrow_capture_moves(
    arrow: &ArrowFnExpr,
    outer: &HashMap<String, Type>,
    state: &mut State,
    ctx: &OwnershipCtx,
) {
    let outer_names: HashSet<String> = outer.keys().cloned().collect();
    for name in collect_arrow_captures(arrow, &outer_names) {
        if ctx.kind_of(outer.get(&name).unwrap_or(&Type::Unknown)).is_move() {
            state.moved.insert(
                name.clone(),
                record_move_origin(&name, arrow.span.clone(), None, arrow.span.clone(), false),
            );
        }
    }
}

fn infer_let_type(expr: &Expression, state: &State, ctx: &OwnershipCtx) -> Type {
    if let Expression::Unary(u) = expr {
        if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
            let inner = match &u.operand {
                Expression::Variable { name, .. } => state
                    .var_types
                    .get(name)
                    .cloned()
                    .filter(|t| !matches!(t, Type::Unknown))
                    .unwrap_or_else(|| ctx.infer_expr_type(&u.operand)),
                other => ctx.infer_expr_type(other),
            };
            return Type::Ref {
                inner: Box::new(inner),
                mutable: u.op == UnaryOp::RefMut,
                lifetime: None,
            };
        }
    }
    if let Expression::Variable { name, .. } = expr {
        if let Some(ty) = state.var_types.get(name) {
            if !matches!(ty, Type::Unknown) {
                return ty.clone();
            }
        }
    }
    ctx.infer_expr_type(expr)
}

fn should_move_binding(expr: &Expression, state: &State, ctx: &OwnershipCtx) -> bool {
    match expr {
        Expression::Variable { name, .. } => state.ownership_of_var(name, ctx).is_move(),
        Expression::Call(c) => ctx.callee_returns_owned(&c.callee),
        Expression::TemplateLiteral(_) => true,
        Expression::Literal(Literal::String(_)) => true,
        _ => false,
    }
}

fn check_expr_moves(
    expr: &Expression,
    stmt_idx: usize,
    state: &mut State,
    ctx: &OwnershipCtx,
    diag: &DiagCtx,
    errors: &mut Vec<NyraError>,
) {
    if uses_moved(expr, state, ctx, diag, errors) {
        return;
    }
    register_borrows_from_expr(expr, stmt_idx, state, errors);
    match expr {
        Expression::Call(c) => {
            if matches!(c.callee.as_str(), "print" | "write" | "println") {
                for arg in &c.args {
                    check_expr_moves(arg, stmt_idx, state, ctx, diag, errors);
                }
                return;
            }
            for arg in &c.args {
                if let Expression::ArrowFn(arrow) = arg {
                    if arrow_has_captures(arrow) {
                        if !state.borrowed_mut.is_empty() || !state.borrowed_imm.is_empty() {
                            errors.push(borrow_active_error(
                                "closure while references are active",
                                arrow.span.clone(),
                                "finish using borrows before passing a capturing closure",
                            ));
                        }
                        let outer = state.outer_vars();
                        check_sync_closure_captures(arrow, &outer, arrow.span.clone(), ctx, errors);
                        mark_arrow_capture_moves(arrow, &outer, state, ctx);
                    }
                } else {
                    try_move_on_call(arg, &c.callee, &c.span, state, ctx, errors);
                }
            }
        }
        Expression::MethodCall(mc) => {
            let borrows_receiver = builtin_method_borrows_receiver(&mc.method);
            if !borrows_receiver {
                try_move_on_call(&mc.object, &mc.method, &mc.span, state, ctx, errors);
            } else if let Expression::Variable { name, .. } = &mc.object {
                state.add_borrow(name, false, stmt_idx, None);
            }
            for arg in &mc.args {
                if move_candidate(arg).is_some() {
                    try_move_on_call(arg, &mc.method, &mc.span, state, ctx, errors);
                }
            }
            if mc.method == "join" {
                try_move_on_call(&mc.object, "join", &mc.span, state, ctx, errors);
            }
        }
        Expression::Spawn { body, .. } => {
            if !state.borrowed_mut.is_empty() || !state.borrowed_imm.is_empty() {
                errors.push(borrow_active_error(
                    "spawn while references are active",
                    Span::default(),
                    "finish using borrows before spawn",
                ));
            }
            let outer = state.outer_vars();
            check_spawn_captures(body, &outer, Span::default(), ctx, errors);
            let declared: std::collections::HashSet<String> = outer.keys().cloned().collect();
            for cap in ownership::collect_captures(body, &declared) {
                if ctx.kind_of(outer.get(&cap).unwrap_or(&Type::Unknown)).is_move() {
                    state.moved.insert(
                        cap.clone(),
                        record_move_origin(&cap, Span::default(), None, Span::default(), false),
                    );
                }
            }
        }
        Expression::ParallelSearch(ps) => {
            if !state.borrowed_mut.is_empty() || !state.borrowed_imm.is_empty() {
                errors.push(borrow_active_error(
                    "parallel search while references are active",
                    Span::default(),
                    "finish using borrows before parallel search",
                ));
            }
            let outer = state.outer_vars();
            check_parallel_for_captures(&ps.body, &outer, Span::default(), ctx, errors);
        }
        _ => {}
    }
}

fn try_move_on_call(
    arg: &Expression,
    callee: &str,
    call_span: &Span,
    state: &mut State,
    ctx: &OwnershipCtx,
    errors: &mut Vec<NyraError>,
) {
    let Some((name, explicit)) = move_candidate(arg) else {
        return;
    };
    if !explicit && state.mutable.contains(name) {
        return;
    }
    if state.ownership_of_var(name, ctx).is_copy() {
        return;
    }
    if state.borrowed_imm.contains(name) || state.borrowed_mut.contains(name) {
        errors.push(move_while_borrowed(name, expr_span(arg)));
        return;
    }
    state.moved.insert(
        name.to_string(),
        record_move_origin(name, expr_span(arg), Some(callee), call_span.clone(), explicit),
    );
}

fn register_borrows_from_expr(
    expr: &Expression,
    stmt_idx: usize,
    state: &mut State,
    errors: &mut Vec<NyraError>,
) {
    match expr {
        Expression::Unary(u) => match u.op {
            UnaryOp::Ref => register_borrow(&u.operand, false, stmt_idx, state, errors),
            UnaryOp::RefMut => register_borrow(&u.operand, true, stmt_idx, state, errors),
            _ => register_borrows_from_expr(&u.operand, stmt_idx, state, errors),
        },
        Expression::Binary(b) => {
            register_borrows_from_expr(&b.left, stmt_idx, state, errors);
            if matches!(b.op, BinaryOp::And | BinaryOp::Or) {
                state.expire_borrows_before(stmt_idx + 1);
            }
            register_borrows_from_expr(&b.right, stmt_idx, state, errors);
        }
        Expression::Call(c) => {
            for a in &c.args {
                register_borrows_from_expr(a, stmt_idx, state, errors);
            }
        }
        Expression::MethodCall(mc) => {
            register_borrows_from_expr(&mc.object, stmt_idx, state, errors);
            for a in &mc.args {
                register_borrows_from_expr(a, stmt_idx, state, errors);
            }
        }
        Expression::If(i) => {
            register_borrows_from_expr(&i.condition, stmt_idx, state, errors);
            for_each_expr_in_block(&i.then_block, &mut |e| register_borrows_from_expr(e, stmt_idx, state, errors));
            for_each_expr_in_block(&i.else_block, &mut |e| register_borrows_from_expr(e, stmt_idx, state, errors));
        }
        Expression::Match(m) => {
            register_borrows_from_expr(&m.scrutinee, stmt_idx, state, errors);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    register_borrows_from_expr(g, stmt_idx, state, errors);
                }
                for_each_expr_in_block(&arm.body, &mut |e| register_borrows_from_expr(e, stmt_idx, state, errors));
            }
        }
        Expression::Await(inner) => register_borrows_from_expr(inner, stmt_idx, state, errors),
        Expression::FieldAccess(f) => register_borrows_from_expr(&f.object, stmt_idx, state, errors),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                register_borrows_from_expr(spread, stmt_idx, state, errors);
            }
            for (_, e) in &s.fields {
                register_borrows_from_expr(e, stmt_idx, state, errors);
            }
        }
        Expression::Grouped(g) => register_borrows_from_expr(g, stmt_idx, state, errors),
        Expression::Index(ix) => {
            register_borrows_from_expr(&ix.object, stmt_idx, state, errors);
            register_borrows_from_expr(&ix.index, stmt_idx, state, errors);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                register_borrows_from_expr(e, stmt_idx, state, errors);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            register_borrows_from_expr(element, stmt_idx, state, errors);
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    register_borrows_from_expr(e, stmt_idx, state, errors);
                }
            }
        }
        _ => {}
    }
}

fn register_borrow(
    operand: &Expression,
    mutable: bool,
    stmt_idx: usize,
    state: &mut State,
    errors: &mut Vec<NyraError>,
) {
    let Some(name) = variable_name(operand) else {
        register_borrows_from_expr(operand, stmt_idx, state, errors);
        return;
    };
    let sp = expr_span(operand);
    if state.moved.contains_key(name) {
        errors.push(cannot_borrow_moved(name, sp.clone()));
        return;
    }
    if mutable {
        if state.borrowed_imm.contains(name) || state.borrowed_mut.contains(name) {
            errors.push(cannot_borrow_mut_alias(name, sp));
            return;
        }
    } else if state.borrowed_mut.contains(name) {
        errors.push(cannot_borrow_while_mut_borrowed(name, sp));
        return;
    }
    // Temporary expression borrows end at the current statement (NLL).
    state.add_borrow(name, mutable, stmt_idx, None);
}

fn mark_moved_from_expr(
    expr: &Expression,
    state: &mut State,
    ctx: &OwnershipCtx,
    callee: Option<&str>,
    call_span: Option<&Span>,
) {
    let sp = expr_span(expr);
    let origin_span = call_span.cloned().unwrap_or_else(|| sp.clone());
    match expr {
        Expression::Variable { name, .. }
            if state.ownership_of_var(name, ctx).is_move() => {
                state.moved.insert(
                    name.clone(),
                    record_move_origin(name, sp, callee, origin_span, false),
                );
            }
        Expression::FieldAccess(fa) => {
            if let Expression::Variable { name: base, .. } = &fa.object {
                if let Type::Struct(sname) = state.type_of(base, ctx) {
                    if let Some(field_ty) = ctx.struct_field_type(&sname, &fa.field) {
                        if ctx.kind_of(&field_ty).is_move() {
                            state.moved.insert(
                                base.clone(),
                                record_move_origin(base, sp, callee, origin_span, false),
                            );
                        }
                    }
                }
            }
        }
        Expression::Call(c) => {
            for arg in &c.args {
                if let Some((n, explicit)) = move_candidate(arg) {
                    if (explicit || !state.mutable.contains(n))
                        && state.ownership_of_var(n, ctx).is_move()
                    {
                        state.moved.insert(
                            n.to_string(),
                            record_move_origin(n, expr_span(arg), Some(&c.callee), c.span.clone(), explicit),
                        );
                    }
                }
            }
        }
        Expression::MethodCall(mc) => {
            let borrows_receiver = builtin_method_borrows_receiver(&mc.method);
            if !borrows_receiver {
                mark_moved_from_expr(&mc.object, state, ctx, Some(&mc.method), Some(&mc.span));
            }
            for arg in &mc.args {
                mark_moved_from_expr(arg, state, ctx, Some(&mc.method), Some(&mc.span));
            }
        }
        Expression::ArrowFn(arrow) if arrow_has_captures(arrow) => {
            let outer_names: HashSet<String> = state.var_types.keys().cloned().collect();
            for name in collect_arrow_captures(arrow, &outer_names) {
                if state.ownership_of_var(&name, ctx).is_move() {
                    state.moved.insert(
                        name.clone(),
                        record_move_origin(&name, arrow.span.clone(), None, arrow.span.clone(), false),
                    );
                }
            }
        }
        _ => {}
    }
}

fn uses_moved(
    expr: &Expression,
    state: &State,
    ctx: &OwnershipCtx,
    diag: &DiagCtx,
    errors: &mut Vec<NyraError>,
) -> bool {
    let mut found = false;
    visit_expr(expr, state, ctx, diag, errors, &mut found);
    found
}

fn visit_expr(
    expr: &Expression,
    state: &State,
    ctx: &OwnershipCtx,
    diag: &DiagCtx,
    errors: &mut Vec<NyraError>,
    found: &mut bool,
) {
    match expr {
        Expression::Variable { name, .. } => {
            let sp = expr_span(expr);
            if let Some(origin) = state.moved.get(name) {
                let var_ty = state.type_of(name, ctx);
                errors.push(use_after_move_error(name, sp, origin, &var_ty, diag));
                *found = true;
            } else if state.borrowed_mut.contains(name) {
                errors.push(
                    NyraError::coded(
                        E011_USE_WHILE_BORROWED,
                        ErrorKind::BorrowCheck,
                        sp,
                        format!("cannot use `{name}` while it is mutably borrowed"),
                    )
                    .help("finish using the mutable borrow before reading this value again"),
                );
                *found = true;
            } else if state.borrowed_imm.contains(name) {
                errors.push(
                    NyraError::coded(
                        E011_USE_WHILE_BORROWED,
                        ErrorKind::BorrowCheck,
                        sp,
                        format!("cannot use `{name}` while it is borrowed"),
                    )
                    .help("the immutable borrow must end before this use"),
                );
                *found = true;
            }
        }
        Expression::Unary(u) => {
            if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
                return;
            }
            visit_expr(&u.operand, state, ctx, diag, errors, found);
        }
        Expression::Binary(b) => {
            visit_expr(&b.left, state, ctx, diag, errors, found);
            visit_expr(&b.right, state, ctx, diag, errors, found);
        }
        Expression::Call(c) => {
            for a in &c.args {
                visit_expr(a, state, ctx, diag, errors, found);
            }
        }
        Expression::FieldAccess(f) => visit_expr(&f.object, state, ctx, diag, errors, found),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                visit_expr(spread, state, ctx, diag, errors, found);
            }
            for (_, e) in &s.fields {
                visit_expr(e, state, ctx, diag, errors, found);
            }
        }
        Expression::Grouped(g) => visit_expr(g, state, ctx, diag, errors, found),
        Expression::Match(m) => {
            visit_expr(&m.scrutinee, state, ctx, diag, errors, found);
            for arm in &m.arms {
                for_each_expr_in_block(&arm.body, &mut |e| visit_expr(e, state, ctx, diag, errors, found));
            }
        }
        Expression::If(i) => {
            visit_expr(&i.condition, state, ctx, diag, errors, found);
            for_each_expr_in_block(&i.then_block, &mut |e| visit_expr(e, state, ctx, diag, errors, found));
            for_each_expr_in_block(&i.else_block, &mut |e| visit_expr(e, state, ctx, diag, errors, found));
        }
        Expression::Index(ix) => {
            visit_expr(&ix.object, state, ctx, diag, errors, found);
            visit_expr(&ix.index, state, ctx, diag, errors, found);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                visit_expr(e, state, ctx, diag, errors, found);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            visit_expr(element, state, ctx, diag, errors, found);
        }
        Expression::MethodCall(mc) => {
            visit_expr(&mc.object, state, ctx, diag, errors, found);
            for a in &mc.args {
                visit_expr(a, state, ctx, diag, errors, found);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    visit_expr(e, state, ctx, diag, errors, found);
                }
            }
        }
        Expression::ArrowFn(arrow) => {
            let block = ownership::arrow_to_block(arrow);
            for stmt in &block.statements {
                visit_stmt_for_moved(stmt, state, ctx, diag, errors, found);
            }
        }
        _ => {}
    }
}

fn visit_stmt_for_moved(
    stmt: &Statement,
    state: &State,
    ctx: &OwnershipCtx,
    diag: &DiagCtx,
    errors: &mut Vec<NyraError>,
    found: &mut bool,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            visit_expr(&l.value, state, ctx, diag, errors, found);
        }
        Statement::Assign(a) => {
            visit_expr(&a.target, state, ctx, diag, errors, found);
            visit_expr(&a.value, state, ctx, diag, errors, found);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                visit_expr(v, state, ctx, diag, errors, found);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            visit_expr(e, state, ctx, diag, errors, found);
        }
        Statement::Print(p) => {
            for arg in &p.args {
                visit_expr(arg, state, ctx, diag, errors, found);
            }
            if let Some(c) = &p.color {
                visit_expr(c, state, ctx, diag, errors, found);
            }
        }
        Statement::If(i) => {
            visit_expr(&i.condition, state, ctx, diag, errors, found);
            for s in &i.then_block.statements {
                visit_stmt_for_moved(s, state, ctx, diag, errors, found);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    visit_stmt_for_moved(s, state, ctx, diag, errors, found);
                }
            }
        }
        Statement::While(w) => {
            visit_expr(&w.condition, state, ctx, diag, errors, found);
            for s in &w.body.statements {
                visit_stmt_for_moved(s, state, ctx, diag, errors, found);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| visit_expr(e, state, ctx, diag, errors, found));
            for s in &f.body.statements {
                visit_stmt_for_moved(s, state, ctx, diag, errors, found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::Lexer;
    use ownership::OwnershipCtx;
    use parser::Parser;

    fn borrow_errors(src: &str) -> Vec<NyraError> {
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        expand::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        expand::coerce_auto_borrow(&mut program);
        const_eval::fold_program_consts(&mut program);
        let ctx = OwnershipCtx::from_program(&program);
        let mut errors = Vec::new();
        check_program(&program, &ctx, &mut errors);
        errors
    }

    #[test]
    fn inspect_reports_borrow_at_line() {
        let src = r#"fn main() {
    let name = "Ada"
    let r = &name
    print(r)
}"#;
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        expand::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        expand::coerce_auto_borrow(&mut program);
        const_eval::fold_program_consts(&mut program);
        let ctx = OwnershipCtx::from_program(&program);
        let query = crate::inspect::InspectQuery {
            file: "test.ny".into(),
            line: 4,
            name: "name".into(),
        };
        let report = crate::inspect::inspect_binding(&program, &ctx, &query).unwrap();
        assert_eq!(report.name, "name");
        assert_eq!(report.binding_status, crate::inspect::BindingStatus::Valid);
        assert!(!report.borrowed_by.is_empty());
    }

    #[test]
    fn inspect_shows_ownership_chain() {
        let src = r#"fn main() {
    let name = "Nyra"
    let myname = name
    let myname2 = myname
    print(myname2)
}"#;
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        expand::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        expand::coerce_auto_borrow(&mut program);
        const_eval::fold_program_consts(&mut program);
        let ctx = OwnershipCtx::from_program(&program);
        let query = crate::inspect::InspectQuery {
            file: "test.ny".into(),
            line: 5,
            name: "myname2".into(),
        };
        let report = crate::inspect::inspect_binding(&program, &ctx, &query).unwrap();
        assert_eq!(
            report.ownership_chain,
            vec!["name", "myname", "myname2"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(report.current_owner, "myname2");
        assert_eq!(report.ty, "string");
        assert_eq!(report.kind, OwnershipKind::Move);

        let query_name = crate::inspect::InspectQuery {
            file: "test.ny".into(),
            line: 5,
            name: "name".into(),
        };
        let report_name = crate::inspect::inspect_binding(&program, &ctx, &query_name).unwrap();
        assert_eq!(report_name.current_owner, "myname2");
        assert!(matches!(
            report_name.role,
            crate::inspect::InspectRole::MovedAway
        ));
    }

    #[test]
    fn inspect_shows_borrow_chain() {
        let src = r#"fn main() {
    let name = "Nyra"
    let myname = &name
    let myname2 = &myname
    print(&myname2)
}"#;
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        expand::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        expand::coerce_auto_borrow(&mut program);
        const_eval::fold_program_consts(&mut program);
        let ctx = OwnershipCtx::from_program(&program);
        let query = crate::inspect::InspectQuery {
            file: "test.ny".into(),
            line: 4,
            name: "myname2".into(),
        };
        let report = crate::inspect::inspect_binding(&program, &ctx, &query).unwrap();
        assert_eq!(report.ty, "&&string");
        assert_eq!(
            report.borrow_chain,
            vec!["name", "myname", "myname2"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(report.heap_owner, "name");
        assert_eq!(report.current_owner, "name");
        assert_eq!(report.role, crate::inspect::InspectRole::Borrower);
        assert!(report.moved_from.is_none());
        assert!(report.ownership_chain.is_empty());
    }

    #[test]
    fn inspect_borrow_chain_survives_shadowed_names() {
        let src = r#"fn main() {
    let name = "Nyra"
    let myname = name
    let myname2 = myname
    print(myname2)

    let name = "Nyra"
    let myname = &name
    let myname2 = &myname
    print(&myname2)
}"#;
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (mut program, _) = Parser::new(tokens).parse();
        expand::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        expand::coerce_auto_borrow(&mut program);
        const_eval::fold_program_consts(&mut program);
        let ctx = OwnershipCtx::from_program(&program);
        let query = crate::inspect::InspectQuery {
            file: "test.ny".into(),
            line: 9,
            name: "myname2".into(),
        };
        let report = crate::inspect::inspect_binding(&program, &ctx, &query).unwrap();
        assert_eq!(report.heap_owner, "name");
        assert_eq!(
            report.borrow_chain,
            vec!["name", "myname", "myname2"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert!(report.moved_from.is_none());
        assert_eq!(report.role, crate::inspect::InspectRole::Borrower);
    }

    #[test]
    fn rejects_use_after_move_string() {
        let errs = borrow_errors(
            r#"fn main() {
    let a = "hello"
    let b = a
    print(a)
}"#,
        );
        assert!(errs.iter().any(|e| e.message.contains("moved")));
    }

    #[test]
    fn allows_copy_i32_after_assign() {
        let errs = borrow_errors(
            r#"fn main() {
    let b = 1
    let a = b
    print(a)
    print(b)
}"#,
        );
        assert!(!errs.iter().any(|e| e.message.contains("moved")));
    }

    #[test]
    fn rejects_assign_while_imm_borrowed() {
        let errs = borrow_errors(
            r#"fn main() {
    let mut score = 100
    let r = &score
    score = score - 10
    print(r)
}"#,
        );
        assert!(
            errs.iter().any(|e| e.code.as_deref() == Some("E010")),
            "{:?}",
            errs
        );
    }

    #[test]
    fn rejects_mut_borrow_conflict() {
        let errs = borrow_errors(
            r#"fn main() {
    mut x = 1
    let r = &mut x
    x = 2
    print(*r)
}"#,
        );
        assert!(errs.iter().any(|e| e.message.contains("borrowed")));
    }

    #[test]
    fn nll_allows_use_after_ref_expires() {
        let errs = borrow_errors(
            r#"fn main() {
    mut x = 1
    let r = &mut x
    print(*r)
    x = 2
}"#,
        );
        assert!(
            !errs.iter().any(|e| e.message.contains("borrowed")),
            "{:?}",
            errs
        );
    }

    #[test]
    fn nll_rejects_use_while_ref_alive() {
        let errs = borrow_errors(
            r#"fn main() {
    mut x = 1
    let r = &mut x
    x = 2
    print(*r)
}"#,
        );
        assert!(errs.iter().any(|e| e.message.contains("borrowed")));
    }
}
