//! Expression type inference and validation.

use ast::*;
use ast::expr_span;

use super::helpers::{logic_op_name, types_assignable};
use super::{TypeChecker, TypeEnv, VarInfo};
use super::date_builtins::DATE_STRUCT;
use super::diagnostics;
use types::{self, float_assignable, integer_assignable, Type};

/// Peel a single reference so `string` and `&string` compare equal for UFCS args.
fn strip_ref(ty: &Type) -> Type {
    match ty {
        Type::Ref { inner, .. } => (**inner).clone(),
        other => other.clone(),
    }
}

impl TypeChecker {
    pub fn check_expr(&mut self, expr: &Expression, env: &mut TypeEnv) -> Type {
        let sp = expr_span(expr);
        match expr {
            Expression::Literal(lit) => types::literal_type(lit),
            Expression::Variable { name, .. } => {
                if let Some(v) = env.variables.get(name) {
                    v.ty.clone()
                } else if let Some(sig) = env.functions.get(name) {
                    Type::FnPtr {
                        lifetime_params: vec![],
                        params: sig.params.clone(),
                        return_type: Some(Box::new(sig.return_type.clone())),
                    }
                } else {
                    diagnostics::undefined_name(self, name, sp.clone(), env);
                    Type::Unknown
                }
            }
            Expression::Binary(bin) => {
                let left = self.check_expr(&bin.left, env);
                let right = self.check_expr(&bin.right, env);
                match bin.op {
                    BinaryOp::Add | BinaryOp::Sub => {
                        // الـ if الأولى بعد التعديل (مظبوطة بالأقواس)
                        if Self::is_raw_pointer_type(&left)
                            && (types::is_integer(&right) || right == Type::Unknown)
                        {
                            if !self.in_unsafe() {
                                diagnostics::unsafe_required(self, "pointer arithmetic", sp.clone());
                            }
                            return left;
                        }
                        // الـ if الثانية بعد التعديل (مظبوطة بالأقواس)
                        if Self::is_raw_pointer_type(&right)
                            && (types::is_integer(&left) || left == Type::Unknown)
                            && bin.op == BinaryOp::Add
                        {
                            if !self.in_unsafe() {
                                diagnostics::unsafe_required(self, "pointer arithmetic", sp.clone());
                            }
                            return right;
                        }
                        if left == Type::String || right == Type::String {
                            if bin.op == BinaryOp::Add {
                                Type::String
                            } else {
                                diagnostics::string_op_invalid(self, "subtraction", sp.clone());
                                Type::Unknown
                            }
                        } else if !Self::is_numeric_type(&left)
                            || !Self::is_numeric_type(&right)
                        {
                            diagnostics::arithmetic_mismatch(self, sp.clone());
                            Type::Unknown
                        } else {
                            Self::unify_numeric_type(left, right)
                        }
                    }
                    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                        if !Self::is_numeric_type(&left) || !Self::is_numeric_type(&right) {
                            diagnostics::arithmetic_mismatch(self, sp.clone());
                            return Type::Unknown;
                        }
                        Self::unify_numeric_type(left, right)
                    }
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Le
                    | BinaryOp::Ge => {
                        if (left == Type::String && right == Type::String)
                            && matches!(bin.op, BinaryOp::Eq | BinaryOp::Ne)
                        {
                            return Type::Bool;
                        }
                        if left != right
                            && left != Type::Unknown
                            && right != Type::Unknown
                            && (!Self::is_numeric_type(&left) || !Self::is_numeric_type(&right))
                        {
                            diagnostics::comparison_mismatch(self, sp.clone());
                        }
                        Type::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if left != Type::Bool && left != Type::Unknown {
                            diagnostics::bool_operand_required(
                                self,
                                logic_op_name(bin.op),
                                sp.clone(),
                            );
                        }
                        if right != Type::Bool && right != Type::Unknown {
                            diagnostics::bool_operand_required(
                                self,
                                logic_op_name(bin.op),
                                sp.clone(),
                            );
                        }
                        Type::Bool
                    }
                    BinaryOp::Shl
                    | BinaryOp::Shr
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor => {
                        if !types::is_integer(&left) || !types::is_integer(&right) {
                            diagnostics::bitwise_requires_integer(self, sp.clone());
                            return Type::Unknown;
                        }
                        types::integer::unify_integer_types(left, right)
                    }
                    BinaryOp::NullishCoalesce => right,
                } 
            } 
            Expression::Call(call) => {
                if matches!(call.callee.as_str(), "print" | "write" | "println") {
                    if self.no_std {
                        diagnostics::no_std_unavailable(self, &call.callee, sp.clone());
                    }
                    if call.args.is_empty() {
                        diagnostics::builtin_wrong_arity(self, &call.callee, 1, 0, sp.clone());
                    }
                    for arg in &call.args {
                        self.check_io_arg(arg, env, sp.clone(), &call.callee);
                    }
                    return Type::Void;
                }
                if call.callee == "flush" {
                    if !call.args.is_empty() {
                        diagnostics::wrong_arity(
                            self,
                            "flush",
                            0,
                            call.args.len(),
                            sp.clone(),
                        );
                    }
                    return Type::Void;
                }
                if call.callee == "input" {
                    if self.no_std {
                        diagnostics::no_std_unavailable(self, "input", sp.clone());
                    }
                    if call.args.len() > 1 {
                        diagnostics::builtin_wrong_arity_range(
                            self,
                            "input",
                            0,
                            1,
                            call.args.len(),
                            sp.clone(),
                        );
                    } else if call.args.len() == 1 {
                        let arg_ty = self.check_expr(&call.args[0], env);
                        if arg_ty != Type::String && arg_ty != Type::Unknown {
                            diagnostics::builtin_arg_type(
                                self,
                                "input",
                                format!(
                                    "prompt must be `string`, found {}",
                                    diagnostics::type_pretty(&arg_ty),
                                ),
                                sp.clone(),
                            );
                        }
                    }
                    return Type::String;
                }
                if call.callee == "date" {
                    if !call.args.is_empty() {
                        diagnostics::wrong_arity(self, "date", 0, call.args.len(), sp.clone());
                    }
                    return Type::Struct(DATE_STRUCT.to_string());
                }
                if let Some(ret) = self.check_math_intrinsic_call(call, env, sp.clone()) {
                    return ret;
                }
                if let Some(ret) = self.check_random_builtin_call(call, env, sp.clone()) {
                    return ret;
                }
                if let Some(ret) = self.check_layout_intrinsic(
                    &call.callee,
                    &call.args,
                    Some(&call.type_args),
                    &sp,
                    env,
                ) {
                    return ret;
                }
                if matches!(
                    call.callee.as_str(),
                    "time_start" | "time_end" | "mem_start" | "mem_end"
                ) {
                    if call.args.len() != 1 {
                        diagnostics::wrong_arity(
                            self,
                            &call.callee,
                            1,
                            call.args.len(),
                            sp.clone(),
                        );
                    } else {
                        let arg_ty = self.check_expr(&call.args[0], env);
                        if arg_ty != Type::String && arg_ty != Type::Unknown {
                            diagnostics::wrong_arg_type(
                                self,
                                &call.callee,
                                format!(
                                    "expected string label, found {}",
                                    diagnostics::type_pretty(&arg_ty),
                                ),
                                sp.clone(),
                            );
                        }
                    }
                    return Type::Void;
                }
                // Locals (params, fn-ptr bindings) shadow global functions with the same name.
                if let Some(var) = env.variables.get(&call.callee) {
                    if let Type::FnPtr {
                        params,
                        return_type,
                        ..
                    } = &var.ty
                    {
                        let fp_params = params.clone();
                        let fp_ret = return_type.as_ref().map(|t| (**t).clone()).unwrap_or(Type::Void);
                        if call.args.len() != fp_params.len() {
                            diagnostics::fn_ptr_wrong_arity(
                                self,
                                &call.callee,
                                fp_params.len(),
                                call.args.len(),
                                sp.clone(),
                            );
                        }
                        for (arg, expected) in call.args.iter().zip(fp_params.iter()) {
                            let at = self.check_expr(arg, env);
                            if !types_assignable(&at, expected) {
                                diagnostics::fn_ptr_arg_mismatch(self, &call.callee, sp.clone());
                            }
                        }
                        fp_ret
                    } else {
                        diagnostics::not_callable(self, &call.callee, sp.clone());
                        Type::Unknown
                    }
                } else if let Some(sig) = env.functions.get(&call.callee).cloned() {
                    if call.args.len() != sig.params.len() {
                        diagnostics::wrong_arity(
                            self,
                            &call.callee,
                            sig.params.len(),
                            call.args.len(),
                            sp.clone(),
                        );
                    }
                    for (i, (arg, expected)) in call.args.iter().zip(sig.params.iter()).enumerate() {
                        let at = self.check_expr(arg, env);
                        if !types_assignable(&at, expected) && !self.signature_inference {
                            diagnostics::wrong_arg_type(
                                self,
                                &call.callee,
                                format!(
                                    "argument {}: expected {}, found {}",
                                    i,
                                    diagnostics::type_pretty(expected),
                                    diagnostics::type_pretty(&at),
                                ),
                                expr_span(arg),
                            );
                        }
                    }
                    sig.return_type
                } else {
                    if self.signature_inference {
                        return self
                            .infer_expr_type_hint(expr, env)
                            .unwrap_or(Type::Unknown);
                    }
                    diagnostics::undefined_function(self, &call.callee, sp.clone(), env);
                    Type::Unknown
                }
            }
            Expression::Grouped(inner) => self.check_expr(inner, env),
            Expression::TupleLiteral(elems) => {
                let types: Vec<Type> = elems.iter().map(|el| self.check_expr(el, env)).collect();
                Type::Tuple { elems: types }
            }
            Expression::FieldAccess(fa) => {
                let obj_ty = self.check_expr(&fa.object, env);
                let resolved = match &obj_ty {
                    Type::Ref { inner, .. } => (**inner).clone(),
                    other => other.clone(),
                };
                match resolved {
                    Type::Struct(name) => {
                        let field = TypeChecker::resolve_date_field(&name, &fa.field);
                        if let Some(info) = self.structs.get(&name) {
                            if let Some(ft) = info.fields.get(field) {
                                ft.clone()
                            } else {
                                let known: Vec<String> =
                                    info.fields.keys().cloned().collect();
                                diagnostics::unknown_struct_field(
                                    self,
                                    &name,
                                    &fa.field,
                                    &known,
                                    sp.clone(),
                                );
                                Type::Unknown
                            }
                        } else {
                            Type::Unknown
                        }
                    }
                    Type::Union(name) => {
                        if !self.in_unsafe() {
                            diagnostics::unsafe_required(
                                self,
                                &format!("reading union field `{}`", fa.field),
                                sp.clone(),
                            );
                        }
                        if let Some(info) = self.unions.get(&name) {
                            if let Some(ft) = info.fields.get(&fa.field) {
                                ft.clone()
                            } else {
                                diagnostics::unknown_union_field(
                                    self,
                                    &name,
                                    &fa.field,
                                    sp.clone(),
                                );
                                Type::Unknown
                            }
                        } else {
                            Type::Unknown
                        }
                    }
                    Type::Tuple { elems } => {
                        if let Ok(idx) = fa.field.parse::<usize>() {
                            elems.get(idx).cloned().unwrap_or_else(|| {
                                diagnostics::tuple_missing_index(self, idx, sp.clone());
                                Type::Unknown
                            })
                        } else {
                            diagnostics::tuple_index_not_number(self, sp.clone());
                            Type::Unknown
                        }
                    }
                    _ => {
                        diagnostics::field_access_invalid_receiver(self, sp.clone());
                        Type::Unknown
                    }
                }
            }
            Expression::StructLiteral(sl) => {
                if sl.name.is_empty() {
                    return self.check_anonymous_struct_literal(sl, env, sp);
                }
                if self.structs.contains_key(&sl.name) {
                    let struct_fields = self.structs.get(&sl.name).cloned();
                    let explicit: std::collections::HashSet<&str> = sl
                        .fields
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect();
                    let mut spread_types: Vec<Type> = Vec::new();
                    for spread in &sl.spreads {
                        let spread_ty = self.check_expr(spread, env);
                        match &spread_ty {
                            Type::Struct(_) | Type::Unknown => {}
                            other => {
                                diagnostics::struct_spread_requires_struct(
                                    self,
                                    &sl.name,
                                    other,
                                    sp.clone(),
                                );
                            }
                        }
                        spread_types.push(spread_ty);
                    }
                    if let Some(def) = struct_fields.as_ref() {
                        for (fname, expected) in &def.fields {
                            if explicit.contains(fname.as_str()) {
                                continue;
                            }
                            let mut found = false;
                            for spread_ty in &spread_types {
                                let spread_field = match spread_ty {
                                    Type::Struct(src_name) => self
                                        .structs
                                        .get(src_name)
                                        .and_then(|src_def| {
                                            src_def
                                                .fields
                                                .get(fname)
                                                .map(|ty| (src_name.clone(), ty.clone()))
                                        }),
                                    _ => None,
                                };
                                if let Some((src_name, src_field_ty)) = spread_field {
                                    if src_field_ty != *expected
                                        && src_field_ty != Type::Unknown
                                        && *expected != Type::Unknown
                                        && !integer_assignable(expected, &src_field_ty)
                                    {
                                        diagnostics::struct_field_spread_mismatch(
                                            self,
                                            fname,
                                            &src_name,
                                            &sl.name,
                                            &src_field_ty,
                                            expected,
                                            sp.clone(),
                                        );
                                    }
                                    found = true;
                                    break;
                                }
                            }
                            if !found && spread_types.iter().all(|t| *t != Type::Unknown) {
                                diagnostics::struct_field_not_set(
                                    self,
                                    fname,
                                    &sl.name,
                                    sp.clone(),
                                );
                            }
                        }
                    }
                    let mut seen = std::collections::HashSet::new();
                    for (fname, fexpr) in &sl.fields {
                        if !seen.insert(fname.clone()) {
                            diagnostics::duplicate_struct_field(self, fname, sp.clone());
                        }
                        let expected = struct_fields
                            .as_ref()
                            .and_then(|i| i.fields.get(fname))
                            .cloned();
                        let got = self.check_expr(fexpr, env);
                        if let Some(exp) = expected {
                            if got != exp
                                && got != Type::Unknown
                                && !integer_assignable(&exp, &got)
                                && !types::float_assignable(&exp, &got)
                            {
                                diagnostics::type_mismatch(
                                    self,
                                    &format!("for field `{fname}` in struct `{}`", sl.name),
                                    &exp,
                                    &got,
                                    sp.clone(),
                                );
                            }
                        } else {
                            diagnostics::unknown_literal_field(self, &sl.name, fname, sp.clone());
                        }
                    }
                    Type::Struct(sl.name.clone())
                } else if self.unions.contains_key(&sl.name) {
                    if !self.in_unsafe() {
                        diagnostics::union_construct_requires_unsafe(self, &sl.name, sp.clone());
                    }
                    for (fname, fexpr) in &sl.fields {
                        let expected = self
                            .unions
                            .get(&sl.name)
                            .and_then(|u| u.fields.get(fname))
                            .cloned();
                        let got = self.check_expr(fexpr, env);
                        if let Some(exp) = expected {
                            if got != exp
                                && got != Type::Unknown
                                && !integer_assignable(&exp, &got)
                                && !types::float_assignable(&exp, &got)
                            {
                                diagnostics::type_mismatch(
                                    self,
                                    &format!("for field `{fname}` in union `{}`", sl.name),
                                    &exp,
                                    &got,
                                    sp.clone(),
                                );
                            }
                        } else {
                            diagnostics::unknown_union_field(self, &sl.name, fname, sp.clone());
                        }
                    }
                    Type::Union(sl.name.clone())
                } else {
                    diagnostics::unknown_struct(self, &sl.name, sp.clone(), env);
                    Type::Unknown
                }
            }
            Expression::Match(m) => self.check_match(m, env),
            Expression::If(i) => {
                let c = self.check_expr(&i.condition, env);
                if c != Type::Bool && c != Type::Unknown {
                    diagnostics::bool_condition_required(self, "if expression", sp.clone());
                }
                let t = self.check_block_expr_value(&i.then_block, env, &sp);
                let e = self.check_block_expr_value(&i.else_block, env, &sp);
                if t != e && t != Type::Unknown && e != Type::Unknown {
                    diagnostics::branch_type_mismatch(self, sp.clone());
                }
                if t != Type::Unknown {
                    t
                } else {
                    e
                }
            }
            Expression::Index(ix) => {
                let obj = self.check_expr(&ix.object, env);
                let idx = self.check_expr(&ix.index, env);
                if !types::is_integer(&idx) && idx != Type::Unknown {
                    diagnostics::array_index_must_be_i32(self, sp.clone());
                }
                match obj {
                    Type::Array { elem, .. } => (*elem).clone(),
                    Type::Bytes => TypeChecker::check_bytes_index(self, &obj, &sp),
                    _ => {
                        diagnostics::index_requires_array_or_bytes(self, sp.clone());
                        Type::Unknown
                    }
                }
            }
            Expression::ArrayLiteral(al) => {
                if al.is_empty() {
                    return Type::Array {
                        elem: Box::new(Type::Unknown),
                        len: Some(0),
                    };
                }
                let mut elem_ty = Type::Unknown;
                let mut total_len = 0usize;
                for spread in &al.spreads {
                    let spread_ty = self.check_expr(spread, env);
                    match &spread_ty {
                        Type::Array { elem, len: Some(n) } => {
                            total_len += n;
                            if elem_ty == Type::Unknown {
                                elem_ty = (**elem).clone();
                            } else if **elem != elem_ty
                                && **elem != Type::Unknown
                                && elem_ty != Type::Unknown
                                && !integer_assignable(&elem_ty, elem)
                                && !float_assignable(&elem_ty, elem)
                            {
                                diagnostics::array_spread_homogeneous(self, sp.clone());
                            }
                        }
                        Type::Struct(name) => {
                            let spread_fields: Vec<(String, Type)> = self
                                .structs
                                .get(name)
                                .map(|def| {
                                    def.field_order
                                        .iter()
                                        .filter_map(|fname| {
                                            def.fields
                                                .get(fname)
                                                .map(|fty| (fname.clone(), fty.clone()))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            total_len += spread_fields.len();
                            for (fname, fty) in spread_fields {
                                if elem_ty == Type::Unknown {
                                    elem_ty = fty;
                                } else if fty != elem_ty
                                    && fty != Type::Unknown
                                    && elem_ty != Type::Unknown
                                    && !integer_assignable(&elem_ty, &fty)
                                    && !float_assignable(&elem_ty, &fty)
                                {
                                    diagnostics::type_mismatch(
                                        self,
                                        &format!("for spread field `{fname}` into array"),
                                        &elem_ty,
                                        &fty,
                                        sp.clone(),
                                    );
                                }
                            }
                        }
                        Type::Unknown => {}
                        other => {
                            diagnostics::array_spread_invalid_source(self, other, sp.clone());
                        }
                    }
                }
                for el in &al.elems {
                    let t = self.check_expr(el, env);
                    total_len += 1;
                    if elem_ty == Type::Unknown {
                        elem_ty = t;
                    } else if t != elem_ty && t != Type::Unknown {
                        if !integer_assignable(&elem_ty, &t) && !float_assignable(&elem_ty, &t) {
                            diagnostics::array_homogeneous_elements(self, sp.clone());
                        }
                    }
                }
                Type::Array {
                    elem: Box::new(elem_ty),
                    len: Some(total_len),
                }
            }
            Expression::ArrayRepeat { element, count, .. } => {
                let elem_ty = self.check_expr(element, env);
                Type::Array {
                    elem: Box::new(elem_ty),
                    len: Some(*count),
                }
            }
            Expression::EnumVariant(ev) => {
                if let Some(en) = &ev.enum_name {
                    if let Some(info) = self.enums.get(en) {
                        if let Some(vinfo) = info.variants.iter().find(|v| v.name == ev.variant) {
                            let expected_fields = vinfo.fields.clone();
                            if expected_fields.len() != ev.args.len() {
                                diagnostics::enum_variant_wrong_arity(
                                    self,
                                    &en,
                                    &ev.variant,
                                    expected_fields.len(),
                                    ev.args.len(),
                                    sp.clone(),
                                );
                            }
                            for (expected, arg) in expected_fields.iter().zip(ev.args.iter()) {
                                let got = self.check_expr(arg, env);
                                if got != *expected && got != Type::Unknown && *expected != Type::Unknown
                                {
                                    diagnostics::enum_variant_payload_mismatch(
                                        self,
                                        &en,
                                        &ev.variant,
                                        sp.clone(),
                                    );
                                }
                            }
                        }
                    }
                    Type::Enum(en.clone())
                } else {
                    Type::Unknown
                }
            }
            Expression::MethodCall(mc) => {
                let obj_ty = self.check_expr(&mc.object, env);
                let obj_ty = match obj_ty {
                    Type::Ref { inner, .. } => (*inner).clone(),
                    other => other,
                };
                if mc.method == "join" {
                    if !mc.args.is_empty() {
                        diagnostics::method_expects_no_args(self, "join", sp.clone());
                    }
                    if obj_ty != Type::JoinHandle && obj_ty != Type::Unknown {
                        diagnostics::unsupported_method_on_type(
                            self,
                            "join",
                            &obj_ty,
                            sp.clone(),
                        );
                        return Type::Unknown;
                    }
                    return Type::Void;
                }
                if mc.method == "clone" {
                    return match obj_ty {
                        Type::String => Type::String,
                        Type::Struct(n) if self.struct_has_clone(&n) => Type::Struct(n),
                        _ => {
                            if self.signature_inference {
                                return Type::Unknown;
                            }
                            diagnostics::unsupported_method_on_type(self, "clone", &obj_ty, sp.clone());
                            Type::Unknown
                        }
                    };
                }
                if mc.method == "length" || mc.method == "len" {
                    if !mc.args.is_empty() {
                        diagnostics::method_expects_no_args(self, &mc.method, sp.clone());
                    }
                    if !matches!(obj_ty, Type::Struct(_)) {
                        return match obj_ty {
                            Type::Bytes => Type::Integer(ast::IntKind::I64),
                            Type::String | Type::VecStr => Type::Integer(ast::IntKind::I32),
                            Type::Array { .. } => {
                                if let Some(ret) = self.check_array_method(mc, &obj_ty, env, &sp) {
                                    return ret;
                                }
                                Type::Unknown
                            }
                            _ => {
                                if self.signature_inference {
                                    return Type::Unknown;
                                }
                                diagnostics::unsupported_method_on_type(
                                    self,
                                    "length",
                                    &obj_ty,
                                    sp.clone(),
                                );
                                Type::Unknown
                            }
                        };
                    }
                }
                if let Some(ret) = self.check_array_method(mc, &obj_ty, env, &sp) {
                    return ret;
                }
                if let Some(ret) =
                    self.check_string_method(mc, &obj_ty, env, &sp)
                {
                    return ret;
                }
                if obj_ty == Type::Bytes {
                    if let Some(ret) = TypeChecker::bytes_method_return_type(&mc.method) {
                        if !mc.args.is_empty() {
                            diagnostics::method_expects_no_args(self, &mc.method, sp.clone());
                        }
                        return ret;
                    }
                }
                if let Some(param) = match &obj_ty {
                    Type::Generic(p) => Some(p.as_str()),
                    Type::Struct(n) if self.current_type_param_bounds.contains_key(n) => {
                        Some(n.as_str())
                    }
                    _ => None,
                } {
                    if let Some(ret) = self.check_generic_bound_method(mc, param, env, &sp) {
                        return ret;
                    }
                }
                if matches!(obj_ty, Type::String | Type::Unknown) {
                    // JS-style UFCS on strings: `name.toUpperCase()` dispatches to
                    // the stdlib free function `String_toUpperCase(name)`. Also
                    // accepts the already-qualified spelling (`name.String_toUpperCase()`).
                    let candidates =
                        [format!("String_{}", mc.method), mc.method.clone()];
                    for cand in candidates {
                        if let Some(sig) = env.functions.get(&cand).cloned() {
                            let mut args = vec![mc.object.clone()];
                            args.extend(mc.args.clone());
                            for (arg, expected) in args.iter().zip(sig.params.iter()) {
                                let at = self.check_expr(arg, env);
                                // A `string` receiver/argument satisfies a `&string`
                                // parameter, so compare after stripping references.
                                let at = strip_ref(&at);
                                let expected = strip_ref(expected);
                                if at != expected
                                    && at != Type::Unknown
                                    && expected != Type::Unknown
                                {
                                    diagnostics::method_arg_mismatch(self, &mc.method, sp.clone());
                                }
                            }
                            return sig.return_type;
                        }
                    }
                }
                let type_name = match obj_ty {
                    Type::Struct(n) => n,
                    _ => {
                        diagnostics::method_receiver_requires_struct(self, sp.clone());
                        return Type::Unknown;
                    }
                };
                if type_name.starts_with("Dyn_") {
                    if let Some(traits) = self.dyn_traits_of(&type_name) {
                        if !self.dyn_combo_has_method(&traits, &mc.method) {
                            let dyn_label = format_dyn_trait(&traits, &[]);
                            diagnostics::trait_method_not_found(
                                self,
                                &dyn_label,
                                &mc.method,
                                sp.clone(),
                            );
                            return Type::Unknown;
                        }
                    }
                }
                let mangled = self.resolve_method_name(&type_name, &mc.method);
                let mut args = vec![mc.object.clone()];
                args.extend(mc.args.clone());
                if let Some(sig) = env.functions.get(&mangled).cloned() {
                    for (arg, expected) in args.iter().zip(sig.params.iter()) {
                        let at = self.check_expr(arg, env);
                        if at != *expected && at != Type::Unknown && *expected != Type::Unknown {
                            diagnostics::method_arg_mismatch(self, &mc.method, sp.clone());
                        }
                    }
                    sig.return_type
                } else {
                    diagnostics::unknown_method(self, &mc.method, sp.clone());
                    Type::Unknown
                }
            }
            Expression::Unary(u) => {
                let inner = self.check_expr(&u.operand, env);
                match u.op {
                    UnaryOp::Ref | UnaryOp::RefMut => Type::Ref {
                        inner: Box::new(inner),
                        mutable: u.op == UnaryOp::RefMut,
                        lifetime: Some("'local".into()),
                    },
                    UnaryOp::Deref => match inner {
                        Type::Ref { inner, .. } => (*inner).clone(),
                        Type::RawPtr { inner } if self.in_unsafe() => (*inner).clone(),
                        Type::Ptr if self.in_unsafe() => Type::Integer(ast::IntKind::I32),
                        Type::RawPtr { .. } | Type::Ptr => {
                            diagnostics::unsafe_required(self, "deref of raw pointer", sp.clone());
                            Type::Unknown
                        }
                        _ => {
                            diagnostics::deref_requires_ref_or_ptr(self, sp.clone());
                            Type::Unknown
                        }
                    },
                    UnaryOp::Neg => {
                        if !types::is_integer(&inner) && inner != Type::Unknown {
                            diagnostics::unary_requires_i32(self, sp.clone());
                        }
                        Type::Integer(ast::IntKind::I32)
                    }
                    UnaryOp::Not => {
                        if inner != Type::Bool && inner != Type::Unknown {
                            diagnostics::unary_requires_bool(self, sp.clone());
                        }
                        Type::Bool
                    }
                    UnaryOp::Move | UnaryOp::Clone | UnaryOp::Try => inner,
                }
            }
            Expression::Await(inner) => {
                if self.target_is_wasm() {
                    diagnostics::platform_unavailable(self, "await", "wasm32", sp.clone());
                }
                let t = self.check_expr(inner, env);
                if let Some(result) = super::future_types::future_await_result_type(&t) {
                    return result;
                }
                if !super::future_types::is_future_handle_type(&t) && t != Type::Unknown {
                    diagnostics::await_wrong_type(self, &t, expr_span(inner));
                }
                Type::Integer(ast::IntKind::I32)
            }
            Expression::TemplateLiteral(t) => {
                for part in &t.parts {
                    if let TemplatePart::Interpolation(expr) = part {
                        let ty = self.check_expr(expr, env);
                        if !types::is_print_scalar(&ty) {
                            diagnostics::template_interpolation_invalid(self, &ty, t.span.clone());
                        }
                    }
                }
                Type::String
            }
            Expression::Cast(c) => {
                let from = self.check_expr(&c.expr, env);
                let to = self.type_from_ann(&c.target_type);
                let ptr_to_fn = matches!(from, Type::Ptr) && matches!(to, Type::FnPtr { .. });
                if (Self::is_raw_pointer_type(&to) || Self::is_raw_pointer_type(&from))
                    && !ptr_to_fn
                    && !self.in_unsafe() {
                        diagnostics::unsafe_required(self, "raw pointer cast", c.span.clone());
                    }
                if from != Type::Unknown && to != Type::Unknown && from == to {
                    return to;
                }
                let numeric = |t: &Type| {
                    types::is_integer(t) || *t == Type::Bool
                };
                if numeric(&from) && numeric(&to) {
                    return to;
                }
                if matches!(from, Type::Ref { .. }) && Self::is_raw_pointer_type(&to) {
                    return to;
                }
                if Self::is_raw_pointer_type(&from) && Self::is_raw_pointer_type(&to) {
                    return to;
                }
                if matches!(from, Type::Ptr | Type::RawPtr { .. })
                    && matches!(to, Type::FnPtr { .. })
                {
                    return to;
                }
                if let TypeAnnotation::DynTrait { traits, auto_bounds } = &c.target_type {
                    let dyn_label = format_dyn_trait(traits, auto_bounds);
                    if let Type::Struct(concrete) = &from {
                        let mut all_impl = true;
                        for trait_name in traits {
                            if !self.trait_impl_exists(trait_name, concrete) {
                                diagnostics::trait_not_implemented(
                                    self,
                                    concrete,
                                    trait_name,
                                    c.span.clone(),
                                );
                                all_impl = false;
                            }
                        }
                        if all_impl {
                            for b in auto_bounds {
                                match b.as_str() {
                                    "Send" if !self.type_is_send(&from) => {
                                        diagnostics::send_bound_required(
                                            self,
                                            concrete,
                                            &dyn_label,
                                            c.span.clone(),
                                        );
                                    }
                                    "Sync" if !self.type_is_sync(&from) => {
                                        diagnostics::sync_bound_required(
                                            self,
                                            concrete,
                                            &dyn_label,
                                            c.span.clone(),
                                        );
                                    }
                                    "Send" | "Sync" => {}
                                    other => {
                                        diagnostics::unknown_dyn_auto_trait(
                                            self,
                                            other,
                                            &dyn_label,
                                            c.span.clone(),
                                        );
                                    }
                                }
                            }
                        }
                    } else if from != Type::Unknown {
                        diagnostics::trait_object_cast_requires_struct(self, c.span.clone());
                    }
                    return Type::Struct(dyn_struct_name(traits));
                }
                if from != Type::Unknown && to != Type::Unknown {
                    diagnostics::invalid_cast(self, &from, &to, c.span.clone());
                }
                to
            }
            Expression::Spawn { body, .. } => {
                if self.no_std {
                    diagnostics::no_std_unavailable(self, "spawn", sp.clone());
                }
                if self.target_is_wasm() {
                    diagnostics::platform_unavailable(self, "spawn", "wasm32", sp.clone());
                }
                self.check_block(body, env, &Type::Void);
                Type::JoinHandle
            }
            Expression::ParallelSearch(ps) => self.check_parallel_search(ps, env),
            Expression::ComptimeBlock { body, span } => {
                let mut inner = TypeEnv {
                    variables: env.variables.clone(),
                    functions: env.functions.clone(),
                };
                let mut last_ty = Type::Unknown;
                for stmt in &body.statements {
                    match stmt {
                        Statement::Expression(e) => {
                            last_ty = self.check_expr(e, &mut inner);
                        }
                        Statement::Return(r) => {
                            if let Some(v) = &r.value {
                                return self.check_expr(v, &mut inner);
                            }
                        }
                        _ => self.check_statement(stmt, &mut inner, &Type::Unknown),
                    }
                }
                if last_ty == Type::Unknown {
                    diagnostics::comptime_must_produce_value(self, span.clone());
                }
                last_ty
            }
            Expression::ArrowFn(a) => {
                let mut inner = TypeEnv {
                    variables: env.variables.clone(),
                    functions: env.functions.clone(),
                };
                let is_inferred = |ty: &TypeAnnotation| {
                    matches!(ty, TypeAnnotation::Generic(n) if n == "_")
                };
                let mut param_types: Vec<Type> = a
                    .params
                    .iter()
                    .map(|p| {
                        if is_inferred(&p.ty) {
                            Type::Unknown
                        } else {
                            self.type_from_ann(&p.ty)
                        }
                    })
                    .collect();
                for (p, ty) in a.params.iter().zip(param_types.iter()) {
                    if p.destructure.is_empty() {
                        inner.variables.insert(
                            p.name.clone(),
                            VarInfo {
                                ty: ty.clone(),
                                mutable: p.mutable,
                            },
                        );
                    } else if let Type::Tuple { elems } = ty {
                        inner.variables.insert(
                            p.name.clone(),
                            VarInfo {
                                ty: ty.clone(),
                                mutable: p.mutable,
                            },
                        );
                        for (name, elem_ty) in p.destructure.iter().zip(elems.iter()) {
                            inner.variables.insert(
                                name.clone(),
                                VarInfo {
                                    ty: elem_ty.clone(),
                                    mutable: false,
                                },
                            );
                        }
                    }
                }
                let ret_ty = match &a.body {
                    ArrowBody::Expr(e) => self.check_expr(e, &mut inner),
                    ArrowBody::Block(b) => {
                        self.check_block(b, &mut inner, &Type::Unknown);
                        let mut ret = Type::Integer(ast::IntKind::I32);
                        for stmt in &b.statements {
                            if let Statement::Return(r) = stmt {
                                ret = if let Some(v) = &r.value {
                                    self.check_expr(v, &mut inner)
                                } else {
                                    Type::Void
                                };
                            }
                        }
                        ret
                    }
                };
                for (i, p) in a.params.iter().enumerate() {
                    if is_inferred(&p.ty) {
                        if let Some(inferred) = self.infer_arrow_param_type(p, &a.body) {
                            param_types[i] = inferred.clone();
                            if p.destructure.is_empty() {
                                if let Some(info) = inner.variables.get_mut(&p.name) {
                                    info.ty = inferred;
                                }
                            } else if let Type::Tuple { elems } = &inferred {
                                for (name, elem_ty) in p.destructure.iter().zip(elems.iter()) {
                                    if let Some(info) = inner.variables.get_mut(name) {
                                        info.ty = elem_ty.clone();
                                    }
                                }
                                if let Some(info) = inner.variables.get_mut(&p.name) {
                                    info.ty = inferred;
                                }
                            }
                        } else if param_types[i] == Type::Unknown {
                            param_types[i] = Type::Integer(ast::IntKind::I32);
                        }
                    }
                }
                Type::FnPtr {
                    lifetime_params: vec![],
                    params: param_types,
                    return_type: Some(Box::new(ret_ty)),
                }
            }
            Expression::Invalid => Type::Unknown,
        }
    }
}

