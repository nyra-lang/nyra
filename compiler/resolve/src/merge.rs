//! Merge imported compilation units into the entry program.

use std::collections::{HashMap, HashSet};

use ast::*;
use errors::{ErrorKind, NyraError, Span, E039_IMPORT_SYMBOL};

use crate::symbols::{collect_program_uses, top_level_export_names};

fn mangle_name(prefix: Option<&str>, name: &str) -> String {
    prefix
        .map(|p| format!("{p}__{name}"))
        .unwrap_or_else(|| name.to_string())
}

/// Merge `other` into `target`.
///
/// - Whole-module import (`selected` empty): all `pub` symbols (existing behavior).
/// - Selective `import { a, b } from "…"`: only named `pub` roots, plus same-file
///   dependency closure (including `priv` helpers referenced by those roots).
pub(crate) fn merge_program(
    target: &mut Program,
    other: Program,
    import_alias: Option<&str>,
    selected: &[ImportName],
) -> Vec<NyraError> {
    if selected.is_empty() {
        merge_filtered(target, other, import_alias, true);
        return vec![];
    }
    match filter_selective(other, selected) {
        Ok(filtered) => {
            // Selective imports flatten into scope (no module alias).
            merge_filtered(target, filtered, None, false);
            vec![]
        }
        Err(errs) => errs,
    }
}

/// `pub_only`: whole-module import skips `priv`. Selective closure keeps helpers.
fn merge_filtered(
    target: &mut Program,
    other: Program,
    import_alias: Option<&str>,
    pub_only: bool,
) {
    for c in other.consts {
        if pub_only && !c.public {
            continue;
        }
        let name = mangle_name(import_alias, &c.name);
        if !target.consts.iter().any(|x| x.name == name) {
            let mut item = c;
            item.name = name;
            target.consts.push(item);
        }
    }
    for s in other.structs {
        if pub_only && !s.public {
            continue;
        }
        let name = mangle_name(import_alias, &s.name);
        if !target.structs.iter().any(|x| x.name == name) {
            let mut item = s;
            item.name = name;
            target.structs.push(item);
        }
    }
    for u in other.unions {
        if pub_only && !u.public {
            continue;
        }
        let name = mangle_name(import_alias, &u.name);
        if !target.unions.iter().any(|x| x.name == name) {
            let mut item = u;
            item.name = name;
            target.unions.push(item);
        }
    }
    for e in other.enums {
        if pub_only && !e.public {
            continue;
        }
        let name = mangle_name(import_alias, &e.name);
        if !target.enums.iter().any(|x| x.name == name) {
            let mut item = e;
            item.name = name;
            target.enums.push(item);
        }
    }
    for t in other.traits {
        if !target.traits.iter().any(|x| x.name == t.name) {
            target.traits.push(t);
        }
    }
    for ti in other.trait_impls {
        if !target
            .trait_impls
            .iter()
            .any(|x| x.type_name == ti.type_name && x.trait_name == ti.trait_name)
        {
            target.trait_impls.push(ti);
        }
    }
    for m in other.macros {
        if !target.macros.iter().any(|x| x.name == m.name) {
            target.macros.push(m);
        }
    }
    for i in other.impls {
        if pub_only && !i.methods.iter().all(|m| m.public) {
            continue;
        }
        let type_name = mangle_name(import_alias, &i.type_name);
        if !target.impls.iter().any(|x| x.type_name == type_name) {
            let mut item = i;
            item.type_name = type_name;
            target.impls.push(item);
        }
    }
    for e in other.externs {
        if !target.externs.iter().any(|x| x.name == e.name) {
            target.externs.push(e);
        }
    }
    let comptime = other.comptime;
    for f in other.functions {
        if comptime {
            continue;
        }
        if pub_only && !f.public {
            continue;
        }
        let name = mangle_name(import_alias, &f.name);
        if !target.functions.iter().any(|x| x.name == name) {
            let mut item = f;
            item.name = name;
            target.functions.push(item);
        }
    }
    for inst in other.export_instances {
        let dup = target
            .export_instances
            .iter()
            .any(|x| x.fn_name == inst.fn_name && x.type_args == inst.type_args);
        if !dup {
            target.export_instances.push(inst);
        }
    }
    if target.module.is_none() {
        target.module = other.module;
    }
}

