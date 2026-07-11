use std::collections::HashMap;

use ast::*;
use errors::Span;

mod infer_calls;
mod trait_bounds;

pub use trait_bounds::validate_trait_bounds;

fn arc_dec_extern_for_payload(payload: &str) -> Option<&'static str> {
    match payload {
        "i32" => Some("arc_dec_i32"),
        "string" => Some("arc_dec_string"),
        _ => None,
    }
}

fn synthesize_arc_drop_impl(program: &mut Program, type_name: &str, dec_fn: &str) {
    let span = Span::default();
    let drop_body = Block {
        statements: vec![Statement::Expression(Expression::Call(CallExpr {
            callee: dec_fn.to_string(),
            type_args: vec![],
            args: vec![Expression::FieldAccess(Box::new(FieldAccessExpr {
                object: Expression::Variable {
                    name: "self".into(),
                    span: span.clone(),
                },
                field: "handle".into(),
                optional: false,
                span: span.clone(),
            }))],
            span: span.clone(),
        }))],
    };
    let drop_fn = Function {
        name: "drop".into(),
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "self".into(),
            ty: TypeAnnotation::Struct(type_name.to_string()),
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Void),
        body: drop_body,
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    };
    program.trait_impls.push(TraitImpl {
        type_name: type_name.to_string(),
        trait_name: "Drop".into(),
        methods: vec![Function {
            name: format!("Drop_{type_name}_drop"),
            ..drop_fn
        }],
    });
}

fn synthesize_arc_drop_impls(program: &mut Program) {
    let candidates: Vec<String> = program
        .structs
        .iter()
        .filter(|s| s.name.starts_with("Arc__"))
        .map(|s| s.name.clone())
        .collect();
    for type_name in candidates {
        if program.trait_impls.iter().any(|ti| {
            ti.trait_name == "Drop" && ti.type_name == type_name
        }) {
            continue;
        }
        let Some(payload) = type_name.strip_prefix("Arc__") else {
            continue;
        };
        let Some(dec_fn) = arc_dec_extern_for_payload(payload) else {
            continue;
        };
        synthesize_arc_drop_impl(program, &type_name, dec_fn);
    }
}

fn mangle_type(t: &TypeAnnotation) -> String {
    match t {
        TypeAnnotation::Integer(k) => k.name().into(),
        TypeAnnotation::F32 => "f32".into(),
        TypeAnnotation::F64 => "f64".into(),
        TypeAnnotation::Char => "char".into(),
        TypeAnnotation::Bool => "bool".into(),
        TypeAnnotation::String => "string".into(),
        TypeAnnotation::Bytes => "bytes".into(),
        TypeAnnotation::VecStr => "vec_str".into(),
        TypeAnnotation::Ptr => "ptr".into(),
        TypeAnnotation::RawPtr { inner } => format!("raw_{}", mangle_type(inner)),
        TypeAnnotation::Void => "void".into(),
        TypeAnnotation::Struct(n) => format!("S_{n}"),
        TypeAnnotation::Enum(n) => format!("E_{n}"),
        TypeAnnotation::Array { elem, len } => {
            format!(
                "A{}_{}",
                len.map(|n| n.to_string()).unwrap_or_else(|| "x".into()),
                mangle_type(elem)
            )
        }
        TypeAnnotation::Tuple(elems) => {
            format!(
                "T{}_{}",
                elems.len(),
                elems.iter().map(mangle_type).collect::<Vec<_>>().join("_")
            )
        }
        TypeAnnotation::Ref { inner, mutable, .. } => format!(
            "R{}{}",
            if *mutable { "mut" } else { "imm" },
            mangle_type(inner)
        ),
        TypeAnnotation::Applied { base, args } => {
            let suffix: String = args.iter().map(mangle_type).collect::<Vec<_>>().join("_");
            format!("{base}__{suffix}")
        }
        TypeAnnotation::Generic(n) => n.clone(),
        TypeAnnotation::Lifetime(_) => "lt".into(),
        TypeAnnotation::ForAll { inner, .. } => mangle_type(inner),
        TypeAnnotation::FnPtr { .. } => "fnptr".into(),
        TypeAnnotation::DynTrait { traits, .. } => format!("dyn_{}", ast::dyn_combo_key(&traits)),
        TypeAnnotation::Simd { elem, lanes } => {
            format!("simd_{}_{}", mangle_type(elem), lanes)
        }
    }
}

pub fn mangle_inst(name: &str, type_args: &[TypeAnnotation]) -> String {
    if let Some(alias) = collection_struct_alias(name, type_args) {
        return alias;
    }
    if type_args.is_empty() {
        name.to_string()
    } else {
        let suffix: String = type_args.iter().map(mangle_type).collect::<Vec<_>>().join("_");
        format!("{name}__{suffix}")
    }
}

/// Map generic collection instantiations to shipped stdlib monomorph names.
fn collection_struct_alias(base: &str, type_args: &[TypeAnnotation]) -> Option<String> {
    match (base, type_args) {
        ("Vec", [TypeAnnotation::String]) => Some("StrVec".into()),
        ("HashMap", [TypeAnnotation::String, TypeAnnotation::Integer(_)]) => {
            Some("HashMap_str_i32".into())
        }
        ("HashMap", [TypeAnnotation::String, TypeAnnotation::String]) => {
            Some("HashMap_str_str".into())
        }
        ("Future", [TypeAnnotation::Integer(_)]) => Some("Future_i32".into()),
        ("Future", [TypeAnnotation::Bool]) => Some("Future_bool".into()),
        ("Future", [TypeAnnotation::String]) => Some("Future_string".into()),
        _ => None,
    }
}

fn substitute_type_args(
    args: &[TypeAnnotation],
    map: &HashMap<String, TypeAnnotation>,
) -> Vec<TypeAnnotation> {
    args.iter().map(|a| substitute_type(a, map)).collect()
}

fn substitute_type(ty: &TypeAnnotation, map: &HashMap<String, TypeAnnotation>) -> TypeAnnotation {
    match ty {
        TypeAnnotation::Generic(n) => map
            .get(n)
            .cloned()
            .unwrap_or_else(|| TypeAnnotation::Generic(n.clone())),
        TypeAnnotation::Struct(n) if map.contains_key(n) => map.get(n).cloned().unwrap(),
        TypeAnnotation::Array { elem, len } => TypeAnnotation::Array {
            elem: Box::new(substitute_type(elem, map)),
            len: *len,
        },
        TypeAnnotation::Tuple(elems) => TypeAnnotation::Tuple(
            elems.iter().map(|e| substitute_type(e, map)).collect(),
        ),
        TypeAnnotation::RawPtr { inner } => TypeAnnotation::RawPtr {
            inner: Box::new(substitute_type(inner, map)),
        },
        TypeAnnotation::Ref { inner, mutable, lifetime } => TypeAnnotation::Ref {
            inner: Box::new(substitute_type(inner, map)),
            mutable: *mutable,
            lifetime: lifetime.clone(),
        },
        TypeAnnotation::ForAll { lifetimes, inner } => TypeAnnotation::ForAll {
            lifetimes: lifetimes.clone(),
            inner: Box::new(substitute_type(inner, map)),
        },
        TypeAnnotation::FnPtr {
            lifetime_params,
            params,
            return_type,
        } => TypeAnnotation::FnPtr {
            lifetime_params: lifetime_params.clone(),
            params: params.iter().map(|p| substitute_type(p, map)).collect(),
            return_type: return_type
                .as_ref()
                .map(|t| Box::new(substitute_type(t, map))),
        },
        TypeAnnotation::Applied { base, args } => TypeAnnotation::Applied {
            base: base.clone(),
            args: args.iter().map(|a| substitute_type(a, map)).collect(),
        },
        other => other.clone(),
    }
}

