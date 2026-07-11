//! Trait object vtables, boxing, and dynamic dispatch (LLVM).
use std::collections::{BTreeSet, HashSet};
use std::fmt::Write;

use ast::*;

use super::Codegen;
use super::util::{llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_value_operand, struct_name_from_llvm_ty};

struct ComboMethod {
    trait_name: String,
    method: TraitMethodSig,
}

impl Codegen {
    pub(super) fn emit_trait_object_infrastructure(&mut self, program: &Program) {
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

        for traits in &combos {
            let dyn_ty = dyn_struct_name(traits);
            if !self.struct_fields.contains_key(&dyn_ty) {
                continue;
            }
            let key = dyn_combo_key(traits);
            let methods = combo_methods(program, traits);
            let mut types = BTreeSet::new();
            for ti in &program.trait_impls {
                if type_implements_all_traits(program, &ti.type_name, traits) {
                    types.insert(ti.type_name.clone());
                }
            }
            for type_name in types {
                self.emit_vtable_and_box(traits, &key, &methods, &type_name);
            }
            for method in &methods {
                self.emit_dyn_dispatch_fn(traits, &key, &methods, method);
            }
            self.emit_dyn_drop_fn(traits, &key, methods.len());
        }
    }

    fn emit_vtable_and_box(
        &mut self,
        traits: &[String],
        key: &str,
        methods: &[ComboMethod],
        type_name: &str,
    ) {
        let dyn_ty = dyn_struct_name(traits);
        let struct_ty = format!("%{type_name}");
        let size = self.struct_byte_size(type_name);
        if size <= 0 {
            return;
        }

        let mut vtable_entries = Vec::new();
        for cm in methods {
            let static_fn = format!("{}_{type_name}_{}", cm.trait_name, cm.method.name);
            let thunk = format!("{key}_dynthunk_{type_name}_{}", cm.method.name);
            self.emit_method_thunk(&thunk, type_name, &static_fn, &cm.method);
            vtable_entries.push(format!("ptr @{thunk}"));
        }
        let drop_thunk = format!("{key}_dynthunk_drop_{type_name}");
        self.emit_drop_thunk(&drop_thunk, type_name, &struct_ty);
        vtable_entries.push(format!("ptr @{drop_thunk}"));

        let vtable_name = format!("vtable_{key}_{type_name}");
        self.emit_lines(&format!(
            "@{vtable_name} = constant [{} x ptr] [{}]",
            vtable_entries.len(),
            vtable_entries.join(", ")
        ));

        let box_fn = format!("{key}_dyn_{type_name}");
        let mut body = String::new();
        writeln!(
            body,
            "define %{dyn_ty} @{box_fn}({struct_ty}* %val) {{"
        )
        .unwrap();
        writeln!(body, "entry:").unwrap();
        writeln!(
            body,
            "  %heap = call ptr @malloc(i64 {size})"
        )
        .unwrap();
        self.needs_malloc_decl = true;
        writeln!(
            body,
            "  call void @llvm.memcpy.p0.p0.i64(ptr %heap, ptr %val, i64 {size}, i1 false)"
        )
        .unwrap();
        writeln!(
            body,
            "  %dyn = insertvalue %{dyn_ty} undef, ptr %heap, 0"
        )
        .unwrap();
        writeln!(
            body,
            "  %dyn1 = insertvalue %{dyn_ty} %dyn, ptr @{vtable_name}, 1"
        )
        .unwrap();
        writeln!(body, "  ret %{dyn_ty} %dyn1").unwrap();
        writeln!(body, "}}").unwrap();
        self.emit_lines(&body);
        self.emit("");

        let ret = format!("%{dyn_ty}");
        self.call_returns.insert(box_fn.clone(), ret.clone());
    }

