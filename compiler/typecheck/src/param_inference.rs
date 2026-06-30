//! Inference for untyped (`_`) function parameters — JS-style zero-type ergonomics.

use ast::*;

use super::TypeChecker;
use types::Type;

impl TypeChecker {
    pub(super) fn unify_param_type_hints(hints: Vec<Type>) -> Result<Type, Vec<Type>> {
        Self::unify_param_type_hints_for_fn(hints, None)
    }

    pub(super) fn unify_param_type_hints_for_fn(
        hints: Vec<Type>,
        func_name: Option<&str>,
    ) -> Result<Type, Vec<Type>> {
        let hints: Vec<Type> = hints
            .into_iter()
            .filter(|t| *t != Type::Unknown && *t != Type::Generic("_".into()))
            .collect();
        if hints.is_empty() {
            return Err(vec![]);
        }
        if hints.iter().any(|t| *t == Type::String) && hints.iter().any(|t| types::is_integer(t)) {
            let strings: Vec<Type> = hints
                .iter()
                .filter(|t| **t == Type::String)
                .cloned()
                .collect();
            if !strings.is_empty() {
                return Ok(Type::String);
            }
        }
        if hints.iter().any(|t| matches!(t, Type::Struct(_))) && hints.iter().any(|t| types::is_integer(t)) {
            let structs: Vec<Type> = hints
                .iter()
                .filter(|t| matches!(t, Type::Struct(_)))
                .cloned()
                .collect();
            let ints: Vec<Type> = hints.iter().filter(|t| types::is_integer(t)).cloned().collect();
            if structs.len() == 1 && !ints.is_empty() {
                let struct_count = hints.iter().filter(|t| *t == &structs[0]).count();
                let int_count = hints.iter().filter(|t| types::is_integer(t)).count();
                if int_count >= struct_count {
                    let first = ints[0].clone();
                    if ints.iter().all(|t| t == &first) {
                        return Ok(first);
                    }
                }
            }
        }
        if hints.iter().any(|t| matches!(t, Type::Enum(_))) {
            let enums: Vec<Type> = hints
                .iter()
                .filter(|t| matches!(t, Type::Enum(_)))
                .cloned()
                .collect();
            let first = enums[0].clone();
            if enums.iter().all(|t| t == &first) {
                return Ok(first);
            }
            return Err(enums);
        }
        if hints.iter().any(|t| matches!(t, Type::Struct(_))) {
            let structs: Vec<Type> = hints
                .iter()
                .filter(|t| matches!(t, Type::Struct(_)))
                .cloned()
                .collect();
            let first = structs[0].clone();
            if structs.iter().all(|t| t == &first) {
                return Ok(first);
            }
            if let Some(ty) = Self::pick_struct_by_hint_count(&hints, &structs) {
                return Ok(ty);
            }
            if let Some(name) = func_name {
                if let Some(ty) = Self::pick_struct_hint_for_function(name, &structs) {
                    return Ok(ty);
                }
            }
            return Err(structs);
        }
        if hints.iter().any(|t| *t == Type::String) {
            if hints.iter().all(|t| *t == Type::String) {
                return Ok(Type::String);
            }
            return Err(hints);
        }
        if hints.iter().any(|t| *t == Type::Bool) {
            if hints.iter().all(|t| *t == Type::Bool) {
                return Ok(Type::Bool);
            }
            return Err(hints);
        }
        if hints.iter().any(|t| types::is_integer(t)) {
            let ints: Vec<Type> = hints.iter().filter(|t| types::is_integer(t)).cloned().collect();
            let first = ints[0].clone();
            if ints.iter().all(|t| t == &first) {
                return Ok(first);
            }
            if ints.iter().all(|t| types::is_integer(t)) {
                if let Some(u8_ty) = ints
                    .iter()
                    .find(|t| *t == &Type::Integer(ast::IntKind::U8))
                {
                    return Ok(u8_ty.clone());
                }
            }
            return Err(ints);
        }
        if hints.iter().any(|t| matches!(t, Type::Array { .. })) {
            let arrays: Vec<Type> = hints
                .iter()
                .filter(|t| matches!(t, Type::Array { .. }))
                .cloned()
                .collect();
            if !arrays.is_empty() {
                if let Some(with_len) = arrays.iter().find(|t| {
                    matches!(t, Type::Array { len: Some(n), .. } if *n > 0)
                }) {
                    return Ok(with_len.clone());
                }
                return Ok(arrays[0].clone());
            }
        }
        if hints.iter().any(|t| *t == Type::F64) {
            if hints.iter().all(|t| *t == Type::F64) {
                return Ok(Type::F64);
            }
            return Err(hints);
        }
        let first = hints[0].clone();
        if hints.iter().all(|t| t == &first) {
            Ok(first)
        } else {
            Err(hints)
        }
    }

    fn pick_struct_by_hint_count(hints: &[Type], structs: &[Type]) -> Option<Type> {
        use std::collections::HashMap;
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for h in hints {
            if let Type::Struct(n) = h {
                *counts.entry(n.as_str()).or_default() += 1;
            }
        }
        let mut best: Option<(usize, Type)> = None;
        for ty in structs {
            let Type::Struct(name) = ty else {
                continue;
            };
            let count = counts.get(name.as_str()).copied().unwrap_or(0);
            if best.as_ref().is_none_or(|(best_count, _)| count > *best_count) {
                best = Some((count, ty.clone()));
            }
        }
        best.filter(|(count, _)| *count > 0).map(|(_, ty)| ty)
    }