fn is_concrete_ann(ty: &TypeAnnotation) -> bool {
    !matches!(ty, TypeAnnotation::Generic(_))
}

fn collect_applied_from_type(ty: &TypeAnnotation, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    if let TypeAnnotation::Applied { base, args } = ty {
        if args.iter().all(is_concrete_ann) {
            out.push((base.clone(), args.clone()));
        }
        for a in args {
            collect_applied_from_type(a, out);
        }
        return;
    }
    match ty {
        TypeAnnotation::Array { elem, .. } => collect_applied_from_type(elem, out),
        TypeAnnotation::Tuple(elems) => {
            for e in elems {
                collect_applied_from_type(e, out);
            }
        }
        TypeAnnotation::RawPtr { inner } => collect_applied_from_type(inner, out),
        TypeAnnotation::Ref { inner, .. } => collect_applied_from_type(inner, out),
        TypeAnnotation::ForAll { inner, .. } => collect_applied_from_type(inner, out),
        TypeAnnotation::FnPtr {
            params,
            return_type,
            ..
        } => {
            for p in params {
                collect_applied_from_type(p, out);
            }
            if let Some(r) = return_type {
                collect_applied_from_type(r, out);
            }
        }
        _ => {}
    }
}

#[allow(clippy::only_used_in_recursion)]
fn collect_applied_from_expr(expr: &Expression, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    match expr {
        Expression::Literal(_) | Expression::Variable { .. } => {}
        Expression::Call(c) => {
            for a in &c.args {
                collect_applied_from_expr(a, out);
            }
        }
        Expression::Binary(b) => {
            collect_applied_from_expr(&b.left, out);
            collect_applied_from_expr(&b.right, out);
        }
        Expression::Unary(u) => collect_applied_from_expr(&u.operand, out),
        Expression::Grouped(g) => collect_applied_from_expr(g, out),
        Expression::If(i) => {
            collect_applied_from_expr(&i.condition, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_applied_from_expr(e, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_applied_from_expr(e, out));
        }
        Expression::Match(m) => {
            collect_applied_from_expr(&m.scrutinee, out);
            for a in &m.arms {
                if let Some(g) = &a.guard {
                    collect_applied_from_expr(g, out);
                }
                for_each_expr_in_block(&a.body, &mut |e| collect_applied_from_expr(e, out));
            }
        }
        Expression::Await(e) => collect_applied_from_expr(e, out),
        Expression::MethodCall(mc) => {
            collect_applied_from_expr(&mc.object, out);
            for a in &mc.args {
                collect_applied_from_expr(a, out);
            }
        }
        Expression::FieldAccess(f) => collect_applied_from_expr(&f.object, out),
        Expression::Index(ix) => {
            collect_applied_from_expr(&ix.object, out);
            collect_applied_from_expr(&ix.index, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_applied_from_expr(e, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => collect_applied_from_expr(element, out),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_applied_from_expr(e, out);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_applied_from_expr(spread, out);
            }
            for (_, e) in &s.fields {
                collect_applied_from_expr(e, out);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_applied_from_expr(e, out);
                }
            }
        }
        Expression::Cast(c) => collect_applied_from_expr(&c.expr, out),
        _ => {}
    }
}

fn collect_applied_from_stmt(stmt: &Statement, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            if let Some(ty) = &l.ty {
                collect_applied_from_type(ty, out);
            }
            collect_applied_from_expr(&l.value, out);
        }
        Statement::Assign(a) => {
            collect_applied_from_expr(&a.target, out);
            collect_applied_from_expr(&a.value, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_applied_from_expr(v, out);
            }
        }
        Statement::If(i) => {
            collect_applied_from_expr(&i.condition, out);
            for s in &i.then_block.statements {
                collect_applied_from_stmt(s, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_applied_from_stmt(s, out);
                }
            }
        }
        Statement::While(w) => {
            collect_applied_from_expr(&w.condition, out);
            for s in &w.body.statements {
                collect_applied_from_stmt(s, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_applied_from_expr(e, out));
            for s in &f.body.statements {
                collect_applied_from_stmt(s, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => collect_applied_from_expr(e, out),
        Statement::Print(p) => {
            for arg in &p.args {
                collect_applied_from_expr(arg, out);
            }
            if let Some(color) = &p.color {
                collect_applied_from_expr(color, out);
            }
        }
        Statement::Spawn(s) => {
            for stmt in &s.body.statements {
                collect_applied_from_stmt(stmt, out);
            }
        }
        Statement::Benchmark(b) => {
            for s in &b.statements {
                collect_applied_from_stmt(s, out);
            }
        }
        Statement::Unsafe(b) => {
            for s in &b.statements {
                collect_applied_from_stmt(s, out);
            }
        }
        _ => {}
    }
}

fn collect_applied_from_program(program: &Program, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    for s in &program.structs {
        for f in &s.fields {
            collect_applied_from_type(&f.ty, out);
        }
    }
    for e in &program.enums {
        for v in &e.variants {
            for f in &v.fields {
                collect_applied_from_type(f, out);
            }
        }
    }
    for f in &program.functions {
        for p in &f.params {
            collect_applied_from_type(&p.ty, out);
        }
        if let Some(ret) = &f.return_type {
            collect_applied_from_type(ret, out);
        }
        for stmt in &f.body.statements {
            collect_applied_from_stmt(stmt, out);
        }
    }
}

fn instantiate_struct(s: &StructDef, type_args: &[TypeAnnotation]) -> StructDef {
    let map: HashMap<String, TypeAnnotation> = s
        .type_params
        .iter()
        .cloned()
        .zip(type_args.iter().cloned())
        .collect();
    StructDef {
        name: mangle_inst(&s.name, type_args),
        doc: s.doc.clone(),
        type_params: vec![],
        attrs: s.attrs.clone(),
        fields: s
            .fields
            .iter()
            .map(|f| StructField {
                name: f.name.clone(),
                ty: substitute_type(&f.ty, &map),
            })
            .collect(),
        public: s.public,
    }
}

fn synthesize_vec_handle_struct(inst_name: &str) -> StructDef {
    StructDef {
        name: inst_name.into(),
        doc: None,
        type_params: vec![],
        attrs: StructAttrs::default(),
        fields: vec![StructField {
            name: "handle".into(),
            ty: TypeAnnotation::Ptr,
        }],
        public: false,
    }
}

fn monomorphize_structs(program: &mut Program) {
    let mut needed = Vec::new();
    collect_applied_from_program(program, &mut needed);

    let originals: HashMap<String, StructDef> = program
        .structs
        .iter()
        .filter(|s| !s.type_params.is_empty())
        .map(|s| (s.name.clone(), s.clone()))
        .collect();

    let mut existing: std::collections::HashSet<String> =
        program.structs.iter().map(|s| s.name.clone()).collect();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for (name, type_args) in needed {
        let inst_name = mangle_inst(&name, &type_args);
        let key = (name.clone(), inst_name.clone());
        if seen.contains(&key) || existing.contains(&inst_name) {
            continue;
        }
        seen.insert(key);
        let Some(orig) = originals.get(&name) else {
            if name == "Vec"
                && type_args.len() == 1
                && matches!(&type_args[0], TypeAnnotation::Struct(_))
            {
                program.structs.push(synthesize_vec_handle_struct(&inst_name));
                existing.insert(inst_name);
            }
            continue;
        };
        program.structs.push(instantiate_struct(orig, &type_args));
        existing.insert(inst_name);
    }

    program.structs.retain(|s| s.type_params.is_empty());
}

fn instantiate_enum(e: &EnumDef, type_args: &[TypeAnnotation]) -> EnumDef {
    let map: HashMap<String, TypeAnnotation> = e
        .type_params
        .iter()
        .cloned()
        .zip(type_args.iter().cloned())
        .collect();
    EnumDef {
        name: mangle_inst(&e.name, type_args),
        type_params: vec![],
        variants: e
            .variants
            .iter()
            .map(|v| EnumVariantDef {
                name: v.name.clone(),
                fields: v
                    .fields
                    .iter()
                    .map(|f| substitute_type(f, &map))
                    .collect(),
            })
            .collect(),
        public: e.public,
    }
}

fn infer_concrete_payload_ann(expr: &Expression) -> Option<TypeAnnotation> {
    match expr {
        Expression::Literal(Literal::Int(_)) => Some(TypeAnnotation::Integer(ast::IntKind::I32)),
        Expression::Literal(Literal::IntKind(_, k)) => Some(TypeAnnotation::Integer(*k)),
        Expression::Literal(Literal::Float(_, _)) => Some(TypeAnnotation::F64),
        Expression::Literal(Literal::Char(_)) => Some(TypeAnnotation::Char),
        Expression::Literal(Literal::Bool(_)) => Some(TypeAnnotation::Bool),
        Expression::Literal(Literal::String(_)) => Some(TypeAnnotation::String),
        Expression::Variable { .. } => None,
        Expression::Call(c) if matches!(
            c.callee.as_str(),
            "read_file" | "strcat" | "substring" | "i32_to_string" | "i64_to_string"
        ) => Some(TypeAnnotation::String),
        _ => None,
    }
}

fn enum_inst_from_type_ann(
    ty: Option<&TypeAnnotation>,
    generic_bases: &std::collections::HashSet<String>,
) -> Option<String> {
    let ty = ty?;
    match ty {
        TypeAnnotation::Enum(n) => Some(n.clone()),
        TypeAnnotation::Applied { base, args }
            if generic_bases.contains(base) && args.iter().all(is_concrete_ann) =>
        {
            Some(mangle_inst(base, args))
        }
        _ => None,
    }
}

fn rewrite_enum_variant_for_expected_type(
    ev: &mut EnumVariantExpr,
    expected: &TypeAnnotation,
    generic_bases: &std::collections::HashSet<String>,
) {
    let Some(inst) = enum_inst_from_type_ann(Some(expected), generic_bases) else {
        return;
    };
    if ev.enum_name.as_ref().is_some_and(|en| generic_bases.contains(en)) {
        ev.enum_name = Some(inst);
    }
}

fn rewrite_match_patterns(
    m: &mut MatchExpr,
    generic_bases: &std::collections::HashSet<String>,
    var_types: &std::collections::HashMap<String, TypeAnnotation>,
) {
    let inst = match &*m.scrutinee {
        Expression::Variable { name, .. } => {
            enum_inst_from_type_ann(var_types.get(name), generic_bases)
        }
        _ => None,
    };
    let Some(inst) = inst else {
        return;
    };
    for arm in &mut m.arms {
        match &mut arm.pattern {
            MatchPattern::Qualified(en, _) | MatchPattern::QualifiedBind(en, _, _) => {
                if generic_bases.contains(en) {
                    *en = inst.clone();
                }
            }
            MatchPattern::Or(ps) => {
                for p in ps {
                    if let MatchPattern::Qualified(en, _) | MatchPattern::QualifiedBind(en, _, _) =
                        p
                    {
                        if generic_bases.contains(en) {
                            *en = inst.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn monomorphize_enums(program: &mut Program) {
    let mut needed = Vec::new();
    collect_applied_from_program(program, &mut needed);
    collect_enum_instantiations_from_program(program, &mut needed);

    let originals: HashMap<String, EnumDef> = program
        .enums
        .iter()
        .filter(|e| !e.type_params.is_empty())
        .map(|e| (e.name.clone(), e.clone()))
        .collect();

    let mut existing: std::collections::HashSet<String> =
        program.enums.iter().map(|e| e.name.clone()).collect();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for (name, type_args) in needed {
        if !originals.contains_key(&name) {
            continue;
        }
        let inst_name = mangle_inst(&name, &type_args);
        let key = (name.clone(), inst_name.clone());
        if seen.contains(&key) || existing.contains(&inst_name) {
            continue;
        }
        seen.insert(key);
        let Some(orig) = originals.get(&name) else {
            continue;
        };
        program.enums.push(instantiate_enum(orig, &type_args));
        existing.insert(inst_name);
    }

    program.enums.retain(|e| e.type_params.is_empty());
}

fn collect_enum_instantiations_from_program(
    program: &Program,
    out: &mut Vec<(String, Vec<TypeAnnotation>)>,
) {
    let generic: std::collections::HashSet<String> = program
        .enums
        .iter()
        .filter(|e| !e.type_params.is_empty())
        .map(|e| e.name.clone())
        .collect();
    if generic.is_empty() {
        return;
    }
    for f in &program.functions {
        if !f.type_params.is_empty() {
            continue;
        }
        for stmt in &f.body.statements {
            collect_enum_instantiations_from_stmt(stmt, &generic, out);
        }
    }
}

fn collect_enum_instantiations_from_stmt(
    stmt: &Statement,
    generic: &std::collections::HashSet<String>,
    out: &mut Vec<(String, Vec<TypeAnnotation>)>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            if let Some(ty) = &l.ty {
                collect_applied_from_type(ty, out);
            }
            collect_enum_instantiations_from_expr(&l.value, generic, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_enum_instantiations_from_expr(v, generic, out);
            }
        }
        Statement::If(i) => {
            collect_enum_instantiations_from_expr(&i.condition, generic, out);
            for s in &i.then_block.statements {
                collect_enum_instantiations_from_stmt(s, generic, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_enum_instantiations_from_stmt(s, generic, out);
                }
            }
        }
        Statement::While(w) => {
            collect_enum_instantiations_from_expr(&w.condition, generic, out);
            for s in &w.body.statements {
                collect_enum_instantiations_from_stmt(s, generic, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_enum_instantiations_from_expr(e, generic, out));
            for s in &f.body.statements {
                collect_enum_instantiations_from_stmt(s, generic, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            collect_enum_instantiations_from_expr(e, generic, out);
        }
        Statement::Print(p) => {
            for a in &p.args {
                collect_enum_instantiations_from_expr(a, generic, out);
            }
            if let Some(c) = &p.color {
                collect_enum_instantiations_from_expr(c, generic, out);
            }
        }
        Statement::Spawn(s) => {
            for stmt in &s.body.statements {
                collect_enum_instantiations_from_stmt(stmt, generic, out);
            }
        }
        Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for s in &b.statements {
                collect_enum_instantiations_from_stmt(s, generic, out);
            }
        }
        _ => {}
    }
}

#[allow(clippy::only_used_in_recursion)]
fn collect_enum_instantiations_from_expr(
    expr: &Expression,
    generic: &std::collections::HashSet<String>,
    out: &mut Vec<(String, Vec<TypeAnnotation>)>,
) {
    if let Expression::EnumVariant(ev) = expr {
        if let Some(en) = &ev.enum_name {
            if (en == "Option" || en == "Result") && generic.contains(en) && !ev.args.is_empty() {
                if let Some(payload) = infer_concrete_payload_ann(&ev.args[0]) {
                    if en == "Option" {
                        out.push((en.clone(), vec![payload]));
                    } else {
                        out.push((en.clone(), vec![payload.clone(), payload]));
                    }
                }
            }
        }
    }
    match expr {
        Expression::Call(c) => {
            for a in &c.args {
                collect_enum_instantiations_from_expr(a, generic, out);
            }
        }
        Expression::Binary(b) => {
            collect_enum_instantiations_from_expr(&b.left, generic, out);
            collect_enum_instantiations_from_expr(&b.right, generic, out);
        }
        Expression::Unary(u) => collect_enum_instantiations_from_expr(&u.operand, generic, out),
        Expression::Grouped(g) => collect_enum_instantiations_from_expr(g, generic, out),
        Expression::If(i) => {
            collect_enum_instantiations_from_expr(&i.condition, generic, out);
            for_each_expr_in_block(&i.then_block, &mut |e| {
                collect_enum_instantiations_from_expr(e, generic, out);
            });
            for_each_expr_in_block(&i.else_block, &mut |e| {
                collect_enum_instantiations_from_expr(e, generic, out);
            });
        }
        Expression::Match(m) => {
            collect_enum_instantiations_from_expr(&m.scrutinee, generic, out);
            for a in &m.arms {
                if let Some(g) = &a.guard {
                    collect_enum_instantiations_from_expr(g, generic, out);
                }
                for_each_expr_in_block(&a.body, &mut |e| collect_enum_instantiations_from_expr(e, generic, out));
            }
        }
        Expression::MethodCall(mc) => {
            collect_enum_instantiations_from_expr(&mc.object, generic, out);
            for a in &mc.args {
                collect_enum_instantiations_from_expr(a, generic, out);
            }
        }
        Expression::FieldAccess(f) => collect_enum_instantiations_from_expr(&f.object, generic, out),
        Expression::Index(ix) => {
            collect_enum_instantiations_from_expr(&ix.object, generic, out);
            collect_enum_instantiations_from_expr(&ix.index, generic, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_enum_instantiations_from_expr(e, generic, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            collect_enum_instantiations_from_expr(element, generic, out);
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_enum_instantiations_from_expr(e, generic, out);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_enum_instantiations_from_expr(spread, generic, out);
            }
            for (_, e) in &s.fields {
                collect_enum_instantiations_from_expr(e, generic, out);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_enum_instantiations_from_expr(e, generic, out);
                }
            }
        }
        Expression::Cast(c) => collect_enum_instantiations_from_expr(&c.expr, generic, out),
        Expression::ArrowFn(a) => {
            if let ArrowBody::Block(b) = &a.body {
                for s in &b.statements {
                    collect_enum_instantiations_from_stmt(s, generic, out);
                }
            } else if let ArrowBody::Expr(e) = &a.body {
                collect_enum_instantiations_from_expr(e, generic, out);
            }
        }
        _ => {}
    }
}

fn rewrite_generic_enum_variants_program(
    program: &mut Program,
    generic_bases: &std::collections::HashSet<String>,
) {
    let func_params: std::collections::HashMap<String, Vec<TypeAnnotation>> = program
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.params.iter().map(|p| p.ty.clone()).collect()))
        .collect();
    for f in &mut program.functions {
        if !f.type_params.is_empty() {
            continue;
        }
        let mut var_types = std::collections::HashMap::new();
        for p in &f.params {
            var_types.insert(p.name.clone(), p.ty.clone());
        }
        rewrite_generic_enum_variants_block(
            &mut f.body.statements,
            generic_bases,
            f.return_type.as_ref(),
            &func_params,
            &mut var_types,
        );
    }
}

fn rewrite_generic_enum_variants_block(
    stmts: &mut [Statement],
    generic_bases: &std::collections::HashSet<String>,
    fn_return_type: Option<&TypeAnnotation>,
    func_params: &std::collections::HashMap<String, Vec<TypeAnnotation>>,
    var_types: &mut std::collections::HashMap<String, TypeAnnotation>,
) {
    for stmt in stmts {
        rewrite_generic_enum_variants_stmt(
            stmt,
            generic_bases,
            var_types,
            fn_return_type,
            func_params,
        );
    }
}

fn rewrite_generic_enum_variants_stmt(
    stmt: &mut Statement,
    generic_bases: &std::collections::HashSet<String>,
    var_types: &mut std::collections::HashMap<String, TypeAnnotation>,
    fn_return_type: Option<&TypeAnnotation>,
    func_params: &std::collections::HashMap<String, Vec<TypeAnnotation>>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let inst_name = enum_inst_from_type_ann(l.ty.as_ref(), generic_bases);
            if let Some(inst) = inst_name.clone() {
                if let Expression::EnumVariant(ev) = &mut l.value {
                    ev.enum_name = Some(inst);
                }
            }
            rewrite_generic_enum_variants_expr(&mut l.value, generic_bases, var_types, func_params);
            if let Some(ty) = l.ty.clone() {
                var_types.insert(l.name.clone(), ty);
            }
        }
        Statement::Assign(a) => {
            rewrite_generic_enum_variants_expr(&mut a.target, generic_bases, var_types, func_params);
            rewrite_generic_enum_variants_expr(&mut a.value, generic_bases, var_types, func_params);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                if let Some(inst) = enum_inst_from_type_ann(fn_return_type, generic_bases) {
                    if let Expression::EnumVariant(ev) = v {
                        ev.enum_name = Some(inst);
                    }
                }
                rewrite_generic_enum_variants_expr(v, generic_bases, var_types, func_params);
            }
        }
        Statement::If(i) => {
            rewrite_generic_enum_variants_expr(&mut i.condition, generic_bases, var_types, func_params);
            let mut then_vars = var_types.clone();
            rewrite_generic_enum_variants_block(
                &mut i.then_block.statements,
                generic_bases,
                fn_return_type,
                func_params,
                &mut then_vars,
            );
            if let Some(e) = &mut i.else_block {
                let mut else_vars = var_types.clone();
                rewrite_generic_enum_variants_block(
                    &mut e.statements,
                    generic_bases,
                    fn_return_type,
                    func_params,
                    &mut else_vars,
                );
            }
        }
        Statement::While(w) => {
            rewrite_generic_enum_variants_expr(&mut w.condition, generic_bases, var_types, func_params);
            let mut body_vars = var_types.clone();
            rewrite_generic_enum_variants_block(
                &mut w.body.statements,
                generic_bases,
                fn_return_type,
                func_params,
                &mut body_vars,
            );
        }
        Statement::For(f) => {
            f.map_exprs_mut(|e| {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            });
            let mut body_vars = var_types.clone();
            rewrite_generic_enum_variants_block(
                &mut f.body.statements,
                generic_bases,
                fn_return_type,
                func_params,
                &mut body_vars,
            );
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
        }
        Statement::Print(p) => {
            for a in &mut p.args {
                rewrite_generic_enum_variants_expr(a, generic_bases, var_types, func_params);
            }
            if let Some(c) = &mut p.color {
                rewrite_generic_enum_variants_expr(c, generic_bases, var_types, func_params);
            }
        }
        Statement::Spawn(s) => {
            let mut spawn_vars = var_types.clone();
            rewrite_generic_enum_variants_block(
                &mut s.body.statements,
                generic_bases,
                None,
                func_params,
                &mut spawn_vars,
            );
        }
        Statement::Unsafe(b) | Statement::Benchmark(b) => {
            let mut spawn_vars = var_types.clone();
            rewrite_generic_enum_variants_block(
                &mut b.statements,
                generic_bases,
                None,
                func_params,
                &mut spawn_vars,
            );
        }
        _ => {}
    }
}

fn rewrite_generic_enum_variants_expr(
    expr: &mut Expression,
    generic_bases: &std::collections::HashSet<String>,
    var_types: &std::collections::HashMap<String, TypeAnnotation>,
    func_params: &std::collections::HashMap<String, Vec<TypeAnnotation>>,
) {
    if let Expression::EnumVariant(ev) = expr {
        if let Some(en) = &ev.enum_name {
            if (en == "Option" || en == "Result") && !ev.args.is_empty() {
                if let Some(payload) = infer_concrete_payload_ann(&ev.args[0]) {
                    let type_args = if en == "Option" {
                        vec![payload]
                    } else {
                        vec![payload.clone(), payload]
                    };
                    ev.enum_name = Some(mangle_inst(en, &type_args));
                }
            }
        }
    }
    match expr {
        Expression::Binary(b) => {
            rewrite_generic_enum_variants_expr(&mut b.left, generic_bases, var_types, func_params);
            rewrite_generic_enum_variants_expr(&mut b.right, generic_bases, var_types, func_params);
        }
        Expression::Unary(u) => {
            rewrite_generic_enum_variants_expr(&mut u.operand, generic_bases, var_types, func_params)
        }
        Expression::Grouped(g) => {
            rewrite_generic_enum_variants_expr(g, generic_bases, var_types, func_params)
        }
        Expression::If(i) => {
            rewrite_generic_enum_variants_expr(&mut i.condition, generic_bases, var_types, func_params);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            });
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            });
        }
        Expression::Match(m) => {
            rewrite_match_patterns(m, generic_bases, var_types);
            rewrite_generic_enum_variants_expr(&mut m.scrutinee, generic_bases, var_types, func_params);
            for a in &mut m.arms {
                if let Some(g) = &mut a.guard {
                    rewrite_generic_enum_variants_expr(g, generic_bases, var_types, func_params);
                }
                for_each_expr_in_block_mut(&mut a.body, &mut |e| {
                    rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
                });
            }
        }
        Expression::Call(c) => {
            if let Some(params) = func_params.get(&c.callee) {
                for (arg, pty) in c.args.iter_mut().zip(params.iter()) {
                    if let Expression::EnumVariant(ev) = arg {
                        rewrite_enum_variant_for_expected_type(ev, pty, generic_bases);
                    }
                }
            }
            for a in &mut c.args {
                rewrite_generic_enum_variants_expr(a, generic_bases, var_types, func_params);
            }
        }
        Expression::MethodCall(mc) => {
            rewrite_generic_enum_variants_expr(&mut mc.object, generic_bases, var_types, func_params);
            for a in &mut mc.args {
                rewrite_generic_enum_variants_expr(a, generic_bases, var_types, func_params);
            }
        }
        Expression::FieldAccess(f) => {
            rewrite_generic_enum_variants_expr(&mut f.object, generic_bases, var_types, func_params);
        }
        Expression::Index(ix) => {
            rewrite_generic_enum_variants_expr(&mut ix.object, generic_bases, var_types, func_params);
            rewrite_generic_enum_variants_expr(&mut ix.index, generic_bases, var_types, func_params);
        }
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                rewrite_generic_enum_variants_expr(spread, generic_bases, var_types, func_params);
            }
            for (_, e) in &mut s.fields {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            }
        }
        Expression::ArrayLiteral(al) => {
            for e in al.spreads.iter_mut().chain(al.elems.iter_mut()) {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            rewrite_generic_enum_variants_expr(element, generic_bases, var_types, func_params);
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
                }
            }
        }
        Expression::Cast(c) => {
            rewrite_generic_enum_variants_expr(&mut c.expr, generic_bases, var_types, func_params)
        }
        Expression::ArrowFn(a) => {
            if let ArrowBody::Block(b) = &mut a.body {
                let mut arrow_vars = var_types.clone();
                rewrite_generic_enum_variants_block(
                    &mut b.statements,
                    generic_bases,
                    None,
                    func_params,
                    &mut arrow_vars,
                );
            } else if let ArrowBody::Expr(e) = &mut a.body {
                rewrite_generic_enum_variants_expr(e, generic_bases, var_types, func_params);
            }
        }
        _ => {}
    }
}

