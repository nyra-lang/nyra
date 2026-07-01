//! Statement and assignment checking.

use ast::*;
use ast::stmt_span;

use super::{TypeChecker, TypeEnv, VarInfo};
use super::diagnostics;
use super::helpers::types_assignable;
use types::{self, float_assignable, integer_assignable, integer_literal_fits, int_literal_value, Type};

impl TypeChecker {
    pub(super) fn check_block(&mut self, block: &Block, env: &mut TypeEnv, expected_ret: &Type) {
        for stmt in &block.statements {
            self.check_statement(stmt, env, expected_ret);
        }
    }

    /// Type of a block used as an expression: last `expr` stmt or `return` value.
    pub(super) fn check_block_expr_value(&mut self, block: &Block, env: &mut TypeEnv, span: &errors::Span) -> Type {
        let mut inner = TypeEnv {
            variables: env.variables.clone(),
            functions: env.functions.clone(),
        };
        let mut last_ty = Type::Unknown;
        for stmt in &block.statements {
            match stmt {
                Statement::Expression(e) => {
                    last_ty = self.check_expr(e, &mut inner);
                }
                Statement::If(i) if i.else_block.is_some() => {
                    let c = self.check_expr(&i.condition, &mut inner);
                    if c != Type::Bool && c != Type::Unknown {
                        diagnostics::bool_condition_required(self, "if expression", span.clone());
                    }
                    let t = self.check_block_expr_value(&i.then_block, &mut inner, span);
                    let e = self.check_block_expr_value(
                        i.else_block.as_ref().unwrap(),
                        &mut inner,
                        span,
                    );
                    if t != e && t != Type::Unknown && e != Type::Unknown {
                        diagnostics::branch_type_mismatch(self, span.clone());
                    }
                    last_ty = if t != Type::Unknown { t } else { e };
                }
                Statement::Return(r) => {
                    return if let Some(v) = &r.value {
                        self.check_expr(v, &mut inner)
                    } else {
                        Type::Void
                    };
                }
                _ => self.check_statement(stmt, &mut inner, &Type::Unknown),
            }
        }
        if last_ty == Type::Unknown {
            diagnostics::block_must_produce_value(self, span.clone());
        }
        last_ty
    }