    fn pick_struct_hint_for_function(func_name: &str, structs: &[Type]) -> Option<Type> {
        let mut best: Option<(usize, Type)> = None;
        for ty in structs {
            let Type::Struct(name) = ty else {
                continue;
            };
            let prefix = format!("{name}_");
            if func_name.starts_with(&prefix) && func_name.len() > prefix.len() {
                if best.as_ref().is_none_or(|(len, _)| name.len() > *len) {
                    best = Some((name.len(), ty.clone()));
                }
            }
        }
        best.map(|(_, t)| t)
    }

    fn infer_param_type_from_returns(&self, func: &Function, param: &str) -> Option<Type> {
        let mut literal_types: Vec<Type> = Vec::new();
        let mut returns_param = false;
        self.collect_return_param_hints(&func.body, param, &mut returns_param, &mut literal_types);
        if !returns_param || literal_types.is_empty() {
            return None;
        }
        Self::unify_param_type_hints(literal_types).ok()
    }

    fn collect_return_param_hints(
        &self,
        block: &Block,
        param: &str,
        returns_param: &mut bool,
        literal_types: &mut Vec<Type>,
    ) {
        for stmt in &block.statements {
            match stmt {
                Statement::Return(r) => {
                    if let Some(v) = &r.value {
                        if Self::expr_is_param_name(v, param) {
                            *returns_param = true;
                        } else if let Some(t) = self.expr_type_hint(v) {
                            if t != Type::Unknown {
                                literal_types.push(t);
                            }
                        }
                    }
                }
                Statement::If(i) => {
                    self.collect_return_param_hints(&i.then_block, param, returns_param, literal_types);
                    if let Some(ref else_b) = i.else_block {
                        self.collect_return_param_hints(else_b, param, returns_param, literal_types);
                    }
                }
                Statement::While(w) => {
                    self.collect_return_param_hints(&w.body, param, returns_param, literal_types);
                }
                Statement::For(f) => {
                    self.collect_return_param_hints(&f.body, param, returns_param, literal_types);
                }
                Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                    self.collect_return_param_hints(b, param, returns_param, literal_types);
                }
                _ => {}
            }
        }
    }

    pub(super) fn infer_struct_from_fn_name_prefix(
        &self,
        func_name: &str,
        param_index: usize,
    ) -> Option<Type> {
        if param_index != 0 || func_name.ends_with("_new") {
            return None;
        }
        let mut best: Option<String> = None;
        for sname in self.structs.keys() {
            let prefix = format!("{sname}_");
            if func_name.starts_with(&prefix) && func_name.len() > prefix.len() {
                if best.as_ref().is_none_or(|b| sname.len() > b.len()) {
                    best = Some(sname.clone());
                }
            }
        }
        best.map(Type::Struct)
    }

    fn param_used_as_struct_receiver(&self, param: &str, block: &Block) -> bool {
        for stmt in &block.statements {
            if self.stmt_uses_struct_receiver(param, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_uses_struct_receiver(&self, param: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Return(r) => r
                .value
                .as_ref()
                .is_some_and(|e| self.expr_uses_struct_receiver(param, e)),
            Statement::Expression(e) | Statement::Defer(e) => {
                self.expr_uses_struct_receiver(param, e)
            }
            Statement::Let(l) => self.expr_uses_struct_receiver(param, &l.value),
            Statement::Assign(a) => {
                self.expr_uses_struct_receiver(param, &a.target)
                    || self.expr_uses_struct_receiver(param, &a.value)
            }
            Statement::If(i) => {
                self.expr_uses_struct_receiver(param, &i.condition)
                    || self.stmt_uses_struct_receiver_in_block(param, &i.then_block)
                    || i.else_block
                        .as_ref()
                        .is_some_and(|b| self.stmt_uses_struct_receiver_in_block(param, b))
            }
            Statement::While(w) => {
                self.expr_uses_struct_receiver(param, &w.condition)
                    || self.stmt_uses_struct_receiver_in_block(param, &w.body)
            }
            Statement::For(f) => {
                self.stmt_uses_struct_receiver_in_block(param, &f.body)
            }
            Statement::Print(p) => p
                .args
                .iter()
                .any(|e| self.expr_uses_struct_receiver(param, e)),
            Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                self.stmt_uses_struct_receiver_in_block(param, b)
            }
            _ => false,
        }
    }

    fn stmt_uses_struct_receiver_in_block(&self, param: &str, block: &Block) -> bool {
        block
            .statements
            .iter()
            .any(|s| self.stmt_uses_struct_receiver(param, s))
    }

    fn expr_uses_struct_receiver(&self, param: &str, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess(f) => Self::expr_is_param_name(&f.object, param)
                || self.expr_uses_struct_receiver(param, &f.object),
            Expression::Index(ix) => {
                self.expr_uses_struct_receiver(param, &ix.object)
                    || self.expr_uses_struct_receiver(param, &ix.index)
            }
            Expression::Binary(b) => {
                self.expr_uses_struct_receiver(param, &b.left)
                    || self.expr_uses_struct_receiver(param, &b.right)
            }
            Expression::Unary(u) => self.expr_uses_struct_receiver(param, &u.operand),
            Expression::Call(c) => c
                .args
                .iter()
                .any(|a| self.expr_uses_struct_receiver(param, a)),
            Expression::MethodCall(m) => {
                self.expr_uses_struct_receiver(param, &m.object)
                    || m.args
                        .iter()
                        .any(|a| self.expr_uses_struct_receiver(param, a))
            }
            Expression::If(i) => {
                self.expr_uses_struct_receiver(param, &i.condition)
                    || self.block_uses_struct_receiver(param, &i.then_block)
                    || self.block_uses_struct_receiver(param, &i.else_block)
            }
            Expression::Match(m) => {
                self.expr_uses_struct_receiver(param, &m.scrutinee)
                    || m.arms.iter().any(|a| {
                        self.block_uses_struct_receiver(param, &a.body)
                            || a.guard
                                .as_ref()
                                .is_some_and(|g| self.expr_uses_struct_receiver(param, g))
                    })
            }
            Expression::Grouped(inner) => self.expr_uses_struct_receiver(param, inner),
            Expression::StructLiteral(sl) => sl
                .fields
                .iter()
                .any(|(_, v)| self.expr_uses_struct_receiver(param, v)),
            Expression::TemplateLiteral(t) => t.parts.iter().any(|part| {
                matches!(part, TemplatePart::Interpolation(e) if self.expr_uses_struct_receiver(param, e))
            }),
            _ => false,
        }
    }

    pub(super) fn infer_inferred_param_ann(
        &self,
        func: &Function,
        program: Option<&Program>,
    ) -> Vec<TypeAnnotation> {
        let mut param_anns: Vec<TypeAnnotation> =
            func.params.iter().map(|p| p.ty.clone()).collect();
        for (i, p) in func.params.iter().enumerate() {
            if !matches!(&p.ty, TypeAnnotation::Generic(n) if n == "_") {
                continue;
            }
            let mut hints = self.collect_param_type_hints(&p.name, &func.body);
            if let Some(ty) = self.infer_struct_from_fn_name_prefix(&func.name, i) {
                if self.param_used_as_struct_receiver(&p.name, &func.body) {
                    hints.push(ty);
                }
            }
            if let Some(prog) = program {
                hints.extend(self.collect_call_site_param_hints(func, i, prog));
                hints.extend(self.collect_fn_value_param_hints(func, i, prog));
            }
            if let Some(ty) = self.infer_param_type_from_returns(func, &p.name) {
                hints.push(ty);
            }
            match Self::unify_param_type_hints_for_fn(hints, Some(&func.name)) {
                Ok(ty) => param_anns[i] = Self::type_to_ann(&ty),
                Err(conflicts) if !conflicts.is_empty() => {
                    param_anns[i] = TypeAnnotation::Generic("_".into());
                    let _ = conflicts;
                }
                Err(_) => {
                    if p.name.starts_with('_') {
                        if let Some(ty) = self.infer_struct_from_fn_name_prefix(&func.name, i) {
                            param_anns[i] = Self::type_to_ann(&ty);
                        } else {
                            param_anns[i] = TypeAnnotation::String;
                        }
                    } else if let Some(ty) =
                        self.infer_arrow_param_type(p, &ArrowBody::Block(func.body.clone()))
                    {
                        param_anns[i] = Self::type_to_ann(&ty);
                    }
                    // else leave `_` — check_function reports cannot_infer_param.
                }
            }
        }
        param_anns
    }

    pub(super) fn collect_call_site_param_hints(
        &self,
        func: &Function,
        param_index: usize,
        program: &Program,
    ) -> Vec<Type> {
        let mut hints = Vec::new();
        for user_fn in &program.functions {
            self.collect_call_site_hints_in_block(
                &user_fn.body,
                &func.name,
                param_index,
                &mut hints,
                &std::collections::HashMap::new(),
            );
        }
        for imp in &program.impls {
            for method in &imp.methods {
                self.collect_call_site_hints_in_block(
                    &method.body,
                    &func.name,
                    param_index,
                    &mut hints,
                    &std::collections::HashMap::new(),
                );
            }
        }
        hints
    }

    /// When `func` is passed as a callback argument, infer param types from the callee's `fn` parameter.
    pub(super) fn collect_fn_value_param_hints(
        &self,
        func: &Function,
        param_index: usize,
        program: &Program,
    ) -> Vec<Type> {
        let mut hints = Vec::new();
        for user_fn in &program.functions {
            self.collect_fn_value_hints_in_block(&user_fn.body, &func.name, param_index, &mut hints);
        }
        for imp in &program.impls {
            for method in &imp.methods {
                self.collect_fn_value_hints_in_block(&method.body, &func.name, param_index, &mut hints);
            }
        }
        hints
    }

    fn collect_fn_value_hints_in_block(
        &self,
        block: &Block,
        callee: &str,
        param_index: usize,
        hints: &mut Vec<Type>,
    ) {
        for stmt in &block.statements {
            self.collect_fn_value_hints_in_stmt(stmt, callee, param_index, hints);
        }
    }

    fn collect_fn_value_hints_in_stmt(
        &self,
        stmt: &Statement,
        callee: &str,
        param_index: usize,
        hints: &mut Vec<Type>,
    ) {
        match stmt {
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    self.collect_fn_value_hints_in_expr(v, callee, param_index, hints);
                }
            }
            Statement::Expression(e) | Statement::Defer(e) => {
                self.collect_fn_value_hints_in_expr(e, callee, param_index, hints);
            }
            Statement::Let(l) | Statement::Const(l) => {
                self.collect_fn_value_hints_in_expr(&l.value, callee, param_index, hints);
            }
            Statement::Assign(a) => {
                self.collect_fn_value_hints_in_expr(&a.value, callee, param_index, hints);
            }
            Statement::Print(p) => {
                for arg in &p.args {
                    self.collect_fn_value_hints_in_expr(arg, callee, param_index, hints);
                }
                if let Some(c) = &p.color {
                    self.collect_fn_value_hints_in_expr(c, callee, param_index, hints);
                }
            }
            Statement::If(i) => {
                self.collect_fn_value_hints_in_expr(&i.condition, callee, param_index, hints);
                self.collect_fn_value_hints_in_block(&i.then_block, callee, param_index, hints);
                if let Some(ref else_b) = i.else_block {
                    self.collect_fn_value_hints_in_block(else_b, callee, param_index, hints);
                }
            }
            Statement::While(w) => {
                self.collect_fn_value_hints_in_expr(&w.condition, callee, param_index, hints);
                self.collect_fn_value_hints_in_block(&w.body, callee, param_index, hints);
            }
            Statement::For(f) => {
                match &f.kind {
                    ForKind::Range { start, end } => {
                        self.collect_fn_value_hints_in_expr(start, callee, param_index, hints);
                        self.collect_fn_value_hints_in_expr(end, callee, param_index, hints);
                    }
                    ForKind::Iterable { iterable } => {
                        self.collect_fn_value_hints_in_expr(iterable, callee, param_index, hints);
                    }
                }
                self.collect_fn_value_hints_in_block(&f.body, callee, param_index, hints);
            }
            Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                self.collect_fn_value_hints_in_block(b, callee, param_index, hints);
            }
            _ => {}
        }
    }

    fn collect_fn_value_hints_in_expr(
        &self,
        expr: &Expression,
        callee: &str,
        param_index: usize,
        hints: &mut Vec<Type>,
    ) {
        match expr {
            Expression::Call(c) => {
                if let Some(sig) = self.env.functions.get(&c.callee) {
                    for (arg_i, arg) in c.args.iter().enumerate() {
                        if let Expression::Variable { name, .. } = arg {
                            if name == callee {
                                if let Some(expected) = sig.params.get(arg_i) {
                                    if let Type::FnPtr { params, .. } = expected {
                                        if let Some(ty) = params.get(param_index) {
                                            hints.push(ty.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for arg in &c.args {
                    self.collect_fn_value_hints_in_expr(arg, callee, param_index, hints);
                }
            }
            Expression::MethodCall(m) => {
                self.collect_fn_value_hints_in_expr(&m.object, callee, param_index, hints);
                for arg in &m.args {
                    self.collect_fn_value_hints_in_expr(arg, callee, param_index, hints);
                }
            }
            Expression::Binary(b) => {
                self.collect_fn_value_hints_in_expr(&b.left, callee, param_index, hints);
                self.collect_fn_value_hints_in_expr(&b.right, callee, param_index, hints);
            }
            Expression::Unary(u) => {
                self.collect_fn_value_hints_in_expr(&u.operand, callee, param_index, hints);
            }
            Expression::If(i) => {
                self.collect_fn_value_hints_in_expr(&i.condition, callee, param_index, hints);
                for_each_expr_in_block(&i.then_block, &mut |e| self.collect_fn_value_hints_in_expr(e, callee, param_index, hints));
                for_each_expr_in_block(&i.else_block, &mut |e| self.collect_fn_value_hints_in_expr(e, callee, param_index, hints));
            }
            Expression::Grouped(inner) => {
                self.collect_fn_value_hints_in_expr(inner, callee, param_index, hints);
            }
            Expression::FieldAccess(f) => {
                self.collect_fn_value_hints_in_expr(&f.object, callee, param_index, hints);
            }
            Expression::EnumVariant(ev) => {
                for arg in &ev.args {
                    self.collect_fn_value_hints_in_expr(arg, callee, param_index, hints);
                }
            }
            _ => {}
        }
    }

    fn collect_call_site_hints_in_block(
        &self,
        block: &Block,
        callee: &str,
        param_index: usize,
        hints: &mut Vec<Type>,
        locals: &std::collections::HashMap<String, Type>,
    ) {
        let mut locals = locals.clone();
        for stmt in &block.statements {
            if let Statement::Let(l) = stmt {
                if let Some(ty) = self.call_arg_type_hint(&l.value, &locals) {
                    locals.insert(l.name.clone(), ty);
                }
            }
            self.collect_call_site_hints_in_stmt(stmt, callee, param_index, hints, &locals);
        }
    }

    fn collect_call_site_hints_in_stmt(
        &self,
        stmt: &Statement,
        callee: &str,
        param_index: usize,
        hints: &mut Vec<Type>,
        locals: &std::collections::HashMap<String, Type>,
    ) {
        match stmt {
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    self.collect_call_site_hints_in_expr(v, callee, param_index, hints, locals);
                }
            }
            Statement::Expression(e) | Statement::Defer(e) => {
                self.collect_call_site_hints_in_expr(e, callee, param_index, hints, locals);
            }
            Statement::Let(l) => {
                self.collect_call_site_hints_in_expr(&l.value, callee, param_index, hints, locals);
            }
            Statement::Assign(a) => {
                self.collect_call_site_hints_in_expr(&a.value, callee, param_index, hints, locals);
            }
            Statement::Print(p) => {
                for arg in &p.args {
                    self.collect_call_site_hints_in_expr(arg, callee, param_index, hints, locals);
                }
            }
            Statement::If(i) => {
                self.collect_call_site_hints_in_expr(&i.condition, callee, param_index, hints, locals);
                self.collect_call_site_hints_in_block(&i.then_block, callee, param_index, hints, locals);
                if let Some(ref else_b) = i.else_block {
                    self.collect_call_site_hints_in_block(else_b, callee, param_index, hints, locals);
                }
            }
            Statement::While(w) => {
                self.collect_call_site_hints_in_expr(&w.condition, callee, param_index, hints, locals);
                self.collect_call_site_hints_in_block(&w.body, callee, param_index, hints, locals);
            }
            Statement::For(f) => {
                match &f.kind {
                    ForKind::Range { start, end } => {
                        self.collect_call_site_hints_in_expr(start, callee, param_index, hints, locals);
                        self.collect_call_site_hints_in_expr(end, callee, param_index, hints, locals);
                    }
                    ForKind::Iterable { iterable } => {
                        self.collect_call_site_hints_in_expr(iterable, callee, param_index, hints, locals);
                    }
                }
                self.collect_call_site_hints_in_block(&f.body, callee, param_index, hints, locals);
            }
            Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                self.collect_call_site_hints_in_block(b, callee, param_index, hints, locals);
            }
            _ => {}
        }
    }

    fn call_arg_type_hint(
        &self,
        expr: &Expression,
        locals: &std::collections::HashMap<String, Type>,
    ) -> Option<Type> {
        if let Expression::Variable { name, .. } = expr {
            if let Some(ty) = locals.get(name) {
                return Some(ty.clone());
            }
            return self
                .env
                .variables
                .get(name)
                .map(|v| v.ty.clone())
                .filter(|t| *t != Type::Unknown);
        }
        self.expr_type_hint(expr)
    }

    fn collect_call_site_hints_in_expr(
        &self,
        expr: &Expression,
        callee: &str,
        param_index: usize,
        hints: &mut Vec<Type>,
        locals: &std::collections::HashMap<String, Type>,
    ) {
        match expr {
            Expression::Call(c) => {
                if c.callee == callee {
                    if let Some(arg) = c.args.get(param_index) {
                        if let Some(ty) = self.call_arg_type_hint(arg, locals) {
                            hints.push(ty);
                        }
                    }
                }
                for arg in &c.args {
                    self.collect_call_site_hints_in_expr(arg, callee, param_index, hints, locals);
                }
            }
            Expression::MethodCall(m) => {
                self.collect_call_site_hints_in_expr(&m.object, callee, param_index, hints, locals);
                for arg in &m.args {
                    self.collect_call_site_hints_in_expr(arg, callee, param_index, hints, locals);
                }
            }
            Expression::Binary(b) => {
                self.collect_call_site_hints_in_expr(&b.left, callee, param_index, hints, locals);
                self.collect_call_site_hints_in_expr(&b.right, callee, param_index, hints, locals);
            }
            Expression::Unary(u) => {
                self.collect_call_site_hints_in_expr(&u.operand, callee, param_index, hints, locals);
            }
            Expression::If(i) => {
                self.collect_call_site_hints_in_expr(&i.condition, callee, param_index, hints, locals);
                for_each_expr_in_block(&i.then_block, &mut |e| self.collect_call_site_hints_in_expr(e, callee, param_index, hints, locals));
                for_each_expr_in_block(&i.else_block, &mut |e| self.collect_call_site_hints_in_expr(e, callee, param_index, hints, locals));
            }
            Expression::Grouped(inner) => {
                self.collect_call_site_hints_in_expr(inner, callee, param_index, hints, locals);
            }
            Expression::FieldAccess(f) => {
                self.collect_call_site_hints_in_expr(&f.object, callee, param_index, hints, locals);
            }
            Expression::EnumVariant(ev) => {
                for arg in &ev.args {
                    self.collect_call_site_hints_in_expr(arg, callee, param_index, hints, locals);
                }
            }
            _ => {}
        }
    }

    pub(super) fn expr_type_hint(&self, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::Literal(Literal::String(_)) => Some(Type::String),
            Expression::Literal(Literal::Bool(_)) => Some(Type::Bool),
            Expression::Literal(Literal::Char(_)) => Some(Type::Char),
            Expression::Literal(Literal::Int(_)) | Expression::Literal(Literal::IntKind(_, _)) => {
                Some(Type::Integer(ast::IntKind::I32))
            }
            Expression::Literal(Literal::Float(_, k)) => Some(types::type_from_float_kind(*k)),
            Expression::Call(c) => self
                .env
                .functions
                .get(&c.callee)
                .map(|sig| sig.return_type.clone())
                .filter(|t| *t != Type::Void && *t != Type::Unknown),
            Expression::StructLiteral(sl) if !sl.name.is_empty() => {
                Some(Type::Struct(sl.name.clone()))
            }
            Expression::FieldAccess(f) => {
                if let Expression::Variable { name: en, .. } = &f.object {
                    if self.enums.contains_key(en) {
                        return Some(Type::Enum(en.clone()));
                    }
                }
                self.expr_type_hint(&f.object).and_then(|obj_ty| {
                    let Type::Struct(name) = obj_ty else {
                        return None;
                    };
                    self.structs
                        .get(&name)
                        .and_then(|info| info.fields.get(&f.field).cloned())
                })
            }
            Expression::EnumVariant(ev) => ev
                .enum_name
                .as_ref()
                .filter(|en| self.enums.contains_key(*en))
                .map(|en| Type::Enum(en.clone())),
            Expression::ArrayLiteral(al) => {
                let len = al.elems.len();
                let elem_ty = al
                    .elems
                    .first()
                    .and_then(|e| self.expr_type_hint(e))
                    .unwrap_or(Type::Integer(ast::IntKind::I32));
                Some(Type::Array {
                    elem: Box::new(elem_ty),
                    len: if len > 0 { Some(len) } else { None },
                })
            }
            Expression::ArrayRepeat { count, .. } => Some(Type::Array {
                elem: Box::new(Type::Integer(ast::IntKind::I32)),
                len: Some(*count),
            }),
            Expression::Variable { name, .. } => self
                .env
                .variables
                .get(name)
                .map(|v| v.ty.clone())
                .filter(|t| *t != Type::Unknown),
            _ => None,
        }
    }

    pub(super) fn collect_param_type_hints(&self, name: &str, block: &Block) -> Vec<Type> {
        let aliases = self.collect_let_param_aliases(name, block);
        let mut names: Vec<&str> = Vec::with_capacity(1 + aliases.len());
        names.push(name);
        for alias in &aliases {
            names.push(alias.as_str());
        }
        let mut out = Vec::new();
        for stmt in &block.statements {
            for n in &names {
                self.collect_param_hints_stmt(n, stmt, &mut out);
            }
        }
        out
    }

    fn collect_let_param_aliases(&self, param: &str, block: &Block) -> Vec<String> {
        let mut aliases = Vec::new();
        for stmt in &block.statements {
            if let Statement::Let(l) = stmt {
                if Self::expr_is_param_name(&l.value, param) {
                    aliases.push(l.name.clone());
                }
            }
        }
        aliases
    }

    pub(super) fn is_numeric_binop(op: &BinaryOp) -> bool {
        matches!(
            op,
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
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
        )
    }

    pub(super) fn infer_struct_type_from_field(&self, field: &str) -> Option<Type> {
        self.infer_struct_type_from_field_use(field, None)
    }

    pub(super) fn infer_struct_type_from_field_use(
        &self,
        field: &str,
        context: Option<&Expression>,
    ) -> Option<Type> {
        let candidates: Vec<(&String, &types::StructInfo)> = self
            .structs
            .iter()
            .filter(|(_, info)| info.fields.contains_key(field))
            .collect();
        if candidates.len() == 1 {
            return Some(Type::Struct(candidates[0].0.clone()));
        }
        if candidates.is_empty() {
            return None;
        }
        if let Some(ctx) = context {
            if Self::expr_in_numeric_context(ctx) {
                let filtered: Vec<_> = candidates
                    .iter()
                    .filter(|(_, info)| types::is_integer(&info.fields[field]))
                    .collect();
                if filtered.len() == 1 {
                    return Some(Type::Struct(filtered[0].0.clone()));
                }
            }
            if Self::expr_in_string_context(ctx) {
                let filtered: Vec<_> = candidates
                    .iter()
                    .filter(|(_, info)| info.fields[field] == Type::String)
                    .collect();
                if filtered.len() == 1 {
                    return Some(Type::Struct(filtered[0].0.clone()));
                }
            }
        }
        let int_only: Vec<_> = candidates
            .iter()
            .filter(|(_, info)| types::is_integer(&info.fields[field]))
            .collect();
        if int_only.len() == 1 {
            return Some(Type::Struct(int_only[0].0.clone()));
        }
        None
    }

    fn expr_in_numeric_context(expr: &Expression) -> bool {
        match expr {
            Expression::Binary(b) => {
                Self::is_numeric_binop(&b.op)
                    || Self::expr_in_numeric_context(&b.left)
                    || Self::expr_in_numeric_context(&b.right)
            }
            Expression::Unary(u) => Self::expr_in_numeric_context(&u.operand),
            Expression::Literal(Literal::Int(_)) | Expression::Literal(Literal::IntKind(_, _)) => {
                true
            }
            Expression::Grouped(inner) => Self::expr_in_numeric_context(inner),
            _ => false,
        }
    }

    fn expr_in_string_context(expr: &Expression) -> bool {
        matches!(expr, Expression::Literal(Literal::String(_)))
            || matches!(
                expr,
                Expression::Call(c) if matches!(
                    c.callee.as_str(),
                    "strcat" | "substring" | "str_pop" | "i32_to_string" | "i64_to_string"
                )
            )
    }

    pub(super) fn infer_param_type_for_field_value(
        &self,
        param: &str,
        value: &Expression,
        field_ty: &Type,
    ) -> Option<Type> {
        if !Self::expr_is_param_name(value, param) {
            return None;
        }
        match field_ty {
            Type::Struct(name) => Some(Type::Struct(name.clone())),
            ty if types::is_integer(ty) => Some(ty.clone()),
            Type::String => Some(Type::String),
            Type::Bool => Some(Type::Bool),
            Type::F32 => Some(Type::F32),
            Type::F64 => Some(Type::F64),
            Type::Array { elem, .. } => Some(Type::Array {
                elem: elem.clone(),
                len: None,
            }),
            _ => None,
        }
    }

    pub(super) fn expr_mentions_param(&self, expr: &Expression, param: &str) -> bool {
        match expr {
            Expression::Variable { name, .. } => name == param,
            Expression::FieldAccess(f) => {
                Self::expr_is_param_name(&f.object, param)
                    || self.expr_mentions_param(&f.object, param)
            }
            Expression::Index(ix) => {
                self.expr_mentions_param(&ix.object, param)
                    || self.expr_mentions_param(&ix.index, param)
            }
            Expression::Binary(b) => {
                self.expr_mentions_param(&b.left, param)
                    || self.expr_mentions_param(&b.right, param)
            }
            Expression::Unary(u) => self.expr_mentions_param(&u.operand, param),
            Expression::Call(c) => c.args.iter().any(|a| self.expr_mentions_param(a, param)),
            Expression::MethodCall(m) => {
                self.expr_mentions_param(&m.object, param)
                    || m.args.iter().any(|a| self.expr_mentions_param(a, param))
            }
            Expression::If(i) => {
                self.expr_mentions_param(&i.condition, param)
                    || self.block_mentions_param(&i.then_block, param)
                    || self.block_mentions_param(&i.else_block, param)
            }
            Expression::Grouped(inner) => self.expr_mentions_param(inner, param),
            _ => false,
        }
    }

    pub(super) fn infer_struct_from_param_root(
        &self,
        param: &str,
        expr: &Expression,
    ) -> Option<Type> {
        match expr {
            Expression::FieldAccess(f) if Self::expr_is_param_name(&f.object, param) => {
                self.infer_struct_type_from_field(&f.field)
            }
            Expression::Index(ix) => self.infer_struct_from_param_root(param, &ix.object),
            _ => None,
        }
    }

    pub(super) fn collect_param_hints_stmt(&self, name: &str, stmt: &Statement, out: &mut Vec<Type>) {
        match stmt {
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    if let Some(t) = self.infer_name_type_in_expr(name, v) {
                        out.push(t);
                    }
                }
            }
            Statement::Expression(e) => {
                if let Some(t) = self.infer_name_type_in_expr(name, e) {
                    out.push(t);
                }
            }
            Statement::Let(l) => {
                if let Some(t) = self.infer_name_type_in_expr(name, &l.value) {
                    out.push(t);
                }
            }
            Statement::If(i) => {
                if let Some(t) = self.infer_name_type_in_expr(name, &i.condition) {
                    out.push(t);
                }
                out.extend(self.collect_param_type_hints(name, &i.then_block));
                if let Some(ref else_b) = i.else_block {
                    out.extend(self.collect_param_type_hints(name, else_b));
                }
            }
            Statement::While(w) => {
                if let Some(t) = self.infer_name_type_in_expr(name, &w.condition) {
                    out.push(t);
                }
                out.extend(self.collect_param_type_hints(name, &w.body));
            }
            Statement::For(f) => {
                match &f.kind {
                    ForKind::Range { start, end } => {
                        if let Some(t) = self.infer_name_type_in_expr(name, start) {
                            out.push(t);
                        }
                        if let Some(t) = self.infer_name_type_in_expr(name, end) {
                            out.push(t);
                        }
                    }
                    ForKind::Iterable { iterable } => {
                        if let Some(t) = self.infer_name_type_in_expr(name, iterable) {
                            out.push(t);
                        }
                    }
                }
                out.extend(self.collect_param_type_hints(name, &f.body));
            }
            Statement::Print(p) => {
                for arg in &p.args {
                    if let Some(t) = self.infer_name_type_in_expr(name, arg) {
                        out.push(t);
                    }
                }
            }
            Statement::Assign(a) => {
                if let Some(t) = self.infer_struct_from_param_root(name, &a.target) {
                    out.push(t);
                }
                if let Some(t) = self.infer_name_type_in_expr(name, &a.target) {
                    out.push(t);
                }
                if let Some(t) = self.infer_name_type_in_expr(name, &a.value) {
                    out.push(t);
                }
            }
            Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                out.extend(self.collect_param_type_hints(name, b));
            }
            Statement::Defer(e) => {
                if let Some(t) = self.infer_name_type_in_expr(name, e) {
                    out.push(t);
                }
            }
            _ => {}
        }
    }

    pub(super) fn infer_method_receiver_type(&self, method: &str) -> Option<Type> {
        if crate::string_builtins::string_method_borrows_receiver(method)
            && !matches!(method, "len" | "length")
        {
            return Some(Type::String);
        }
        let suffix = format!("_{method}");
        let mut struct_types: Vec<String> = Vec::new();
        for (name, sig) in &self.env.functions {
            let Some(ty_name) = name.strip_suffix(&suffix) else {
                continue;
            };
            if ty_name.is_empty() || !self.structs.contains_key(ty_name) {
                continue;
            }
            if sig
                .params
                .first()
                .is_some_and(|p| *p == Type::Struct(ty_name.to_string()))
            {
                struct_types.push(ty_name.to_string());
            }
        }
        struct_types.sort();
        struct_types.dedup();
        if struct_types.len() == 1 {
            return Some(Type::Struct(struct_types[0].clone()));
        }
        // Fast path before impl sigs are visible in the first registration sweep.
        if matches!(method, "get" | "push") && self.env.functions.contains_key("StrVec_get") {
            return Some(Type::Struct("StrVec".into()));
        }
        None
    }

    pub(super) fn infer_expr_type_hint(
        &self,
        expr: &Expression,
        env: &super::TypeEnv,
    ) -> Option<Type> {
        match expr {
            Expression::Variable { name, .. } => env
                .variables
                .get(name)
                .map(|v| v.ty.clone())
                .filter(|t| *t != Type::Unknown),
            Expression::Literal(_) => self.expr_type_hint(expr),
            Expression::Call(_) => self.expr_type_hint(expr),
            Expression::FieldAccess(f) => {
                let obj_ty = self.infer_expr_type_hint(&f.object, env)?;
                let Type::Struct(name) = obj_ty else {
                    return None;
                };
                self.structs
                    .get(&name)
                    .and_then(|info| info.fields.get(&f.field).cloned())
            }
            Expression::MethodCall(m) => {
                let obj_ty = self.infer_expr_type_hint(&m.object, env)?;
                let type_name = match obj_ty {
                    Type::Struct(n) => n,
                    _ => return None,
                };
                let mangled = self.resolve_method_name(&type_name, &m.method);
                self.env
                    .functions
                    .get(&mangled)
                    .map(|sig| sig.return_type.clone())
                    .filter(|t| *t != Type::Void && *t != Type::Unknown)
            }
            Expression::Grouped(inner) => self.infer_expr_type_hint(inner, env),
            Expression::StructLiteral(sl) if !sl.name.is_empty() => {
                Some(Type::Struct(sl.name.clone()))
            }
            Expression::If(i) => self
                .infer_block_type_hint(&i.then_block, env).or_else(|| self.infer_block_type_hint(&i.else_block, env)),
            Expression::Binary(b) => self
                .infer_expr_type_hint(&b.left, env)
                .or_else(|| self.infer_expr_type_hint(&b.right, env)),
            _ => None,
        }
    }

    pub(super) fn expr_is_param_name(expr: &Expression, name: &str) -> bool {
        Self::expr_bare_param(expr).is_some_and(|n| n == name)
    }

    fn expr_bare_param(expr: &Expression) -> Option<&str> {
        match expr {
            Expression::Variable { name, .. } => Some(name.as_str()),
            Expression::MethodCall(m)
                if m.method == "clone" && m.args.is_empty() && !m.optional =>
            {
                Self::expr_bare_param(&m.object)
            }
            _ => None,
        }
    }

    pub(super) fn infer_name_type_in_call_arg(
        &self,
        param_name: &str,
        call: &CallExpr,
    ) -> Option<Type> {
        if let Some(sig) = self.env.functions.get(&call.callee) {
            for (arg, expected) in call.args.iter().zip(sig.params.iter()) {
                if Self::expr_is_param_name(arg, param_name) && *expected != Type::Unknown {
                    return Some(expected.clone());
                }
            }
        }
        if call.callee == "strcat" && call.args.len() == 2 {
            if Self::expr_is_param_name(&call.args[1], param_name)
                && matches!(&call.args[0], Expression::Literal(Literal::String(_)))
            {
                return Some(Type::String);
            }
            if Self::expr_is_param_name(&call.args[0], param_name)
                && matches!(&call.args[1], Expression::Literal(Literal::String(_)))
            {
                return Some(Type::String);
            }
        }
        let i32 = Type::Integer(ast::IntKind::I32);
        match (call.callee.as_str(), call.args.len()) {
            ("strlen" | "str_pop" | "str_to_i32", 1) | ("read_file" | "write_file" | "file_exists", 1) => {
                if Self::expr_is_param_name(&call.args[0], param_name) {
                    return Some(Type::String);
                }
            }
            ("i32_to_string", 1) => {
                if Self::expr_is_param_name(&call.args[0], param_name) {
                    return Some(i32);
                }
            }
            ("i64_to_string", 1) => {
                if Self::expr_is_param_name(&call.args[0], param_name) {
                    return Some(Type::Integer(ast::IntKind::I64));
                }
            }
            ("char_at" | "strstr_pos", _) if !call.args.is_empty() => {
                if Self::expr_is_param_name(&call.args[0], param_name) {
                    return Some(Type::String);
                }
                if call.args.len() > 1 && Self::expr_is_param_name(&call.args[1], param_name) {
                    return Some(i32.clone());
                }
            }
            ("substring", _) if call.args.len() >= 3 => {
                if Self::expr_is_param_name(&call.args[0], param_name) {
                    return Some(Type::String);
                }
                if Self::expr_is_param_name(&call.args[1], param_name)
                    || Self::expr_is_param_name(&call.args[2], param_name)
                {
                    return Some(i32);
                }
            }
            ("strcmp", 2) => {
                if Self::expr_is_param_name(&call.args[0], param_name)
                    || Self::expr_is_param_name(&call.args[1], param_name)
                {
                    return Some(Type::String);
                }
            }
            _ => {}
        }
        None
    }

    fn block_uses_struct_receiver(&self, param: &str, block: &Block) -> bool {
        let mut found = false;
        for_each_expr_in_block(block, &mut |e| {
            if self.expr_uses_struct_receiver(param, e) {
                found = true;
            }
        });
        found
    }

    fn block_mentions_param(&self, block: &Block, param: &str) -> bool {
        let mut found = false;
        for_each_expr_in_block(block, &mut |e| {
            if self.expr_mentions_param(e, param) {
                found = true;
            }
        });
        found
    }

    fn infer_block_type_hint(&self, block: &Block, env: &super::TypeEnv) -> Option<Type> {
        let mut last = None;
        for_each_expr_in_block(block, &mut |e| {
            if let Some(ty) = self.infer_expr_type_hint(e, env) {
                last = Some(ty);
            }
        });
        last
    }
}
