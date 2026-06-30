use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ast::{ImportDecl, Program};
use errors::{ErrorKind, NyraError, W002_UNUSED_IMPORT};

pub fn check_unused_imports(entry: &Path, merged: Option<&Program>) -> Vec<NyraError> {
    let entry = match entry.canonicalize() {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    let merged_uses = merged.map(collect_identifier_uses);
    let mut visited = HashSet::new();
    let mut warnings = Vec::new();
    lint_file_imports(&entry, &entry, merged_uses.as_ref(), &mut visited, &mut warnings);
    warnings
}

fn lint_file_imports(
    entry: &Path,
    path: &Path,
    merged_uses: Option<&HashSet<String>>,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<NyraError>,
) {
    let path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return,
    };
    if !visited.insert(path.clone()) {
        return;
    }

    let program = match resolve::parse_file_only(&path) {
        Ok(p) => p,
        Err(_) => return,
    };

    let base_dir = path.parent().unwrap_or(Path::new("."));
    let file_uses = collect_identifier_uses(&program);
    let uses = if path == entry {
        merged_uses.unwrap_or(&file_uses)
    } else {
        &file_uses
    };

    let barrel = is_barrel_module(&program);
    for imp in &program.imports {
        lint_import(
            entry,
            base_dir,
            imp,
            uses,
            merged_uses,
            visited,
            out,
            barrel,
        );
    }
}

fn lint_import(
    entry: &Path,
    base_dir: &Path,
    imp: &ImportDecl,
    uses: &HashSet<String>,
    merged_uses: Option<&HashSet<String>>,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<NyraError>,
    skip_unused: bool,
) {
    match resolve::resolve_import_path(base_dir, &imp.path) {
        Ok(resolved) => {
            if let Ok(imported) = resolve::parse_file_only(&resolved) {
                if !skip_unused {
                    let exported = top_level_names(&imported);
                    let used = exported.iter().any(|name| uses.contains(name));
                    if !used && !exported.is_empty() {
                        out.push(unused_import_diag(&imp.path, &imported, &imp.span));
                    }
                }
                lint_file_imports(entry, &resolved, merged_uses, visited, out);
            }
        }
        Err(_) => {}
    }
}

fn is_barrel_module(program: &Program) -> bool {
    !program.imports.is_empty()
        && program.functions.is_empty()
        && program.consts.is_empty()
        && program.structs.is_empty()
        && program.enums.is_empty()
        && program.traits.is_empty()
        && program.macros.is_empty()
        && program.externs.is_empty()
        && program.impls.is_empty()
        && program.trait_impls.is_empty()
        && program.export_instances.is_empty()
}

fn unused_import_diag(import_path: &str, imported: &Program, span: &errors::Span) -> NyraError {
    let sample_fn = imported
        .functions
        .iter()
        .find(|f| !f.is_test)
        .map(|f| f.name.as_str());

    let mut err = NyraError::coded_warning(
        W002_UNUSED_IMPORT,
        ErrorKind::Lint,
        span.clone(),
        format!("unused import `{import_path}`"),
    )
    .label("imported module is never used")
    .note(format!(
        "none of the symbols from `{import_path}` are referenced in this compilation unit"
    ))
    .help(format!("remove this line: `import \"{import_path}\"`"));

    if import_path.starts_with("stdlib/") || import_path.starts_with("std/") {
        err = err.help("stdlib imports expose functions like `Vec::new` — call one if you need it");
    } else if let Some(name) = sample_fn {
        err = err.help(format!(
            "or call a function from the module, e.g. `{name}()`"
        ));
    }
    err
}

fn top_level_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::new();
    for f in &program.functions {
        names.insert(f.name.clone());
    }
    for c in &program.consts {
        names.insert(c.name.clone());
    }
    for s in &program.structs {
        names.insert(s.name.clone());
    }
    for e in &program.enums {
        names.insert(e.name.clone());
    }
    for t in &program.traits {
        names.insert(t.name.clone());
    }
    for m in &program.macros {
        names.insert(m.name.clone());
    }
    for e in &program.externs {
        names.insert(e.name.clone());
    }
    names
}