fn resolve_applied_type(
    ty: TypeAnnotation,
    generic_enums: &std::collections::HashSet<String>,
) -> TypeAnnotation {
    match ty {
        TypeAnnotation::Applied { base, args } if args.iter().all(is_concrete_ann) => {
            let inst = mangle_inst(&base, &args);
            if generic_enums.contains(&base) {
                TypeAnnotation::Enum(inst)
            } else {
                TypeAnnotation::Struct(inst)
            }
        }
        TypeAnnotation::Applied { base, args } => TypeAnnotation::Applied {
            base,
            args: args
                .into_iter()
                .map(|a| resolve_applied_type(a, generic_enums))
                .collect(),
        },
        TypeAnnotation::Array { elem, len } => TypeAnnotation::Array {
            elem: Box::new(resolve_applied_type(*elem, generic_enums)),
            len,
        },
        TypeAnnotation::Tuple(elems) => TypeAnnotation::Tuple(
            elems
                .into_iter()
                .map(|e| resolve_applied_type(e, generic_enums))
                .collect(),
        ),
        TypeAnnotation::RawPtr { inner } => TypeAnnotation::RawPtr {
            inner: Box::new(resolve_applied_type(*inner, generic_enums)),
        },
        TypeAnnotation::Ref {
            inner,
            mutable,
            lifetime,
        } => TypeAnnotation::Ref {
            inner: Box::new(resolve_applied_type(*inner, generic_enums)),
            mutable,
            lifetime,
        },
        TypeAnnotation::ForAll { lifetimes, inner } => TypeAnnotation::ForAll {
            lifetimes,
            inner: Box::new(resolve_applied_type(*inner, generic_enums)),
        },
        TypeAnnotation::FnPtr {
            lifetime_params,
            params,
            return_type,
        } => TypeAnnotation::FnPtr {
            lifetime_params,
            params: params
                .into_iter()
                .map(|p| resolve_applied_type(p, generic_enums))
                .collect(),
            return_type: return_type.map(|t| Box::new(resolve_applied_type(*t, generic_enums))),
        },
        other => other,
    }
}

