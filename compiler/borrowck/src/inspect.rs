//! Compile-time ownership inspection for `nyra inspect` and `--ownership-verbose`.

use std::path::Path;

use ast::{stmt_span, Block, Statement};
use ownership::{OwnershipCtx, OwnershipKind};
use types::Type;

use crate::diag::{type_label, MoveOrigin};
use crate::State;

/// Point query for `nyra inspect name --at file:line`.
#[derive(Debug, Clone)]
pub struct InspectQuery {
    pub file: String,
    pub line: usize,
    pub name: String,
}

/// One hop in the ownership chain (`a ──move──► b`).
#[derive(Debug, Clone)]
pub struct ChainHop {
    pub from: String,
    pub to: String,
}

/// One hop in the borrow chain (`myname ◄──borrow── myname2`).
#[derive(Debug, Clone)]
pub struct BorrowChainHop {
    pub from: String,
    pub to: String,
}

/// Ownership snapshot for a single binding.
#[derive(Debug, Clone)]
pub struct BindingInspectReport {
    pub name: String,
    pub func: String,
    pub file: String,
    pub line: usize,
    pub ty: String,
    pub kind: OwnershipKind,
    pub binding_status: BindingStatus,
    /// Root → … → current owner (Move bindings only).
    pub ownership_chain: Vec<String>,
    pub chain_hops: Vec<ChainHop>,
    /// Binding that currently owns the heap value (follows move edges).
    pub current_owner: String,
    /// Root → … → tip for ref bindings (`let x = &y` chains).
    pub borrow_chain: Vec<String>,
    pub borrow_chain_hops: Vec<BorrowChainHop>,
    /// Move-type binding that owns the heap value behind a borrow chain.
    pub heap_owner: String,
    pub role: InspectRole,
    pub borrowed_by: Vec<BorrowerInfo>,
    pub borrows_from: Option<BorrowSource>,
    pub move_origin: Option<MoveOrigin>,
    pub moved_to: Option<String>,
    pub moved_from: Option<String>,
    pub drop: Option<DropInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStatus {
    Valid,
    Moved,
    NotInScope,
}

/// How the inspected binding relates to the value at this line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectRole {
    /// This binding currently owns the value.
    Owner,
    /// Ownership was moved to another binding.
    MovedAway,
    /// Binding holds a reference (`let x = &y`); does not own the heap value.
    Borrower,
    /// Binding is a Copy type (independent of move chain).
    CopyBinding,
    NotInScope,
}

#[derive(Debug, Clone)]
pub struct BorrowerInfo {
    pub name: String,
    pub mutable: bool,
    pub ty: String,
    pub expires_after_line: usize,
}

#[derive(Debug, Clone)]
pub struct BorrowSource {
    pub name: String,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub struct DropInfo {
    pub at_scope_exit_line: usize,
    pub kind: &'static str,
}

/// Per-function summary for `--ownership-verbose`.
#[derive(Debug, Clone, Default)]
pub struct OwnershipVerbosePlan {
    pub entries: Vec<VerboseEntry>,
}

#[derive(Debug, Clone)]
pub struct VerboseEntry {
    pub func: String,
    pub name: String,
    pub ty: String,
    pub kind: OwnershipKind,
    pub status: BindingStatus,
}

impl OwnershipVerbosePlan {
    pub fn report_lines(&self) -> Vec<String> {
        if self.entries.is_empty() {
            return vec!["ownership: no bindings tracked".into()];
        }
        let mut lines = vec![format!(
            "ownership: {} binding(s) at function exit",
            self.entries.len()
        )];
        let mut funcs: Vec<_> = self
            .entries
            .iter()
            .map(|e| e.func.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        funcs.sort();
        for func in funcs {
            let mut names: Vec<_> = self
                .entries
                .iter()
                .filter(|e| e.func == func)
                .collect();
            names.sort_by(|a, b| a.name.cmp(&b.name));
            for e in names {
                lines.push(format!(
                    "  ownership: {func}::{} → {} ({})",
                    e.name,
                    kind_label(e.kind),
                    status_label(e.status),
                ));
            }
        }
        lines
    }
}

/// Collector wired into the borrow-check walk.
#[derive(Debug, Default)]
pub struct InspectCollector {
    query: Option<InspectQuery>,
    verbose: bool,
    pub query_result: Option<BindingInspectReport>,
    verbose_entries: Vec<VerboseEntry>,
    current_func: String,
}

#[derive(Debug, Clone)]
struct BlockLines {
    lines: Vec<usize>,
    scope_exit_line: usize,
}

impl InspectCollector {
    pub fn for_query(query: InspectQuery) -> Self {
        Self {
            query: Some(query),
            ..Self::default()
        }
    }

