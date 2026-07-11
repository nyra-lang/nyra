//! IDE-oriented document analysis: symbols, hover, completion, rename.

mod inlay;
mod rename_edits;
mod semantic;
mod signature;
mod workspace;

pub use inlay::{collect_inlay_hints, InlayHintInfo, InlayHintKind};
pub use rename_edits::RenameTextEdit;
pub use semantic::{collect_semantic_tokens, DocumentToken, DocumentTokenKind, TokenModifiers};
pub use signature::{signature_help_at, SignatureHelpInfo};
pub use workspace::{span_to_lsp_range, SymbolLocation, WorkspaceIndex};

use std::collections::HashSet;

use ast::{
    expr_span, for_each_expr_in_block, Expression, Function, LetStmt, Param, Program, Statement,
    StructDef, TypeAnnotation,
};
use errors::Span;
use expand::expand_program;
use expand::desugar_try;
use expand::synthesize_struct_json_helpers;
use lexer::Lexer;
use monomorph::monomorphize_program;
use parser::Parser;
use typecheck::TypeChecker;

pub(crate) const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "const", "if", "else", "while", "for", "return", "struct", "enum",
    "match", "impl", "import", "module", "extern", "export", "spawn", "print", "in", "trait",
    "async", "await", "defer", "unsafe", "asm", "test", "no_std", "true", "false", "void",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Parameter,
    Variable,
    Constant,
    Struct,
    Enum,
    Field,
    Method,
    Extern,
    Module,
    Keyword,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub detail: Option<String>,
    pub renameable: bool,
}

#[derive(Debug, Clone)]
pub struct DocumentAnalysis {
    pub symbols: Vec<Symbol>,
    pub parse_ok: bool,
    pub typecheck_ok: bool,
    pub inlay_hints: Vec<InlayHintInfo>,
}

impl DocumentAnalysis {
    pub fn analyze(source: &str, file: &str) -> Self {
        let (tokens, lex_errs) = Lexer::new(source, file).tokenize();
        if !lex_errs.is_empty() {
            return Self::keywords_only();
        }
        let (mut program, parse_errs) = Parser::new(tokens).parse();
        if !parse_errs.is_empty() {
            return Self::keywords_only();
        }
        expand_program(&mut program);
        monomorphize_program(&mut program);
        synthesize_struct_json_helpers(&mut program);
        desugar_try(&mut program);

        let mut checker = TypeChecker::new();
        checker.check_program(&program);
        let typecheck_ok = !checker.has_errors();
        let inlay_hints = collect_inlay_hints(&checker, &program);

        let mut symbols = collect_symbols(&program);
        symbols.extend(keyword_symbols());
        symbols.sort_by(|a, b| {
            a.span
                .start
                .line
                .cmp(&b.span.start.line)
                .then(a.span.start.column.cmp(&b.span.start.column))
        });

        Self {
            symbols,
            parse_ok: true,
            typecheck_ok,
            inlay_hints,
        }
    }

    fn keywords_only() -> Self {
        Self {
            symbols: keyword_symbols(),
            parse_ok: false,
            typecheck_ok: false,
            inlay_hints: vec![],
        }
    }

    pub fn symbol_at(&self, line: u32, character: u32) -> Option<&Symbol> {
        let line1 = line as usize + 1;
        let col1 = character as usize + 1;
        if let Some(sym) = self
            .symbols
            .iter()
            .filter(|s| s.kind != SymbolKind::Keyword)
            .find(|s| position_in_span(line1, col1, &s.span))
        {
            return Some(sym);
        }
        None
    }

    pub fn symbol_at_position(&self, source: &str, line: u32, character: u32) -> Option<&Symbol> {
        if let Some(sym) = self.symbol_at(line, character) {
            return Some(sym);
        }
        let word = word_at(source, line, character)?;
        self.symbols
            .iter()
            .filter(|s| s.kind != SymbolKind::Keyword && s.name == word)
            .max_by_key(|s| match s.kind {
                SymbolKind::Function | SymbolKind::Variable | SymbolKind::Parameter => 3,
                SymbolKind::Struct | SymbolKind::Enum => 2,
                _ => 1,
            })
    }

    pub fn completions(&self, prefix: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let pfx = prefix.to_lowercase();
        for s in &self.symbols {
            if s.name.to_lowercase().starts_with(&pfx) && seen.insert(s.name.clone()) {
                out.push(s.name.clone());
            }
        }
        out.sort();
        out
    }

