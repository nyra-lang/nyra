//! AST-level escape analysis: classify locals as `NoEscape`, `ArgEscape`, or `GlobalEscape`.
//!
//! Phase 1 infrastructure — results feed verbose diagnostics today; codegen stack promotion
//! will consume `EscapePlan` in a later phase.

use std::collections::{HashMap, HashSet};

use ast::*;
use ast::variable_name;

use crate::nll::{arrow_has_captures, collect_arrow_captures, collect_captures};

/// How far a binding's storage may outlive its declaring function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EscapeState {
    /// Born and dies inside the function; candidate for stack promotion / SROA.
    NoEscape = 0,
    /// Reaches another function only as a borrow (caller stack still valid).
    ArgEscape = 1,
    /// Returned, sent on a channel, captured by `spawn`, or otherwise heap/global.
    GlobalEscape = 2,
}

impl EscapeState {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoEscape => "NoEscape",
            Self::ArgEscape => "ArgEscape",
            Self::GlobalEscape => "GlobalEscape",
        }
    }

    fn merge_into(self, slot: &mut Self) {
        if self > *slot {
            *slot = self;
        }
    }
}

/// Per-function escape classification for local bindings.
#[derive(Debug, Clone, Default)]
pub struct EscapePlan {
    /// `func_name` → (`binding` → state).
    pub bindings: HashMap<String, HashMap<String, EscapeState>>,
    /// `func_name` → channel bindings eligible for stack `LocalChannel` (NoEscape, single-thread).
    pub local_channels: HashMap<String, HashSet<String>>,
    /// `func_name` → parameter names marked `#[no_escape]`.
    pub no_escape_params: HashMap<String, HashSet<String>>,
}

impl EscapePlan {
    pub fn state_in(&self, func: &str, name: &str) -> EscapeState {
        self.bindings
            .get(func)
            .and_then(|m| m.get(name))
            .copied()
            .unwrap_or(EscapeState::NoEscape)
    }

    pub fn is_local_channel(&self, func: &str, name: &str) -> bool {
        self.local_channels
            .get(func)
            .is_some_and(|set| set.contains(name))
    }

    pub fn is_no_escape_param(&self, func: &str, name: &str) -> bool {
        self.no_escape_params
            .get(func)
            .is_some_and(|set| set.contains(name))
    }

    pub fn bindings_in<'a>(&'a self, func: &'a str) -> impl Iterator<Item = (&'a str, EscapeState)> + 'a {
        self.bindings
            .get(func)
            .into_iter()
            .flat_map(|m| m.iter().map(|(k, v)| (k.as_str(), *v)))
    }

    pub fn tracked_count(&self) -> usize {
        self.bindings.values().map(|m| m.len()).sum()
    }

    /// Human-readable lines for `nyra build --verbose`.
    pub fn report_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "escape analysis: {} local binding(s) tracked",
            self.tracked_count()
        )];
        let mut funcs: Vec<_> = self.bindings.keys().cloned().collect();
        funcs.sort();
        for func in funcs {
            let Some(map) = self.bindings.get(&func) else {
                continue;
            };
            let mut names: Vec<_> = map.keys().cloned().collect();
            names.sort();
            for name in names {
                let state = map[&name];
                lines.push(format!(
                    "  escape: {func}::{name} → {}",
                    state.label()
                ));
            }
            if let Some(chans) = self.local_channels.get(&func) {
                let mut chan_names: Vec<_> = chans.iter().cloned().collect();
                chan_names.sort();
                for name in chan_names {
                    lines.push(format!(
                        "  local channel: {func}::{name} → LocalChannel (stack ring buffer)"
                    ));
                }
            }
            if let Some(params) = self.no_escape_params.get(&func) {
                let mut param_names: Vec<_> = params.iter().cloned().collect();
                param_names.sort();
                for name in param_names {
                    lines.push(format!(
                        "  no_escape param: {func}::{name} (must not return/spawn/send)"
                    ));
                }
            }
        }
        lines
    }
}

