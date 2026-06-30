//! Statement and assignment checking.

use ast::*;
use ast::stmt_span;
use errors::{ErrorKind, NyraError, Span};

use super::{TypeChecker, TypeEnv, VarInfo};
use super::diagnostics;
use types::{self, float_assignable, integer_assignable, integer_literal_fits, int_literal_value, Type};

impl TypeChecker {
    pub(super) fn check_block(&mut self, block: &Block, env: &mut TypeEnv, expected_ret: &Type) {
        for stmt in &block.statements {
            self.check_statement(stmt, env, expected_ret);
        }
    }

    /// Type of a block used as an expression: last `expr` stmt or `return` value.
    pub(super) fn check_block_expr_value(&mut self, block: &Block, env: &mut TypeEnv, span: &Span) -> Type {
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
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                span.clone(),
                "Block must produce a value (use a trailing expression or `return`)",
            ));
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
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            "Destructuring let cannot be mutable",
                        ));
                    }
                    match &value_ty {
                        Type::Tuple { elems } => {
                            if l.destructure.len() != elems.len() {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    "Destructure pattern length must match tuple length",
                                ));
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
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Destructure requires tuple value",
                            ));
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
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!(
                                    "Integer literal {n} is out of range for type {}",
                                    diagnostics::type_pretty(&dt),
                                ),
                            ));
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
                        && ty != *expected_ret
                        && *expected_ret != Type::Void
                        && *expected_ret != Type::Generic("_".into())
                    {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!(
                                "Return type mismatch: expected {}, got {}",
                                diagnostics::type_pretty(expected_ret),
                                diagnostics::type_pretty(&ty),
                            ),
                        ));
                    }
                }
            }
            Statement::If(i) => {
                let cond = self.check_expr(&i.condition, env);
                if cond != Type::Bool && cond != Type::Unknown {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "If condition must be bool",
                    ));
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "While condition must be bool",
                    ));
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
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "For range start must be i32",
                            ));
                        }
                        if !types::is_integer(&end_ty) && end_ty != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "For range end must be i32",
                            ));
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    "For-in requires a fixed-size array",
                                ));
                                Type::Unknown
                            }
                            Type::String => Type::Char,
                            Type::VecStr => Type::String,
                            _ => {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!(
                                        "For-in requires array, string, or split result, got {:?}",
                                        iter_ty
                                    ),
                                ));
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "`parallel for` and `progress for` cannot be combined",
                    ));
                }
                if let Some(cfg) = &f.progress {
                    if let Some(label) = &cfg.label {
                        let ty = self.check_expr(label, env);
                        if ty != Type::String && ty != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "progress label must be string",
                            ));
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "`break` is only valid inside `while` or `for`",
                    ));
                }
            }
            Statement::Continue { .. } => {
                if self.loop_depth == 0 {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "`continue` is only valid inside `while` or `for`",
                    ));
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "print is not available in no_std programs (use extern I/O or UART)",
                    ));
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
            Statement::Spawn(body) => {
                if self.no_std {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "spawn is not available in no_std programs",
                    ));
                }
                if self.target_is_wasm() {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        Span::default(),
                        "spawn is not available on wasm32 targets",
                    ));
                }
                self.check_block(body, env, &Type::Void);
            }
            Statement::Unsafe(body) => {
                self.unsafe_depth += 1;
                self.check_block(body, env, expected_ret);
                self.unsafe_depth -= 1;
            }
            Statement::Asm { span, .. } => {
                if !self.in_unsafe() {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        span.clone(),
                        "inline asm requires an unsafe block",
                    ));
                }
            }
        }
    }

    pub(super) fn check_assign_target(
        &mut self,
        target: &Expression,
        value_ty: &Type,
        env: &mut TypeEnv,
        sp: Span,
    ) {
        match target {
            Expression::Variable { name, .. } => match env.variables.get(name) {
                Some(info) if info.mutable => {
                    if *value_ty != Type::Unknown
                        && info.ty != Type::Unknown
                        && *value_ty != info.ty
                    {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!(
                                "Type mismatch assigning to '{name}': expected {:?}, got {:?}",
                                info.ty, value_ty
                            ),
                        ));
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "Writing through raw pointer requires unsafe",
                    ));
                }
                let ptr_ty = self.check_expr(&u.operand, env);
                let pointee = match ptr_ty {
                    Type::RawPtr { inner } => *inner,
                    Type::Ref { inner, mutable: true, .. } if self.in_unsafe() => *inner,
                    Type::Ptr if self.in_unsafe() => Type::Integer(ast::IntKind::I32),
                    _ => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp,
                            "Assignment through deref requires *T, ptr, or &mut T in unsafe",
                        ));
                        return;
                    }
                };
                if *value_ty != Type::Unknown
                    && pointee != Type::Unknown
                    && *value_ty != pointee
                {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp,
                        format!(
                            "Type mismatch in pointer store: expected {:?}, got {:?}",
                            pointee, value_ty
                        ),
                    ));
                }
            }
            Expression::FieldAccess(fa) => {
                let obj_ty = self.check_expr(&fa.object, env);
                if let Type::Struct(name) = obj_ty {
                    if let Some(info) = self.structs.get(&name) {
                        if let Some(field_ty) = info.fields.get(&fa.field) {
                            if *value_ty != Type::Unknown
                                && *value_ty != *field_ty
                                && *field_ty != Type::Unknown
                            {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp,
                                    format!(
                                        "Field '{}' type mismatch: expected {:?}, got {:?}",
                                        fa.field, field_ty, value_ty
                                    ),
                                ));
                            }
                        }
                    }
                } else {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp,
                        "Field assignment requires struct receiver",
                    ));
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
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp,
                            format!(
                                "Array element type mismatch: expected {:?}, got {:?}",
                                elem, value_ty
                            ),
                        ));
                    }
                } else {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp,
                        "Index assignment requires array value",
                    ));
                }
            }
            _ => {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    sp,
                    "Invalid assignment target",
                ));
            }
        }
    }

    pub(super) fn check_parallel_config(
        &mut self,
        cfg: &ParallelConfig,
        sp: Span,
        env: &mut TypeEnv,
    ) {
        let expr = match &cfg.threads {
            ParallelThreads::Auto => return,
            ParallelThreads::Max(e) => (e, "max_threads"),
            ParallelThreads::Exact(e) => (e, "threads"),
            ParallelThreads::CpuPercent(e) => (e, "cpu percent"),
        };
        let ty = self.check_expr(expr.0, env);
        if !types::is_integer(&ty) && ty != Type::Unknown {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                sp,
                format!("parallel {} must be i32, got {ty:?}", expr.1),
            ));
        }
    }
}