    pub fn rename_ranges(&self, symbol: &Symbol) -> Vec<Span> {
        if !symbol.renameable {
            return vec![];
        }
        self.symbols
            .iter()
            .filter(|s| s.name == symbol.name && s.kind == symbol.kind)
            .map(|s| s.span.clone())
            .collect()
    }
}

fn keyword_symbols() -> Vec<Symbol> {
    KEYWORDS
        .iter()
        .map(|kw| Symbol {
            name: (*kw).into(),
            kind: SymbolKind::Keyword,
            span: Span::default(),
            detail: Some("keyword".into()),
            renameable: false,
        })
        .collect()
}

fn collect_symbols(program: &Program) -> Vec<Symbol> {
    let mut out = Vec::new();
    if let Some(module) = &program.module {
        out.push(Symbol {
            name: module.clone(),
            kind: SymbolKind::Module,
            span: Span::default(),
            detail: None,
            renameable: false,
        });
    }
    for s in &program.structs {
        collect_struct(s, &mut out);
    }
    for e in &program.enums {
        out.push(Symbol {
            name: e.name.clone(),
            kind: SymbolKind::Enum,
            span: Span::default(),
            detail: Some(enum_detail(e)),
            renameable: false,
        });
    }
    for ext in &program.externs {
        out.push(Symbol {
            name: ext.name.clone(),
            kind: SymbolKind::Extern,
            span: Span::default(),
            detail: Some(fn_sig_detail(
                &ext.params,
                ext.return_type.as_ref(),
            )),
            renameable: false,
        });
    }
    for f in &program.functions {
        collect_function(f, &mut out);
    }
    for imp in &program.impls {
        for m in &imp.methods {
            collect_function(m, &mut out);
        }
    }
    for c in &program.consts {
        out.push(Symbol {
            name: c.name.clone(),
            kind: SymbolKind::Constant,
            span: Span::default(),
            detail: c.ty.as_ref().map(|t| format_type_ann(t)),
            renameable: true,
        });
    }
    out
}

fn collect_struct(s: &StructDef, out: &mut Vec<Symbol>) {
    out.push(Symbol {
        name: s.name.clone(),
        kind: SymbolKind::Struct,
        span: Span::default(),
        detail: Some(struct_detail(s)),
        renameable: false,
    });
    for field in &s.fields {
        out.push(Symbol {
            name: field.name.clone(),
            kind: SymbolKind::Field,
            span: Span::default(),
            detail: Some(format!("{}: {}", s.name, format_type_ann(&field.ty))),
            renameable: false,
        });
    }
    let enc = format!("{}_json_encode", s.name);
    let dec = format!("{}_json_decode", s.name);
    out.push(Symbol {
        name: enc.clone(),
        kind: SymbolKind::Function,
        span: Span::default(),
        detail: Some(format!(
            "fn {enc}(self: {}) -> string  // synthesized JSON encode",
            s.name
        )),
        renameable: false,
    });
    out.push(Symbol {
        name: dec,
        kind: SymbolKind::Function,
        span: Span::default(),
        detail: Some(format!(
            "fn {}(json: string) -> {}  // synthesized JSON decode",
            format!("{}_json_decode", s.name),
            s.name
        )),
        renameable: false,
    });
}

fn collect_function(f: &Function, out: &mut Vec<Symbol>) {
    out.push(Symbol {
        name: f.name.clone(),
        kind: SymbolKind::Function,
        span: f.span.clone(),
        detail: Some(fn_sig_detail(&f.params, f.return_type.as_ref())),
        renameable: true,
    });
    for p in &f.params {
        if !p.destructure.is_empty() {
            for name in &p.destructure {
                out.push(param_symbol(name, &p.ty));
            }
        } else {
            out.push(param_symbol(&p.name, &p.ty));
        }
    }
    collect_block_symbols(&f.body, out);
}

fn param_symbol(name: &str, ty: &TypeAnnotation) -> Symbol {
    Symbol {
        name: name.into(),
        kind: SymbolKind::Parameter,
        span: Span::default(),
        detail: Some(format_type_ann(ty)),
        renameable: true,
    }
}

fn collect_block_symbols(block: &ast::Block, out: &mut Vec<Symbol>) {
    for stmt in &block.statements {
        collect_stmt_symbols(stmt, out);
    }
}

