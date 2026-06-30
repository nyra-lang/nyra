mod comptime;

pub use comptime::{finalize_comptime_module, fold_attributed_comptime_functions, strip_comptime_artifacts};

use std::collections::{BTreeMap, HashMap};

use ast::{for_each_expr_in_block_mut, BinaryOp, Expression, Literal, UnaryOp};
use errors::Span;

fn const_wrap_i32(n: i64) -> i64 {
    n as i32 as i64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstValue {
    Int(i64),
    Bool(bool),
    /// Fixed comptime array.
    Array(Vec<ConstValue>),
    /// Compile-time string (UTF-8).
    String(String),
    /// Enum variant at comptime (unit or payload — tuple when multiple args).
    Enum {
        enum_name: String,
        variant: String,
        payload: Option<Box<ConstValue>>,
    },
    /// Struct value at comptime.
    Struct {
        name: String,
        fields: BTreeMap<String, ConstValue>,
    },
    /// Tuple value at comptime.
    Tuple(Vec<ConstValue>),
}

const MAX_CONST_EVAL_DEPTH: usize = 256;

pub fn eval_const_expr(
    expr: &Expression,
    consts: &HashMap<String, ConstValue>,
) -> Option<ConstValue> {
    eval_const_expr_depth(expr, consts, 0)
}

fn eval_const_expr_depth(
    expr: &Expression,
    consts: &HashMap<String, ConstValue>,
    depth: usize,
) -> Option<ConstValue> {
    if depth > MAX_CONST_EVAL_DEPTH {
        return None;
    }
    let next = depth + 1;
    match expr {
        Expression::Literal(Literal::Int(n)) => Some(ConstValue::Int(*n)),
        Expression::Literal(Literal::IntKind(n, _)) => Some(ConstValue::Int(*n)),
        Expression::Literal(Literal::Float(_, _)) => None,
        Expression::Literal(Literal::Char(_)) => None,
        Expression::Literal(Literal::Bool(b)) => Some(ConstValue::Bool(*b)),
        Expression::Literal(Literal::String(s)) => Some(ConstValue::String(s.clone())),
        Expression::Variable { name, .. } => consts.get(name).cloned(),
        Expression::Binary(b) => {
            let l = eval_const_expr_depth(&b.left, consts, next)?;
            let r = eval_const_expr_depth(&b.right, consts, next)?;
            match (b.op, l, r) {
                (BinaryOp::Add, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Int(const_wrap_i32(a.wrapping_add(b))))
                }
                (BinaryOp::Add, ConstValue::String(a), ConstValue::String(b)) => {
                    Some(ConstValue::String(format!("{a}{b}")))
                }
                (BinaryOp::Sub, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Int(const_wrap_i32(a.wrapping_sub(b))))
                }
                (BinaryOp::Mul, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Int(const_wrap_i32(a.wrapping_mul(b))))
                }
                (BinaryOp::Div, ConstValue::Int(a), ConstValue::Int(b)) => {
                    if b == 0 {
                        None
                    } else {
                        Some(ConstValue::Int(const_wrap_i32(a / b)))
                    }
                }
                (BinaryOp::Mod, ConstValue::Int(a), ConstValue::Int(b)) => {
                    if b == 0 {
                        None
                    } else {
                        Some(ConstValue::Int(const_wrap_i32(a % b)))
                    }
                }
                (BinaryOp::BitOr, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Int(a | b))
                }
                (BinaryOp::BitAnd, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Int(a & b))
                }
                (BinaryOp::BitXor, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Int(a ^ b))
                }
                (BinaryOp::Shl, ConstValue::Int(a), ConstValue::Int(b)) => {
                    if b < 0 || b >= 64 {
                        None
                    } else {
                        Some(ConstValue::Int(a.wrapping_shl(b as u32)))
                    }
                }
                (BinaryOp::Shr, ConstValue::Int(a), ConstValue::Int(b)) => {
                    if b < 0 || b >= 64 {
                        None
                    } else {
                        Some(ConstValue::Int(a.wrapping_shr(b as u32)))
                    }
                }
                (BinaryOp::Eq, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Bool(a == b))
                }
                (BinaryOp::Eq, ConstValue::String(a), ConstValue::String(b)) => {
                    Some(ConstValue::Bool(a == b))
                }
                (BinaryOp::Ne, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Bool(a != b))
                }
                (BinaryOp::Ne, ConstValue::String(a), ConstValue::String(b)) => {
                    Some(ConstValue::Bool(a != b))
                }
                (BinaryOp::Lt, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Bool(a < b))
                }
                (BinaryOp::Gt, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Bool(a > b))
                }
                (BinaryOp::Le, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Bool(a <= b))
                }
                (BinaryOp::Ge, ConstValue::Int(a), ConstValue::Int(b)) => {
                    Some(ConstValue::Bool(a >= b))
                }
                (BinaryOp::And, ConstValue::Bool(a), ConstValue::Bool(b)) => {
                    Some(ConstValue::Bool(a && b))
                }
                (BinaryOp::Or, ConstValue::Bool(a), ConstValue::Bool(b)) => {
                    Some(ConstValue::Bool(a || b))
                }
                _ => None,
            }
        }
        Expression::Unary(u) => {
            let v = eval_const_expr_depth(&u.operand, consts, next)?;
            match (&u.op, v) {
                (UnaryOp::Neg, ConstValue::Int(n)) => Some(ConstValue::Int(const_wrap_i32(-n))),
                (UnaryOp::Not, ConstValue::Bool(b)) => Some(ConstValue::Bool(!b)),
                _ => None,
            }
        }
        Expression::Grouped(inner) => eval_const_expr_depth(inner, consts, next),
        _ => None,
    }
}

