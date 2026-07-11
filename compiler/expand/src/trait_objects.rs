//! Synthesize `Dyn_*` fat-pointer structs for trait object types (single- and multi-trait).

use ast::*;
use std::collections::BTreeSet;

pub fn synthesize_trait_object_structs(program: &mut Program) {
    let mut combos = collect_dyn_trait_combos(program);
    for trait_def in &program.traits {
        if trait_def.name == "Drop" || trait_def.name == "Clone" {
            continue;
        }
        combos.insert(vec![trait_def.name.clone()]);
    }

    for traits in combos {
        if traits.is_empty() {
            continue;
        }
        if traits.len() == 1 && (traits[0] == "Drop" || traits[0] == "Clone") {
            continue;
        }
        let dyn_name = dyn_struct_name(&traits);
        if program.structs.iter().any(|s| s.name == dyn_name) {
            continue;
        }
        program.structs.push(dyn_fat_pointer_struct(&dyn_name));
    }
}

fn dyn_fat_pointer_struct(name: &str) -> StructDef {
    StructDef {
        name: name.into(),
        doc: None,
        type_params: vec![],
        attrs: StructAttrs::default(),
        fields: vec![
            StructField {
                name: "data".into(),
                ty: TypeAnnotation::Ptr,
            },
            StructField {
                name: "vtable".into(),
                ty: TypeAnnotation::Ptr,
            },
        ],
        public: true,
    }
}

fn insert_combo(out: &mut BTreeSet<Vec<String>>, traits: Vec<String>) {
    let filtered: Vec<String> = traits
        .into_iter()
        .filter(|t| t != "Drop" && t != "Clone")
        .collect();
    if !filtered.is_empty() {
        out.insert(filtered);
    }
}

fn collect_dyn_trait_combos(program: &Program) -> BTreeSet<Vec<String>> {
    let mut out = BTreeSet::new();

    for c in &program.consts {
        if let Some(ty) = &c.ty {
            collect_from_type(ty, &mut out);
        }
        collect_from_expr(&c.value, &mut out);
    }
    for s in &program.structs {
        for f in &s.fields {
            collect_from_type(&f.ty, &mut out);
        }
    }
    for e in &program.externs {
        for p in &e.params {
            collect_from_type(&p.ty, &mut out);
        }
        if let Some(rt) = &e.return_type {
            collect_from_type(rt, &mut out);
        }
    }
    for t in &program.traits {
        for m in &t.methods {
            for p in &m.params {
                collect_from_type(&p.ty, &mut out);
            }
            if let Some(rt) = &m.return_type {
                collect_from_type(rt, &mut out);
            }
        }
    }
    for ti in &program.trait_impls {
        for f in &ti.methods {
            collect_from_function(f, &mut out);
        }
    }
    for imp in &program.impls {
        for f in &imp.methods {
            collect_from_function(f, &mut out);
        }
    }
    for f in &program.functions {
        collect_from_function(f, &mut out);
    }

    out
}

fn collect_from_function(f: &Function, out: &mut BTreeSet<Vec<String>>) {
    for p in &f.params {
        collect_from_type(&p.ty, out);
    }
    if let Some(rt) = &f.return_type {
        collect_from_type(rt, out);
    }
    for stmt in &f.body.statements {
        collect_from_stmt(stmt, out);
    }
}

fn collect_from_stmt(stmt: &Statement, out: &mut BTreeSet<Vec<String>>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            if let Some(ty) = &l.ty {
                collect_from_type(ty, out);
            }
            collect_from_expr(&l.value, out);
        }
        Statement::Assign(a) => {
            collect_from_expr(&a.target, out);
            collect_from_expr(&a.value, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_from_expr(v, out);
            }
        }
        Statement::If(i) => {
            collect_from_expr(&i.condition, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_from_expr(e, out));
            if let Some(el) = &i.else_block {
                for_each_expr_in_block(el, &mut |e| collect_from_expr(e, out));
            }
        }
        Statement::While(w) => {
            collect_from_expr(&w.condition, out);
            for_each_expr_in_block(&w.body, &mut |e| collect_from_expr(e, out));
        }
        Statement::For(fo) => {
            match &fo.kind {
                ForKind::Range { start, end } => {
                    collect_from_expr(start, out);
                    collect_from_expr(end, out);
                }
                ForKind::Iterable { iterable } => collect_from_expr(iterable, out),
            }
            for_each_expr_in_block(&fo.body, &mut |e| collect_from_expr(e, out));
        }
        Statement::Expression(e) | Statement::Defer(e) => collect_from_expr(e, out),
        Statement::Print(p) => {
            for a in &p.args {
                collect_from_expr(a, out);
            }
            if let Some(c) = &p.color {
                collect_from_expr(c, out);
            }
        }
        Statement::Spawn(s) => for_each_expr_in_block(&s.body, &mut |e| collect_from_expr(e, out)),
        Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for_each_expr_in_block(b, &mut |e| collect_from_expr(e, out));
        }
        Statement::Asm { .. }
        | Statement::Import(_)
        | Statement::Break { .. }
        | Statement::Continue { .. } => {}
    }
}

