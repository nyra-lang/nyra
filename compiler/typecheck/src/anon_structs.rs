//! Infer compile-time struct types for `{ field: value }` object literals (zero-types style).

use std::collections::HashMap;

use ast::*;
use errors::{ErrorKind, NyraError, Span};

use super::{TypeChecker, TypeEnv};
use types::{integer_assignable, float_assignable, StructInfo, Type};

impl TypeChecker {
    pub(super) fn check_anonymous_struct_literal(
        &mut self,
        sl: &StructLiteralExpr,
        env: &mut TypeEnv,
        sp: Span,
    ) -> Type {
        let mut merged_fields: HashMap<String, Type> = HashMap::new();
        let mut field_order: Vec<String> = Vec::new();

        for spread in &sl.spreads {
            let spread_ty = self.check_expr(spread, env);
            match &spread_ty {
                Type::Struct(name) => {
                    if let Some(def) = self.structs.get(name) {
                        for fname in &def.field_order {
                            let fty = def.fields.get(fname).cloned().unwrap_or(Type::Unknown);
                            if !merged_fields.contains_key(fname) {
                                field_order.push(fname.clone());
                            }
                            merged_fields.insert(fname.clone(), fty);
                        }
                    }
                }
                Type::Unknown => {}
                other => {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!(
                            "Object spread `...expr` requires a struct value, got {:?}",
                            other
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
                    format!("Duplicate field '{fname}' in object literal"),
                ));
                continue;
            }
            let mut got = self.check_expr(fexpr, env);
            if got == Type::Unknown {
                got = self
                    .infer_expr_type_hint(fexpr, env)
                    .unwrap_or(Type::Unknown);
            }
            if got == Type::Unknown {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    sp.clone(),
                    format!(
                        "cannot infer type of field '{fname}' in object literal; add a struct declaration or type annotation"
                    ),
                ));
            }
            if let Some(prev) = merged_fields.get(fname) {
                if got != *prev && got != Type::Unknown && *prev != Type::Unknown
                    && !integer_assignable(prev, &got)
                    && !float_assignable(prev, &got)
                {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!(
                            "Field '{fname}' override has type {:?}, expected {:?} from spread",
                            got, prev
                        ),
                    ));
                }
            } else {
                field_order.push(fname.clone());
            }
            merged_fields.insert(fname.clone(), got);
        }

        let inferred_fields: Vec<(String, Type)> = field_order
            .into_iter()
            .filter_map(|fname| {
                merged_fields
                    .get(&fname)
                    .map(|fty| (fname, fty.clone()))
            })
            .collect();

        if inferred_fields.is_empty() {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                sp.clone(),
                "object literal must have at least one field or spread",
            ));
            return Type::Unknown;
        }

        if inferred_fields.iter().any(|(_, t)| *t == Type::Unknown) {
            return Type::Unknown;
        }

        let struct_name = self.resolve_anonymous_struct_name(&inferred_fields);
        self.anon_name_queue.push(struct_name.clone());
        Type::Struct(struct_name)
    }

    fn resolve_anonymous_struct_name(&mut self, fields: &[(String, Type)]) -> String {
        let shape = anon_shape_key(fields);
        if let Some(name) = self.anon_shape_index.get(&shape).cloned() {
            return name;
        }

        if let Some(name) = self.find_matching_declared_struct(fields) {
            self.anon_shape_index.insert(shape, name.clone());
            return name;
        }

        let name = format!("__Anon{}", self.anon_counter);
        self.anon_counter += 1;
        self.anon_shape_index.insert(shape.clone(), name.clone());

        let mut field_map = HashMap::new();
        let mut field_order = Vec::new();
        for (fname, fty) in fields {
            field_map.insert(fname.clone(), fty.clone());
            field_order.push(fname.clone());
        }
        self.structs.insert(
            name.clone(),
            StructInfo {
                fields: field_map,
                field_order,
                repr_c: false,
            },
        );

        let struct_fields: Vec<StructField> = fields
            .iter()
            .map(|(fname, fty)| StructField {
                name: fname.clone(),
                ty: Self::type_to_ann(fty),
            })
            .collect();
        self.synthesized_struct_defs.push(StructDef {
            name: name.clone(),
            doc: None,
            type_params: vec![],
            attrs: StructAttrs::default(),
            fields: struct_fields,
            public: true,
        });

        name
    }

    fn find_matching_declared_struct(&self, fields: &[(String, Type)]) -> Option<String> {
        let mut best: Option<(bool, String)> = None;
        for (name, info) in &self.structs {
            if name.starts_with("__Anon") {
                continue;
            }
            if fields_match_struct(fields, info) {
                let prefer = !name.starts_with('_');
                match &best {
                    None => best = Some((prefer, name.clone())),
                    Some((prev_prefer, _)) if prefer && !prev_prefer => {
                        best = Some((prefer, name.clone()));
                    }
                    _ => {}
                }
            }
        }
        best.map(|(_, n)| n)
    }

    pub fn apply_anonymous_struct_literals(&mut self, program: &mut Program) {
        let mut queue = std::mem::take(&mut self.anon_name_queue);
        for func in &mut program.functions {
            patch_anon_names_in_block(&mut func.body, &mut queue);
        }
        for imp in &mut program.impls {
            for method in &mut imp.methods {
                patch_anon_names_in_block(&mut method.body, &mut queue);
            }
        }
        for ti in &mut program.trait_impls {
            for method in &mut ti.methods {
                patch_anon_names_in_block(&mut method.body, &mut queue);
            }
        }
        for c in &mut program.consts {
            patch_anon_names_in_expr(&mut c.value, &mut queue);
        }
        for sdef in std::mem::take(&mut self.synthesized_struct_defs) {
            if !program.structs.iter().any(|s| s.name == sdef.name) {
                program.structs.push(sdef);
            }
        }
    }

    pub fn synthesized_anon_structs(&self) -> bool {
        !self.anon_shape_index.is_empty()
    }
}