fn filter_selective(other: Program, selected: &[ImportName]) -> Result<Program, Vec<NyraError>> {
    let mut errors = Vec::new();
    let mut rename_map: HashMap<String, String> = HashMap::new();

    for sel in selected {
        let local = sel
            .rename
            .clone()
            .unwrap_or_else(|| sel.name.clone());
        if let Some(err) = validate_selected_symbol(&other, sel) {
            errors.push(err);
            continue;
        }
        rename_map.insert(sel.name.clone(), local);
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let defined = module_defined_names(&other);
    let mut keep: HashSet<String> = rename_map.keys().cloned().collect();

    // Fixed-point: pull same-file helpers (pub or priv) referenced by kept items.
    loop {
        let subset = project_names(&other, &keep);
        let uses = collect_program_uses(&subset);
        let mut added = false;
        for u in uses {
            if defined.contains(&u) && keep.insert(u) {
                added = true;
            }
        }
        if !added {
            break;
        }
    }

    let mut filtered = project_names(&other, &keep);

    for (from, to) in &rename_map {
        if from != to {
            rename_symbol_in_program(&mut filtered, from, to);
        }
    }

    for sel in selected {
        let local = sel.rename.as_ref().unwrap_or(&sel.name);
        mark_public(&mut filtered, local);
    }

    Ok(filtered)
}

fn validate_selected_symbol(other: &Program, sel: &ImportName) -> Option<NyraError> {
    let name = &sel.name;
    if let Some(f) = other.functions.iter().find(|f| f.name == *name) {
        if !f.public {
            return Some(priv_import_error(name, &sel.span));
        }
        return None;
    }
    if let Some(c) = other.consts.iter().find(|c| c.name == *name) {
        if !c.public {
            return Some(priv_import_error(name, &sel.span));
        }
        return None;
    }
    if let Some(s) = other.structs.iter().find(|s| s.name == *name) {
        if !s.public {
            return Some(priv_import_error(name, &sel.span));
        }
        return None;
    }
    if let Some(e) = other.enums.iter().find(|e| e.name == *name) {
        if !e.public {
            return Some(priv_import_error(name, &sel.span));
        }
        return None;
    }
    if let Some(u) = other.unions.iter().find(|u| u.name == *name) {
        if !u.public {
            return Some(priv_import_error(name, &sel.span));
        }
        return None;
    }
    if other.traits.iter().any(|t| t.name == *name)
        || other.macros.iter().any(|m| m.name == *name)
        || other.externs.iter().any(|e| e.name == *name)
    {
        return None;
    }

    let mut available: Vec<String> = top_level_export_names(other)
        .into_iter()
        .filter(|n| is_public_export(other, n))
        .collect();
    available.sort();
    let hint = if available.is_empty() {
        "module exports no public symbols".to_string()
    } else {
        format!("available: {}", available.join(", "))
    };
    Some(
        NyraError::coded(
            E039_IMPORT_SYMBOL,
            ErrorKind::NameResolution,
            sel.span.clone(),
            format!("cannot import `{name}`"),
        )
        .label(format!("`{name}` not found in imported module ({hint})"))
        .help("use `import { name } from \"path.ny\"` with a public symbol from that file"),
    )
}

fn is_public_export(program: &Program, name: &str) -> bool {
    if let Some(f) = program.functions.iter().find(|f| f.name == name) {
        return f.public;
    }
    if let Some(c) = program.consts.iter().find(|c| c.name == name) {
        return c.public;
    }
    if let Some(s) = program.structs.iter().find(|s| s.name == name) {
        return s.public;
    }
    if let Some(e) = program.enums.iter().find(|e| e.name == name) {
        return e.public;
    }
    if let Some(u) = program.unions.iter().find(|u| u.name == name) {
        return u.public;
    }
    true
}

fn priv_import_error(name: &str, span: &Span) -> NyraError {
    NyraError::coded(
        E039_IMPORT_SYMBOL,
        ErrorKind::NameResolution,
        span.clone(),
        format!("cannot import `{name}`"),
    )
    .label(format!(
        "`{name}` is private — only `pub` symbols can be imported"
    ))
    .help("remove `priv` on the definition, or import a public API that uses it")
}

fn module_defined_names(program: &Program) -> HashSet<String> {
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
    for u in &program.unions {
        names.insert(u.name.clone());
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

fn project_names(program: &Program, keep: &HashSet<String>) -> Program {
    Program {
        module: program.module.clone(),
        no_std: program.no_std,
        comptime: program.comptime,
        allow_extended: program.allow_extended,
        imports: vec![],
        consts: program
            .consts
            .iter()
            .filter(|c| keep.contains(&c.name))
            .cloned()
            .collect(),
        structs: program
            .structs
            .iter()
            .filter(|s| keep.contains(&s.name))
            .cloned()
            .collect(),
        unions: program
            .unions
            .iter()
            .filter(|u| keep.contains(&u.name))
            .cloned()
            .collect(),
        enums: program
            .enums
            .iter()
            .filter(|e| keep.contains(&e.name))
            .cloned()
            .collect(),
        traits: program
            .traits
            .iter()
            .filter(|t| keep.contains(&t.name))
            .cloned()
            .collect(),
        trait_impls: program
            .trait_impls
            .iter()
            .filter(|ti| keep.contains(&ti.type_name) || keep.contains(&ti.trait_name))
            .cloned()
            .collect(),
        macros: program
            .macros
            .iter()
            .filter(|m| keep.contains(&m.name))
            .cloned()
            .collect(),
        impls: program
            .impls
            .iter()
            .filter(|i| keep.contains(&i.type_name))
            .cloned()
            .collect(),
        externs: program
            .externs
            .iter()
            .filter(|e| keep.contains(&e.name))
            .cloned()
            .collect(),
        functions: program
            .functions
            .iter()
            .filter(|f| keep.contains(&f.name))
            .cloned()
            .collect(),
        export_instances: program
            .export_instances
            .iter()
            .filter(|i| keep.contains(&i.fn_name))
            .cloned()
            .collect(),
    }
}

fn mark_public(program: &mut Program, name: &str) {
    if let Some(f) = program.functions.iter_mut().find(|f| f.name == name) {
        f.public = true;
    }
    if let Some(c) = program.consts.iter_mut().find(|c| c.name == name) {
        c.public = true;
    }
    if let Some(s) = program.structs.iter_mut().find(|s| s.name == name) {
        s.public = true;
    }
    if let Some(e) = program.enums.iter_mut().find(|e| e.name == name) {
        e.public = true;
    }
    if let Some(u) = program.unions.iter_mut().find(|u| u.name == name) {
        u.public = true;
    }
}

fn rename_symbol_in_program(program: &mut Program, from: &str, to: &str) {
    for f in &mut program.functions {
        if f.name == from {
            f.name = to.to_string();
        }
        rename_in_block(&mut f.body, from, to);
    }
    for c in &mut program.consts {
        if c.name == from {
            c.name = to.to_string();
        }
        rename_in_expr(&mut c.value, from, to);
    }
    for s in &mut program.structs {
        if s.name == from {
            s.name = to.to_string();
        }
    }
    for e in &mut program.enums {
        if e.name == from {
            e.name = to.to_string();
        }
    }
    for u in &mut program.unions {
        if u.name == from {
            u.name = to.to_string();
        }
    }
    for t in &mut program.traits {
        if t.name == from {
            t.name = to.to_string();
        }
    }
    for m in &mut program.macros {
        if m.name == from {
            m.name = to.to_string();
        }
    }
    for e in &mut program.externs {
        if e.name == from {
            e.name = to.to_string();
        }
    }
    for i in &mut program.impls {
        if i.type_name == from {
            i.type_name = to.to_string();
        }
        for m in &mut i.methods {
            rename_in_block(&mut m.body, from, to);
        }
    }
    for ti in &mut program.trait_impls {
        if ti.type_name == from {
            ti.type_name = to.to_string();
        }
        if ti.trait_name == from {
            ti.trait_name = to.to_string();
        }
        for m in &mut ti.methods {
            rename_in_block(&mut m.body, from, to);
        }
    }
    for inst in &mut program.export_instances {
        if inst.fn_name == from {
            inst.fn_name = to.to_string();
        }
    }
}

fn rename_in_block(block: &mut Block, from: &str, to: &str) {
    for_each_expr_in_block_mut(block, &mut |e| rename_in_expr(e, from, to));
}

fn rename_in_expr(expr: &mut Expression, from: &str, to: &str) {
    match expr {
        Expression::Variable { name, .. } if name == from => {
            *name = to.to_string();
        }
        Expression::Call(c) => {
            if c.callee == from {
                c.callee = to.to_string();
            }
            for a in &mut c.args {
                rename_in_expr(a, from, to);
            }
        }
        Expression::MethodCall(m) => {
            rename_in_expr(&mut m.object, from, to);
            for a in &mut m.args {
                rename_in_expr(a, from, to);
            }
        }
        Expression::FieldAccess(f) => rename_in_expr(&mut f.object, from, to),
        Expression::Binary(b) => {
            rename_in_expr(&mut b.left, from, to);
            rename_in_expr(&mut b.right, from, to);
        }
        Expression::Unary(u) => rename_in_expr(&mut u.operand, from, to),
        Expression::StructLiteral(s) => {
            if s.name == from {
                s.name = to.to_string();
            }
            for e in &mut s.spreads {
                rename_in_expr(e, from, to);
            }
            for (_, v) in &mut s.fields {
                rename_in_expr(v, from, to);
            }
        }
        Expression::EnumVariant(ev) => {
            if ev.enum_name.as_deref() == Some(from) {
                ev.enum_name = Some(to.to_string());
            }
            for a in &mut ev.args {
                rename_in_expr(a, from, to);
            }
        }
        Expression::Match(m) => {
            rename_in_expr(&mut m.scrutinee, from, to);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    rename_in_expr(g, from, to);
                }
                rename_in_block(&mut arm.body, from, to);
            }
        }
        Expression::If(i) => {
            rename_in_expr(&mut i.condition, from, to);
            rename_in_block(&mut i.then_block, from, to);
            rename_in_block(&mut i.else_block, from, to);
        }
        Expression::Index(ix) => {
            rename_in_expr(&mut ix.object, from, to);
            rename_in_expr(&mut ix.index, from, to);
        }
        Expression::ArrayLiteral(a) => {
            for e in a.all_exprs_mut() {
                rename_in_expr(e, from, to);
            }
        }
        Expression::ArrayRepeat {
            element,
            count_expr,
            ..
        } => {
            rename_in_expr(element, from, to);
            if let Some(c) = count_expr {
                rename_in_expr(c, from, to);
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                rename_in_expr(e, from, to);
            }
        }
        Expression::Grouped(e) | Expression::Await(e) => rename_in_expr(e, from, to),
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    rename_in_expr(e, from, to);
                }
            }
        }
        Expression::Cast(c) => rename_in_expr(&mut c.expr, from, to),
        Expression::ArrowFn(a) => match &mut a.body {
            ArrowBody::Expr(e) => rename_in_expr(e, from, to),
            ArrowBody::Block(b) => rename_in_block(b, from, to),
        },
        Expression::ComptimeBlock { body, .. } => rename_in_block(body, from, to),
        Expression::Spawn { body, .. } => rename_in_block(body, from, to),
        Expression::ParallelSearch(p) => {
            p.map_exprs_mut(|e| rename_in_expr(e, from, to));
            rename_in_block(&mut p.body, from, to);
        }
        Expression::Literal(_) | Expression::Variable { .. } | Expression::Invalid => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn merge_skips_priv_functions_and_mangles_alias() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let helpers = root.join("tests/nyra/modules/helpers.ny");
        let other = crate::parse_file_only(&helpers).expect("parse helpers");
        let secret = other
            .functions
            .iter()
            .find(|f| f.name == "secret")
            .expect("secret fn");
        assert!(!secret.public, "priv fn should parse as public=false");
        let greet = other
            .functions
            .iter()
            .find(|f| f.name == "greet")
            .expect("greet fn");
        assert!(greet.public, "pub fn should parse as public=true");

        let mut target = Program::default();
        let errs = merge_program(&mut target, other, Some("h"), &[]);
        assert!(errs.is_empty(), "{errs:?}");
        let names: Vec<_> = target.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"h__greet"),
            "expected h__greet, got {names:?}"
        );
        assert!(
            !names.iter().any(|n| *n == "h__secret"),
            "priv fn must not merge: {names:?}"
        );
    }

    #[test]
    fn selective_merge_pulls_priv_closure() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let math = root.join("tests/nyra/modules/math.ny");
        let other = crate::parse_file_only(&math).expect("parse math");
        let mut target = Program::default();
        let sel = vec![ImportName {
            name: "twice".into(),
            rename: None,
            span: Span::default(),
        }];
        let errs = merge_program(&mut target, other, None, &sel);
        assert!(errs.is_empty(), "{errs:?}");
        let names: HashSet<_> = target.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("twice"), "{names:?}");
        assert!(names.contains("double"), "priv helper should merge: {names:?}");
        assert!(names.contains("mul"), "mul used by double: {names:?}");
        assert!(!names.contains("add"), "add not in closure: {names:?}");
        assert!(!names.contains("unused_export"), "unused pub skipped: {names:?}");
    }

    #[test]
    fn selective_merge_unknown_symbol_errors() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let helpers = root.join("tests/nyra/modules/helpers.ny");
        let other = crate::parse_file_only(&helpers).expect("parse helpers");
        let mut target = Program::default();
        let sel = vec![ImportName {
            name: "nope".into(),
            rename: None,
            span: Span::default(),
        }];
        let errs = merge_program(&mut target, other, None, &sel);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code.as_deref(), Some("E039"));
    }
}
