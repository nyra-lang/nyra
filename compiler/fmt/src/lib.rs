//! AST-based Nyra source formatter.
//!
//! Parses the program and pretty-prints it. On parse failure, returns `None` so
//! callers can fall back to a line-based formatter.

mod legacy;
mod comments;

use ast::{
    format_dyn_trait, ArrowBody, BinaryOp, Block, ConstDef, EnumDef, EnumVariantDef, Expression,
    ExternFn, ForKind, ForStmt, Function, IfStmt, ImplDef, LetStmt, Literal, MatchArm,
    MatchPayloadPattern, MatchPattern, Param, ParallelConfig, ParallelMode, ParallelOp,
    ParallelThreads, ProgressConfig, Program, SpawnKind, Statement, StructDef, StructField,
    TraitDef, TraitImpl, TypeAnnotation, UnaryOp, WhileStmt,
};
use lexer::Lexer;
use parser::Parser;

const INDENT: &str = "    ";

pub fn format_source(source: &str, file: &str) -> Option<String> {
    let (tokens, lex_errs) = Lexer::new(source, file).tokenize();
    if !lex_errs.is_empty() {
        return None;
    }
    let (program, parse_errs) = Parser::new(tokens).parse();
    if !parse_errs.is_empty() {
        return None;
    }
    Some(format_program(&program))
}

/// AST format when possible; otherwise line-based fallback. Preserves anchored `//` comments.
pub fn format_source_or_fallback(source: &str, file: &str) -> String {
    let formatted = format_source(source, file)
        .unwrap_or_else(|| legacy::format_source_line_based(source));
    comments::merge_comments(source, &formatted)
}

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    if program.no_std {
        out.push_str("no_std\n\n");
    }
    if program.allow_extended {
        out.push_str("allow_extended\n\n");
    }
    if let Some(module) = &program.module {
        out.push_str(&format!("module {module}\n\n"));
    }
    for import in &program.imports {
        out.push_str(&format!("import \"{}\"\n", import.path));
    }
    if !program.imports.is_empty() {
        out.push('\n');
    }
    for c in &program.consts {
        emit_const(c, 0, &mut out);
        out.push('\n');
    }
    for s in &program.structs {
        emit_struct(s, &mut out);
        out.push('\n');
    }
    for e in &program.enums {
        emit_enum(e, &mut out);
        out.push('\n');
    }
    for t in &program.traits {
        emit_trait(t, &mut out);
        out.push('\n');
    }
    for ti in &program.trait_impls {
        emit_trait_impl(ti, &mut out);
        out.push('\n');
    }
    for imp in &program.impls {
        emit_impl(imp, &mut out);
        out.push('\n');
    }
    for ext in &program.externs {
        emit_extern(ext, &mut out);
        out.push('\n');
    }
    for f in &program.functions {
        emit_function(f, &mut out);
        out.push('\n');
    }
    if out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn emit_const(c: &ConstDef, indent: usize, out: &mut String) {
    pad(indent, out);
    out.push_str("const ");
    out.push_str(&c.name);
    if let Some(ty) = &c.ty {
        out.push_str(": ");
        out.push_str(&format_type(ty));
    }
    out.push_str(" = ");
    emit_expr(&c.value, out);
}

fn emit_struct(s: &StructDef, out: &mut String) {
    out.push_str("struct ");
    out.push_str(&s.name);
    if !s.type_params.is_empty() {
        out.push('<');
        out.push_str(&s.type_params.join(", "));
        out.push('>');
    }
    if s.attrs.copy {
        out.push_str(" Copy");
    }
    out.push_str(" {\n");
    for field in &s.fields {
        pad(1, out);
        emit_struct_field(field, out);
        out.push('\n');
    }
    out.push('}');
}

fn emit_struct_field(f: &StructField, out: &mut String) {
    out.push_str(&f.name);
    out.push_str(": ");
    out.push_str(&format_type(&f.ty));
}