fn collect_identifier_uses(program: &Program) -> HashSet<String> {
    let mut uses = HashSet::new();
    for c in &program.consts {
        if let Some(ty) = &c.ty {
            collect_type_uses(ty, &mut uses);
        }
        collect_expr_uses(&c.value, &mut uses);
    }
    for f in &program.functions {
        for p in &f.params {
            collect_type_uses(&p.ty, &mut uses);
        }
        if let Some(ty) = &f.return_type {
            collect_type_uses(ty, &mut uses);
        }
        collect_block_uses(&f.body, &mut uses);
    }
    uses
}

fn collect_type_uses(ty: &ast::TypeAnnotation, uses: &mut HashSet<String>) {
    use ast::TypeAnnotation;
    match ty {
        TypeAnnotation::Struct(name)
        | TypeAnnotation::Enum(name)
        | TypeAnnotation::Generic(name) => {
            uses.insert(name.clone());
        }
        TypeAnnotation::Applied { base, args } => {
            uses.insert(base.clone());
            for arg in args {
                collect_type_uses(arg, uses);
            }
        }
        TypeAnnotation::RawPtr { inner }
        | TypeAnnotation::Ref { inner, .. }
        | TypeAnnotation::ForAll { inner, .. } => {
            collect_type_uses(inner, uses);
        }
        TypeAnnotation::Array { elem, .. } => collect_type_uses(elem, uses),
        TypeAnnotation::Tuple(items) => {
            for item in items {
                collect_type_uses(item, uses);
            }
        }
        TypeAnnotation::FnPtr {
            params,
            return_type,
            ..
        } => {
            for p in params {
                collect_type_uses(p, uses);
            }
            if let Some(r) = return_type {
                collect_type_uses(r, uses);
            }
        }
        TypeAnnotation::Lifetime(_) => {}
        _ => {}
    }
}

fn collect_block_uses(block: &ast::Block, uses: &mut HashSet<String>) {
    for stmt in &block.statements {
        collect_stmt_uses(stmt, uses);
    }
}