fn resolve_applied_in_function(
    f: &mut Function,
    generic_enums: &std::collections::HashSet<String>,
) {
    for p in &mut f.params {
        p.ty = resolve_applied_type(p.ty.clone(), generic_enums);
    }
    if let Some(ret) = &mut f.return_type {
        *ret = resolve_applied_type(ret.clone(), generic_enums);
    }
    for stmt in &mut f.body.statements {
        resolve_applied_in_stmt(stmt, generic_enums);
    }
}

fn resolve_applied_in_stmt(stmt: &mut Statement, generic_enums: &std::collections::HashSet<String>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            if let Some(ty) = &mut l.ty {
                *ty = resolve_applied_type(ty.clone(), generic_enums);
            }
        }
        _ => {}
    }
}

fn resolve_applied_in_program(
    program: &mut Program,
    generic_enums: &std::collections::HashSet<String>,
) {
    for f in &mut program.functions {
        if f.type_params.is_empty() {
            resolve_applied_in_function(f, generic_enums);
        }
    }
}

fn singleton_struct_map(program: &Program) -> HashMap<String, String> {
    let mut instances: HashMap<String, Vec<String>> = HashMap::new();
    for s in &program.structs {
        if let Some(pos) = s.name.find("__") {
            let base = s.name[..pos].to_string();
            instances
                .entry(base)
                .or_default()
                .push(s.name.clone());
        }
    }
    instances
        .into_iter()
        .filter(|(_, v)| v.len() == 1)
        .map(|(k, v)| (k, v[0].clone()))
        .collect()
}