fn emit_enum(e: &EnumDef, out: &mut String) {
    out.push_str("enum ");
    out.push_str(&e.name);
    if !e.type_params.is_empty() {
        out.push('<');
        out.push_str(&e.type_params.join(", "));
        out.push('>');
    }
    out.push_str(" {\n");
    for (i, v) in e.variants.iter().enumerate() {
        pad(1, out);
        emit_enum_variant(v, out);
        if i + 1 < e.variants.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push('}');
}

fn emit_enum_variant(v: &EnumVariantDef, out: &mut String) {
    out.push_str(&v.name);
    if !v.fields.is_empty() {
        out.push('(');
        for (i, ty) in v.fields.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format_type(ty));
        }
        out.push(')');
    }
}

fn emit_trait(t: &TraitDef, out: &mut String) {
    out.push_str("trait ");
    out.push_str(&t.name);
    out.push_str(" {\n");
    for m in &t.methods {
        pad(1, out);
        out.push_str("fn ");
        out.push_str(&m.name);
        emit_params(&m.params, out);
        if let Some(ret) = &m.return_type {
            out.push_str(" -> ");
            out.push_str(&format_type(ret));
        }
        out.push('\n');
    }
    out.push('}');
}

fn emit_trait_impl(ti: &TraitImpl, out: &mut String) {
    out.push_str("impl ");
    out.push_str(&ti.trait_name);
    out.push_str(" for ");
    out.push_str(&ti.type_name);
    out.push_str(" {\n");
    for m in &ti.methods {
        pad(1, out);
        emit_function_body(m, 1, out);
        out.push('\n');
    }
    out.push('}');
}

fn emit_impl(imp: &ImplDef, out: &mut String) {
    out.push_str("impl ");
    out.push_str(&imp.type_name);
    out.push_str(" {\n");
    for m in &imp.methods {
        pad(1, out);
        emit_function_body(m, 1, out);
        out.push('\n');
    }
    out.push('}');
}

fn emit_extern(ext: &ExternFn, out: &mut String) {
    out.push_str("extern fn ");
    out.push_str(&ext.name);
    emit_params(&ext.params, out);
    if let Some(ret) = &ext.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type(ret));
    }
}

fn emit_function(f: &Function, out: &mut String) {
    if f.exported {
        out.push_str("export ");
    }
    if f.is_async {
        out.push_str("async ");
    }
    if f.is_test {
        out.push_str("test ");
    }
    out.push_str("fn ");
    out.push_str(&f.name);
    if !f.type_params.is_empty() {
        out.push('<');
        let parts: Vec<String> = f
            .type_params
            .iter()
            .map(|p| {
                if let Some(bs) = f.type_param_bounds.get(p) {
                    if bs.is_empty() {
                        p.clone()
                    } else {
                        format!("{}: {}", p, bs.join(" + "))
                    }
                } else {
                    p.clone()
                }
            })
            .collect();
        out.push_str(&parts.join(", "));
        out.push('>');
    }
    emit_params(&f.params, out);
    if let Some(ret) = &f.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type(ret));
    }
    out.push(' ');
    emit_block(&f.body, 0, out);
}

fn emit_function_body(f: &Function, indent: usize, out: &mut String) {
    if f.is_async {
        out.push_str("async ");
    }
    out.push_str("fn ");
    out.push_str(&f.name);
    emit_params(&f.params, out);
    if let Some(ret) = &f.return_type {
        out.push_str(" -> ");
        out.push_str(&format_type(ret));
    }
    out.push(' ');
    emit_block(&f.body, indent, out);
}

fn emit_params(params: &[Param], out: &mut String) {
    out.push('(');
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        if !p.destructure.is_empty() {
            out.push('(');
            for (j, name) in p.destructure.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
            }
            out.push(')');
        } else {
            if p.mutable {
                out.push_str("mut ");
            }
            out.push_str(&p.name);
        }
        out.push_str(": ");
        out.push_str(&format_type(&p.ty));
    }
    out.push(')');
}