/// Run escape analysis on every monomorphic function / method body.
pub fn analyze_escapes(program: &Program) -> EscapePlan {
    let mut plan = EscapePlan::default();
    for func in &program.functions {
        if !func.type_params.is_empty() {
            continue;
        }
        analyze_function(&func.name, &func.params, &func.body, &mut plan);
    }
    for imp in &program.impls {
        for method in &imp.methods {
            analyze_function(&method.name, &method.params, &method.body, &mut plan);
        }
    }
    for ti in &program.trait_impls {
        for method in &ti.methods {
            analyze_function(&method.name, &method.params, &method.body, &mut plan);
        }
    }
    plan
}

fn analyze_function(name: &str, params: &[Param], body: &Block, plan: &mut EscapePlan) {
    let mut graph = EscapeGraph::new();
    let outer: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
    for p in params {
        graph.register(&p.name);
    }
    graph.analyze_block(body, &outer);
    let channel_candidates = graph.channel_candidates.clone();
    let map = graph.finish();
    if !map.is_empty() {
        plan.bindings.insert(name.to_string(), map.clone());
    }
    let local: HashSet<String> = channel_candidates
        .into_iter()
        .filter(|n| {
            map.get(n)
                .copied()
                .unwrap_or(EscapeState::NoEscape)
                == EscapeState::NoEscape
        })
        .collect();
    if !local.is_empty() {
        plan.local_channels.insert(name.to_string(), local);
    }
    let no_esc: HashSet<String> = params
        .iter()
        .filter(|p| p.no_escape)
        .map(|p| p.name.clone())
        .collect();
    if !no_esc.is_empty() {
        plan.no_escape_params.insert(name.to_string(), no_esc);
    }
}

struct EscapeGraph {
    parent: HashMap<String, String>,
    states: HashMap<String, EscapeState>,
    declared: HashSet<String>,
    channel_candidates: HashSet<String>,
}