fn rewrite_struct_literals_expr(expr: &mut Expression, map: &HashMap<String, String>) {
    match expr {
        Expression::StructLiteral(s) => {
            if let Some(inst) = map.get(&s.name) {
                s.name = inst.clone();
            }
            for spread in &mut s.spreads {
                rewrite_struct_literals_expr(spread, map);
            }
            for (_, e) in &mut s.fields {
                rewrite_struct_literals_expr(e, map);
            }
        }
        Expression::Call(c) => {
            for a in &mut c.args {
                rewrite_struct_literals_expr(a, map);
            }
        }
        Expression::Binary(b) => {
            rewrite_struct_literals_expr(&mut b.left, map);
            rewrite_struct_literals_expr(&mut b.right, map);
        }
        Expression::Unary(u) => rewrite_struct_literals_expr(&mut u.operand, map),
        Expression::Grouped(g) => rewrite_struct_literals_expr(g, map),
        Expression::If(i) => {
            rewrite_struct_literals_expr(&mut i.condition, map);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| {
                rewrite_struct_literals_expr(e, map);
            });
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| {
                rewrite_struct_literals_expr(e, map);
            });
        }
        Expression::Match(m) => {
            rewrite_struct_literals_expr(&mut m.scrutinee, map);
            for a in &mut m.arms {
                if let Some(g) = &mut a.guard {
                    rewrite_struct_literals_expr(g, map);
                }
                for_each_expr_in_block_mut(&mut a.body, &mut |e| rewrite_struct_literals_expr(e, map));
            }
        }
        Expression::Await(e) => rewrite_struct_literals_expr(e, map),
        Expression::MethodCall(mc) => {
            rewrite_struct_literals_expr(&mut mc.object, map);
            for a in &mut mc.args {
                rewrite_struct_literals_expr(a, map);
            }
        }
        Expression::FieldAccess(f) => rewrite_struct_literals_expr(&mut f.object, map),
        Expression::Index(ix) => {
            rewrite_struct_literals_expr(&mut ix.object, map);
            rewrite_struct_literals_expr(&mut ix.index, map);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.spreads.iter_mut().chain(al.elems.iter_mut()) {
                rewrite_struct_literals_expr(e, map);
            }
        }
        Expression::ArrayRepeat { element, .. } => rewrite_struct_literals_expr(element, map),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                rewrite_struct_literals_expr(e, map);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    rewrite_struct_literals_expr(e, map);
                }
            }
        }
        Expression::Cast(c) => rewrite_struct_literals_expr(&mut c.expr, map),
        _ => {}
    }
}