fn collect_from_type(ann: &TypeAnnotation, out: &mut BTreeSet<Vec<String>>) {
    match ann {
        TypeAnnotation::DynTrait { traits, .. } => insert_combo(out, traits.clone()),
        TypeAnnotation::Applied { args, .. } => {
            for a in args {
                collect_from_type(a, out);
            }
        }
        TypeAnnotation::Array { elem, .. } => collect_from_type(elem, out),
        TypeAnnotation::Tuple(elems) => {
            for e in elems {
                collect_from_type(e, out);
            }
        }
        TypeAnnotation::Ref { inner, .. } | TypeAnnotation::RawPtr { inner } => {
            collect_from_type(inner, out);
        }
        TypeAnnotation::ForAll { inner, .. } => collect_from_type(inner, out),
        TypeAnnotation::FnPtr {
            params,
            return_type,
            ..
        } => {
            for p in params {
                collect_from_type(p, out);
            }
            if let Some(rt) = return_type {
                collect_from_type(rt, out);
            }
        }
        TypeAnnotation::Simd { elem, .. } => collect_from_type(elem, out),
        _ => {}
    }
}

fn collect_from_expr(expr: &Expression, out: &mut BTreeSet<Vec<String>>) {
    match expr {
        Expression::Cast(c) => {
            collect_from_type(&c.target_type, out);
            collect_from_expr(&c.expr, out);
        }
        Expression::Call(c) => {
            for ta in &c.type_args {
                collect_from_type(ta, out);
            }
            for a in &c.args {
                collect_from_expr(a, out);
            }
        }
        Expression::MethodCall(mc) => {
            collect_from_expr(&mc.object, out);
            for a in &mc.args {
                collect_from_expr(a, out);
            }
        }
        Expression::Binary(b) => {
            collect_from_expr(&b.left, out);
            collect_from_expr(&b.right, out);
        }
        Expression::Unary(u) => collect_from_expr(&u.operand, out),
        Expression::FieldAccess(f) => collect_from_expr(&f.object, out),
        Expression::StructLiteral(s) => {
            for sp in &s.spreads {
                collect_from_expr(sp, out);
            }
            for (_, v) in &s.fields {
                collect_from_expr(v, out);
            }
        }
        Expression::Index(ix) => {
            collect_from_expr(&ix.object, out);
            collect_from_expr(&ix.index, out);
        }
        Expression::ArrayLiteral(a) => {
            for e in a.spreads.iter().chain(a.elems.iter()) {
                collect_from_expr(e, out);
            }
        }
        Expression::ArrayRepeat {
            element,
            count_expr,
            ..
        } => {
            collect_from_expr(element, out);
            if let Some(ce) = count_expr {
                collect_from_expr(ce, out);
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_from_expr(e, out);
            }
        }
        Expression::Grouped(g) => collect_from_expr(g, out),
        Expression::Await(e) => collect_from_expr(e, out),
        Expression::If(i) => {
            collect_from_expr(&i.condition, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_from_expr(e, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_from_expr(e, out));
        }
        Expression::Match(m) => {
            collect_from_expr(&m.scrutinee, out);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    collect_from_expr(g, out);
                }
                for_each_expr_in_block(&arm.body, &mut |e| collect_from_expr(e, out));
            }
        }
        Expression::ArrowFn(a) => {
            for p in &a.params {
                collect_from_type(&p.ty, out);
            }
            match &a.body {
                ArrowBody::Expr(e) => collect_from_expr(e, out),
                ArrowBody::Block(b) => {
                    for_each_expr_in_block(b, &mut |e| collect_from_expr(e, out));
                }
            }
        }
        Expression::ComptimeBlock { body, .. } => {
            for_each_expr_in_block(body, &mut |e| collect_from_expr(e, out));
        }
        Expression::Spawn { body, .. } => {
            for_each_expr_in_block(body, &mut |e| collect_from_expr(e, out));
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_from_expr(e, out);
                }
            }
        }
        Expression::ParallelSearch(ps) => {
            ps.for_each_expr(|e| collect_from_expr(e, out));
        }
        Expression::EnumVariant(ev) => {
            for a in &ev.args {
                collect_from_expr(a, out);
            }
        }
        _ => {}
    }
}
