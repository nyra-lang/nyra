//! Identifier use/export collection for lazy stdlib prelude resolution.

use std::collections::HashSet;

use ast::{for_each_expr_in_block, BinaryOp, Expression, ImplDef, Program, Statement, TypeAnnotation, UnaryOp};

/// Top-level names exported by a compilation unit (for prelude index + lint).
pub fn top_level_export_names(program: &Program) -> HashSet<String> {
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
    for imp in &program.impls {
        names.insert(imp.type_name.clone());
    }
    names
}

/// Collect identifier and type names referenced by a program (expressions, types, impls).
pub fn collect_program_uses(program: &Program) -> HashSet<String> {
    let mut uses = HashSet::new();
    for c in &program.consts {
        if let Some(ty) = &c.ty {
            collect_type_uses(ty, &mut uses);
        }
        collect_expr_uses(&c.value, &mut uses);
    }
    for s in &program.structs {
        for field in &s.fields {
            collect_type_uses(&field.ty, &mut uses);
        }
    }
    for e in &program.enums {
        for v in &e.variants {
            for field in &v.fields {
                collect_type_uses(field, &mut uses);
            }
        }
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
    for imp in &program.impls {
        collect_impl_uses(imp, &mut uses);
    }
    for ti in &program.trait_impls {
        uses.insert(ti.type_name.clone());
        uses.insert(ti.trait_name.clone());
        for m in &ti.methods {
            for p in &m.params {
                collect_type_uses(&p.ty, &mut uses);
            }
            if let Some(ty) = &m.return_type {
                collect_type_uses(ty, &mut uses);
            }
            collect_block_uses(&m.body, &mut uses);
        }
    }
    uses
}

fn collect_impl_uses(imp: &ImplDef, uses: &mut HashSet<String>) {
    uses.insert(imp.type_name.clone());
    for m in &imp.methods {
        for p in &m.params {
            collect_type_uses(&p.ty, uses);
        }
        if let Some(ty) = &m.return_type {
            collect_type_uses(ty, uses);
        }
        collect_block_uses(&m.body, uses);
    }
}

fn collect_type_uses(ty: &TypeAnnotation, uses: &mut HashSet<String>) {
    match ty {
        TypeAnnotation::Struct(name)
        | TypeAnnotation::Enum(name)
        | TypeAnnotation::Generic(name) => {
            if name != "_" {
                uses.insert(name.clone());
            }
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
        TypeAnnotation::Lifetime(_) | TypeAnnotation::Void => {}
        TypeAnnotation::Integer(_)
        | TypeAnnotation::F32
        | TypeAnnotation::F64
        | TypeAnnotation::Char
        | TypeAnnotation::Bool
        | TypeAnnotation::String
        | TypeAnnotation::Bytes
        | TypeAnnotation::VecStr
        | TypeAnnotation::Ptr
        | TypeAnnotation::DynTrait { .. } => {}
        TypeAnnotation::Simd { elem, .. } => collect_type_uses(elem, uses),
    }
}

fn collect_block_uses(block: &ast::Block, uses: &mut HashSet<String>) {
    for stmt in &block.statements {
        collect_stmt_uses(stmt, uses);
    }
}

fn collect_stmt_uses(stmt: &Statement, uses: &mut HashSet<String>) {
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

fn collect_expr_uses(expr: &Expression, uses: &mut HashSet<String>) {
    match expr {
        Expression::Variable { name, .. } => {
            uses.insert(name.clone());
        }
        Expression::Binary(b) => {
            collect_expr_uses(&b.left, uses);
            collect_expr_uses(&b.right, uses);
            if b.op == BinaryOp::NullishCoalesce {
                uses.insert("Option".into());
            }
        }
        Expression::Unary(u) => {
            if u.op == UnaryOp::Try {
                uses.insert("Result".into());
                uses.insert("Option".into());
            }
            collect_expr_uses(&u.operand, uses);
        }
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
            if matches!(m.method.as_str(), "get" | "push") {
                uses.insert("StrVec".into());
            }
        }
        Expression::FieldAccess(f) => collect_expr_uses(&f.object, uses),
        Expression::StructLiteral(s) => {
            uses.insert(s.name.clone());
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

fn collect_match_pattern_uses(pattern: &ast::MatchPattern, uses: &mut HashSet<String>) {
    match pattern {
        ast::MatchPattern::Variant(v) => {
            uses.insert(v.clone());
        }
        ast::MatchPattern::Qualified(en, v) | ast::MatchPattern::QualifiedBind(en, v, _) => {
            uses.insert(en.clone());
            uses.insert(v.clone());
        }
        ast::MatchPattern::Wildcard | ast::MatchPattern::Literal(_) => {}
        ast::MatchPattern::Or(ps) => {
            for p in ps {
                collect_match_pattern_uses(p, uses);
            }
        }
        ast::MatchPattern::Struct(_, _) | ast::MatchPattern::Tuple(_) => {}
    }
}