fn collect_stmt_symbols(stmt: &Statement, out: &mut Vec<Symbol>) {
    match stmt {
        Statement::Let(ls) | Statement::Const(ls) => {
            collect_let_symbols(ls, out);
            collect_expr_refs(&ls.value, out);
        }
        Statement::Assign(a) => {
            collect_expr_refs(&a.target, out);
            collect_expr_refs(&a.value, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_expr_refs(v, out);
            }
        }
        Statement::If(i) => {
            collect_expr_refs(&i.condition, out);
            collect_block_symbols(&i.then_block, out);
            if let Some(e) = &i.else_block {
                collect_block_symbols(e, out);
            }
        }
        Statement::While(w) => {
            collect_expr_refs(&w.condition, out);
            collect_block_symbols(&w.body, out);
        }
        Statement::For(f) => {
            match &f.kind {
                ast::ForKind::Range { start, end } => {
                    collect_expr_refs(start, out);
                    collect_expr_refs(end, out);
                }
                ast::ForKind::Iterable { iterable } => collect_expr_refs(iterable, out),
            }
            out.push(Symbol {
                name: f.var.clone(),
                kind: SymbolKind::Variable,
                span: Span::default(),
                detail: Some("i32".into()),
                renameable: true,
            });
            collect_block_symbols(&f.body, out);
        }
        Statement::Expression(e) | Statement::Defer(e) => collect_expr_refs(e, out),
        Statement::Print(p) => {
            for a in &p.args {
                collect_expr_refs(a, out);
            }
            if let Some(c) = &p.color {
                collect_expr_refs(c, out);
            }
        }
        Statement::Spawn(s) => collect_block_symbols(&s.body, out),
        Statement::Unsafe(b) | Statement::Benchmark(b) => collect_block_symbols(b, out),
        _ => {}
    }
}

fn collect_let_symbols(ls: &LetStmt, out: &mut Vec<Symbol>) {
    if !ls.destructure.is_empty() {
        for name in &ls.destructure {
            out.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable,
                span: ls.span.clone(),
                detail: ls.ty.as_ref().map(format_type_ann),
                renameable: true,
            });
        }
    } else {
        out.push(Symbol {
            name: ls.name.clone(),
            kind: SymbolKind::Variable,
            span: ls.span.clone(),
            detail: ls.ty.as_ref().map(format_type_ann),
            renameable: true,
        });
    }
}

fn collect_expr_refs(expr: &Expression, out: &mut Vec<Symbol>) {
    match expr {
        Expression::Variable { name, span } => {
            out.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable,
                span: span.clone(),
                detail: None,
                renameable: false,
            });
        }
        Expression::Binary(b) => {
            collect_expr_refs(&b.left, out);
            collect_expr_refs(&b.right, out);
        }
        Expression::Unary(u) => collect_expr_refs(&u.operand, out),
        Expression::Call(c) => {
            out.push(Symbol {
                name: c.callee.clone(),
                kind: SymbolKind::Function,
                span: c.span.clone(),
                detail: None,
                renameable: false,
            });
            for a in &c.args {
                collect_expr_refs(a, out);
            }
        }
        Expression::MethodCall(m) => {
            collect_expr_refs(&m.object, out);
            for a in &m.args {
                collect_expr_refs(a, out);
            }
        }
        Expression::FieldAccess(f) => collect_expr_refs(&f.object, out),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_expr_refs(spread, out);
            }
            for (_, v) in &s.fields {
                collect_expr_refs(v, out);
            }
        }
        Expression::EnumVariant(v) => {
            for a in &v.args {
                collect_expr_refs(a, out);
            }
        }
        Expression::Match(m) => {
            collect_expr_refs(&m.scrutinee, out);
            for arm in &m.arms {
                for_each_expr_in_block(&arm.body, &mut |e| collect_expr_refs(e, out));
                if let Some(g) = &arm.guard {
                    collect_expr_refs(g, out);
                }
            }
        }
        Expression::If(i) => {
            collect_expr_refs(&i.condition, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_expr_refs(e, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_expr_refs(e, out));
        }
        Expression::Index(ix) => {
            collect_expr_refs(&ix.object, out);
            collect_expr_refs(&ix.index, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_expr_refs(e, out);
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_expr_refs(e, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => collect_expr_refs(element, out),
        Expression::Grouped(inner) => collect_expr_refs(inner, out),
        Expression::Await(inner) => collect_expr_refs(inner, out),
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let ast::TemplatePart::Interpolation(e) = part {
                    collect_expr_refs(e, out);
                }
            }
        }
        Expression::Cast(c) => collect_expr_refs(&c.expr, out),
        Expression::ArrowFn(a) => match &a.body {
            ast::ArrowBody::Expr(e) => collect_expr_refs(e, out),
            ast::ArrowBody::Block(b) => collect_block_symbols(b, out),
        },
        Expression::ComptimeBlock { body, .. } => collect_block_symbols(body, out),
        Expression::Spawn { body, .. } => collect_block_symbols(body, out),
        Expression::ParallelSearch(ps) => {
            ps.for_each_expr(|e| collect_expr_refs(e, out));
            collect_block_symbols(&ps.body, out);
        }
        Expression::Literal(_) | Expression::Invalid => {}
    }
    let _ = expr_span(expr);
}

