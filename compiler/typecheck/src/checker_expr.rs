//! Expression type inference and validation.

use ast::*;
use ast::expr_span;
use errors::{ErrorKind, NyraError};

use super::helpers::{logic_op_name, types_assignable};
use super::{TypeChecker, TypeEnv, VarInfo};
use super::date_builtins::DATE_STRUCT;
use super::diagnostics;
use types::{self, float_assignable, integer_assignable, Type};

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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    "Pointer arithmetic requires unsafe",
                                ));
                            }
                            return left;
                        }
                        // الـ if الثانية بعد التعديل (مظبوطة بالأقواس)
                        if Self::is_raw_pointer_type(&right)
                            && (types::is_integer(&left) || left == Type::Unknown)
                            && bin.op == BinaryOp::Add
                        {
                            if !self.in_unsafe() {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    "Pointer arithmetic requires unsafe",
                                ));
                            }
                            return right;
                        }
                        if left == Type::String || right == Type::String {
                            if bin.op == BinaryOp::Add {
                                Type::String
                            } else {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    "Invalid operation on string",
                                ));
                                Type::Unknown
                            }
                        } else if !Self::is_numeric_type(&left)
                            || !Self::is_numeric_type(&right)
                        {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Type mismatch in arithmetic operation",
                            ));
                            Type::Unknown
                        } else {
                            Self::unify_numeric_type(left, right)
                        }
                    }
                    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                        if !Self::is_numeric_type(&left) || !Self::is_numeric_type(&right) {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Type mismatch in arithmetic operation",
                            ));
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
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Type mismatch in comparison",
                            ));
                        }
                        Type::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if left != Type::Bool && left != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("'{}' requires bool operands", logic_op_name(bin.op)),
                            ));
                        }
                        if right != Type::Bool && right != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("'{}' requires bool operands", logic_op_name(bin.op)),
                            ));
                        }
                        Type::Bool
                    }
                    BinaryOp::Shl
                    | BinaryOp::Shr
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor => {
                        if !types::is_integer(&left) || !types::is_integer(&right) {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Bitwise operators require integer operands",
                            ));
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
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!(
                                "'{}' is not available in no_std programs (use extern I/O or UART)",
                                call.callee
                            ),
                        ));
                    }
                    if call.args.is_empty() {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!("'{}' expects at least 1 argument", call.callee),
                        ));
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
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            "'input' is not available in no_std programs (use extern I/O or UART)",
                        ));
                    }
                    if call.args.len() > 1 {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!(
                                "'input' expects 0 or 1 arguments, got {}",
                                call.args.len()
                            ),
                        ));
                    } else if call.args.len() == 1 {
                        let arg_ty = self.check_expr(&call.args[0], env);
                        if arg_ty != Type::String && arg_ty != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("'input' prompt must be a string, got {:?}", arg_ty),
                            ));
                        }
                    }
                    return Type::String;
                }
                if call.callee == "date" {
                    if !call.args.is_empty() {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!(
                                "'date' expects 0 arguments, got {}",
                                call.args.len()
                            ),
                        ));
                    }
                    return Type::Struct(DATE_STRUCT.to_string());
                }
                if let Some(ret) = self.check_math_intrinsic_call(call, env, sp.clone()) {
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
                if let Some(sig) = env.functions.get(&call.callee).cloned() {
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
                } else if let Some(var) = env.variables.get(&call.callee) {
                    if let Type::FnPtr {
                        params,
                        return_type,
                        ..
                    } = &var.ty
                    {
                        let fp_params = params.clone();
                        let fp_ret = return_type.as_ref().map(|t| (**t).clone()).unwrap_or(Type::Void);
                        if call.args.len() != fp_params.len() {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!(
                                    "Function pointer '{}' expects {} arguments, got {}",
                                    call.callee,
                                    fp_params.len(),
                                    call.args.len()
                                ),
                            ));
                        }
                        for (arg, expected) in call.args.iter().zip(fp_params.iter()) {
                            let at = self.check_expr(arg, env);
                            if !types_assignable(&at, expected) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!(
                                        "Argument type mismatch calling function pointer '{}'",
                                        call.callee
                                    ),
                                ));
                            }
                        }
                        fp_ret
                    } else {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!("'{}' is not callable", call.callee),
                        ));
                        Type::Unknown
                    }
                } else {
                    if self.signature_inference {
                        return self
                            .infer_expr_type_hint(expr, env)
                            .unwrap_or(Type::Unknown);
                    }
                    self.errors.push(NyraError::new(
                        ErrorKind::NameResolution,
                        sp.clone(),
                        format!("Undefined function '{}'", call.callee),
                    ));
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!("Struct '{name}' has no field '{}'", fa.field),
                                ));
                                Type::Unknown
                            }
                        } else {
                            Type::Unknown
                        }
                    }
                    Type::Tuple { elems } => {
                        if let Ok(idx) = fa.field.parse::<usize>() {
                            elems.get(idx).cloned().unwrap_or_else(|| {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!("Tuple has no field index {idx}"),
                                ));
                                Type::Unknown
                            })
                        } else {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Tuple field index must be a number",
                            ));
                            Type::Unknown
                        }
                    }
                    _ => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            "Field access requires struct or tuple value",
                        ));
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!(
                                        "Struct spread '..expr' on '{}' requires a struct value, got {:?}",
                                        sl.name, other
                                    ),
                                ));
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
                                if let Type::Struct(src_name) = spread_ty {
                                    if let Some(src_def) = self.structs.get(src_name) {
                                        if let Some(src_field_ty) = src_def.fields.get(fname) {
                                            if *src_field_ty != *expected
                                                && *src_field_ty != Type::Unknown
                                                && *expected != Type::Unknown
                                                && !integer_assignable(expected, src_field_ty)
                                            {
                                                self.errors.push(NyraError::new(
                                                    ErrorKind::Type,
                                                    sp.clone(),
                                                    format!(
                                                        "Field '{fname}' spread from '{src_name}' has type {:?}, expected {:?} on '{}'",
                                                        src_field_ty, expected, sl.name
                                                    ),
                                                ));
                                            }
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            if !found && spread_types.iter().all(|t| *t != Type::Unknown) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!(
                                        "Field '{fname}' on '{}' not set explicitly or via struct spread",
                                        sl.name
                                    ),
                                ));
                            }
                        }
                    }
                    let mut seen = std::collections::HashSet::new();
                    for (fname, fexpr) in &sl.fields {
                        if !seen.insert(fname.clone()) {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("Duplicate field '{fname}' in struct literal"),
                            ));
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!(
                                        "Field '{fname}' in struct '{}' expected {:?}, got {:?}",
                                        sl.name, exp, got
                                    ),
                                ));
                            }
                        } else {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("Unknown field '{fname}' on struct '{}'", sl.name),
                            ));
                        }
                    }
                    Type::Struct(sl.name.clone())
                } else {
                    diagnostics::unknown_struct(self, &sl.name, sp.clone(), env);
                    Type::Unknown
                }
            }
            Expression::Match(m) => self.check_match(m, env),
            Expression::If(i) => {
                let c = self.check_expr(&i.condition, env);
                if c != Type::Bool && c != Type::Unknown {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "If expression condition must be bool",
                    ));
                }
                let t = self.check_block_expr_value(&i.then_block, env, &sp);
                let e = self.check_block_expr_value(&i.else_block, env, &sp);
                if t != e && t != Type::Unknown && e != Type::Unknown {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "If expression branches must have the same type",
                    ));
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "Array index must be i32",
                    ));
                }
                match obj {
                    Type::Array { elem, .. } => (*elem).clone(),
                    _ => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            "Index requires array value",
                        ));
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    "Array spread elements must have the same type",
                                ));
                            }
                        }
                        Type::Struct(name) => {
                            if let Some(def) = self.structs.get(name) {
                                total_len += def.field_order.len();
                                for fname in &def.field_order {
                                    let fty = def
                                        .fields
                                        .get(fname)
                                        .cloned()
                                        .unwrap_or(Type::Unknown);
                                    if elem_ty == Type::Unknown {
                                        elem_ty = fty;
                                    } else if fty != elem_ty
                                        && fty != Type::Unknown
                                        && elem_ty != Type::Unknown
                                        && !integer_assignable(&elem_ty, &fty)
                                        && !float_assignable(&elem_ty, &fty)
                                    {
                                        self.errors.push(NyraError::new(
                                            ErrorKind::Type,
                                            sp.clone(),
                                            format!(
                                                "Object spread into array: field '{fname}' has type {:?}, expected {:?}",
                                                fty, elem_ty
                                            ),
                                        ));
                                    }
                                }
                            }
                        }
                        Type::Unknown => {}
                        other => {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!(
                                    "Array spread `...expr` requires an array or struct value, got {:?}",
                                    other
                                ),
                            ));
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
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Array elements must have the same type",
                            ));
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!(
                                        "Variant '{}.{}' expects {} args, got {}",
                                        en,
                                        ev.variant,
                                        expected_fields.len(),
                                        ev.args.len()
                                    ),
                                ));
                            }
                            for (expected, arg) in expected_fields.iter().zip(ev.args.iter()) {
                                let got = self.check_expr(arg, env);
                                if got != *expected && got != Type::Unknown && *expected != Type::Unknown
                                {
                                    self.errors.push(NyraError::new(
                                        ErrorKind::Type,
                                        sp.clone(),
                                        format!(
                                            "Variant payload type mismatch for '{}.{}'",
                                            en, ev.variant
                                        ),
                                    ));
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
                if mc.method == "clone" {
                    return match obj_ty {
                        Type::String => Type::String,
                        Type::Struct(n) if self.struct_has_clone(&n) => Type::Struct(n),
                        _ => {
                            if self.signature_inference {
                                return Type::Unknown;
                            }
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("Type {:?} does not support .clone()", obj_ty),
                            ));
                            Type::Unknown
                        }
                    };
                }
                if mc.method == "length" || mc.method == "len" {
                    if !mc.args.is_empty() {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!("'.{}' expects no arguments", mc.method),
                        ));
                    }
                    if !matches!(obj_ty, Type::Struct(_)) {
                        return match obj_ty {
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
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    sp.clone(),
                                    format!("Type {:?} does not support .length()", obj_ty),
                                ));
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
                let type_name = match obj_ty {
                    Type::Struct(n) => n,
                    _ => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            "Method call requires struct receiver",
                        ));
                        return Type::Unknown;
                    }
                };
                if let Some(trait_name) = TypeChecker::dyn_trait_name(&type_name) {
                    if !self.trait_has_method(trait_name, &mc.method) {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            format!("Trait '{trait_name}' has no method '{}'", mc.method),
                        ));
                        return Type::Unknown;
                    }
                }
                let mangled = self.resolve_method_name(&type_name, &mc.method);
                let mut args = vec![mc.object.clone()];
                args.extend(mc.args.clone());
                if let Some(sig) = env.functions.get(&mangled).cloned() {
                    for (arg, expected) in args.iter().zip(sig.params.iter()) {
                        let at = self.check_expr(arg, env);
                        if at != *expected && at != Type::Unknown && *expected != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                format!("Method '{}' argument mismatch", mc.method),
                            ));
                        }
                    }
                    sig.return_type
                } else {
                    self.errors.push(NyraError::new(
                        ErrorKind::NameResolution,
                        sp.clone(),
                        format!("Unknown method '{}'", mc.method),
                    ));
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
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Deref of raw pointer requires unsafe",
                            ));
                            Type::Unknown
                        }
                        _ => {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Deref requires reference or raw pointer",
                            ));
                            Type::Unknown
                        }
                    },
                    UnaryOp::Neg => {
                        if !types::is_integer(&inner) && inner != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Unary '-' requires i32",
                            ));
                        }
                        Type::Integer(ast::IntKind::I32)
                    }
                    UnaryOp::Not => {
                        if inner != Type::Bool && inner != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                sp.clone(),
                                "Unary '!' requires bool",
                            ));
                        }
                        Type::Bool
                    }
                    UnaryOp::Move | UnaryOp::Clone | UnaryOp::Try => inner,
                }
            }
            Expression::Await(inner) => {
                if self.target_is_wasm() {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        "await is not available on wasm32 targets".to_string(),
                    ));
                }
                let t = self.check_expr(inner, env);
                if let Some(result) = super::future_types::future_await_result_type(&t) {
                    return result;
                }
                if !super::future_types::is_future_handle_type(&t) && t != Type::Unknown {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        expr_span(inner),
                        format!("await expects Future handle (i32) or Future<T>, got {t:?}"),
                    ));
                }
                Type::Integer(ast::IntKind::I32)
            }
            Expression::TemplateLiteral(t) => {
                for part in &t.parts {
                    if let TemplatePart::Interpolation(expr) = part {
                        let ty = self.check_expr(expr, env);
                        if !types::is_print_scalar(&ty) {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                t.span.clone(),
                                format!(
                                    "Template interpolation must be string, i32, f32, f64, char, or bool, got {ty:?}"
                                ),
                            ));
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
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            c.span.clone(),
                            "Raw pointer cast requires unsafe",
                        ));
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
                if let TypeAnnotation::DynTrait { trait_name, bounds } = &c.target_type {
                    if let Type::Struct(concrete) = &from {
                        if self.trait_impl_exists(trait_name, concrete) {
                            for b in bounds {
                                match b.as_str() {
                                    "Send" if !self.type_is_send(&from) => {
                                        self.errors.push(
                                            errors::NyraError::new(
                                                ErrorKind::Type,
                                                c.span.clone(),
                                                format!(
                                                    "Type '{concrete}' is not Send; cannot cast to dyn {trait_name} + Send"
                                                ),
                                            )
                                            .note(
                                                "Raw pointers and types with non-Send fields cannot cross thread boundaries",
                                            ),
                                        );
                                    }
                                    "Sync" if !self.type_is_sync(&from) => {
                                        self.errors.push(
                                            errors::NyraError::new(
                                                ErrorKind::Type,
                                                c.span.clone(),
                                                format!(
                                                    "Type '{concrete}' is not Sync; cannot cast to dyn {trait_name} + Sync"
                                                ),
                                            )
                                            .note(
                                                "Shared references across threads require all fields to be Sync",
                                            ),
                                        );
                                    }
                                    "Send" | "Sync" => {}
                                    other => {
                                        self.errors.push(NyraError::new(
                                            ErrorKind::Type,
                                            c.span.clone(),
                                            format!(
                                                "Unknown auto trait bound '{other}' on dyn {trait_name} (supported: Send, Sync)"
                                            ),
                                        ));
                                    }
                                }
                            }
                            return Type::Struct(format!("Dyn_{trait_name}"));
                        }
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            c.span.clone(),
                            format!(
                                "Type '{concrete}' does not implement trait '{trait_name}'"
                            ),
                        ));
                    } else if from != Type::Unknown {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            c.span.clone(),
                            "Trait object cast requires a concrete struct value",
                        ));
                    }
                    return Type::Struct(format!("Dyn_{trait_name}"));
                }
                if from != Type::Unknown && to != Type::Unknown {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        c.span.clone(),
                        format!("Invalid cast from {:?} to {:?}", from, to),
                    ));
                }
                to
            }
            Expression::Invalid => Type::Unknown,
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
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        span.clone(),
                        "comptime block must produce a value at compile time",
                    ));
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
        }
    }
}