fn emit_block(block: &Block, indent: usize, out: &mut String) {
    out.push('{');
    if block.statements.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    for stmt in &block.statements {
        emit_stmt(stmt, indent + 1, out);
        out.push('\n');
    }
    pad(indent, out);
    out.push('}');
}

fn emit_stmt(stmt: &Statement, indent: usize, out: &mut String) {
    match stmt {
        Statement::Let(ls) => emit_let(ls, "let", indent, out),
        Statement::Const(ls) => emit_let(ls, "const", indent, out),
        Statement::Assign(a) => {
            pad(indent, out);
            emit_expr(&a.target, out);
            out.push_str(" = ");
            emit_expr(&a.value, out);
        }
        Statement::Return(r) => {
            pad(indent, out);
            out.push_str("return");
            if let Some(v) = &r.value {
                out.push(' ');
                emit_expr(v, out);
            }
        }
        Statement::If(i) => emit_if(i, indent, out),
        Statement::While(w) => emit_while(w, indent, out),
        Statement::For(f) => emit_for(f, indent, out),
        Statement::Break { .. } => {
            pad(indent, out);
            out.push_str("break");
        }
        Statement::Continue { .. } => {
            pad(indent, out);
            out.push_str("continue");
        }
        Statement::Expression(e) => {
            pad(indent, out);
            emit_expr(e, out);
        }
        Statement::Print(p) => {
            pad(indent, out);
            out.push_str("print(");
            for (i, arg) in p.args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(arg, out);
            }
            if let Some(color) = &p.color {
                if !p.args.is_empty() {
                    out.push_str(", ");
                }
                out.push_str("color: ");
                emit_expr(color, out);
            }
            out.push(')');
        }
        Statement::Defer(e) => {
            pad(indent, out);
            out.push_str("defer ");
            emit_expr(e, out);
        }
        Statement::Benchmark(b) => {
            pad(indent, out);
            out.push_str("benchmark ");
            emit_block(b, indent, out);
        }
        Statement::Spawn(s) => {
            pad(indent, out);
            match s.kind {
                SpawnKind::Task => out.push_str("spawn "),
                SpawnKind::Thread => out.push_str("spawn:thread "),
            }
            emit_block(&s.body, indent, out);
        }
        Statement::Unsafe(b) => {
            pad(indent, out);
            out.push_str("unsafe ");
            emit_block(b, indent, out);
        }
        Statement::Asm { template, .. } => {
            pad(indent, out);
            out.push_str("asm \"");
            out.push_str(template);
            out.push('"');
        }
        Statement::Import(path) => {
            pad(indent, out);
            out.push_str("import \"");
            out.push_str(path);
            out.push('"');
        }
    }
}

fn emit_let(ls: &LetStmt, kw: &str, indent: usize, out: &mut String) {
    pad(indent, out);
    out.push_str(kw);
    if ls.mutable {
        out.push_str(" mut");
    }
    out.push(' ');
    if !ls.destructure.is_empty() {
        out.push('(');
        out.push_str(&ls.destructure.join(", "));
        out.push(')');
    } else {
        out.push_str(&ls.name);
    }
    if let Some(ty) = &ls.ty {
        out.push_str(": ");
        out.push_str(&format_type(ty));
    }
    out.push_str(" = ");
    emit_expr(&ls.value, out);
}

fn emit_if(i: &IfStmt, indent: usize, out: &mut String) {
    pad(indent, out);
    out.push_str("if ");
    emit_expr(&i.condition, out);
    out.push(' ');
    emit_block(&i.then_block, indent, out);
    if let Some(else_b) = &i.else_block {
        if else_b.statements.len() == 1 {
            if let Statement::If(nested) = &else_b.statements[0] {
                out.push_str(" else ");
                emit_if(nested, indent, out);
                return;
            }
        }
        out.push_str(" else ");
        emit_block(else_b, indent, out);
    }
}