pub fn const_value_to_expr(v: &ConstValue) -> Expression {
    const_value_to_expr_typed(v, None)
}

pub fn const_value_to_expr_typed(v: &ConstValue, ty: Option<&ast::TypeAnnotation>) -> Expression {
    match v {
        ConstValue::Int(n) => {
            if let Some(ast::TypeAnnotation::Integer(kind)) = ty {
                Expression::Literal(Literal::IntKind(*n, *kind))
            } else {
                Expression::Literal(Literal::Int(*n))
            }
        }
        ConstValue::Bool(b) => Expression::Literal(Literal::Bool(*b)),
        ConstValue::String(s) => Expression::Literal(Literal::String(s.clone())),
        ConstValue::Array(elems) => {
            let elem_ty = ty.and_then(|t| match t {
                ast::TypeAnnotation::Array { elem, .. } => Some(elem.as_ref()),
                _ => None,
            });
            Expression::ArrayLiteral(ast::ArrayLiteralExpr::from_elems(
                elems
                    .iter()
                    .map(|e| const_value_to_expr_typed(e, elem_ty))
                    .collect(),
            ))
        }
        ConstValue::Enum {
            enum_name,
            variant,
            payload,
        } => Expression::EnumVariant(ast::EnumVariantExpr {
            enum_name: Some(enum_name.clone()),
            variant: variant.clone(),
            args: match payload.as_deref() {
                None => vec![],
                Some(ConstValue::Tuple(elems)) => {
                    elems.iter().map(const_value_to_expr).collect()
                }
                Some(single) => vec![const_value_to_expr(single)],
            },
            span: Span::default(),
        }),
        ConstValue::Struct { name, fields } => Expression::StructLiteral(ast::StructLiteralExpr {
            name: name.clone(),
            spreads: vec![],
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), const_value_to_expr(v)))
                .collect(),
            span: Span::default(),
        }),
        ConstValue::Tuple(elems) => Expression::TupleLiteral(
            elems.iter().map(const_value_to_expr).collect(),
        ),
    }
}

pub fn fold_program_consts(program: &mut ast::Program) {
    let mut consts = HashMap::new();
    for c in &program.consts {
        if let Some(v) = eval_const_expr(&c.value, &consts) {
            consts.insert(c.name.clone(), v);
        }
    }
    for c in &mut program.consts {
        if let Some(v) = eval_const_expr(&c.value, &consts) {
            c.value = const_value_to_expr_typed(&v, c.ty.as_ref());
        }
    }
    for f in &mut program.functions {
        resolve_array_repeat_counts_block(&mut f.body, &consts);
        fold_block_consts(&mut f.body, &consts);
    }
}

fn resolve_array_repeat_counts_block(block: &mut ast::Block, consts: &HashMap<String, ConstValue>) {
    for stmt in &mut block.statements {
        resolve_array_repeat_counts_stmt(stmt, consts);
    }
}