fn rewrite_struct_literals_program(program: &mut Program, map: &HashMap<String, String>) {
    for f in &mut program.functions {
        for stmt in &mut f.body.statements {
            rewrite_struct_literals_stmt(stmt, map);
        }
    }
}

fn rewrite_struct_literals_stmt(stmt: &mut Statement, map: &HashMap<String, String>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            rewrite_struct_literals_expr(&mut l.value, map);
        }
        Statement::Assign(a) => {
            rewrite_struct_literals_expr(&mut a.target, map);
            rewrite_struct_literals_expr(&mut a.value, map);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                rewrite_struct_literals_expr(v, map);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => rewrite_struct_literals_expr(e, map),
        Statement::Print(p) => {
            for arg in &mut p.args {
                rewrite_struct_literals_expr(arg, map);
            }
            if let Some(c) = &mut p.color {
                rewrite_struct_literals_expr(c, map);
            }
        }
        _ => {}
    }
}

fn substitute_expr(expr: &Expression, map: &HashMap<String, TypeAnnotation>) -> Expression {
    match expr {
        Expression::Call(c) => Expression::Call(CallExpr {
            callee: c.callee.clone(),
            type_args: substitute_type_args(&c.type_args, map),
            args: c.args.iter().map(|a| substitute_expr(a, map)).collect(),
            span: c.span.clone(),
        }),
        Expression::Binary(b) => Expression::Binary(Box::new(BinaryExpr {
            left: substitute_expr(&b.left, map),
            op: b.op,
            right: substitute_expr(&b.right, map),
            span: b.span.clone(),
        })),
        Expression::Unary(u) => Expression::Unary(Box::new(UnaryExpr {
            op: u.op,
            operand: substitute_expr(&u.operand, map),
            span: u.span.clone(),
        })),
        Expression::Grouped(inner) => {
            Expression::Grouped(Box::new(substitute_expr(inner, map)))
        }
        Expression::If(i) => Expression::If(Box::new(IfExpr {
            condition: substitute_expr(&i.condition, map),
            then_block: substitute_block(&i.then_block, map),
            else_block: substitute_block(&i.else_block, map),
            span: i.span.clone(),
        })),
        Expression::Match(m) => Expression::Match(Box::new(MatchExpr {
            scrutinee: Box::new(substitute_expr(&m.scrutinee, map)),
            arms: m
                .arms
                .iter()
                .map(|a| MatchArm {
                    pattern: a.pattern.clone(),
                    guard: a.guard.as_ref().map(|g| substitute_expr(g, map)),
                    body: substitute_block(&a.body, map),
                })
                .collect(),
            span: m.span.clone(),
        })),
        Expression::Await(inner) => {
            Expression::Await(Box::new(substitute_expr(inner, map)))
        }
        Expression::TemplateLiteral(t) => Expression::TemplateLiteral(TemplateLiteralExpr {
            parts: t
                .parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Static(s) => TemplatePart::Static(s.clone()),
                    TemplatePart::Interpolation(e) => {
                        TemplatePart::Interpolation(Box::new(substitute_expr(e, map)))
                    }
                })
                .collect(),
            span: t.span.clone(),
        }),
        Expression::Cast(c) => Expression::Cast(Box::new(CastExpr {
            expr: substitute_expr(&c.expr, map),
            target_type: substitute_type(&c.target_type, map),
            span: c.span.clone(),
        })),
        other => other.clone(),
    }
}

fn substitute_block(block: &Block, map: &HashMap<String, TypeAnnotation>) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|s| substitute_stmt(s, map))
            .collect(),
    }
}