fn emit_while(w: &WhileStmt, indent: usize, out: &mut String) {
    pad(indent, out);
    out.push_str("while ");
    emit_expr(&w.condition, out);
    out.push(' ');
    emit_block(&w.body, indent, out);
}

fn emit_for(f: &ForStmt, indent: usize, out: &mut String) {
    pad(indent, out);
    if let Some(cfg) = &f.parallel {
        out.push_str("parallel");
        if cfg.kind == SpawnKind::Thread {
            out.push_str(":thread");
        }
        emit_parallel_config(cfg, out);
        emit_parallel_op(cfg.op, out);
        out.push(' ');
    }
    if f.progress.is_some() {
        if let Some(cfg) = &f.progress {
            out.push_str("progress");
            emit_progress_config(cfg, out);
            out.push(' ');
        }
    }
    out.push_str("for ");
    out.push_str(&f.var);
    out.push_str(" in ");
    match &f.kind {
        ForKind::Range { start, end } => {
            emit_expr(start, out);
            out.push_str("..");
            emit_expr(end, out);
        }
        ForKind::Iterable { iterable } => emit_expr(iterable, out),
    }
    out.push(' ');
    emit_block(&f.body, indent, out);
}

fn emit_parallel_config(cfg: &ParallelConfig, out: &mut String) {
    let mut opts: Vec<String> = Vec::new();
    match cfg.mode {
        ParallelMode::Auto => {}
        ParallelMode::Balanced => opts.push("mode = balanced".into()),
        ParallelMode::MaxPerformance => opts.push("mode = max_performance".into()),
        ParallelMode::Background => opts.push("mode = background".into()),
    }
    match &cfg.threads {
        ParallelThreads::Auto => {}
        ParallelThreads::Max(e) => {
            let mut s = String::from("max = ");
            emit_expr(e, &mut s);
            opts.push(s);
        }
        ParallelThreads::Exact(e) => {
            let mut s = String::from("threads = ");
            emit_expr(e, &mut s);
            opts.push(s);
        }
        ParallelThreads::CpuPercent(e) => {
            let mut s = String::from("cpu = ");
            emit_expr(e, &mut s);
            s.push('%');
            opts.push(s);
        }
    }
    if opts.is_empty() {
        return;
    }
    out.push('(');
    for (i, opt) in opts.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(opt);
    }
    out.push(')');
}

fn emit_parallel_op(op: ParallelOp, out: &mut String) {
    match op {
        ParallelOp::Iterate => {}
        ParallelOp::Any => {
            out.push(' ');
            out.push_str("any");
        }
        ParallelOp::Find => {
            out.push(' ');
            out.push_str("find");
        }
        ParallelOp::All => {
            out.push(' ');
            out.push_str("all");
        }
    }
}

fn emit_progress_config(cfg: &ProgressConfig, out: &mut String) {
    let Some(label) = &cfg.label else {
        return;
    };
    out.push('(');
    out.push_str("label = ");
    emit_expr(label, out);
    out.push(')');
}