fn resolve_array_repeat_counts_stmt(stmt: &mut ast::Statement, consts: &HashMap<String, ConstValue>) {
    match stmt {
        ast::Statement::Let(l) | ast::Statement::Const(l) => {
            resolve_array_repeat_counts_expr(&mut l.value, consts);
        }
        ast::Statement::Assign(a) => {
            resolve_array_repeat_counts_expr(&mut a.target, consts);
            resolve_array_repeat_counts_expr(&mut a.value, consts);
        }
        ast::Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                resolve_array_repeat_counts_expr(v, consts);
            }
        }
        ast::Statement::If(i) => {
            resolve_array_repeat_counts_expr(&mut i.condition, consts);
            resolve_array_repeat_counts_block(&mut i.then_block, consts);
            if let Some(el) = &mut i.else_block {
                resolve_array_repeat_counts_block(el, consts);
            }
        }
        ast::Statement::While(w) => {
            resolve_array_repeat_counts_expr(&mut w.condition, consts);
            resolve_array_repeat_counts_block(&mut w.body, consts);
        }
        ast::Statement::For(f) => {
            f.map_exprs_mut(|e| {
                resolve_array_repeat_counts_expr(e, consts);
            });
            resolve_array_repeat_counts_block(&mut f.body, consts);
        }
        ast::Statement::Expression(e) | ast::Statement::Defer(e) => {
            resolve_array_repeat_counts_expr(e, consts);
        }
        ast::Statement::Print(p) => {
            for a in &mut p.args {
                resolve_array_repeat_counts_expr(a, consts);
            }
            if let Some(c) = &mut p.color {
                resolve_array_repeat_counts_expr(c, consts);
            }
        }
        ast::Statement::Benchmark(b) => resolve_array_repeat_counts_block(b, consts),
        ast::Statement::Spawn(b) => resolve_array_repeat_counts_block(b, consts),
        ast::Statement::Unsafe(b) => resolve_array_repeat_counts_block(b, consts),
        _ => {}
    }
}

fn resolve_array_repeat_counts_expr(expr: &mut Expression, consts: &HashMap<String, ConstValue>) {
    match expr {
        Expression::ArrayRepeat {
            count,
            count_from,
            count_expr,
            element,
            ..
        } => {
            resolve_array_repeat_counts_expr(element, consts);
            if let Some(expr) = count_expr.take() {
                if let Some(ConstValue::Int(n)) = eval_const_expr(&expr, consts) {
                    if n >= 0 {
                        *count = n as usize;
                    }
                }
            } else if let Some(name) = count_from.take() {
                if let Some(ConstValue::Int(n)) = consts.get(&name) {
                    if *n >= 0 {
                        *count = *n as usize;
                    }
                }
            }
        }
        Expression::Binary(b) => {
            resolve_array_repeat_counts_expr(&mut b.left, consts);
            resolve_array_repeat_counts_expr(&mut b.right, consts);
        }
        Expression::Unary(u) => resolve_array_repeat_counts_expr(&mut u.operand, consts),
        Expression::Grouped(g) => resolve_array_repeat_counts_expr(g, consts),
        Expression::If(i) => {
            resolve_array_repeat_counts_expr(&mut i.condition, consts);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| resolve_array_repeat_counts_expr(e, consts));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| resolve_array_repeat_counts_expr(e, consts));
        }
        Expression::Call(c) => {
            for a in &mut c.args {
                resolve_array_repeat_counts_expr(a, consts);
            }
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                resolve_array_repeat_counts_expr(e, consts);
            }
        }
        Expression::StructLiteral(s) => {
            for (_, e) in &mut s.fields {
                resolve_array_repeat_counts_expr(e, consts);
            }
            for e in &mut s.spreads {
                resolve_array_repeat_counts_expr(e, consts);
            }
        }
        Expression::Index(ix) => {
            resolve_array_repeat_counts_expr(&mut ix.object, consts);
            resolve_array_repeat_counts_expr(&mut ix.index, consts);
        }
        Expression::FieldAccess(f) => resolve_array_repeat_counts_expr(&mut f.object, consts),
        Expression::Match(m) => {
            resolve_array_repeat_counts_expr(&mut m.scrutinee, consts);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    resolve_array_repeat_counts_expr(g, consts);
                }
                for_each_expr_in_block_mut(&mut arm.body, &mut |e| resolve_array_repeat_counts_expr(e, consts));
            }
        }
        Expression::Cast(c) => resolve_array_repeat_counts_expr(&mut c.expr, consts),
        Expression::MethodCall(mc) => {
            resolve_array_repeat_counts_expr(&mut mc.object, consts);
            for a in &mut mc.args {
                resolve_array_repeat_counts_expr(a, consts);
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                resolve_array_repeat_counts_expr(e, consts);
            }
        }
        Expression::Await(e) => resolve_array_repeat_counts_expr(e, consts),
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let ast::TemplatePart::Interpolation(e) = part {
                    resolve_array_repeat_counts_expr(e, consts);
                }
            }
        }
        Expression::ArrowFn(a) => match &mut a.body {
            ast::ArrowBody::Expr(e) => resolve_array_repeat_counts_expr(e, consts),
            ast::ArrowBody::Block(b) => resolve_array_repeat_counts_block(b, consts),
        },
        _ => {}
    }
}