impl EscapeGraph {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
            states: HashMap::new(),
            declared: HashSet::new(),
            channel_candidates: HashSet::new(),
        }
    }

    fn find(&mut self, name: &str) -> String {
        let name = name.to_string();
        if !self.parent.contains_key(&name) {
            self.parent.insert(name.clone(), name.clone());
        }
        let parent = self.parent[&name].clone();
        if parent != name {
            let root = self.find(&parent);
            self.parent.insert(name, root.clone());
            root
        } else {
            name
        }
    }

    fn union(&mut self, a: &str, b: &str) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent.insert(rb, ra);
        }
    }

    fn register(&mut self, name: &str) {
        self.declared.insert(name.to_string());
        self.find(name);
    }

    fn link_ref(&mut self, ref_name: &str, source: &str) {
        self.register(ref_name);
        self.register(source);
        self.union(ref_name, source);
    }

    fn mark(&mut self, name: &str, level: EscapeState) {
        if !self.declared.contains(name) {
            return;
        }
        let root = self.find(name);
        let entry = self
            .states
            .entry(root)
            .or_insert(EscapeState::NoEscape);
        level.merge_into(entry);
    }

    fn mark_expr_vars(&mut self, expr: &Expression, level: EscapeState) {
        for name in vars_in_expr(expr) {
            self.mark(&name, level);
        }
    }

    fn merge_bindings(&mut self, a: &str, b: &str) {
        if !self.declared.contains(a) || !self.declared.contains(b) {
            return;
        }
        let a_is_chan = self.channel_candidates.contains(a);
        let b_is_chan = self.channel_candidates.contains(b);
        if a_is_chan {
            self.channel_candidates.insert(b.to_string());
        }
        if b_is_chan {
            self.channel_candidates.insert(a.to_string());
        }
        self.union(a, b);
        let ra = self.find(a);
        let rb = self.find(b);
        let merged = self
            .states
            .get(&ra)
            .copied()
            .unwrap_or(EscapeState::NoEscape)
            .max(self.states.get(&rb).copied().unwrap_or(EscapeState::NoEscape));
        let root = self.find(a);
        self.states.insert(root, merged);
    }

    fn analyze_block(&mut self, block: &Block, outer: &HashSet<String>) {
        let mut scope = outer.clone();
        for stmt in &block.statements {
            self.analyze_stmt(stmt, &mut scope);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Statement, scope: &mut HashSet<String>) {
        match stmt {
            Statement::Let(l) | Statement::Const(l) => {
                self.register_binding(l, scope);
                self.register_ref_alias(l);
                if l.destructure.is_empty() && is_channel_origin_expr(&l.value) {
                    self.channel_candidates.insert(l.name.clone());
                }
                if l.destructure.is_empty() {
                    if let Expression::Variable { name: src, .. } = &l.value {
                        self.merge_bindings(&l.name, src);
                    }
                }
                self.analyze_expr_escapes(&l.value, scope);
            }
            Statement::Assign(a) => {
                if let (Some(dst), Some(src)) =
                    (variable_name(&a.target), variable_name(&a.value))
                {
                    self.merge_bindings(dst, src);
                }
                self.analyze_expr_escapes(&a.target, scope);
                self.analyze_expr_escapes(&a.value, scope);
            }
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    self.mark_expr_vars(v, EscapeState::GlobalEscape);
                }
            }
            Statement::If(i) => {
                self.analyze_expr_escapes(&i.condition, scope);
                self.analyze_block(&i.then_block, scope);
                if let Some(e) = &i.else_block {
                    self.analyze_block(e, scope);
                }
            }
            Statement::While(w) => {
                self.analyze_expr_escapes(&w.condition, scope);
                self.analyze_block(&w.body, scope);
            }
            Statement::For(f) => {
                f.for_each_expr(|e| self.analyze_expr_escapes(e, scope));
                scope.insert(f.var.clone());
                self.register(&f.var);
                self.analyze_block(&f.body, scope);
                scope.remove(&f.var);
            }
            Statement::Print(p) => {
                for arg in &p.args {
                    self.analyze_expr_escapes(arg, scope);
                }
                if let Some(c) = &p.color {
                    self.analyze_expr_escapes(c, scope);
                }
            }
            Statement::Expression(e) | Statement::Defer(e) => {
                self.analyze_expr_escapes(e, scope);
            }
            Statement::Spawn(body) => {
                let caps = collect_captures(body, scope);
                for name in caps {
                    self.mark(&name, EscapeState::GlobalEscape);
                }
                self.analyze_block(body, scope);
            }
            Statement::Unsafe(body) => self.analyze_block(body, scope),
            Statement::Benchmark(body) => self.analyze_block(body, scope),
            Statement::Asm { .. } | Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn register_binding(&mut self, l: &LetStmt, scope: &mut HashSet<String>) {
        if l.destructure.is_empty() {
            self.register(&l.name);
            scope.insert(l.name.clone());
        } else {
            for name in &l.destructure {
                self.register(name);
                scope.insert(name.clone());
            }
        }
    }

    fn register_ref_alias(&mut self, l: &LetStmt) {
        let Some(unary) = as_ref_expr(&l.value) else {
            return;
        };
        let Expression::Variable { name: src, .. } = &unary.operand else {
            return;
        };
        let target = if l.destructure.is_empty() {
            l.name.as_str()
        } else {
            return;
        };
        self.link_ref(target, src);
    }

    fn analyze_expr_escapes(&mut self, expr: &Expression, scope: &HashSet<String>) {
        match expr {
            Expression::Call(c) => self.analyze_call(c, scope),
            Expression::MethodCall(mc) => self.analyze_method_call(mc, scope),
            Expression::Unary(u) => {
                if u.op == UnaryOp::Move {
                    self.mark_expr_vars(&u.operand, EscapeState::GlobalEscape);
                }
                self.analyze_expr_escapes(&u.operand, scope);
            }
            Expression::Binary(b) => {
                self.analyze_expr_escapes(&b.left, scope);
                self.analyze_expr_escapes(&b.right, scope);
            }
            Expression::If(i) => {
                self.analyze_expr_escapes(&i.condition, scope);
                for_each_expr_in_block(&i.then_block, &mut |e| self.analyze_expr_escapes(e, scope));
                for_each_expr_in_block(&i.else_block, &mut |e| self.analyze_expr_escapes(e, scope));
            }
            Expression::Match(m) => {
                self.analyze_expr_escapes(&m.scrutinee, scope);
                for arm in &m.arms {
                    for_each_expr_in_block(&arm.body, &mut |e| self.analyze_expr_escapes(e, scope));
                    if let Some(g) = &arm.guard {
                        self.analyze_expr_escapes(g, scope);
                    }
                }
            }
            Expression::FieldAccess(f) => self.analyze_expr_escapes(&f.object, scope),
            Expression::Index(ix) => {
                self.analyze_expr_escapes(&ix.object, scope);
                self.analyze_expr_escapes(&ix.index, scope);
            }
            Expression::StructLiteral(s) => {
                for spread in &s.spreads {
                    self.analyze_expr_escapes(spread, scope);
                }
                for (_, v) in &s.fields {
                    self.analyze_expr_escapes(v, scope);
                }
            }
            Expression::ArrayLiteral(al) => {
                for e in al.all_exprs() {
                    self.analyze_expr_escapes(e, scope);
                }
            }
            Expression::TupleLiteral(elems) => {
                for e in elems {
                    self.analyze_expr_escapes(e, scope);
                }
            }
            Expression::ArrayRepeat { element, .. } => {
                self.analyze_expr_escapes(element, scope);
            }
            Expression::Grouped(inner) => self.analyze_expr_escapes(inner, scope),
            Expression::Await(inner) => self.analyze_expr_escapes(inner, scope),
            Expression::Cast(c) => self.analyze_expr_escapes(&c.expr, scope),
            Expression::TemplateLiteral(t) => {
                for part in &t.parts {
                    if let TemplatePart::Interpolation(e) = part {
                        self.analyze_expr_escapes(e, scope);
                    }
                }
            }
            Expression::ArrowFn(a) => {
                if arrow_has_captures(a) {
                    let caps = collect_arrow_captures(a, scope);
                    for name in caps {
                        self.mark(&name, EscapeState::ArgEscape);
                    }
                }
                match &a.body {
                    ArrowBody::Expr(e) => self.analyze_expr_escapes(e, scope),
                    ArrowBody::Block(b) => self.analyze_block(b, scope),
                }
            }
            Expression::ComptimeBlock { body, .. } => self.analyze_block(body, scope),
            Expression::EnumVariant(v) => {
                for a in &v.args {
                    self.analyze_expr_escapes(a, scope);
                }
            }
            Expression::Variable { .. } | Expression::Literal(_) | Expression::Invalid => {}
        }
    }

    fn analyze_call(&mut self, c: &CallExpr, scope: &HashSet<String>) {
        if channel_send_callee(&c.callee) {
            if let Some(arg) = c.args.get(1) {
                self.mark_expr_vars(arg, EscapeState::GlobalEscape);
                self.analyze_expr_escapes(arg, scope);
            }
            if let Some(ch) = c.args.first() {
                self.analyze_expr_escapes(ch, scope);
            }
            return;
        }
        if channel_recv_callee(&c.callee) || channel_free_callee(&c.callee) {
            if let Some(ch) = c.args.first() {
                self.analyze_expr_escapes(ch, scope);
            }
            return;
        }
        for arg in &c.args {
            self.analyze_call_arg(arg, scope);
        }
    }

    fn analyze_method_call(&mut self, mc: &MethodCallExpr, scope: &HashSet<String>) {
        if mc.method == "send" {
            for arg in &mc.args {
                self.mark_expr_vars(arg, EscapeState::GlobalEscape);
            }
        }
        self.analyze_expr_escapes(&mc.object, scope);
        for arg in &mc.args {
            self.analyze_call_arg(arg, scope);
        }
    }

    fn analyze_call_arg(&mut self, arg: &Expression, scope: &HashSet<String>) {
        match arg {
            Expression::Unary(u)
                if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) =>
            {
                if let Expression::Variable { name, .. } = &u.operand {
                    self.mark(name, EscapeState::ArgEscape);
                }
            }
            Expression::Unary(u) if u.op == UnaryOp::Move => {
                self.mark_expr_vars(&u.operand, EscapeState::GlobalEscape);
            }
            Expression::Variable { name, .. } => {
                self.mark(name, EscapeState::ArgEscape);
            }
            Expression::ArrowFn(a) => {
                if arrow_has_captures(a) {
                    for name in collect_arrow_captures(a, scope) {
                        self.mark(&name, EscapeState::ArgEscape);
                    }
                }
                self.analyze_expr_escapes(arg, scope);
            }
            _ => self.analyze_expr_escapes(arg, scope),
        }
    }

    fn finish(mut self) -> HashMap<String, EscapeState> {
        let declared: Vec<_> = self.declared.iter().cloned().collect();
        let mut out = HashMap::new();
        for name in &declared {
            let root = self.find(name);
            let state = self
                .states
                .get(&root)
                .copied()
                .unwrap_or(EscapeState::NoEscape);
            out.insert(name.clone(), state);
        }
        // Propagate max state across alias components.
        let mut changed = true;
        while changed {
            changed = false;
            for name in &declared {
                let root = self.find(&name);
                let state = out.get(name).copied().unwrap_or(EscapeState::NoEscape);
                let root_state = out.get(&root).copied().unwrap_or(EscapeState::NoEscape);
                let merged = state.max(root_state);
                if merged > out.get(name).copied().unwrap_or(EscapeState::NoEscape) {
                    out.insert(name.clone(), merged);
                    changed = true;
                }
                if merged > out.get(&root).copied().unwrap_or(EscapeState::NoEscape) {
                    out.insert(root.clone(), merged);
                    changed = true;
                }
            }
        }
        out
    }
}

