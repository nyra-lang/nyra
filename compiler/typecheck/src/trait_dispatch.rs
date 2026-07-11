//! Trait object dispatch signatures and static trait method resolution.

use ast::*;
use errors::Span;
use std::collections::{BTreeSet, HashSet};

use super::{FunctionSignature, TypeChecker, TypeEnv};
use super::diagnostics;
use types::Type;

impl TypeChecker {
    pub(super) fn register_trait_dispatch_sigs(&mut self, program: &Program) {
        let mut combos = BTreeSet::new();
        for trait_def in &program.traits {
            if trait_def.name == "Drop" || trait_def.name == "Clone" {
                continue;
            }
            combos.insert(vec![trait_def.name.clone()]);
        }
        for traits in collect_dyn_trait_combos(program) {
            if traits.len() > 1 {
                combos.insert(traits);
            }
        }
        for traits in combos {
            self.register_one_dyn_combo(program, &traits);
        }
    }

    fn register_one_dyn_combo(&mut self, program: &Program, traits: &[String]) {
        if traits.is_empty() {
            return;
        }
        let key = dyn_combo_key(traits);
        let dyn_name = dyn_struct_name(traits);
        let methods = combo_methods(program, traits);

        let mut types = BTreeSet::new();
        for ti in &program.trait_impls {
            if type_implements_all_traits(program, &ti.type_name, traits) {
                types.insert(ti.type_name.clone());
            }
        }
        for type_name in types {
            let box_fn = format!("{key}_dyn_{type_name}");
            self.env.functions.insert(
                box_fn,
                FunctionSignature {
                    params: vec![Type::Struct(type_name.clone())],
                    return_type: Type::Struct(dyn_name.clone()),
                },
            );
        }

        for method in &methods {
            let dispatch = format!("__dyn_{key}_{}", method.name);
            let mut params = vec![Type::Struct(dyn_name.clone())];
            for p in method.params.iter().skip(1) {
                params.push(self.type_from_ann(&p.ty));
            }
            let return_type = method
                .return_type
                .clone()
                .map(|a| self.type_from_ann(&a))
                .unwrap_or(Type::Void);
            self.env.functions.insert(
                dispatch,
                FunctionSignature {
                    params,
                    return_type,
                },
            );
        }

        self.env.functions.insert(
            format!("__dyn_{key}_drop"),
            FunctionSignature {
                params: vec![Type::Struct(dyn_name.clone())],
                return_type: Type::Void,
            },
        );
    }

    pub(super) fn known_trait_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.trait_methods.keys().cloned().collect();
        names.sort_by_key(|t| std::cmp::Reverse(t.len()));
        names
    }

    pub(super) fn dyn_traits_of(&self, dyn_struct: &str) -> Option<Vec<String>> {
        traits_from_dyn_struct(dyn_struct, &self.known_trait_names())
    }

    pub(super) fn dyn_combo_has_method(&self, traits: &[String], method: &str) -> bool {
        for trait_name in traits {
            if self.trait_has_method(trait_name, method) {
                return true;
            }
        }
        false
    }

    pub(super) fn trait_impl_exists(&self, trait_name: &str, type_name: &str) -> bool {
        self.trait_impl_pairs
            .iter()
            .any(|(ty, tr)| ty == type_name && tr == trait_name)
    }

    pub(super) fn dyn_trait_name(type_name: &str) -> Option<&str> {
        type_name.strip_prefix("Dyn_")
    }

    pub fn resolve_method_name(&self, type_name: &str, method: &str) -> String {
        if let Some(key) = Self::dyn_trait_name(type_name) {
            return format!("__dyn_{key}_{method}");
        }
        let plain = format!("{type_name}_{method}");
        if self.env.functions.contains_key(&plain) {
            return plain;
        }
        let suffix = format!("_{method}");
        for (concrete, trait_name) in &self.trait_impl_pairs {
            if concrete == type_name {
                let mangled = format!("{trait_name}_{type_name}_{method}");
                if self.env.functions.contains_key(&mangled) {
                    return mangled;
                }
                if mangled.ends_with(&suffix) && self.env.functions.contains_key(&mangled) {
                    return mangled;
                }
            }
        }
        plain
    }

    pub(super) fn trait_has_method(&self, trait_name: &str, method: &str) -> bool {
        self.trait_methods
            .get(trait_name)
            .is_some_and(|ms| ms.iter().any(|m| m.name == method))
    }

    /// Method call on a generic parameter with trait bounds (e.g. `x.hello()` when `T: Greet`).
    pub(super) fn check_generic_bound_method(
        &mut self,
        mc: &MethodCallExpr,
        type_param: &str,
        env: &mut TypeEnv,
        sp: &Span,
    ) -> Option<Type> {
        let bounds = self.current_type_param_bounds.get(type_param)?;
        for trait_name in bounds {
            if !self.trait_has_method(trait_name, &mc.method) {
                continue;
            }
            let Some(sig) = self
                .trait_methods
                .get(trait_name)
                .and_then(|methods| methods.iter().find(|m| m.name == mc.method))
                .cloned()
            else {
                continue;
            };
            let expected_args = sig.params.len().saturating_sub(1);
            if mc.args.len() != expected_args {
                diagnostics::wrong_arity(
                    self,
                    &format!("{}::{}", trait_name, mc.method),
                    expected_args,
                    mc.args.len(),
                    sp.clone(),
                );
            }
            for (arg, p) in mc.args.iter().zip(sig.params.iter().skip(1)) {
                let at = self.check_expr(arg, env);
                let expected = self.type_from_ann(&p.ty);
                if at != expected && at != Type::Unknown && expected != Type::Unknown {
                    diagnostics::method_arg_mismatch(self, &mc.method, sp.clone());
                }
            }
            return sig
                .return_type
                .map(|a| self.type_from_ann(&a))
                .or(Some(Type::Void));
        }
        None
    }
}

fn type_implements_all_traits(program: &Program, type_name: &str, traits: &[String]) -> bool {
    traits.iter().all(|t| {
        program
            .trait_impls
            .iter()
            .any(|ti| ti.trait_name == *t && ti.type_name == type_name)
    })
}

fn combo_methods(program: &Program, traits: &[String]) -> Vec<TraitMethodSig> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for trait_name in traits {
        let Some(trait_def) = program.traits.iter().find(|t| t.name == *trait_name) else {
            continue;
        };
        for method in &trait_def.methods {
            if seen.insert(method.name.clone()) {
                out.push(method.clone());
            }
        }
    }
    out
}

fn insert_combo(out: &mut BTreeSet<Vec<String>>, traits: Vec<String>) {
    let filtered: Vec<String> = traits
        .into_iter()
        .filter(|t| t != "Drop" && t != "Clone")
        .collect();
    if filtered.len() > 1 {
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