fn fold_block_consts(block: &mut ast::Block, consts: &HashMap<String, ConstValue>) {
    let mut local = consts.clone();
    for stmt in &mut block.statements {
        if let ast::Statement::Const(c) = stmt {
            if let Some(v) = eval_const_expr(&c.value, &local) {
                local.insert(c.name.clone(), v.clone());
                c.value = const_value_to_expr_typed(&v, c.ty.as_ref());
            }
        } else if let ast::Statement::Let(l) = stmt {
            if let Some(v) = eval_const_expr(&l.value, &local) {
                if !l.mutable {
                    local.insert(l.name.clone(), v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast::{BinaryOp, Expression, Literal, UnaryOp};

    #[test]
    fn eval_const_depth_limit_avoids_deep_unary_stack_overflow() {
        let mut expr = Expression::Literal(Literal::Bool(true));
        for _ in 0..512 {
            expr = Expression::Unary(Box::new(ast::UnaryExpr {
                op: UnaryOp::Not,
                operand: expr,
                span: Default::default(),
            }));
        }
        assert_eq!(eval_const_expr(&expr, &HashMap::new()), None);
    }

    #[test]
    fn eval_const_addition() {
        let expr = Expression::Binary(Box::new(ast::BinaryExpr {
            left: Expression::Literal(Literal::Int(2)),
            op: BinaryOp::Add,
            right: Expression::Literal(Literal::Int(3)),
            span: Default::default(),
        }));
        assert_eq!(
            eval_const_expr(&expr, &HashMap::new()),
            Some(ConstValue::Int(5))
        );
    }

    #[test]
    fn eval_const_bool_and() {
        let expr = Expression::Binary(Box::new(ast::BinaryExpr {
            left: Expression::Literal(Literal::Bool(true)),
            op: BinaryOp::And,
            right: Expression::Literal(Literal::Bool(false)),
            span: Default::default(),
        }));
        assert_eq!(
            eval_const_expr(&expr, &HashMap::new()),
            Some(ConstValue::Bool(false))
        );
    }

    #[test]
    fn eval_const_variable_lookup() {
        let mut consts = HashMap::new();
        consts.insert("N".into(), ConstValue::Int(10));
        let expr = Expression::Variable {
            name: "N".into(),
            span: Default::default(),
        };
        assert_eq!(eval_const_expr(&expr, &consts), Some(ConstValue::Int(10)));
    }

    #[test]
    fn resolve_array_repeat_from_const() {
        let src = r#"const N = 4
fn main() {
    let a = [0; N]
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "c.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        fold_program_consts(&mut program);
        match &program.functions[0].body.statements[0] {
            ast::Statement::Let(l) => match &l.value {
                Expression::ArrayRepeat { count, count_from, .. } => {
                    assert_eq!(*count, 4);
                    assert!(count_from.is_none());
                }
                other => panic!("expected array repeat, got {other:?}"),
            },
            other => panic!("expected let, got {other:?}"),
        }
    }

    #[test]
    fn fold_program_const_in_function() {
        let src = r#"const N = 2 + 3
fn main() {
    let x = N
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "c.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        fold_program_consts(&mut program);
        assert!(matches!(
            program.consts[0].value,
            Expression::Literal(Literal::Int(5))
        ));
    }
}