fn substitute_stmt(stmt: &Statement, map: &HashMap<String, TypeAnnotation>) -> Statement {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let mut s = if matches!(stmt, Statement::Const(_)) {
                Statement::Const(l.clone())
            } else {
                Statement::Let(l.clone())
            };
            if let Statement::Let(ref mut x) | Statement::Const(ref mut x) = s {
                x.ty = x.ty.as_ref().map(|t| substitute_type(t, map));
                x.value = substitute_expr(&l.value, map);
            }
            s
        }
        Statement::Assign(a) => Statement::Assign(AssignStmt {
            target: substitute_expr(&a.target, map),
            value: substitute_expr(&a.value, map),
            span: a.span.clone(),
        }),
        Statement::Return(r) => Statement::Return(ReturnStmt {
            value: r.value.as_ref().map(|v| substitute_expr(v, map)),
        }),
        Statement::If(i) => Statement::If(IfStmt {
            condition: substitute_expr(&i.condition, map),
            then_block: substitute_block(&i.then_block, map),
            else_block: i
                .else_block
                .as_ref()
                .map(|b| substitute_block(b, map)),
        }),
        Statement::While(w) => Statement::While(WhileStmt {
            condition: substitute_expr(&w.condition, map),
            body: substitute_block(&w.body, map),
        }),
        Statement::For(f) => Statement::For(ForStmt {
            var: f.var.clone(),
            parallel: f.parallel.clone(),
            progress: f.progress.clone(),
            kind: match &f.kind {
                ForKind::Range { start, end } => ForKind::Range {
                    start: substitute_expr(start, map),
                    end: substitute_expr(end, map),
                },
                ForKind::Iterable { iterable } => ForKind::Iterable {
                    iterable: substitute_expr(iterable, map),
                },
            },
            body: substitute_block(&f.body, map),
        }),
        Statement::Expression(e) => Statement::Expression(substitute_expr(e, map)),
        Statement::Print(p) => Statement::Print(p.clone().map_expressions(|a| substitute_expr(&a, map))),
        Statement::Defer(e) => Statement::Defer(substitute_expr(e, map)),
        Statement::Spawn(s) => Statement::Spawn(SpawnStmt {
            kind: s.kind,
            body: substitute_block(&s.body, map),
        }),
        Statement::Benchmark(b) => Statement::Benchmark(substitute_block(b, map)),
        Statement::Unsafe(b) => Statement::Unsafe(substitute_block(b, map)),
        Statement::Asm { template, span } => Statement::Asm {
            template: template.clone(),
            span: span.clone(),
        },
        other => other.clone(),
    }
}

fn instantiate_function(f: &Function, type_args: &[TypeAnnotation]) -> Function {
    let map: HashMap<String, TypeAnnotation> = f
        .type_params
        .iter()
        .cloned()
        .zip(type_args.iter().cloned())
        .collect();
    Function {
        name: mangle_inst(&f.name, type_args),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        params: f
            .params
            .iter()
            .map(|p| Param {
                ty: substitute_type(&p.ty, &map),
                name: p.name.clone(),
                destructure: p.destructure.clone(),
                no_escape: p.no_escape,
                mutable: p.mutable,
            })
            .collect(),
        return_type: f
            .return_type
            .as_ref()
            .map(|t| substitute_type(t, &map)),
        body: substitute_block(&f.body, &map),
        ..f.clone()
    }
}

fn collect_calls(expr: &Expression, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    match expr {
        Expression::Call(c) => {
            if !c.type_args.is_empty() {
                out.push((c.callee.clone(), c.type_args.clone()));
            }
            for a in &c.args {
                collect_calls(a, out);
            }
        }
        Expression::Binary(b) => {
            collect_calls(&b.left, out);
            collect_calls(&b.right, out);
        }
        Expression::Unary(u) => collect_calls(&u.operand, out),
        Expression::Grouped(g) => collect_calls(g, out),
        Expression::If(i) => {
            collect_calls(&i.condition, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_calls(e, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_calls(e, out));
        }
        Expression::Match(m) => {
            collect_calls(&m.scrutinee, out);
            for a in &m.arms {
                if let Some(g) = &a.guard {
                    collect_calls(g, out);
                }
                for_each_expr_in_block(&a.body, &mut |e| collect_calls(e, out));
            }
        }
        Expression::Await(e) => collect_calls(e, out),
        Expression::MethodCall(mc) => {
            for a in &mc.args {
                collect_calls(a, out);
            }
            collect_calls(&mc.object, out);
        }
        Expression::FieldAccess(f) => collect_calls(&f.object, out),
        Expression::Index(ix) => {
            collect_calls(&ix.object, out);
            collect_calls(&ix.index, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_calls(e, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => collect_calls(element, out),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_calls(e, out);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_calls(spread, out);
            }
            for (_, e) in &s.fields {
                collect_calls(e, out);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_calls(e, out);
                }
            }
        }
        Expression::Cast(c) => collect_calls(&c.expr, out),
        _ => {}
    }
}

fn collect_from_program(program: &Program, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    for c in &program.consts {
        collect_calls(&c.value, out);
    }
    for f in &program.functions {
        for stmt in &f.body.statements {
            collect_from_stmt(stmt, out);
        }
    }
    for imp in &program.impls {
        for m in &imp.methods {
            for stmt in &m.body.statements {
                collect_from_stmt(stmt, out);
            }
        }
    }
    for ti in &program.trait_impls {
        for m in &ti.methods {
            for stmt in &m.body.statements {
                collect_from_stmt(stmt, out);
            }
        }
    }
}

fn collect_from_stmt(stmt: &Statement, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => collect_calls(&l.value, out),
        Statement::Assign(a) => {
            collect_calls(&a.target, out);
            collect_calls(&a.value, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_calls(v, out);
            }
        }
        Statement::If(i) => {
            collect_calls(&i.condition, out);
            for s in &i.then_block.statements {
                collect_from_stmt(s, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_from_stmt(s, out);
                }
            }
        }
        Statement::While(w) => {
            collect_calls(&w.condition, out);
            for s in &w.body.statements {
                collect_from_stmt(s, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_calls(e, out));
            for s in &f.body.statements {
                collect_from_stmt(s, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => collect_calls(e, out),
        Statement::Print(p) => {
            for arg in &p.args {
                collect_calls(arg, out);
            }
            if let Some(c) = &p.color {
                collect_calls(c, out);
            }
        }
        Statement::Spawn(s) => {
            for stmt in &s.body.statements {
                collect_from_stmt(stmt, out);
            }
        }
        Statement::Benchmark(b) => {
            for s in &b.statements {
                collect_from_stmt(s, out);
            }
        }
        _ => {}
    }
}

fn collect_from_function(f: &Function, out: &mut Vec<(String, Vec<TypeAnnotation>)>) {
    for stmt in &f.body.statements {
        collect_from_stmt(stmt, out);
    }
}

/// Duplicate generic functions for each monomorph instance used in the program.
/// Returns trait-bound validation errors (empty when all bounds satisfied).
pub fn monomorphize_program(program: &mut Program) -> Vec<errors::NyraError> {
    infer_calls::infer_generic_call_sites(program);
    let bound_errors = trait_bounds::validate_trait_bounds(program);

    let generic_enum_bases: std::collections::HashSet<String> = program
        .enums
        .iter()
        .filter(|e| !e.type_params.is_empty())
        .map(|e| e.name.clone())
        .chain(["Result".into(), "Option".into()])
        .collect();

    monomorphize_structs(program);
    monomorphize_enums(program);
    synthesize_arc_drop_impls(program);
    let struct_map = singleton_struct_map(program);
    resolve_applied_in_program(program, &generic_enum_bases);
    rewrite_generic_enum_variants_program(program, &generic_enum_bases);
    rewrite_struct_literals_program(program, &struct_map);

    let mut worklist: Vec<(String, Vec<TypeAnnotation>)> = Vec::new();
    collect_from_program(program, &mut worklist);
    for inst in &program.export_instances {
        worklist.push((inst.fn_name.clone(), inst.type_args.clone()));
    }

    let originals: HashMap<String, Function> = program
        .functions
        .iter()
        .filter(|f| !f.type_params.is_empty())
        .map(|f| (f.name.clone(), f.clone()))
        .collect();

    let mut existing: std::collections::HashSet<String> =
        program.functions.iter().map(|f| f.name.clone()).collect();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    while let Some((name, type_args)) = worklist.pop() {
        let inst_name = mangle_inst(&name, &type_args);
        let key = (name.clone(), inst_name.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        if existing.contains(&inst_name) {
            continue;
        }
        let Some(orig) = originals.get(&name) else {
            continue;
        };
        let specialized = instantiate_function(orig, &type_args);
        collect_from_function(&specialized, &mut worklist);
        program.functions.push(specialized);
        existing.insert(inst_name);
    }

    for f in &mut program.functions {
        for stmt in &mut f.body.statements {
            rewrite_stmt(stmt);
        }
    }
    for c in &mut program.consts {
        rewrite_expr(&mut c.value);
    }
    bound_errors
}

fn rewrite_expr(expr: &mut Expression) {
    match expr {
        Expression::Call(c) => {
            if !c.type_args.is_empty()
                && !matches!(c.callee.as_str(), "size_of" | "align_of")
            {
                c.callee = mangle_inst(&c.callee, &c.type_args);
                c.type_args.clear();
            }
            for a in &mut c.args {
                rewrite_expr(a);
            }
        }
        Expression::Binary(b) => {
            rewrite_expr(&mut b.left);
            rewrite_expr(&mut b.right);
        }
        Expression::Unary(u) => rewrite_expr(&mut u.operand),
        Expression::Grouped(g) => rewrite_expr(g),
        Expression::If(i) => {
            rewrite_expr(&mut i.condition);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| rewrite_expr(e));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| rewrite_expr(e));
        }
        Expression::Match(m) => {
            rewrite_expr(&mut m.scrutinee);
            for a in &mut m.arms {
                if let Some(g) = &mut a.guard {
                    rewrite_expr(g);
                }
                for_each_expr_in_block_mut(&mut a.body, &mut |e| rewrite_expr(e));
            }
        }
        Expression::Await(e) => rewrite_expr(e),
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    rewrite_expr(e);
                }
            }
        }
        _ => {}
    }
}