fn word_at(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.lines().nth(line as usize)?;
    let col = character as usize;
    if col > line_text.len() {
        return None;
    }
    let bytes: Vec<(usize, char)> = line_text.char_indices().collect();
    let mut start_idx = 0;
    let mut end_idx = line_text.len();
    for (i, (byte_idx, _ch)) in bytes.iter().enumerate() {
        if *byte_idx <= col && (i + 1 >= bytes.len() || bytes[i + 1].0 > col) {
            let mut s = i;
            let mut e = i;
            while s > 0 && (bytes[s - 1].1.is_ascii_alphanumeric() || bytes[s - 1].1 == '_') {
                s -= 1;
            }
            while e + 1 < bytes.len()
                && (bytes[e + 1].1.is_ascii_alphanumeric() || bytes[e + 1].1 == '_')
            {
                e += 1;
            }
            start_idx = bytes[s].0;
            end_idx = if e + 1 < bytes.len() {
                bytes[e + 1].0
            } else {
                line_text.len()
            };
            break;
        }
    }
    let word = &line_text[start_idx..end_idx];
    if word.is_empty()
        || !word
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(word.to_string())
}

pub fn find_name_occurrences(source: &str, file: &str, name: &str) -> Vec<Span> {
    let mut out = Vec::new();
    if name.is_empty() {
        return out;
    }
    for (line_idx, line) in source.lines().enumerate() {
        let mut start = 0;
        while let Some(rel) = line[start..].find(name) {
            let col_start = start + rel;
            let col_end = col_start + name.len();
            let before_ok = col_start == 0 || !is_ident_char(line.as_bytes()[col_start - 1]);
            let after_ok =
                col_end >= line.len() || !is_ident_char(line.as_bytes()[col_end]);
            if before_ok && after_ok {
                out.push(Span {
                    file: file.into(),
                    start: errors::Position {
                        line: line_idx + 1,
                        column: col_start + 1,
                    },
                    end: errors::Position {
                        line: line_idx + 1,
                        column: col_end + 1,
                    },
                });
            }
            start = col_end;
        }
    }
    out
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub fn position_in_span(line: usize, column: usize, span: &Span) -> bool {
    if span.start.line == 0 && span.end.line == 0 {
        return false;
    }
    let start = span.start;
    let end = span.end;
    if line < start.line || line > end.line {
        return false;
    }
    if line == start.line && column < start.column {
        return false;
    }
    if line == end.line && column > end.column {
        return false;
    }
    true
}

pub fn offset_at_position(source: &str, line: u32, character: u32) -> Option<usize> {
    let target_line = line as usize + 1;
    let target_col = character as usize + 1;
    let mut line_no = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if line_no == target_line && col == target_col {
            return Some(i);
        }
        if ch == '\n' {
            line_no += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    if line_no == target_line && col == target_col {
        Some(source.len())
    } else {
        None
    }
}

pub fn apply_rename(source: &str, symbol: &Symbol, new_name: &str) -> String {
    if new_name.is_empty() || !is_valid_ident(new_name) {
        return source.to_string();
    }
    let analysis = DocumentAnalysis::analyze(source, "rename.ny");
    let ranges = analysis.rename_ranges(symbol);
    if ranges.is_empty() {
        return source.to_string();
    }
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for span in ranges {
        if let (Some(start), Some(end)) = (
            offset_at_position(source, (span.start.line - 1) as u32, (span.start.column - 1) as u32),
            offset_at_position(source, (span.end.line - 1) as u32, (span.end.column - 1) as u32),
        ) {
            edits.push((start, end, new_name.to_string()));
        }
    }
    edits.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = source.to_string();
    for (start, end, text) in edits {
        if start <= out.len() && end <= out.len() && start < end {
            out.replace_range(start..end, &text);
        }
    }
    out
}

fn is_valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn format_type_ann(ty: &TypeAnnotation) -> String {
    nyra_fmt::format_type(ty)
}

fn fn_sig_detail(params: &[Param], ret: Option<&TypeAnnotation>) -> String {
    let ps: Vec<String> = params
        .iter()
        .map(|p| format!("{}: {}", p.name, format_type_ann(&p.ty)))
        .collect();
    let ret_s = ret.map(format_type_ann).unwrap_or_else(|| "void".into());
    format!("fn({}) -> {ret_s}", ps.join(", "))
}

fn struct_detail(s: &StructDef) -> String {
    let fields: Vec<String> = s
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.name, format_type_ann(&f.ty)))
        .collect();
    format!("struct {} {{ {} }}", s.name, fields.join(", "))
}