fn emit_expr(expr: &Expression, out: &mut String) {
    match expr {
        Expression::Literal(l) => emit_literal(l, out),
        Expression::Variable { name, .. } => out.push_str(name),
        Expression::Binary(b) => {
            emit_expr(&b.left, out);
            out.push(' ');
            out.push_str(binary_op(&b.op));
            out.push(' ');
            emit_expr(&b.right, out);
        }
        Expression::Unary(u) => {
            match u.op {
                UnaryOp::Neg => out.push('-'),
                UnaryOp::Not => out.push('!'),
                UnaryOp::Ref => out.push('&'),
                UnaryOp::RefMut => out.push_str("&mut "),
                UnaryOp::Deref => out.push('*'),
                UnaryOp::Move => out.push_str("move "),
                UnaryOp::Clone => out.push_str("clone "),
                UnaryOp::Try => {}
            }
            emit_expr(&u.operand, out);
            if u.op == UnaryOp::Try {
                out.push('?');
            }
        }
        Expression::Call(c) => {
            out.push_str(&c.callee);
            if !c.type_args.is_empty() {
                out.push('<');
                for (i, ty) in c.type_args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format_type(ty));
                }
                out.push('>');
            }
            out.push('(');
            for (i, arg) in c.args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(arg, out);
            }
            out.push(')');
        }
        Expression::MethodCall(m) => {
            emit_expr(&m.object, out);
            if m.optional {
                out.push_str("?.");
            } else {
                out.push('.');
            }
            out.push_str(&m.method);
            out.push('(');
            for (i, arg) in m.args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(arg, out);
            }
            out.push(')');
        }
        Expression::FieldAccess(f) => {
            emit_expr(&f.object, out);
            if f.optional {
                out.push_str("?.");
            } else {
                out.push('.');
            }
            out.push_str(&f.field);
        }
        Expression::StructLiteral(s) => {
            out.push_str(&s.name);
            out.push_str(" { ");
            for (i, spread) in s.spreads.iter().enumerate() {
                out.push_str("..");
                emit_expr(spread, out);
                if i + 1 < s.spreads.len() || !s.fields.is_empty() {
                    out.push_str(", ");
                }
            }
            for (i, (name, val)) in s.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
                out.push_str(": ");
                emit_expr(val, out);
            }
            out.push_str(" }");
        }
        Expression::EnumVariant(v) => {
            if let Some(en) = &v.enum_name {
                out.push_str(en);
                out.push('.');
            }
            out.push_str(&v.variant);
            if !v.args.is_empty() {
                out.push('(');
                for (i, arg) in v.args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    emit_expr(arg, out);
                }
                out.push(')');
            }
        }
        Expression::Match(m) => {
            out.push_str("match ");
            emit_expr(&m.scrutinee, out);
            out.push_str(" {\n");
            for arm in &m.arms {
                out.push_str(INDENT);
                emit_match_arm(arm, out);
                out.push('\n');
            }
            out.push('}');
        }
        Expression::If(i) => {
            out.push_str("if ");
            emit_expr(&i.condition, out);
            out.push(' ');
            emit_block(&i.then_block, 0, out);
            out.push_str(" else ");
            emit_block(&i.else_block, 0, out);
        }
        Expression::Index(ix) => {
            emit_expr(&ix.object, out);
            out.push('[');
            emit_expr(&ix.index, out);
            out.push(']');
        }
        Expression::ArrayLiteral(al) => {
            out.push('[');
            let mut first = true;
            for spread in &al.spreads {
                if !first {
                    out.push_str(", ");
                }
                out.push_str("...");
                emit_expr(spread, out);
                first = false;
            }
            for e in &al.elems {
                if !first {
                    out.push_str(", ");
                }
                emit_expr(e, out);
                first = false;
            }
            out.push(']');
        }
        Expression::ArrayRepeat { element, count, .. } => {
            out.push('[');
            emit_expr(element, out);
            out.push_str("; ");
            out.push_str(&count.to_string());
            out.push(']');
        }
        Expression::TupleLiteral(elems) => {
            out.push('(');
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(e, out);
            }
            out.push(')');
        }
        Expression::Grouped(inner) => {
            out.push('(');
            emit_expr(inner, out);
            out.push(')');
        }
        Expression::Await(inner) => {
            out.push_str("await ");
            emit_expr(inner, out);
        }
        Expression::TemplateLiteral(t) => {
            out.push('`');
            for part in &t.parts {
                match part {
                    ast::TemplatePart::Static(s) => out.push_str(s),
                    ast::TemplatePart::Interpolation(e) => {
                        out.push_str("${");
                        emit_expr(e, out);
                        out.push('}');
                    }
                }
            }
            out.push('`');
        }
        Expression::Cast(c) => {
            emit_expr(&c.expr, out);
            out.push_str(" as ");
            out.push_str(&format_type(&c.target_type));
        }
        Expression::ArrowFn(a) => {
            out.push('(');
            for (i, p) in a.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                out.push_str(": ");
                out.push_str(&format_type(&p.ty));
            }
            out.push_str(") => ");
            match &a.body {
                ArrowBody::Expr(e) => emit_expr(e, out),
                ArrowBody::Block(b) => emit_block(b, 0, out),
            }
        }
        Expression::ComptimeBlock { body, .. } => {
            out.push_str("comptime ");
            emit_block(body, 0, out);
        }
        Expression::Spawn { kind, body, .. } => {
            match kind {
                SpawnKind::Task => out.push_str("spawn "),
                SpawnKind::Thread => out.push_str("spawn:thread "),
            }
            emit_block(body, 0, out);
        }
        Expression::ParallelSearch(ps) => {
            out.push_str("parallel");
            if ps.config.kind == SpawnKind::Thread {
                out.push_str(":thread");
            }
            emit_parallel_config(&ps.config, out);
            emit_parallel_op(ps.config.op, out);
            out.push_str("for ");
            out.push_str(&ps.var);
            out.push_str(" in ");
            match &ps.kind {
                ForKind::Range { start, end } => {
                    emit_expr(start, out);
                    out.push_str("..");
                    emit_expr(end, out);
                }
                ForKind::Iterable { iterable } => emit_expr(iterable, out),
            }
            out.push(' ');
            emit_block(&ps.body, 0, out);
        }
        Expression::Invalid => out.push_str("/* invalid */"),
    }
}