fn channel_new_callee(name: &str) -> bool {
    matches!(name, "channel_new" | "Channel_i32_new")
}

fn channel_send_callee(name: &str) -> bool {
    matches!(name, "channel_send")
}

fn channel_recv_callee(name: &str) -> bool {
    matches!(name, "channel_recv")
}

fn channel_free_callee(name: &str) -> bool {
    matches!(name, "channel_free")
}

fn is_channel_origin_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Call(c) => channel_new_callee(&c.callee),
        Expression::StructLiteral(s) => s
            .fields
            .iter()
            .any(|(_, v)| matches!(v, Expression::Call(c) if channel_new_callee(&c.callee))),
        _ => false,
    }
}

fn as_ref_expr(expr: &Expression) -> Option<&UnaryExpr> {
    match expr {
        Expression::Unary(u) if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) => Some(u),
        Expression::Grouped(inner) => as_ref_expr(inner),
        _ => None,
    }
}

fn vars_in_expr(expr: &Expression) -> Vec<String> {
    let mut out = Vec::new();
    collect_vars(expr, &mut out);
    out
}

fn collect_vars(expr: &Expression, out: &mut Vec<String>) {
    match expr {
        Expression::Variable { name, .. } => out.push(name.clone()),
        Expression::Unary(u) => collect_vars(&u.operand, out),
        Expression::Binary(b) => {
            collect_vars(&b.left, out);
            collect_vars(&b.right, out);
        }
        Expression::Call(c) => {
            for a in &c.args {
                collect_vars(a, out);
            }
        }
        Expression::MethodCall(mc) => {
            collect_vars(&mc.object, out);
            for a in &mc.args {
                collect_vars(a, out);
            }
        }
        Expression::FieldAccess(f) => collect_vars(&f.object, out),
        Expression::Index(ix) => {
            collect_vars(&ix.object, out);
            collect_vars(&ix.index, out);
        }
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_vars(spread, out);
            }
            for (_, v) in &s.fields {
                collect_vars(v, out);
            }
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_vars(e, out);
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_vars(e, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => collect_vars(element, out),
        Expression::Grouped(inner) => collect_vars(inner, out),
        Expression::If(i) => {
            collect_vars(&i.condition, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_vars(e, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_vars(e, out));
        }
        Expression::Match(m) => {
            collect_vars(&m.scrutinee, out);
            for arm in &m.arms {
                for_each_expr_in_block(&arm.body, &mut |e| collect_vars(e, out));
                if let Some(g) = &arm.guard {
                    collect_vars(g, out);
                }
            }
        }
        Expression::Await(inner) => collect_vars(inner, out),
        Expression::Cast(c) => collect_vars(&c.expr, out),
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_vars(e, out);
                }
            }
        }
        Expression::ArrowFn(a) => match &a.body {
            ArrowBody::Expr(e) => collect_vars(e, out),
            ArrowBody::Block(b) => {
                for stmt in &b.statements {
                    collect_vars_from_stmt(stmt, out);
                }
            }
        },
        Expression::ComptimeBlock { body, .. } => {
            for stmt in &body.statements {
                collect_vars_from_stmt(stmt, out);
            }
        }
        Expression::EnumVariant(v) => {
            for a in &v.args {
                collect_vars(a, out);
            }
        }
        Expression::Literal(_) | Expression::Invalid => {}
    }
}