fn enum_detail(e: &ast::EnumDef) -> String {
    let variants: Vec<String> = e
        .variants
        .iter()
        .map(|v| {
            if v.fields.is_empty() {
                v.name.clone()
            } else {
                format!(
                    "{}({})",
                    v.name,
                    v.fields.iter().map(format_type_ann).collect::<Vec<_>>().join(", ")
                )
            }
        })
        .collect();
    format!("enum {} {{ {} }}", e.name, variants.join(", "))
}

pub fn hover_at(source: &str, file: &str, line: u32, character: u32) -> Option<String> {
    let analysis = DocumentAnalysis::analyze(source, file);
    let word = word_at(source, line, character);
    if let Some(w) = &word {
        if let Some(stripped) = w.strip_suffix("_json_encode") {
            return Some(format!(
                "**synthesized** `{}`\n\nJSON encode for struct `{}` (compiler-generated when fields are `string` / `i32` / `bool` / nested structs).",
                w, stripped
            ));
        }
        if let Some(stripped) = w.strip_suffix("_json_decode") {
            return Some(format!(
                "**synthesized** `{}`\n\nJSON decode for struct `{}` (compiler-generated).",
                w, stripped
            ));
        }
    }
    let sym = analysis.symbol_at_position(source, line, character)?;
    let kind = match sym.kind {
        SymbolKind::Function => "function",
        SymbolKind::Parameter => "parameter",
        SymbolKind::Variable => "variable",
        SymbolKind::Constant => "constant",
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Field => "field",
        SymbolKind::Method => "method",
        SymbolKind::Extern => "extern fn",
        SymbolKind::Module => "module",
        SymbolKind::Keyword => "keyword",
    };
    if let Some(detail) = &sym.detail {
        Some(format!("**{kind}** `{}`\n\n```ny\n{detail}\n```", sym.name))
    } else {
        Some(format!("**{kind}** `{}`", sym.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_function_symbol() {
        let src = "fn main() {\n    let x = 1\n}\n";
        let a = DocumentAnalysis::analyze(src, "t.ny");
        assert!(a.symbols.iter().any(|s| s.name == "main"));
    }

    #[test]
    fn completions_include_let_binding() {
        let src = "fn main() {\n    let counter = 0\n    print(counter)\n}\n";
        let a = DocumentAnalysis::analyze(src, "t.ny");
        let items = a.completions("cou");
        assert!(items.iter().any(|s| s == "counter"));
    }

    #[test]
    fn inlay_hints_for_inferred_let() {
        let src = "fn main() {\n    let x = 42\n    print(x)\n}\n";
        let a = DocumentAnalysis::analyze(src, "t.ny");
        assert!(a.inlay_hints.iter().any(|h| h.label.contains("i32")));
    }

    #[test]
    fn semantic_tokens_include_keywords() {
        let src = "fn main() { let x = 1 }\n";
        let a = DocumentAnalysis::analyze(src, "t.ny");
        let tokens = crate::semantic::collect_semantic_tokens(src, &a);
        assert!(tokens.iter().any(|t| matches!(t.kind, crate::semantic::DocumentTokenKind::Keyword)));
    }
}