fn emit_match_arm(arm: &MatchArm, out: &mut String) {
    emit_match_pattern(&arm.pattern, out);
    if let Some(guard) = &arm.guard {
        out.push_str(" if ");
        emit_expr(guard, out);
    }
    out.push_str(" => ");
    emit_block(&arm.body, 0, out);
}

fn emit_match_pattern(p: &MatchPattern, out: &mut String) {
    match p {
        MatchPattern::Wildcard => out.push('_'),
        MatchPattern::Literal(lit) => {
            out.push('"');
            out.push_str(lit);
            out.push('"');
        }
        MatchPattern::Variant(v) => out.push_str(v),
        MatchPattern::Qualified(en, v) => {
            out.push_str(en);
            out.push('.');
            out.push_str(v);
        }
        MatchPattern::QualifiedBind(en, v, payload) => {
            out.push_str(en);
            out.push('.');
            out.push_str(v);
            out.push('(');
            emit_match_payload(payload, out);
            out.push(')');
        }
        MatchPattern::Or(ps) => {
            for (i, p) in ps.iter().enumerate() {
                if i > 0 {
                    out.push_str(" | ");
                }
                emit_match_pattern(p, out);
            }
        }
        MatchPattern::Struct(name, fields) => {
            out.push_str(name);
            out.push_str(" { ");
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&f.field);
                if let Some(bind) = &f.bind {
                    if bind != "_" && bind != &f.field {
                        out.push_str(": ");
                        out.push_str(bind);
                    }
                }
            }
            out.push_str(" }");
        }
        MatchPattern::Tuple(elems) => {
            out.push('(');
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_match_payload(e, out);
            }
            out.push(')');
        }
    }
}

fn emit_match_payload(p: &MatchPayloadPattern, out: &mut String) {
    match p {
        MatchPayloadPattern::Bind(name) => out.push_str(name),
        MatchPayloadPattern::Wildcard => out.push('_'),
        MatchPayloadPattern::Nested(pat) => emit_match_pattern(pat, out),
    }
}