    pub fn verbose() -> Self {
        Self {
            verbose: true,
            ..Self::default()
        }
    }

    pub fn set_func(&mut self, func: &str) {
        self.current_func = func.to_string();
    }

    pub fn into_verbose_plan(self) -> OwnershipVerbosePlan {
        OwnershipVerbosePlan {
            entries: self.verbose_entries,
        }
    }

    pub fn on_after_stmt(
        &mut self,
        stmt: &Statement,
        stmt_idx: usize,
        block: &Block,
        state: &State,
        ctx: &OwnershipCtx,
    ) {
        let block_lines = block_line_map(block);
        let line = stmt_span(stmt).start.line;
        let file = stmt_span(stmt).file.clone();

        if let Some(ref query) = self.query {
            if query.line == line && file_matches(&file, &query.file) {
                self.query_result = Some(build_report(
                    &query.name,
                    &self.current_func,
                    &file,
                    line,
                    state,
                    ctx,
                    &block_lines,
                    stmt_idx,
                ));
            }
        }
    }

    pub fn on_block_exit(&mut self, block: &Block, state: &State, ctx: &OwnershipCtx) {
        if !self.verbose {
            return;
        }
        let _block_lines = block_line_map(block);
        for (name, ty) in &state.var_types {
            let status = if state.moved.contains_key(name) {
                BindingStatus::Moved
            } else {
                BindingStatus::Valid
            };
            self.verbose_entries.push(VerboseEntry {
                func: self.current_func.clone(),
                name: name.clone(),
                ty: type_label(ty),
                kind: ctx.kind_of(ty),
                status,
            });
        }
    }
}

pub fn inspect_binding(
    program: &ast::Program,
    ctx: &OwnershipCtx,
    query: &InspectQuery,
) -> Result<BindingInspectReport, String> {
    let mut collector = InspectCollector::for_query(query.clone());
    let mut errors = Vec::new();
    let mut inspect_opt = Some(&mut collector);
    super::check_program_with_collector(program, ctx, &mut errors, &mut inspect_opt);
    collector.query_result.ok_or_else(|| {
        format!(
            "binding `{}` not in scope at {}:{}",
            query.name, query.file, query.line
        )
    })
}

pub fn analyze_ownership_verbose(
    program: &ast::Program,
    ctx: &OwnershipCtx,
) -> OwnershipVerbosePlan {
    let mut collector = InspectCollector::verbose();
    let mut errors = Vec::new();
    let mut inspect_opt = Some(&mut collector);
    super::check_program_with_collector(program, ctx, &mut errors, &mut inspect_opt);
    collector.into_verbose_plan()
}

/// Follow `move_to` edges to the binding that currently holds ownership.
fn current_owner_of(name: &str, state: &State) -> String {
    let mut cur = name.to_string();
    while let Some(next) = state.move_to.get(&cur) {
        cur = next.clone();
    }
    cur
}

/// Build root → … → tip chain for a Move value reachable from `name`.
fn ownership_chain_for(name: &str, state: &State) -> Vec<String> {
    let tip = current_owner_of(name, state);
    let mut chain = vec![tip];
    let mut cur = chain[0].clone();
    while let Some(src) = state.move_from.get(&cur) {
        chain.insert(0, src.clone());
        cur = src.clone();
    }
    chain
}

fn chain_hops_from(chain: &[String]) -> Vec<ChainHop> {
    chain
        .windows(2)
        .map(|w| ChainHop {
            from: w[0].clone(),
            to: w[1].clone(),
        })
        .collect()
}

fn format_chain_diagram(chain: &[String]) -> String {
    if chain.is_empty() {
        return "(none)".into();
    }
    if chain.len() == 1 {
        return chain[0].clone();
    }
    chain.join(" ──move──► ")
}

fn format_borrow_chain_diagram(chain: &[String]) -> String {
    if chain.is_empty() {
        return "(none)".into();
    }
    if chain.len() == 1 {
        return chain[0].clone();
    }
    chain
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == 0 {
                name.clone()
            } else {
                format!("◄──borrow── {name}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn borrow_chain_for(name: &str, state: &State) -> Vec<String> {
    let mut chain = vec![name.to_string()];
    let mut cur = name;
    while let Some(src) = state.ref_sources.get(cur) {
        chain.insert(0, src.clone());
        cur = src;
    }
    chain
}

fn borrow_chain_hops_from(chain: &[String]) -> Vec<BorrowChainHop> {
    chain
        .windows(2)
        .map(|w| BorrowChainHop {
            from: w[0].clone(),
            to: w[1].clone(),
        })
        .collect()
}

fn heap_owner_for(name: &str, state: &State, ctx: &OwnershipCtx) -> String {
    let root = borrow_chain_for(name, state)
        .first()
        .cloned()
        .unwrap_or_else(|| name.to_string());
    if state
        .var_types
        .get(&root)
        .is_some_and(|ty| ctx.kind_of(ty).is_move())
    {
        current_owner_of(&root, state)
    } else {
        root
    }
}

fn is_ref_binding(name: &str, ty: &Type, state: &State) -> bool {
    state.ref_sources.contains_key(name) || matches!(ty, Type::Ref { .. })
}

fn build_report(
    name: &str,
    func: &str,
    file: &str,
    line: usize,
    state: &State,
    ctx: &OwnershipCtx,
    block_lines: &BlockLines,
    _stmt_idx: usize,
) -> BindingInspectReport {
    let Some(ty) = state.var_types.get(name) else {
        return BindingInspectReport {
            name: name.to_string(),
            func: func.to_string(),
            file: file.to_string(),
            line,
            ty: "unknown".into(),
            kind: OwnershipKind::Copy,
            binding_status: BindingStatus::NotInScope,
            ownership_chain: vec![],
            chain_hops: vec![],
            current_owner: name.to_string(),
            borrow_chain: vec![],
            borrow_chain_hops: vec![],
            heap_owner: name.to_string(),
            role: InspectRole::NotInScope,
            borrowed_by: vec![],
            borrows_from: None,
            move_origin: None,
            moved_to: None,
            moved_from: None,
            drop: None,
        };
    };

    let kind = ctx.kind_of(ty);
    let binding_status = if state.moved.contains_key(name) {
        BindingStatus::Moved
    } else {
        BindingStatus::Valid
    };

    let is_borrower = is_ref_binding(name, ty, state);
    let borrow_chain = if is_borrower {
        borrow_chain_for(name, state)
    } else {
        vec![]
    };
    let borrow_chain_hops = borrow_chain_hops_from(&borrow_chain);
    let heap_owner = if is_borrower {
        heap_owner_for(name, state, ctx)
    } else {
        name.to_string()
    };

    let ownership_chain = if !is_borrower && kind.is_move() {
        ownership_chain_for(name, state)
    } else {
        vec![]
    };
    let chain_hops = chain_hops_from(&ownership_chain);
    let current_owner = if is_borrower {
        heap_owner.clone()
    } else if kind.is_move() {
        current_owner_of(name, state)
    } else {
        name.to_string()
    };

    let role = match binding_status {
        BindingStatus::NotInScope => InspectRole::NotInScope,
        BindingStatus::Valid if is_borrower => InspectRole::Borrower,
        BindingStatus::Valid if kind.is_copy() => InspectRole::CopyBinding,
        BindingStatus::Valid => InspectRole::Owner,
        BindingStatus::Moved => InspectRole::MovedAway,
    };

    let moved_to = if is_borrower {
        None
    } else {
        state.move_to.get(name).cloned()
    };
    let moved_from = if is_borrower {
        None
    } else {
        state.move_from.get(name).cloned()
    };

    let borrowed_by = active_borrowers(name, state, block_lines);
    let borrows_from = state.ref_sources.get(name).map(|source| BorrowSource {
        name: source.clone(),
        mutable: state.borrowed_mut.contains(source),
    });

    let move_origin = state.moved.get(name).cloned();

    let drop = if kind.is_move() && binding_status == BindingStatus::Valid {
        Some(DropInfo {
            at_scope_exit_line: block_lines.scope_exit_line,
            kind: "auto-drop",
        })
    } else {
        None
    };

    BindingInspectReport {
        name: name.to_string(),
        func: func.to_string(),
        file: file.to_string(),
        line,
        ty: type_label(ty),
        kind,
        binding_status,
        ownership_chain,
        chain_hops,
        current_owner,
        borrow_chain,
        borrow_chain_hops,
        heap_owner,
        role,
        borrowed_by,
        borrows_from,
        move_origin,
        moved_to,
        moved_from,
        drop,
    }
}

fn active_borrowers(name: &str, state: &State, block_lines: &BlockLines) -> Vec<BorrowerInfo> {
    let mut out = Vec::new();
    for b in &state.active_borrows {
        if b.source != name {
            continue;
        }
        let expires_after_line = block_lines
            .lines
            .get(b.expires_after)
            .copied()
            .unwrap_or(block_lines.scope_exit_line);
        if let Some(borrower) = &b.borrower {
            let ty = state
                .var_types
                .get(borrower)
                .map(type_label)
                .unwrap_or_else(|| if b.mutable { "&mut".into() } else { "&".into() });
            out.push(BorrowerInfo {
                name: borrower.clone(),
                mutable: b.mutable,
                ty,
                expires_after_line,
            });
        } else {
            out.push(BorrowerInfo {
                name: "<expr>".into(),
                mutable: b.mutable,
                ty: if b.mutable {
                    "&mut".into()
                } else {
                    "&".into()
                },
                expires_after_line,
            });
        }
    }
    out
}

fn block_line_map(block: &Block) -> BlockLines {
    let lines: Vec<usize> = block
        .statements
        .iter()
        .map(|s| stmt_span(s).start.line)
        .collect();
    let scope_exit_line = block
        .statements
        .last()
        .map(|s| stmt_span(s).end.line)
        .unwrap_or(1);
    BlockLines {
        lines,
        scope_exit_line,
    }
}

fn file_matches(span_file: &str, query_file: &str) -> bool {
    if span_file == query_file {
        return true;
    }
    let span_name = Path::new(span_file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(span_file);
    let query_name = Path::new(query_file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(query_file);
    span_name == query_name
}

fn kind_label(kind: OwnershipKind) -> &'static str {
    match kind {
        OwnershipKind::Copy => "Copy",
        OwnershipKind::Move => "Move",
    }
}

fn status_label(status: BindingStatus) -> &'static str {
    match status {
        BindingStatus::Valid => "valid",
        BindingStatus::Moved => "moved",
        BindingStatus::NotInScope => "not in scope",
    }
}

impl BindingInspectReport {
    pub fn format_lines(&self) -> Vec<String> {
        let file_display = Path::new(&self.file)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.file);

        let mut lines = vec![format!(
            "{} ({}) @ {}:{} in {}:",
            self.name, self.ty, file_display, self.line, self.func
        )];

        if self.role == InspectRole::Borrower {
            lines.push(format!("  you inspect: `{}` (borrower — does not own heap value)", self.name));
            lines.push(format!(
                "  heap owner: `{}` (owns the Move value on the heap)",
                self.heap_owner
            ));
            if self.borrow_chain.len() > 1 {
                lines.push("  borrow chain:".into());
                lines.push(format!("    {}", format_borrow_chain_diagram(&self.borrow_chain)));
            } else if let Some(src) = &self.borrows_from {
                lines.push(format!("  borrow chain: `{}` ◄──borrow── `{}`", src.name, self.name));
            }
        } else {
            if self.kind.is_move() && self.ownership_chain.len() > 1 {
                lines.push("  ownership chain:".into());
                lines.push(format!("    {}", format_chain_diagram(&self.ownership_chain)));
            } else if self.kind.is_move() {
                lines.push(format!("  ownership chain: {}", self.name));
            }

            lines.push(format!("  you inspect: `{}`", self.name));
            match &self.role {
                InspectRole::Owner => {
                    lines.push(format!(
                        "  current owner: `{}` (this binding owns the value)",
                        self.current_owner
                    ));
                }
                InspectRole::MovedAway => {
                    lines.push(format!(
                        "  current owner: `{}` (`{}` no longer owns — follow chain above)",
                        self.current_owner,
                        self.name
                    ));
                }
                InspectRole::CopyBinding => {
                    lines.push(format!(
                        "  current owner: `{}` (Copy — independent value)",
                        self.name
                    ));
                }
                InspectRole::Borrower | InspectRole::NotInScope => {}
            }
        }

        lines.push(format!(
            "  kind: {}",
            if self.role == InspectRole::Borrower {
                format!("{} (reference)", kind_label(self.kind))
            } else {
                kind_label(self.kind).to_string()
            }
        ));
        lines.push(format!(
            "  binding: {}",
            match self.binding_status {
                BindingStatus::Valid if self.role == InspectRole::Borrower => "valid (borrow)",
                BindingStatus::Valid => {
                    if self.kind.is_move() {
                        "owned (valid)"
                    } else {
                        "valid (Copy)"
                    }
                }
                BindingStatus::Moved => "moved (invalid)",
                BindingStatus::NotInScope => "not in scope",
            }
        ));

        if self.binding_status == BindingStatus::Moved {
            if let Some(dest) = &self.moved_to {
                lines.push(format!("  moved to: `{dest}`"));
            } else if let Some(origin) = &self.move_origin {
                if let Some(callee) = &origin.callee {
                    lines.push(format!("  moved into: `{callee}`"));
                } else {
                    lines.push("  moved: yes".into());
                }
            } else {
                lines.push("  moved: yes".into());
            }
        } else {
            lines.push("  moved: no".into());
        }

        if let Some(src) = &self.moved_from {
            lines.push(format!("  moved from: `{src}`"));
        }
        if let Some(src) = &self.borrows_from {
            let mutability = if src.mutable { "&mut" } else { "&" };
            lines.push(format!(
                "  borrows from: `{}` ({mutability})",
                src.name
            ));
        }

        if self.borrowed_by.is_empty() {
            lines.push("  borrowed by: (none)".into());
        } else {
            for b in &self.borrowed_by {
                let mutability = if b.mutable { "&mut" } else { "&" };
                lines.push(format!(
                    "  borrowed by: `{}` ({mutability}{}), expires after line {}",
                    b.name, b.ty, b.expires_after_line
                ));
            }
        }

        if let Some(drop) = &self.drop {
            lines.push(format!(
                "  drop: {} at scope exit (line {})",
                drop.kind, drop.at_scope_exit_line
            ));
        }

        lines
    }
}