    fn emit_method_thunk(
        &mut self,
        thunk_name: &str,
        type_name: &str,
        static_fn: &str,
        method: &TraitMethodSig,
    ) {
        let struct_ty = format!("%{type_name}");
        let param_tys: Vec<String> = method
            .params
            .iter()
            .skip(1)
            .map(|p| llvm_type_ann_resolved(&p.ty, &self.struct_fields, &self.enum_names))
            .collect();
        let ret_ty = method
            .return_type
            .as_ref()
            .map(|t| llvm_type_ann_resolved(t, &self.struct_fields, &self.enum_names))
            .unwrap_or_else(|| "void".into());

        let mut sig_params = Vec::new();
        for (i, ty) in param_tys.iter().enumerate() {
            sig_params.push(format!("{ty} %arg{i}"));
        }
        let call_args = {
            let mut args = vec![format!("{struct_ty}* %data")];
            for i in 0..param_tys.len() {
                args.push(format!("{} %arg{i}", param_tys[i]));
            }
            args.join(", ")
        };

        let mut body = String::new();
        writeln!(
            body,
            "define {ret_ty} @{thunk_name}(ptr %data{}) {{",
            if sig_params.is_empty() {
                String::new()
            } else {
                format!(", {}", sig_params.join(", "))
            }
        )
        .unwrap();
        writeln!(body, "entry:").unwrap();
        if ret_ty == "void" {
            writeln!(body, "  call void @{static_fn}({call_args})").unwrap();
            writeln!(body, "  ret void").unwrap();
        } else {
            writeln!(
                body,
                "  %r = call {ret_ty} @{static_fn}({call_args})"
            )
            .unwrap();
            writeln!(body, "  ret {ret_ty} %r").unwrap();
        }
        writeln!(body, "}}").unwrap();
        self.emit_lines(&body);
        self.emit("");
    }

    fn emit_drop_thunk(&mut self, thunk_name: &str, type_name: &str, struct_ty: &str) {
        let mut body = String::new();
        writeln!(
            body,
            "define void @{thunk_name}(ptr %data) {{"
        )
        .unwrap();
        writeln!(body, "entry:").unwrap();
        if self.drop_plan.custom_drop_fns.contains_key(type_name) {
            let drop_fn = self
                .drop_plan
                .custom_drop_fns
                .get(type_name)
                .cloned()
                .unwrap_or_else(|| format!("Drop_{type_name}_drop"));
            writeln!(
                body,
                "  call void @{drop_fn}({struct_ty}* %data)"
            )
            .unwrap();
        }
        writeln!(body, "  call void @free(ptr %data)").unwrap();
        self.needs_malloc_decl = true;
        writeln!(body, "  ret void").unwrap();
        writeln!(body, "}}").unwrap();
        self.emit_lines(&body);
        self.emit("");
    }

    fn emit_dyn_drop_fn(&mut self, traits: &[String], key: &str, method_count: usize) {
        let dyn_ty = dyn_struct_name(traits);
        let fn_name = format!("__dyn_{key}_drop");
        let drop_index = method_count as i32;

        let mut body = String::new();
        writeln!(
            body,
            "define void @{fn_name}(%{dyn_ty}* %obj) {{"
        )
        .unwrap();
        writeln!(body, "entry:").unwrap();
        writeln!(
            body,
            "  %data_ptr = getelementptr inbounds %{dyn_ty}, %{dyn_ty}* %obj, i32 0, i32 0"
        )
        .unwrap();
        writeln!(body, "  %data = load ptr, ptr %data_ptr").unwrap();
        writeln!(
            body,
            "  %vt_ptr = getelementptr inbounds %{dyn_ty}, %{dyn_ty}* %obj, i32 0, i32 1"
        )
        .unwrap();
        writeln!(body, "  %vt = load ptr, ptr %vt_ptr").unwrap();
        writeln!(
            body,
            "  %drop_ptr = getelementptr ptr, ptr %vt, i32 {drop_index}"
        )
        .unwrap();
        writeln!(
            body,
            "  %drop_fn = load void (ptr)*, void (ptr)** %drop_ptr"
        )
        .unwrap();
        writeln!(body, "  call void %drop_fn(ptr %data)").unwrap();
        writeln!(body, "  ret void").unwrap();
        writeln!(body, "}}").unwrap();
        self.emit_lines(&body);
        self.emit("");
    }