fn emit_literal(l: &Literal, out: &mut String) {
    match l {
        Literal::Int(n) => out.push_str(&n.to_string()),
        Literal::IntKind(n, k) => {
            out.push_str(&n.to_string());
            out.push_str(k.name());
        }
        Literal::Float(f, _) => out.push_str(&f.to_string()),
        Literal::Char(c) => {
            out.push('\'');
            if *c == b'\n' as u32 {
                out.push_str("\\n");
            } else {
                out.push(char::from_u32(*c).unwrap_or('?'));
            }
            out.push('\'');
        }
        Literal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Literal::String(s) => {
            out.push('"');
            for ch in s.chars() {
                match ch {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
    }
}

pub fn format_type(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Integer(k) => k.name().into(),
        TypeAnnotation::F32 => "f32".into(),
        TypeAnnotation::F64 => "f64".into(),
        TypeAnnotation::Char => "char".into(),
        TypeAnnotation::Bool => "bool".into(),
        TypeAnnotation::String => "string".into(),
        TypeAnnotation::Bytes => "bytes".into(),
        TypeAnnotation::VecStr => "VecStr".into(),
        TypeAnnotation::Ptr => "ptr".into(),
        TypeAnnotation::RawPtr { inner } => format!("*{}", format_type(inner)),
        TypeAnnotation::Void => "void".into(),
        TypeAnnotation::Struct(n) => n.clone(),
        TypeAnnotation::Applied { base, args } => {
            let inner: Vec<String> = args.iter().map(format_type).collect();
            format!("{}<{}>", base, inner.join(", "))
        }
        TypeAnnotation::Enum(n) => n.clone(),
        TypeAnnotation::Array { elem, len } => match len {
            Some(n) => format!("[{}; {}]", format_type(elem), n),
            None => format!("[{}]", format_type(elem)),
        },
        TypeAnnotation::Tuple(elems) => {
            let inner: Vec<String> = elems.iter().map(format_type).collect();
            format!("({})", inner.join(", "))
        }
        TypeAnnotation::Ref {
            inner,
            mutable,
            lifetime,
        } => {
            let lt = lifetime
                .as_ref()
                .map(|l| format!("{l} "))
                .unwrap_or_default();
            if *mutable {
                format!("&{lt}mut {}", format_type(inner))
            } else {
                format!("&{lt}{}", format_type(inner))
            }
        }
        TypeAnnotation::Generic(n) => n.clone(),
        TypeAnnotation::Lifetime(l) => l.clone(),
        TypeAnnotation::ForAll { lifetimes, inner } => {
            format!(
                "for<{}> {}",
                lifetimes.join(", "),
                format_type(inner)
            )
        }
        TypeAnnotation::FnPtr {
            lifetime_params,
            params,
            return_type,
        } => {
            let lts = if lifetime_params.is_empty() {
                String::new()
            } else {
                format!("for<{}> ", lifetime_params.join(", "))
            };
            let ps: Vec<String> = params.iter().map(format_type).collect();
            let ret = return_type
                .as_ref()
                .map(|t| format!(" -> {}", format_type(t)))
                .unwrap_or_default();
            format!("{lts}fn({}){ret}", ps.join(", "))
        }
        TypeAnnotation::Simd { elem, lanes } => {
            let base = format_type(elem);
            format!("{base}x{lanes}")
        }
        TypeAnnotation::DynTrait {
            traits,
            auto_bounds,
        } => format_dyn_trait(&traits, &auto_bounds),
    }
}

fn binary_op(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::NullishCoalesce => "??",
    }
}

fn pad(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str(INDENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_simple_fn() {
        let src = "fn main(){let x=1 print(x)}";
        let out = format_source(src, "test.ny").unwrap();
        assert!(out.contains("fn main()"));
        assert!(out.contains("let x = 1"));
        assert!(out.contains("print(x)"));
    }

    #[test]
    fn parse_error_returns_none() {
        assert!(format_source("fn {{{", "bad.ny").is_none());
    }
}