fn collect_stmt_uses(stmt: &ast::Statement, uses: &mut HashSet<String>) {
    use ast::Statement;
    match stmt {
        Statement::Let(ls) | Statement::Const(ls) => {
            if let Some(ty) = &ls.ty {
                collect_type_uses(ty, uses);
            }
            collect_expr_uses(&ls.value, uses);
            for name in &ls.destructure {
                uses.insert(name.clone());
            }
        }
        Statement::Assign(a) => {
            collect_expr_uses(&a.target, uses);
            collect_expr_uses(&a.value, uses);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_expr_uses(v, uses);
            }
        }
        Statement::If(i) => {
            collect_expr_uses(&i.condition, uses);
            collect_block_uses(&i.then_block, uses);
            if let Some(el) = &i.else_block {
                collect_block_uses(el, uses);
            }
        }
        Statement::While(w) => {
            collect_expr_uses(&w.condition, uses);
            collect_block_uses(&w.body, uses);
        }
        Statement::For(f) => {
            match &f.kind {
                ast::ForKind::Range { start, end } => {
                    collect_expr_uses(start, uses);
                    collect_expr_uses(end, uses);
                }
                ast::ForKind::Iterable { iterable } => collect_expr_uses(iterable, uses),
            }
            collect_block_uses(&f.body, uses);
        }
        Statement::Expression(e) => collect_expr_uses(e, uses),
        Statement::Print(p) => {
            for e in &p.args {
                collect_expr_uses(e, uses);
            }
            if let Some(c) = &p.color {
                collect_expr_uses(c, uses);
            }
        }
        Statement::Defer(e) => collect_expr_uses(e, uses),
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => collect_block_uses(b, uses),
        Statement::Asm { .. } | Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn collect_expr_uses(expr: &ast::Expression, uses: &mut HashSet<String>) {
    use ast::{for_each_expr_in_block, Expression};
    match expr {
        Expression::Variable { name, .. } => {
            uses.insert(name.clone());
        }
        Expression::Binary(b) => {
            collect_expr_uses(&b.left, uses);
            collect_expr_uses(&b.right, uses);
        }
        Expression::Unary(u) => collect_expr_uses(&u.operand, uses),
        Expression::Call(c) => {
            uses.insert(c.callee.clone());
            for ty in &c.type_args {
                collect_type_uses(ty, uses);
            }
            for a in &c.args {
                collect_expr_uses(a, uses);
            }
        }
        Expression::MethodCall(m) => {
            collect_expr_uses(&m.object, uses);
            for a in &m.args {
                collect_expr_uses(a, uses);
            }
        }
        Expression::FieldAccess(f) => collect_expr_uses(&f.object, uses),
        Expression::StructLiteral(s) => {
            uses.insert(s.name.clone());
            for spread in &s.spreads {
                collect_expr_uses(spread, uses);
            }
            for (_, v) in &s.fields {
                collect_expr_uses(v, uses);
            }
        }
        Expression::EnumVariant(v) => {
            if let Some(en) = &v.enum_name {
                uses.insert(en.clone());
            }
            uses.insert(v.variant.clone());
            for a in &v.args {
                collect_expr_uses(a, uses);
            }
        }
        Expression::Match(m) => {
            collect_expr_uses(&m.scrutinee, uses);
            for arm in &m.arms {
                collect_match_pattern_uses(&arm.pattern, uses);
                if let Some(g) = &arm.guard {
                    collect_expr_uses(g, uses);
                }
                for_each_expr_in_block(&arm.body, &mut |e| collect_expr_uses(e, uses));
            }
        }
        Expression::If(i) => {
            collect_expr_uses(&i.condition, uses);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_expr_uses(e, uses));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_expr_uses(e, uses));
        }
        Expression::Index(i) => {
            collect_expr_uses(&i.object, uses);
            collect_expr_uses(&i.index, uses);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_expr_uses(e, uses);
            }
        }
        Expression::TupleLiteral(items) => {
            for e in items {
                collect_expr_uses(e, uses);
            }
        }
        Expression::ArrayRepeat { element, .. } => collect_expr_uses(element, uses),
        Expression::Grouped(e) | Expression::Await(e) => collect_expr_uses(e, uses),
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let ast::TemplatePart::Interpolation(e) = part {
                    collect_expr_uses(e, uses);
                }
            }
        }
        Expression::Cast(c) => collect_expr_uses(&c.expr, uses),
        Expression::ArrowFn(a) => match &a.body {
            ast::ArrowBody::Expr(e) => collect_expr_uses(e, uses),
            ast::ArrowBody::Block(b) => collect_block_uses(b, uses),
        },
        Expression::ComptimeBlock { body, .. } => collect_block_uses(body, uses),
        Expression::Literal(_) | Expression::Invalid => {}
    }
}

fn collect_match_payload_uses(pattern: &ast::MatchPayloadPattern, uses: &mut HashSet<String>) {
    use ast::MatchPayloadPattern;
    match pattern {
        MatchPayloadPattern::Bind(name) => {
            uses.insert(name.clone());
        }
        MatchPayloadPattern::Wildcard => {}
        MatchPayloadPattern::Nested(p) => collect_match_pattern_uses(p, uses),
    }
}

fn collect_match_pattern_uses(pattern: &ast::MatchPattern, uses: &mut HashSet<String>) {
    use ast::MatchPattern;
    match pattern {
        MatchPattern::Variant(v) => {
            uses.insert(v.clone());
        }
        MatchPattern::Qualified(en, v) | MatchPattern::QualifiedBind(en, v, _) => {
            uses.insert(en.clone());
            uses.insert(v.clone());
        }
        MatchPattern::Wildcard | MatchPattern::Literal(_) => {}
        MatchPattern::Or(ps) => {
            for p in ps {
                collect_match_pattern_uses(p, uses);
            }
        }
        MatchPattern::Struct(_, fields) => {
            for f in fields {
                if let Some(bind) = &f.bind {
                    if bind != "_" {
                        uses.insert(bind.clone());
                    }
                } else {
                    uses.insert(f.field.clone());
                }
            }
        }
        MatchPattern::Tuple(elems) => {
            for e in elems {
                collect_match_payload_uses(e, uses);
            }
        }
    }
}