    fn emit_dyn_dispatch_fn(
        &mut self,
        traits: &[String],
        key: &str,
        methods: &[ComboMethod],
        target: &ComboMethod,
    ) {
        let dyn_ty = dyn_struct_name(traits);
        let fn_name = format!("__dyn_{key}_{}", target.method.name);
        let method_index = methods
            .iter()
            .position(|m| m.method.name == target.method.name)
            .unwrap_or(0) as i32;
        let param_tys: Vec<String> = target
            .method
            .params
            .iter()
            .skip(1)
            .map(|p| llvm_type_ann_resolved(&p.ty, &self.struct_fields, &self.enum_names))
            .collect();
        let ret_ty = target
            .method
            .return_type
            .as_ref()
            .map(|t| llvm_type_ann_resolved(t, &self.struct_fields, &self.enum_names))
            .unwrap_or_else(|| "void".into());

        let mut sig_params = vec![format!("%{dyn_ty}* %obj")];
        for (i, ty) in param_tys.iter().enumerate() {
            sig_params.push(format!("{ty} %arg{i}"));
        }

        let mut fn_ptr_sig = format!("{ret_ty} (ptr");
        for ty in &param_tys {
            write!(fn_ptr_sig, ", {ty}").unwrap();
        }
        fn_ptr_sig.push(')');

        let mut call_args = vec!["ptr %data".to_string()];
        for i in 0..param_tys.len() {
            call_args.push(format!("{} %arg{i}", param_tys[i]));
        }

        let mut body = String::new();
        writeln!(
            body,
            "define {ret_ty} @{fn_name}({}) {{",
            sig_params.join(", ")
        )
        .unwrap();
        writeln!(body, "entry:").unwrap();
        writeln!(
            body,
            "  %data_ptr = getelementptr inbounds %{dyn_ty}, %{dyn_ty}* %obj, i32 0, i32 0"
        )
        .unwrap();
        writeln!(body, "  %data = load ptr, ptr %data_ptr").unwrap();
        writeln!(
            body,
            "  %vt_ptr = getelementptr inbounds %{dyn_ty}, %{dyn_ty}* %obj, i32 0, i32 1"
        )
        .unwrap();
        writeln!(body, "  %vt = load ptr, ptr %vt_ptr").unwrap();
        writeln!(
            body,
            "  %fn_ptr = getelementptr ptr, ptr %vt, i32 {method_index}"
        )
        .unwrap();
        writeln!(body, "  %fn = load {fn_ptr_sig}*, {fn_ptr_sig}** %fn_ptr").unwrap();
        if ret_ty == "void" {
            writeln!(
                body,
                "  call void %fn({})",
                call_args.join(", ")
            )
            .unwrap();
            writeln!(body, "  ret void").unwrap();
        } else {
            writeln!(
                body,
                "  %r = call {ret_ty} %fn({})",
                call_args.join(", ")
            )
            .unwrap();
            writeln!(body, "  ret {ret_ty} %r").unwrap();
        }
        writeln!(body, "}}").unwrap();
        self.emit_lines(&body);
        self.emit("");
        self.call_returns.insert(fn_name, ret_ty);
    }

    fn emit_lines(&mut self, text: &str) {
        for line in text.lines() {
            self.emit(line);
        }
    }

    pub(super) fn compile_trait_object_box(
        &mut self,
        traits: &[String],
        expr: &Expression,
        env: &super::Env,
    ) -> super::ExprValue {
        let inner = self.compile_expr(expr, env);
        let concrete = struct_name_from_llvm_ty(&inner.ty)
            .or_else(|| {
                if inner.ty.starts_with('%') && !inner.ty.ends_with('*') {
                    Some(inner.ty.trim_start_matches('%').to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "Unknown".into());
        let key = dyn_combo_key(traits);
        let box_fn = format!("{key}_dyn_{concrete}");
        let dyn_ty = format!("%{}", dyn_struct_name(traits));
        let struct_ptr = if inner.ty.ends_with('*') {
            llvm_value_operand(&self.materialize_ptr_reg(&inner.reg))
        } else {
            let slot = self.materialize_struct_ssa_slot(&inner);
            format!("%{slot}")
        };
        let struct_ty = format!("%{concrete}");
        let reg = self.fresh("dynbox");
        self.emit(&format!(
            "  %{reg} = call {dyn_ty} @{box_fn}({struct_ty}* {struct_ptr})"
        ));
        super::ExprValue {
            reg: format!("%{reg}"),
            ty: dyn_ty,
        }
    }

    fn struct_byte_size(&self, name: &str) -> i64 {
        let Some(fields) = self.struct_fields.get(name) else {
            return 0;
        };
        let llvm_fields: Vec<String> = fields
            .iter()
            .map(|(_, ty)| llvm_type_ann_resolved(ty, &self.struct_fields, &self.enum_names))
            .collect();
        llvm_struct_size_bytes(&llvm_fields)
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

fn combo_methods(program: &Program, traits: &[String]) -> Vec<ComboMethod> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for trait_name in traits {
        let Some(trait_def) = program.traits.iter().find(|t| t.name == *trait_name) else {
            continue;
        };
        for method in &trait_def.methods {
            if seen.insert(method.name.clone()) {
                out.push(ComboMethod {
                    trait_name: trait_name.clone(),
                    method: method.clone(),
                });
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