fn collect_vars_from_stmt(stmt: &Statement, out: &mut Vec<String>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => collect_vars(&l.value, out),
        Statement::Assign(a) => {
            collect_vars(&a.target, out);
            collect_vars(&a.value, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_vars(v, out);
            }
        }
        Statement::If(i) => {
            collect_vars(&i.condition, out);
            for s in &i.then_block.statements {
                collect_vars_from_stmt(s, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_vars_from_stmt(s, out);
                }
            }
        }
        Statement::While(w) => {
            collect_vars(&w.condition, out);
            for s in &w.body.statements {
                collect_vars_from_stmt(s, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_vars(e, out));
            for s in &f.body.statements {
                collect_vars_from_stmt(s, out);
            }
        }
        Statement::Print(p) => {
            for a in &p.args {
                collect_vars(a, out);
            }
            if let Some(c) = &p.color {
                collect_vars(c, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => collect_vars(e, out),
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for s in &b.statements {
                collect_vars_from_stmt(s, out);
            }
        }
        Statement::Asm { .. } | Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::Parser;

    fn plan_for(src: &str) -> EscapePlan {
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (program, _) = Parser::new(tokens).parse();
        analyze_escapes(&program)
    }

    fn state(plan: &EscapePlan, func: &str, name: &str) -> EscapeState {
        plan.state_in(func, name)
    }

    #[test]
    fn local_i32_no_escape() {
        let plan = plan_for(
            r#"fn main() {
    let x = 42
    print(x)
}"#,
        );
        assert_eq!(state(&plan, "main", "x"), EscapeState::NoEscape);
    }

    #[test]
    fn local_string_no_escape() {
        let plan = plan_for(
            r#"fn main() {
    let s = "hello"
    print(s)
}"#,
        );
        assert_eq!(state(&plan, "main", "s"), EscapeState::NoEscape);
    }

    #[test]
    fn return_owned_string_global_escape() {
        let plan = plan_for(
            r#"fn mk() -> string {
    let s = "hi"
    return s
}"#,
        );
        assert_eq!(state(&plan, "mk", "s"), EscapeState::GlobalEscape);
    }

    #[test]
    fn spawn_capture_global_escape() {
        let plan = plan_for(
            r#"fn main() {
    let n = 42
    spawn { print(n) }
}"#,
        );
        assert_eq!(state(&plan, "main", "n"), EscapeState::GlobalEscape);
    }

    #[test]
    fn ref_passed_to_call_arg_escape() {
        let plan = plan_for(
            r#"extern fn use_ref(p: &i32) -> void
fn main() {
    let x = 42
    let r = &x
    use_ref(r)
}"#,
        );
        assert_eq!(state(&plan, "main", "x"), EscapeState::ArgEscape);
    }

    #[test]
    fn channel_send_marks_value() {
        let plan = plan_for(
            r#"extern fn channel_new() -> ptr
extern fn channel_send(ch: ptr, value: i32) -> void
fn main() {
    let ch = channel_new()
    let v = 10
    channel_send(ch, v)
}"#,
        );
        assert_eq!(state(&plan, "main", "v"), EscapeState::GlobalEscape);
    }

    #[test]
    fn local_channel_sequential_no_escape() {
        let plan = plan_for(
            r#"extern fn channel_new() -> ptr
extern fn channel_send(ch: ptr, value: i32) -> void
extern fn channel_recv(ch: ptr) -> i32
fn main() {
    let ch = channel_new()
    channel_send(ch, 42)
    let n = channel_recv(ch)
    print(n)
}"#,
        );
        assert_eq!(state(&plan, "main", "ch"), EscapeState::NoEscape);
        assert!(plan.is_local_channel("main", "ch"));
    }

    #[test]
    fn spawn_channel_not_local() {
        let plan = plan_for(
            r#"extern fn channel_new() -> ptr
extern fn channel_send(ch: ptr, value: i32) -> void
extern fn channel_recv(ch: ptr) -> i32
fn main() {
    let ch = channel_new()
    spawn {
        channel_send(ch, 42)
    }
    print(channel_recv(ch))
}"#,
        );
        assert_eq!(state(&plan, "main", "ch"), EscapeState::GlobalEscape);
        assert!(!plan.is_local_channel("main", "ch"));
    }

    #[test]
    fn move_call_arg_global_escape() {
        let plan = plan_for(
            r#"extern fn take(s: string) -> void
fn main() {
    let s = "payload"
    take(move s)
}"#,
        );
        assert_eq!(state(&plan, "main", "s"), EscapeState::GlobalEscape);
    }

    #[test]
    fn assign_merges_escape_state() {
        let plan = plan_for(
            r#"fn main() {
    let b = "y"
    let c = b
    return c
}"#,
        );
        assert_eq!(state(&plan, "main", "b"), EscapeState::GlobalEscape);
        assert_eq!(state(&plan, "main", "c"), EscapeState::GlobalEscape);
    }

    #[test]
    fn struct_local_no_escape() {
        let plan = plan_for(
            r#"struct Point { x: i32 y: i32 }
fn main() {
    let p = Point { x: 1 y: 2 }
    print(p.x)
}"#,
        );
        assert_eq!(state(&plan, "main", "p"), EscapeState::NoEscape);
    }

    #[test]
    fn return_struct_global_escape() {
        let plan = plan_for(
            r#"struct Point { x: i32 y: i32 }
fn mk() -> Point {
    let p = Point { x: 1 y: 2 }
    return p
}"#,
        );
        assert_eq!(state(&plan, "mk", "p"), EscapeState::GlobalEscape);
    }

    #[test]
    fn report_lines_include_bindings() {
        let plan = plan_for(
            r#"fn main() {
    let x = 1
}"#,
        );
        let lines = plan.report_lines();
        assert!(lines.iter().any(|l| l.contains("main::x")));
    }
}