fn fields_match_struct(fields: &[(String, Type)], info: &StructInfo) -> bool {
    if info.fields.len() != fields.len() {
        return false;
    }
    for (fname, fty) in fields {
        let Some(expected) = info.fields.get(fname) else {
            return false;
        };
        if !field_types_compatible(expected, fty) {
            return false;
        }
    }
    true
}

fn field_types_compatible(expected: &Type, got: &Type) -> bool {
    if expected == got {
        return true;
    }
    integer_assignable(expected, got) || float_assignable(expected, got)
}

fn anon_shape_key(fields: &[(String, Type)]) -> String {
    fields
        .iter()
        .map(|(n, t)| format!("{n}:{}", type_shape_mangle(t)))
        .collect::<Vec<_>>()
        .join("|")
}

fn type_shape_mangle(ty: &Type) -> String {
    match ty {
        Type::Integer(k) => k.name().into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::Char => "char".into(),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Struct(n) => format!("S_{n}"),
        Type::Enum(n) => format!("E_{n}"),
        Type::Array { elem, len } => format!(
            "A{}_{}",
            len.map(|n| n.to_string()).unwrap_or_else(|| "x".into()),
            type_shape_mangle(elem)
        ),
        Type::Tuple { elems } => format!(
            "T{}",
            elems
                .iter()
                .map(type_shape_mangle)
                .collect::<Vec<_>>()
                .join("_")
        ),
        _ => "?".into(),
    }
}

fn patch_anon_names_in_block(block: &mut Block, queue: &mut Vec<String>) {
    for stmt in &mut block.statements {
        patch_anon_names_in_stmt(stmt, queue);
    }
}

fn patch_anon_names_in_stmt(stmt: &mut Statement, queue: &mut Vec<String>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => patch_anon_names_in_expr(&mut l.value, queue),
        Statement::Assign(a) => {
            patch_anon_names_in_expr(&mut a.target, queue);
            patch_anon_names_in_expr(&mut a.value, queue);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                patch_anon_names_in_expr(v, queue);
            }
        }
        Statement::If(i) => {
            patch_anon_names_in_expr(&mut i.condition, queue);
            patch_anon_names_in_block(&mut i.then_block, queue);
            if let Some(e) = &mut i.else_block {
                patch_anon_names_in_block(e, queue);
            }
        }
        Statement::While(w) => {
            patch_anon_names_in_expr(&mut w.condition, queue);
            patch_anon_names_in_block(&mut w.body, queue);
        }
        Statement::For(f) => {
            f.map_exprs_mut(|e| patch_anon_names_in_expr(e, queue));
            patch_anon_names_in_block(&mut f.body, queue);
        }
        Statement::Expression(e) | Statement::Defer(e) => patch_anon_names_in_expr(e, queue),
        Statement::Print(p) => {
            for a in &mut p.args {
                patch_anon_names_in_expr(a, queue);
            }
            if let Some(c) = &mut p.color {
                patch_anon_names_in_expr(c, queue);
            }
        }
        Statement::Spawn(b) | Statement::Benchmark(b) | Statement::Unsafe(b) => {
            patch_anon_names_in_block(b, queue);
        }
        _ => {}
    }
}