fn rewrite_stmt(stmt: &mut Statement) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => rewrite_expr(&mut l.value),
        Statement::Assign(a) => rewrite_expr(&mut a.value),
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                rewrite_expr(v);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => rewrite_expr(e),
        Statement::Print(p) => {
            for arg in &mut p.args {
                rewrite_expr(arg);
            }
            if let Some(c) = &mut p.color {
                rewrite_expr(c);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangle_i32_generic_instance() {
        let src = r#"fn id<T>(x: T) -> T { return x }
fn main() {
    let n = id<i32>(7)
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorphize_program(&mut program);
        assert!(
            program.functions.iter().any(|f| f.name == "id__i32"),
            "expected monomorphized id__i32"
        );
    }

    #[test]
    fn infers_generic_call_site_id_i32() {
        let src = r#"fn id<T>(x: T) -> T { return x }
fn main() { print(id(7)) }"#;
        let (tokens, _) = lexer::Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorphize_program(&mut program);
        assert!(
            program.functions.iter().any(|f| f.name == "id__i32"),
            "expected monomorphized id__i32"
        );
    }

    #[test]
    fn rewrites_generic_call_callee() {
        let src = r#"fn id<T>(x: T) -> T { return x }
fn main() {
    let n = id<i32>(7)
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorphize_program(&mut program);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let has_inst_call = main.body.statements.iter().any(|s| {
            if let Statement::Let(l) = s {
                matches!(
                    l.value,
                    Expression::Call(ref c) if c.callee == "id__i32"
                )
            } else {
                false
            }
        });
        assert!(has_inst_call, "expected rewritten id__i32 call");
    }

    #[test]
    fn rewrites_result_err_inferred_let_binding() {
        let src = r#"enum Result<T, E> { Ok(T), Err(E) }
fn main() {
    let err = Result.Err(0)
    print(match err { Result.Ok(_) => 1, Result.Err(_) => 0 })
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorphize_program(&mut program);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let let_stmt = match &main.body.statements[0] {
            Statement::Let(l) => l,
            other => panic!("expected let err, got {other:?}"),
        };
        match &let_stmt.value {
            Expression::EnumVariant(ev) => {
                assert_eq!(ev.enum_name.as_deref(), Some("Result__i32_i32"));
            }
            other => panic!("expected Result__i32_i32 variant, got {other:?}"),
        }
    }

    #[test]
    fn rewrites_result_ok_for_call_param() {
        let src = r#"enum Result<T, E> { Ok(T), Err(E) }
fn take(r: Result<i32, i32>) -> i32 {
    return match r { Result.Ok(v) => v, Result.Err(e) => e }
}
fn main() { print(take(Result.Ok(7))) }"#;
        let (tokens, _) = lexer::Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorphize_program(&mut program);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let call = match &main.body.statements[0] {
            Statement::Print(p) => match p.args.first() {
                Some(Expression::Call(c)) => c,
                other => panic!("expected call, got {other:?}"),
            },
            other => panic!("expected print, got {other:?}"),
        };
        assert_eq!(call.callee, "take");
        match call.args.first() {
            Some(Expression::EnumVariant(ev)) => {
                assert_eq!(ev.enum_name.as_deref(), Some("Result__i32_i32"));
            }
            other => panic!("expected Result__i32_i32 variant, got {other:?}"),
        }
    }

    #[test]
    fn rewrites_trait_bound_call_in_assert_eq_arg() {
        let src = r#"trait Add {
    fn add(self, other: i32) -> i32
}
struct Counter {
    value: i32
}
impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}
fn sum_one<T: Add>(x: T) -> i32 {
    return x.add(1)
}
test fn test_trait_bound_generic_call() {
    let c = Counter { value: 10 }
    assert_eq(sum_one(c), 11)
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "g.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorphize_program(&mut program);
        assert!(
            program.functions.iter().any(|f| f.name == "sum_one__S_Counter"),
            "expected monomorphized sum_one__S_Counter"
        );
        let test_fn = program
            .functions
            .iter()
            .find(|f| f.name == "test_trait_bound_generic_call")
            .unwrap();
        let assert_call = test_fn.body.statements.iter().find_map(|s| {
            if let Statement::Expression(Expression::Call(c)) = s {
                Some(c)
            } else {
                None
            }
        }).expect("assert_eq call");
        let sum_call = assert_call
            .args
            .first()
            .and_then(|e| {
                if let Expression::Call(c) = e {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("sum_one nested in assert_eq");
        assert_eq!(sum_call.callee, "sum_one__S_Counter");
        assert!(sum_call.type_args.is_empty());
    }
}
