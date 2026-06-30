//! Per-function checking and arrow-fn return/param inference.

use ast::*;
use errors::{ErrorKind, NyraError};

use super::helpers::{block_has_return, collect_return_types_from_block, unify_return_types};
use super::{FunctionSignature, TypeChecker, TypeEnv, VarInfo};
use types::{self, Type};

impl TypeChecker {
    pub(super) fn function_return_type(&mut self, func: &Function) -> Type {
        let param_anns = self.resolve_inferred_param_anns(func);
        let param_types: Vec<Type> = param_anns.iter().map(|a| self.type_from_ann(a)).collect();
        self.function_return_type_with_param_types(func, &param_types)
    }

    pub(super) fn function_return_type_with_params(&mut self, func: &Function, param_types: &[Type]) -> Type {
        self.function_return_type_with_param_types(func, param_types)
    }

    fn function_return_type_with_param_types(&mut self, func: &Function, param_types: &[Type]) -> Type {
        if func.is_async {
            if func.exported {
                return Type::Handle;
            }
            if let Some(ann) = &func.return_type {
                if let Some(name) = super::future_types::future_struct_from_ann(ann) {
                    return Type::Struct(name);
                }
            }
            return Type::Struct("Future_i32".into());
        }
        if let Some(t) = &func.return_type {
            return self.type_from_ann(t);
        }
        if func.name.starts_with("__arrow_") {
            if let Some(ty) = self.infer_hoisted_arrow_return(func) {
                return ty;
            }
        }
        if func.name == "main" {
            if let Some(t) = &func.return_type {
                return self.type_from_ann(t);
            }
            return Type::Void;
        }
        if func.is_test || func.name.ends_with("_drop") {
            return Type::Void;
        }
        let mut local = TypeEnv {
            variables: self.env.variables.clone(),
            functions: self.env.functions.clone(),
        };
        for (p, ty) in func.params.iter().zip(param_types.iter()) {
            local.variables.insert(
                p.name.clone(),
                VarInfo {
                    ty: ty.clone(),
                    mutable: p.mutable,
                },
            );
        }
        let mut returns = Vec::new();
        collect_return_types_from_block(&func.body, self, &mut local, &mut returns);
        if let Some(ctor_ty) = self.infer_return_from_ctor_binding(&func.body, "StrVec_new") {
            if let Some(ty) = unify_return_types(&returns) {
                if ty == Type::Unknown || ty == Type::String || ty == Type::VecStr {
                    return ctor_ty;
                }
            } else {
                return ctor_ty;
            }
        }
        if let Some(ty) = unify_return_types(&returns) {
            if ty != Type::Unknown {
                return ty;
            }
        }
        if let Some(ty) = self.infer_return_from_param_field_access(func, param_types) {
            return ty;
        }
        if let Some(ty) = Self::infer_return_from_struct_literal(&func.body) {
            return ty;
        }
        if let Some(ty) = unify_return_types(&returns) {
            return ty;
        }
        if !returns.is_empty() {
            self.errors.push(
                NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!(
                        "Function '{}' has incompatible return types; add an explicit return type",
                        func.name
                    ),
                )
                .note("Example: `fn run() -> i32 { return 1 }`"),
            );
            return Type::Unknown;
        }
        Type::Void
    }

    fn infer_return_from_ctor_binding(&self, body: &Block, ctor: &str) -> Option<Type> {
        let mut ctor_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
        Self::collect_ctor_binding_vars(body, ctor, &mut ctor_vars);
        if ctor_vars.is_empty() {
            return None;
        }
        if Self::block_returns_bound_var(body, &ctor_vars) {
            if let Some(sig) = self.env.functions.get(ctor) {
                if sig.return_type != Type::Void && sig.return_type != Type::Unknown {
                    return Some(sig.return_type.clone());
                }
            }
            if ctor == "StrVec_new" && self.structs.contains_key("StrVec") {
                return Some(Type::Struct("StrVec".into()));
            }
        }
        None
    }

    fn infer_return_from_param_field_access(
        &self,
        func: &Function,
        param_types: &[Type],
    ) -> Option<Type> {
        let first = func.params.first()?;
        let Type::Struct(struct_name) = param_types.first()? else {
            return None;
        };
        let info = self.structs.get(struct_name)?;
        let mut fields = Vec::new();
        Self::collect_param_field_return_types(&func.body, &first.name, info, &mut fields);
        unify_return_types(&fields)
    }

    fn collect_param_field_return_types(
        block: &Block,
        param: &str,
        info: &types::StructInfo,
        out: &mut Vec<Type>,
    ) {
        for stmt in &block.statements {
            match stmt {
                Statement::Return(r) => {
                    if let Some(Expression::FieldAccess(f)) = &r.value {
                        if matches!(&f.object, Expression::Variable { name, .. } if name == param) {
                            if let Some(ft) = info.fields.get(&f.field) {
                                out.push(ft.clone());
                            }
                        }
                    }
                }
                Statement::If(i) => {
                    Self::collect_param_field_return_types(&i.then_block, param, info, out);
                    if let Some(ref else_b) = i.else_block {
                        Self::collect_param_field_return_types(else_b, param, info, out);
                    }
                }
                Statement::While(w) => {
                    Self::collect_param_field_return_types(&w.body, param, info, out);
                }
                Statement::For(f) => {
                    Self::collect_param_field_return_types(&f.body, param, info, out);
                }
                Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                    Self::collect_param_field_return_types(b, param, info, out);
                }
                _ => {}
            }
        }
    }

    fn infer_return_from_struct_literal(block: &Block) -> Option<Type> {
        let mut types = Vec::new();
        Self::collect_struct_literal_return_types(block, &mut types);
        unify_return_types(&types)
    }

    fn collect_struct_literal_return_types(block: &Block, out: &mut Vec<Type>) {
        for stmt in &block.statements {
            match stmt {
                Statement::Return(r) => {
                    if let Some(Expression::StructLiteral(sl)) = &r.value {
                        out.push(Type::Struct(sl.name.clone()));
                    }
                }
                Statement::If(i) => {
                    Self::collect_struct_literal_return_types(&i.then_block, out);
                    if let Some(ref else_b) = i.else_block {
                        Self::collect_struct_literal_return_types(else_b, out);
                    }
                }
                Statement::While(w) => {
                    Self::collect_struct_literal_return_types(&w.body, out);
                }
                Statement::For(f) => {
                    Self::collect_struct_literal_return_types(&f.body, out);
                }
                Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                    Self::collect_struct_literal_return_types(b, out);
                }
                _ => {}
            }
        }
    }

    fn collect_ctor_binding_vars(block: &Block, ctor: &str, out: &mut std::collections::HashSet<String>) {
        for stmt in &block.statements {
            match stmt {
                Statement::Let(l) | Statement::Const(l) => {
                    if let Expression::Call(c) = &l.value {
                        if c.callee == ctor {
                            out.insert(l.name.clone());
                        }
                    }
                }
                Statement::If(i) => {
                    Self::collect_ctor_binding_vars(&i.then_block, ctor, out);
                    if let Some(ref else_b) = i.else_block {
                        Self::collect_ctor_binding_vars(else_b, ctor, out);
                    }
                }
                Statement::While(w) => Self::collect_ctor_binding_vars(&w.body, ctor, out),
                Statement::For(f) => Self::collect_ctor_binding_vars(&f.body, ctor, out),
                Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                    Self::collect_ctor_binding_vars(b, ctor, out);
                }
                _ => {}
            }
        }
    }

    fn block_returns_bound_var(block: &Block, vars: &std::collections::HashSet<String>) -> bool {
        for stmt in &block.statements {
            match stmt {
                Statement::Return(r) => {
                    if let Some(Expression::Variable { name, .. }) = &r.value {
                        if vars.contains(name) {
                            return true;
                        }
                    }
                }
                Statement::If(i) => {
                    if Self::block_returns_bound_var(&i.then_block, vars) {
                        return true;
                    }
                    if let Some(ref else_b) = i.else_block {
                        if Self::block_returns_bound_var(else_b, vars) {
                            return true;
                        }
                    }
                }
                Statement::While(w) => {
                    if Self::block_returns_bound_var(&w.body, vars) {
                        return true;
                    }
                }
                Statement::For(f) => {
                    if Self::block_returns_bound_var(&f.body, vars) {
                        return true;
                    }
                }
                Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                    if Self::block_returns_bound_var(b, vars) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(super) fn infer_hoisted_arrow_return(&mut self, func: &Function) -> Option<Type> {
        let mut local = TypeEnv {
            variables: self.env.variables.clone(),
            functions: self.env.functions.clone(),
        };
        for p in &func.params {
            let ann = if matches!(&p.ty, TypeAnnotation::Generic(n) if n == "_") {
                self.infer_arrow_param_type(p, &ArrowBody::Block(func.body.clone()))
                    .map(|t| Self::type_to_ann(&t))
                    .unwrap_or(TypeAnnotation::Generic("_".into()))
            } else {
                p.ty.clone()
            };
            local.variables.insert(
                p.name.clone(),
                VarInfo {
                    ty: self.type_from_ann(&ann),
                    mutable: p.mutable,
                },
            );
        }
        for stmt in &func.body.statements {
            if let Statement::Return(r) = stmt {
                return Some(if let Some(v) = &r.value {
                    self.check_expr(v, &mut local)
                } else {
                    Type::Void
                });
            }
        }
        None
    }

    pub(super) fn infer_arrow_param_type(&self, param: &Param, body: &ArrowBody) -> Option<Type> {
        if !param.destructure.is_empty() {
            let mut elems = Vec::new();
            for name in &param.destructure {
                if let Some(ty) = self.infer_name_type_in_arrow_body(name, body) {
                    elems.push(ty);
                } else {
                    elems.push(Type::Unknown);
                }
            }
            if elems.iter().any(|t| *t != Type::Unknown) {
                return Some(Type::Tuple { elems });
            }
            return None;
        }
        self.infer_name_type_in_arrow_body(&param.name, body)
    }

    pub(super) fn infer_name_type_in_arrow_body(&self, name: &str, body: &ArrowBody) -> Option<Type> {
        match body {
            ArrowBody::Expr(e) => self.infer_name_type_in_expr(name, e),
            ArrowBody::Block(b) => {
                Self::unify_param_type_hints(self.collect_param_type_hints(name, b)).ok()
            }
        }
    }

    pub(super) fn infer_name_type_in_stmt(&self, name: &str, stmt: &Statement) -> Option<Type> {
        Self::unify_param_type_hints({
            let mut hints = Vec::new();
            self.collect_param_hints_stmt(name, stmt, &mut hints);
            hints
        })
        .ok()
    }

    fn infer_string_from_strcat_expr(&self, name: &str, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::Call(c) if c.callee == "strcat" && c.args.len() == 2 => {
                let (left, right) = (&c.args[0], &c.args[1]);
                if Self::expr_is_param_name(left, name)
                    && matches!(right, Expression::Literal(Literal::String(_)))
                {
                    return Some(Type::String);
                }
                if Self::expr_is_param_name(right, name)
                    && matches!(left, Expression::Literal(Literal::String(_)))
                {
                    return Some(Type::String);
                }
                c.args
                    .iter()
                    .find_map(|arg| self.infer_string_from_strcat_expr(name, arg))
            }
            Expression::Grouped(inner) => self.infer_string_from_strcat_expr(name, inner),
            Expression::Call(c) => c
                .args
                .iter()
                .find_map(|arg| self.infer_string_from_strcat_expr(name, arg)),
            _ => None,
        }
    }

    pub(super) fn infer_name_type_in_expr(&self, name: &str, expr: &Expression) -> Option<Type> {
        if let Some(ty) = self.infer_string_from_strcat_expr(name, expr) {
            return Some(ty);
        }
        match expr {
            Expression::Binary(b) => {
                if Self::is_numeric_binop(&b.op) {
                    if Self::expr_is_param_name(&b.left, name) {
                        if Self::expr_in_float_context(&b.right) {
                            return Some(Type::F64);
                        }
                        return Some(Type::Integer(ast::IntKind::I32));
                    }
                    if Self::expr_is_param_name(&b.right, name) {
                        if Self::expr_in_float_context(&b.left) {
                            return Some(Type::F64);
                        }
                        return Some(Type::Integer(ast::IntKind::I32));
                    }
                }
                let from_left = if matches!(&b.left, Expression::Variable { name: n, .. } if n == name) {
                    self.infer_type_from_operand(&b.op, &b.right)
                } else {
                    None
                };
                let from_right = if matches!(&b.right, Expression::Variable { name: n, .. } if n == name) {
                    self.infer_type_from_operand(&b.op, &b.left)
                } else {
                    None
                };
                from_left
                    .or(from_right)
                    .or_else(|| self.infer_name_type_in_expr(name, &b.left))
                    .or_else(|| self.infer_name_type_in_expr(name, &b.right))
            }
            Expression::Unary(u) => self.infer_name_type_in_expr(name, &u.operand),
            Expression::Call(c) => self
                .infer_name_type_in_call_arg(name, c)
                .or_else(|| {
                    c.args
                        .iter()
                        .find_map(|a| self.infer_name_type_in_expr(name, a))
                }),
            Expression::MethodCall(mc) => {
                if let Expression::FieldAccess(f) = &mc.object {
                    if Self::expr_is_param_name(&f.object, name) {
                        if let Some(ty) = self.infer_struct_type_from_field(&f.field) {
                            return Some(ty);
                        }
                    }
                }
                if Self::expr_is_param_name(&mc.object, name) {
                    if let Some(ty) = self.infer_method_receiver_type(&mc.method) {
                        return Some(ty);
                    }
                }
                if matches!(mc.method.as_str(), "push" | "append") {
                    for arg in &mc.args {
                        if Self::expr_is_param_name(arg, name) {
                            return Some(Type::String);
                        }
                    }
                }
                if matches!(
                    mc.method.as_str(),
                    "starts_with" | "ends_with" | "replace" | "replacen" | "contains"
                ) {
                    for arg in &mc.args {
                        if Self::expr_is_param_name(arg, name) {
                            return Some(Type::String);
                        }
                    }
                }
                self.infer_name_type_in_expr(name, &mc.object).or_else(|| {
                    mc.args
                        .iter()
                        .find_map(|a| self.infer_name_type_in_expr(name, a))
                })
            }
            Expression::If(i) => self
                .infer_name_type_in_expr(name, &i.condition)
                .or_else(|| self.infer_name_type_in_expr(name, &i.then_expr))
                .or_else(|| self.infer_name_type_in_expr(name, &i.else_expr)),
            Expression::Match(m) => {
                if Self::expr_is_param_name(&m.scrutinee, name) {
                    if let Some(ty) = self.infer_enum_type_from_match(m) {
                        return Some(ty);
                    }
                    if let Some(ty) = self.infer_string_type_from_match(m) {
                        return Some(ty);
                    }
                    if let Some(ty) = self.infer_struct_type_from_match(m) {
                        return Some(ty);
                    }
                    if let Some(ty) = self.infer_tuple_type_from_match(m) {
                        return Some(ty);
                    }
                }
                for arm in &m.arms {
                    if let Some(t) = self.infer_name_type_in_expr(name, &arm.body) {
                        return Some(t);
                    }
                    if let Some(ref guard) = arm.guard {
                        if let Some(t) = self.infer_name_type_in_expr(name, guard) {
                            return Some(t);
                        }
                    }
                }
                self.infer_name_type_in_expr(name, &m.scrutinee)
            }
            Expression::Grouped(inner) => self.infer_name_type_in_expr(name, inner),
            Expression::TemplateLiteral(t) => {
                let pipe_format = t.parts.iter().any(|part| {
                    matches!(part, TemplatePart::Static(s) if s.contains('|'))
                });
                for part in &t.parts {
                    if let TemplatePart::Interpolation(expr) = part {
                        if Self::expr_is_param_name(expr, name) {
                            if pipe_format {
                                return Some(Type::String);
                            }
                            return Some(Type::Integer(ast::IntKind::I32));
                        }
                        if let Some(ty) = self.infer_name_type_in_expr(name, expr) {
                            return Some(ty);
                        }
                    }
                }
                None
            }
            Expression::FieldAccess(f) => {
                if Self::expr_is_param_name(&f.object, name) {
                    if let Some(ty) =
                        self.infer_struct_type_from_field_use(&f.field, Some(expr))
                    {
                        return Some(ty);
                    }
                }
                self.infer_name_type_in_expr(name, &f.object)
            }
            Expression::StructLiteral(sl) => {
                if let Some(info) = self.structs.get(&sl.name) {
                    for (field_name, value) in &sl.fields {
                        if let Some(field_ty) = info.fields.get(field_name) {
                            if let Some(t) =
                                self.infer_param_type_for_field_value(name, value, field_ty)
                            {
                                return Some(t);
                            }
                        }
                    }
                }
                for (_, value) in &sl.fields {
                    if let Some(t) = self.infer_name_type_in_expr(name, value) {
                        return Some(t);
                    }
                }
                None
            }
            Expression::Index(ix) => {
                if Self::expr_is_param_name(&ix.object, name) {
                    return Some(Type::Array {
                        elem: Box::new(Type::Integer(ast::IntKind::I32)),
                        len: None,
                    });
                }
                if Self::expr_is_param_name(&ix.index, name) {
                    return Some(Type::Integer(ast::IntKind::I32));
                }
                self.infer_struct_from_param_root(name, &ix.object)
                    .or_else(|| self.infer_name_type_in_expr(name, &ix.object))
                    .or_else(|| self.infer_name_type_in_expr(name, &ix.index))
            }
            _ => None,
        }
    }

    fn infer_enum_type_from_match(&self, m: &MatchExpr) -> Option<Type> {
        let mut enums: Vec<String> = m
            .arms
            .iter()
            .filter_map(|arm| match &arm.pattern {
                MatchPattern::Qualified(en, _) | MatchPattern::QualifiedBind(en, _, _) => {
                    if self.enums.contains_key(en) {
                        Some(en.clone())
                    } else {
                        None
                    }
                }
                MatchPattern::Or(ps) => ps.iter().find_map(|p| match p {
                    MatchPattern::Qualified(en, _) | MatchPattern::QualifiedBind(en, _, _) => {
                        if self.enums.contains_key(en) {
                            Some(en.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }),
                _ => None,
            })
            .collect();
        enums.sort();
        enums.dedup();
        if enums.len() == 1 {
            Some(Type::Enum(enums[0].clone()))
        } else {
            None
        }
    }

    fn infer_string_type_from_match(&self, m: &MatchExpr) -> Option<Type> {
        let has_string_lit = m
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, MatchPattern::Literal(_)));
        if has_string_lit {
            Some(Type::String)
        } else {
            None
        }
    }

    fn infer_struct_type_from_match(&self, m: &MatchExpr) -> Option<Type> {
        let mut names: Vec<String> = m
            .arms
            .iter()
            .filter_map(|arm| match &arm.pattern {
                MatchPattern::Struct(name, _) if self.structs.contains_key(name) => {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        names.sort();
        names.dedup();
        if names.len() == 1 {
            Some(Type::Struct(names[0].clone()))
        } else {
            None
        }
    }

    fn infer_tuple_type_from_match(&self, m: &MatchExpr) -> Option<Type> {
        let mut arities = Vec::new();
        for arm in &m.arms {
            if let MatchPattern::Tuple(binds) = &arm.pattern {
                arities.push(binds.len());
            }
        }
        arities.sort();
        arities.dedup();
        if arities.len() == 1 {
            Some(Type::Tuple {
                elems: vec![Type::Unknown; arities[0]],
            })
        } else {
            None
        }
    }

    fn expr_in_float_context(expr: &Expression) -> bool {
        match expr {
            Expression::Literal(Literal::Float(_, _)) => true,
            Expression::Unary(u) => Self::expr_in_float_context(&u.operand),
            Expression::Binary(b) => {
                Self::expr_in_float_context(&b.left) || Self::expr_in_float_context(&b.right)
            }
            Expression::Grouped(inner) => Self::expr_in_float_context(inner),
            _ => false,
        }
    }

    fn infer_type_from_enum_variant(&self, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::FieldAccess(f) => {
                if let Expression::Variable { name: en, .. } = &f.object {
                    if self.enums.contains_key(en) {
                        return Some(Type::Enum(en.clone()));
                    }
                }
                None
            }
            Expression::EnumVariant(ev) => ev
                .enum_name
                .as_ref()
                .filter(|en| self.enums.contains_key(*en))
                .map(|en| Type::Enum(en.clone())),
            Expression::Grouped(inner) => self.infer_type_from_enum_variant(inner),
            _ => None,
        }
    }

    pub(super) fn infer_type_from_operand(&self, op: &BinaryOp, other: &Expression) -> Option<Type> {
        if let Some(ty) = self.infer_type_from_enum_variant(other) {
            return match op {
                BinaryOp::Eq | BinaryOp::Ne => Some(ty),
                _ => None,
            };
        }
        match other {
            Expression::Literal(Literal::Char(_)) => match op {
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Le
                | BinaryOp::Ge => Some(Type::Char),
                _ => None,
            },
            Expression::Literal(Literal::Float(_, _)) => match op {
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Le
                | BinaryOp::Ge => Some(Type::F64),
                _ => None,
            },
            Expression::Literal(Literal::Int(_)) | Expression::Literal(Literal::IntKind(_, _)) => match op {
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Shl
                | BinaryOp::Shr
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
                | BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Le
                | BinaryOp::Ge => Some(Type::Integer(ast::IntKind::I32)),
                _ => None,
            },
            Expression::Literal(Literal::Bool(_)) => Some(Type::Bool),
            Expression::Literal(Literal::String(_)) => Some(Type::String),
            Expression::Call(c) if c.callee == "char_at" => match op {
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Le
                | BinaryOp::Ge => Some(Type::Integer(ast::IntKind::I32)),
                _ => None,
            },
            Expression::Binary(b) if Self::is_numeric_binop(&b.op) && Self::expr_in_float_context(other) => {
                Some(Type::F64)
            }
            Expression::Binary(b) if Self::is_numeric_binop(&b.op) => {
                Some(Type::Integer(ast::IntKind::I32))
            }
            Expression::Variable { .. } => match op {
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Shl
                | BinaryOp::Shr
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
                | BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge => Some(Type::Integer(ast::IntKind::I32)),
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn resolve_inferred_param_anns(&self, func: &Function) -> Vec<TypeAnnotation> {
        let program = self
            .program_for_inference
            .map(|p| unsafe { &*p });
        self.infer_inferred_param_ann(func, program)
    }

    pub(super) fn check_function(&mut self, func: &Function) {
        if func.is_async && self.target_is_wasm() {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                func.span.clone(),
                "async functions are not available on wasm32 targets".to_string(),
            ));
        }
        self.check_export_fn_abi(func);
        let mut local = TypeEnv {
            variables: self.env.variables.clone(),
            functions: self.env.functions.clone(),
        };

        let param_anns = self.resolve_inferred_param_anns(func);

        let mut param_types = Vec::new();
        for (i, (p, ty_ann)) in func.params.iter().zip(param_anns.iter()).enumerate() {
            if p.no_escape && !matches!(ty_ann, TypeAnnotation::Ref { .. }) {
                self.errors.push(
                    NyraError::coded(
                        "E0601",
                        ErrorKind::Type,
                        func.span.clone(),
                        format!(
                            "parameter `{}` has `#[no_escape]` but type is not a reference (`&T`)",
                            p.name
                        ),
                    )
                    .note("Example: `fn f(#[no_escape] data: &string) { ... }`"),
                );
            }
            let ty: Type = if matches!(ty_ann, TypeAnnotation::Generic(n) if n == "_") {
                let mut hints = self.collect_param_type_hints(&p.name, &func.body);
                if let Some(prog) = self
                    .program_for_inference
                    .map(|ptr| unsafe { &*ptr })
                {
                    hints.extend(self.collect_call_site_param_hints(func, i, prog));
                    hints.extend(self.collect_fn_value_param_hints(func, i, prog));
                }
                match Self::unify_param_type_hints_for_fn(hints.clone(), Some(&func.name)) {
                    Ok(ty) => ty,
                    Err(conflicts) if !conflicts.is_empty() => {
                        super::diagnostics::conflicting_param_types(
                            self,
                            &p.name,
                            &func.name,
                            &conflicts,
                            func.span.clone(),
                        );
                        Type::Unknown
                    }
                    Err(_) => {
                        super::diagnostics::cannot_infer_param(
                            self,
                            &p.name,
                            &func.name,
                            func.span.clone(),
                        );
                        Type::Unknown
                    }
                }
            } else {
                self.type_from_ann(ty_ann)
            };
            local.variables.insert(
                p.name.clone(),
                VarInfo {
                    ty: ty.clone(),
                    mutable: p.mutable,
                },
            );
            if !p.destructure.is_empty() {
                if let Type::Tuple { elems } = &ty {
                    for (name, elem_ty) in p.destructure.iter().zip(elems.iter()) {
                        local.variables.insert(
                            name.clone(),
                            VarInfo {
                                ty: elem_ty.clone(),
                                mutable: false,
                            },
                        );
                    }
                }
            }
            param_types.push(ty);
        }

        let ret = self
            .env
            .functions
            .get(&func.name)
            .map(|sig| sig.return_type.clone())
            .filter(|t| *t != Type::Unknown && *t != Type::Generic("_".into()))
            .unwrap_or_else(|| self.function_return_type_with_params(func, &param_types));

        self.env.functions.insert(
            func.name.clone(),
            FunctionSignature {
                params: param_types.clone(),
                return_type: ret.clone(),
            },
        );

        let body_ret = if func.is_async {
            func.return_type
                .as_ref()
                .map(|ann| self.type_from_ann(ann))
                .unwrap_or(Type::Integer(ast::IntKind::I32))
        } else {
            ret.clone()
        };

        let prev_bounds = std::mem::replace(
            &mut self.current_type_param_bounds,
            func.type_param_bounds.clone(),
        );
        self.check_block(&func.body, &mut local, &body_ret);
        self.current_type_param_bounds = prev_bounds;

        if func.return_type.is_none()
            && !block_has_return(&func.body)
            && func.name != "main"
            && !func.is_test
            && ret != Type::Void
        {
            self.errors.push(
                NyraError::new(
                    ErrorKind::Type,
                    func.span.clone(),
                    format!(
                        "Function '{}' is missing a return value; add `return` or declare `-> void`",
                        func.name
                    ),
                )
                .note("Example: `fn run() -> void { print(1) }`"),
            );
        }
    }
}