fn patch_anon_names_in_expr(expr: &mut Expression, queue: &mut Vec<String>) {
    match expr {
        Expression::StructLiteral(sl) if sl.name.is_empty() => {
            if let Some(name) = queue.first() {
                sl.name = name.clone();
                queue.remove(0);
            }
            for spread in &mut sl.spreads {
                patch_anon_names_in_expr(spread, queue);
            }
            for (_, v) in &mut sl.fields {
                patch_anon_names_in_expr(v, queue);
            }
        }
        Expression::Binary(b) => {
            patch_anon_names_in_expr(&mut b.left, queue);
            patch_anon_names_in_expr(&mut b.right, queue);
        }
        Expression::Unary(u) => patch_anon_names_in_expr(&mut u.operand, queue),
        Expression::Call(c) => {
            for a in &mut c.args {
                patch_anon_names_in_expr(a, queue);
            }
        }
        Expression::MethodCall(m) => {
            patch_anon_names_in_expr(&mut m.object, queue);
            for a in &mut m.args {
                patch_anon_names_in_expr(a, queue);
            }
        }
        Expression::FieldAccess(f) => patch_anon_names_in_expr(&mut f.object, queue),
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                patch_anon_names_in_expr(spread, queue);
            }
            for (_, v) in &mut s.fields {
                patch_anon_names_in_expr(v, queue);
            }
        }
        Expression::EnumVariant(v) => {
            for a in &mut v.args {
                patch_anon_names_in_expr(a, queue);
            }
        }
        Expression::Match(m) => {
            patch_anon_names_in_expr(&mut m.scrutinee, queue);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    patch_anon_names_in_expr(g, queue);
                }
                patch_anon_names_in_expr(&mut arm.body, queue);
            }
        }
        Expression::If(i) => {
            patch_anon_names_in_expr(&mut i.condition, queue);
            patch_anon_names_in_expr(&mut i.then_expr, queue);
            patch_anon_names_in_expr(&mut i.else_expr, queue);
        }
        Expression::Index(ix) => {
            patch_anon_names_in_expr(&mut ix.object, queue);
            patch_anon_names_in_expr(&mut ix.index, queue);
        }
        Expression::ArrayLiteral(al) => {
            for item in al.all_exprs_mut() {
                patch_anon_names_in_expr(item, queue);
            }
        }
        Expression::TupleLiteral(items) => {
            for item in items {
                patch_anon_names_in_expr(item, queue);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            patch_anon_names_in_expr(element, queue);
        }
        Expression::Cast(c) => patch_anon_names_in_expr(&mut c.expr, queue),
        Expression::ArrowFn(a) => match &mut a.body {
            ArrowBody::Expr(e) => patch_anon_names_in_expr(e, queue),
            ArrowBody::Block(b) => patch_anon_names_in_block(b, queue),
        },
        Expression::ComptimeBlock { body, .. } => patch_anon_names_in_block(body, queue),
        Expression::Grouped(e) | Expression::Await(e) => patch_anon_names_in_expr(e, queue),
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    patch_anon_names_in_expr(e, queue);
                }
            }
        }
        Expression::Invalid | Expression::Literal(_) | Expression::Variable { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_key_orders_fields() {
        let fields = vec![
            ("name".into(), Type::String),
            ("age".into(), Type::Integer(ast::IntKind::I32)),
        ];
        let k = anon_shape_key(&fields);
        assert!(k.contains("name:string"));
        assert!(k.contains("age:i32"));
    }
}