    pub(super) fn check_statement(&mut self, stmt: &Statement, env: &mut TypeEnv, expected_ret: &Type) {
        let sp = stmt_span(stmt);
        match stmt {
            Statement::Let(l) => {
                let errors_before = self.errors.len();
                let mut value_ty = self.check_expr(&l.value, env);
                if let Expression::Call(c) = &l.value {
                    if let Some(sig) = env.functions.get(&c.callee) {
                        if sig.return_type != Type::Unknown && sig.return_type != Type::Void {
                            value_ty = sig.return_type.clone();
                        }
                    }
                } else if value_ty == Type::Unknown {
                    if let Some(hint) = self.infer_expr_type_hint(&l.value, env) {
                        value_ty = hint;
                    }
                }
                let had_error_in_value = self.errors.len() > errors_before;
                if !l.destructure.is_empty() {
                    if l.mutable {
                        diagnostics::destructure_not_mutable(self, sp.clone());
                    }
                    match &value_ty {
                        Type::Tuple { elems } => {
                            if l.destructure.len() != elems.len() {
                                diagnostics::destructure_length_mismatch(self, sp.clone());
                            } else {
                                for (name, ty) in l.destructure.iter().zip(elems.iter()) {
                                    env.variables.insert(
                                        name.clone(),
                                        VarInfo {
                                            ty: ty.clone(),
                                            mutable: false,
                                        },
                                    );
                                }
                            }
                        }
                        _ => {
                            diagnostics::destructure_requires_tuple(self, sp.clone());
                        }
                    }
                    return;
                }
                let declared = l.ty.clone().map(|a| self.type_from_ann(&a));
                let var_ty = if let Some(dt) = declared {
                    if value_ty != Type::Unknown
                        && dt != value_ty
                        && !integer_assignable(&dt, &value_ty)
                        && !float_assignable(&dt, &value_ty)
                    {
                        diagnostics::type_mismatch_var(
                            self,
                            &l.name,
                            &diagnostics::type_pretty(&dt),
                            &diagnostics::type_pretty(&value_ty),
                            sp.clone(),
                        );
                    } else if let Some(n) = int_literal_value(&l.value) {
                        if !integer_literal_fits(&dt, n) {
                            diagnostics::integer_out_of_range(self, n, &dt, sp.clone());
                        }
                    }
                    dt
                } else {
                    if value_ty == Type::Unknown
                        && !matches!(l.value, Expression::Invalid)
                        && !had_error_in_value
                    {
                        diagnostics::cannot_infer(self, &l.name, sp.clone());
                    }
                    if l.ty.is_none() && value_ty != Type::Unknown {
                        self.inferred_bindings.push(super::InferredBinding {
                            name: l.name.clone(),
                            span: l.span.clone(),
                            ty: value_ty.clone(),
                        });
                    }
                    value_ty
                };
                if l.mutable {
                    env.variables.insert(
                        l.name.clone(),
                        VarInfo {
                            ty: var_ty,
                            mutable: true,
                        },
                    );
                } else {
                    env.variables.insert(
                        l.name.clone(),
                        VarInfo {
                            ty: var_ty,
                            mutable: false,
                        },
                    );
                }
            }
            Statement::Assign(a) => {
                let value_ty = self.check_expr(&a.value, env);
                self.check_assign_target(&a.target, &value_ty, env, sp.clone());
            }
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    let ty = self.check_expr(v, env);
                    if ty != Type::Unknown
                        && *expected_ret != Type::Void
                        && *expected_ret != Type::Generic("_".into())
                        && !types_assignable(&ty, expected_ret)
                    {
                        diagnostics::return_type_mismatch(self, expected_ret, &ty, sp.clone());
                    }
                }
            }
            Statement::If(i) => {
                let cond = self.check_expr(&i.condition, env);
                if cond != Type::Bool && cond != Type::Unknown {
                    diagnostics::bool_condition_required(self, "if", sp.clone());
                }
                let base = env.clone();
                let mut then_env = base.clone();
                self.check_block(&i.then_block, &mut then_env, expected_ret);
                if let Some(else_b) = &i.else_block {
                    let mut else_env = base.clone();
                    self.check_block(else_b, &mut else_env, expected_ret);
                    *env = super::helpers::merge_if_branch_envs(&base, &then_env, &else_env);
                } else {
                    *env = then_env;
                }
            }
            Statement::While(w) => {
                let cond = self.check_expr(&w.condition, env);
                if cond != Type::Bool && cond != Type::Unknown {
                    diagnostics::bool_condition_required(self, "while", sp.clone());
                }
                self.loop_depth += 1;
                self.check_block(&w.body, env, expected_ret);
                self.loop_depth -= 1;
            }
            Statement::For(f) => {
                let outer_before: std::collections::HashSet<String> =
                    env.variables.keys().cloned().collect();
                match &f.kind {
                    ForKind::Range { start, end } => {
                        let start_ty = self.check_expr(start, env);
                        let end_ty = self.check_expr(end, env);
                        if !types::is_integer(&start_ty) && start_ty != Type::Unknown {
                            diagnostics::for_range_requires_integer(self, "start", sp.clone());
                        }
                        if !types::is_integer(&end_ty) && end_ty != Type::Unknown {
                            diagnostics::for_range_requires_integer(self, "end", sp.clone());
                        }
                        env.variables.insert(
                            f.var.clone(),
                            VarInfo {
                                ty: Type::Integer(ast::IntKind::I32),  // loop index
                                mutable: true,
                            },
                        );
                    }
                    ForKind::Iterable { iterable } => {
                        let iter_ty = self.check_expr(iterable, env);
                        let elem_ty = match &iter_ty {
                            Type::Array { elem, len: Some(_) } => elem.as_ref().clone(),
                            Type::Array { .. } => {
                                diagnostics::for_in_requires_fixed_array(self, sp.clone());
                                Type::Unknown
                            }
                            Type::String => Type::Char,
                            Type::VecStr => Type::String,
                            _ => {
                                diagnostics::for_in_requires_iterable(self, &iter_ty, sp.clone());
                                Type::Unknown
                            }
                        };
                        env.variables.insert(
                            f.var.clone(),
                            VarInfo {
                                ty: elem_ty,
                                mutable: true,
                            },
                        );
                    }
                }
                if f.parallel.is_some() && f.progress.is_some() {
                    diagnostics::for_parallel_progress_conflict(self, sp.clone());
                }
                if let Some(cfg) = &f.progress {
                    if let Some(label) = &cfg.label {
                        let ty = self.check_expr(label, env);
                        if ty != Type::String && ty != Type::Unknown {
                            diagnostics::progress_label_must_be_string(self, sp.clone());
                        }
                    }
                }
                if let Some(cfg) = &f.parallel {
                    ownership::check_parallel_for_body(
                        &f.body,
                        &f.var,
                        &outer_before,
                        sp.clone(),
                        &mut self.errors,
                    );
                    self.check_parallel_config(cfg, sp.clone(), env);
                }
                self.loop_depth += 1;
                self.check_block(&f.body, env, expected_ret);
                self.loop_depth -= 1;
            }
            Statement::Break { .. } => {
                if self.loop_depth == 0 {
                    diagnostics::break_outside_loop(self, sp.clone());
                }
            }
            Statement::Continue { .. } => {
                if self.loop_depth == 0 {
                    diagnostics::continue_outside_loop(self, sp.clone());
                }
            }
            Statement::Const(c) => {
                let value_ty = self.check_expr(&c.value, env);
                let declared = c.ty.clone().map(Type::from);
                let var_ty = declared.unwrap_or(value_ty);
                env.variables.insert(
                    c.name.clone(),
                    VarInfo {
                        ty: var_ty,
                        mutable: false,
                    },
                );
            }
            Statement::Import(_) => {}
            Statement::Print(p) => {
                if self.no_std {
                    diagnostics::no_std_unavailable(self, "print", sp.clone());
                }
                for arg in &p.args {
                    self.check_io_arg(arg, env, sp.clone(), "print");
                }
                if let Some(color) = &p.color {
                    self.check_print_color(color, env, sp.clone());
                }
            }
            Statement::Expression(expr) => {
                self.check_expr(expr, env);
            }
            Statement::Defer(expr) => {
                self.check_expr(expr, env);
            }
            Statement::Benchmark(body) => {
                self.check_block(body, env, expected_ret);
            }
            Statement::Spawn(spawn) => {
                if self.no_std {
                    diagnostics::no_std_unavailable(self, "spawn", sp.clone());
                }
                if self.target_is_wasm() {
                    diagnostics::platform_unavailable(self, "spawn", "wasm32", sp.clone());
                }
                self.check_block(&spawn.body, env, &Type::Void);
            }
            Statement::Unsafe(body) => {
                self.unsafe_depth += 1;
                self.check_block(body, env, expected_ret);
                self.unsafe_depth -= 1;
            }
            Statement::Asm { span, .. } => {
                if !self.in_unsafe() {
                    diagnostics::unsafe_required(self, "inline asm", span.clone());
                }
            }
        }
    }

    pub(super) fn check_assign_target(
        &mut self,
        target: &Expression,
        value_ty: &Type,
        env: &mut TypeEnv,
        sp: errors::Span,
    ) {
        match target {
            Expression::Variable { name, .. } => match env.variables.get(name) {
                Some(info) if info.mutable => {
                    if *value_ty != Type::Unknown
                        && info.ty != Type::Unknown
                        && *value_ty != info.ty
                    {
                        diagnostics::type_mismatch(
                            self,
                            &format!("assigning to `{name}`"),
                            &info.ty,
                            value_ty,
                            sp.clone(),
                        );
                    }
                }
                Some(_) => {
                    diagnostics::immutable_assign(self, name, sp);
                }
                None => {
                    diagnostics::undefined_name(self, name, sp, env);
                }
            },
            Expression::Unary(u) if u.op == UnaryOp::Deref => {
                if !self.in_unsafe() {
                    diagnostics::deref_store_requires_unsafe(self, sp.clone());
                }
                let ptr_ty = self.check_expr(&u.operand, env);
                let pointee = match ptr_ty {
                    Type::RawPtr { inner } => *inner,
                    Type::Ref { inner, mutable: true, .. } if self.in_unsafe() => *inner,
                    Type::Ptr if self.in_unsafe() => Type::Integer(ast::IntKind::I32),
                    _ => {
                        diagnostics::deref_store_invalid_target(self, sp);
                        return;
                    }
                };
                if *value_ty != Type::Unknown
                    && pointee != Type::Unknown
                    && *value_ty != pointee
                {
                    diagnostics::type_mismatch(
                        self,
                        "in pointer store",
                        &pointee,
                        value_ty,
                        sp,
                    );
                }
            }
            Expression::FieldAccess(fa) => {
                let obj_ty = self.check_expr(&fa.object, env);
                if let Type::Struct(name) = obj_ty {
                    if let Some(info) = self.structs.get(&name) {
                        if let Some(field_ty) = info.fields.get(&fa.field).cloned() {
                            if *value_ty != Type::Unknown
                                && *value_ty != field_ty
                                && field_ty != Type::Unknown
                            {
                                diagnostics::type_mismatch(
                                    self,
                                    &format!("for field `{}`", fa.field),
                                    &field_ty,
                                    value_ty,
                                    sp,
                                );
                            }
                        }
                    }
                } else {
                    diagnostics::field_assign_requires_struct(self, sp);
                }
            }
            Expression::Index(ix) => {
                let obj_ty = self.check_expr(&ix.object, env);
                let _ = self.check_expr(&ix.index, env);
                if let Type::Array { elem, .. } = obj_ty {
                    if *value_ty != Type::Unknown
                        && *value_ty != *elem
                        && *elem != Type::Unknown
                    {
                        diagnostics::type_mismatch(
                            self,
                            "for array element",
                            &elem,
                            value_ty,
                            sp,
                        );
                    }
                } else {
                    diagnostics::index_assign_requires_array(self, sp);
                }
            }
            _ => {
                diagnostics::invalid_assign_target(self, sp);
            }
        }
    }

    pub(super) fn check_parallel_config(
        &mut self,
        cfg: &ParallelConfig,
        sp: errors::Span,
        env: &mut TypeEnv,
    ) {
        let expr = match &cfg.threads {
            ParallelThreads::Auto => return,
            ParallelThreads::Max(e) => (e, "max"),
            ParallelThreads::Exact(e) => (e, "threads"),
            ParallelThreads::CpuPercent(e) => (e, "cpu percent"),
        };
        let ty = self.check_expr(expr.0, env);
        if !types::is_integer(&ty) && ty != Type::Unknown {
            diagnostics::parallel_threads_must_be_integer(self, expr.1, &ty, sp);
        }
    }
}
