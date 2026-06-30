//! Expression parsing (precedence chain, primary, match, postfix).
use ast::*;
use ast::expr_span;
use errors::coded_parser_error;
use lexer::TokenKind;
use super::recovery::{check, consume, is_at_end, merge_spans, skip_newlines};

use super::Parser;

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Expression {
        if self.errors_over_limit() {
            return Expression::Invalid;
        }
        skip_newlines(&self.tokens, &mut self.position);
        self.parse_conditional()
    }

    pub(super) fn parse_conditional(&mut self) -> Expression {
        let condition = self.parse_or();
        if !check(&self.tokens, self.position, &TokenKind::Question) {
            return condition;
        }
        if !super::recovery::looks_like_ternary_question(&self.tokens, self.position) {
            return condition;
        }
        self.advance();
        let then_expr = self.parse_expression();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Colon,
            "Expected ':' in ternary expression",
            &mut self.errors,
        );
        let else_expr = self.parse_conditional();
        let span = merge_spans(&expr_span(&condition), &expr_span(&else_expr));
        Expression::If(Box::new(IfExpr {
            condition,
            then_block: block_from_expr(then_expr),
            else_block: block_from_expr(else_expr),
            span,
        }))
    }

    pub(super) fn parse_or(&mut self) -> Expression {
        let mut left = self.parse_nullish();
        while matches!(self.current_kind(), TokenKind::OrOr) {
            self.advance();
            let right = self.parse_nullish();
            left = self.make_binary(left, BinaryOp::Or, right);
        }
        left
    }

    pub(super) fn parse_nullish(&mut self) -> Expression {
        let mut left = self.parse_and();
        while matches!(self.current_kind(), TokenKind::NullishCoalesce) {
            self.advance();
            let right = self.parse_and();
            left = self.make_binary(left, BinaryOp::NullishCoalesce, right);
        }
        left
    }

    pub(super) fn parse_and(&mut self) -> Expression {
        let mut left = self.parse_bit_or();
        while matches!(self.current_kind(), TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_bit_or();
            left = self.make_binary(left, BinaryOp::And, right);
        }
        left
    }

    pub(super) fn parse_bit_or(&mut self) -> Expression {
        let mut left = self.parse_bit_xor();
        while matches!(self.current_kind(), TokenKind::BitOr) {
            self.advance();
            let right = self.parse_bit_xor();
            left = self.make_binary(left, BinaryOp::BitOr, right);
        }
        left
    }

    pub(super) fn parse_bit_xor(&mut self) -> Expression {
        let mut left = self.parse_bit_and();
        while matches!(self.current_kind(), TokenKind::BitXor) {
            self.advance();
            let right = self.parse_bit_and();
            left = self.make_binary(left, BinaryOp::BitXor, right);
        }
        left
    }

    pub(super) fn parse_bit_and(&mut self) -> Expression {
        let mut left = self.parse_equality();
        while matches!(self.current_kind(), TokenKind::Ampersand) {
            self.advance();
            let right = self.parse_equality();
            left = self.make_binary(left, BinaryOp::BitAnd, right);
        }
        left
    }

    pub(super) fn parse_equality(&mut self) -> Expression {
        let mut left = self.parse_comparison();
        loop {
            let op = match self.current_kind() {
                TokenKind::EqualEqual => BinaryOp::Eq,
                TokenKind::BangEqual => BinaryOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison();
            left = self.make_binary(left, op, right);
        }
        left
    }

    pub(super) fn parse_comparison(&mut self) -> Expression {
        let mut left = self.parse_shift();
        loop {
            let op = match self.current_kind() {
                TokenKind::Less => BinaryOp::Lt,
                TokenKind::Greater => BinaryOp::Gt,
                TokenKind::LessEqual => BinaryOp::Le,
                TokenKind::GreaterEqual => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift();
            left = self.make_binary(left, op, right);
        }
        left
    }

    pub(super) fn parse_shift(&mut self) -> Expression {
        let mut left = self.parse_term();
        loop {
            let op = match self.current_kind() {
                TokenKind::Shl => BinaryOp::Shl,
                TokenKind::Shr => BinaryOp::Shr,
                _ => break,
            };
            self.advance();
            let right = self.parse_term();
            left = self.make_binary(left, op, right);
        }
        left
    }

    pub(super) fn parse_term(&mut self) -> Expression {
        let mut left = self.parse_factor();
        loop {
            if matches!(self.current_kind(), TokenKind::Newline) {
                break;
            }
            let op = match self.current_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor();
            left = self.make_binary(left, op, right);
        }
        left
    }

    /// Add/sub without `%` modulo — for `parallel(cpu = 80%)`.
    pub(super) fn parse_term_no_mod(&mut self) -> Expression {
        let mut left = self.parse_cast();
        loop {
            if matches!(self.current_kind(), TokenKind::Newline) {
                break;
            }
            let op = match self.current_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_cast();
            left = self.make_binary(left, op, right);
        }
        left
    }

    pub(super) fn parse_factor(&mut self) -> Expression {
        let mut left = self.parse_cast();
        loop {
            if matches!(self.current_kind(), TokenKind::Newline) {
                break;
            }
            let op = match self.current_kind() {
                TokenKind::Star => {
                    // `expr as *T` followed by `*p` on the next line must not become multiplication.
                    if matches!(left, Expression::Cast(_)) {
                        break;
                    }
                    BinaryOp::Mul
                }
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_cast();
            left = self.make_binary(left, op, right);
        }
        left
    }

    pub(super) fn parse_cast(&mut self) -> Expression {
        let mut expr = self.parse_unary();
        while check(&self.tokens, self.position, &TokenKind::As) {
            let start = expr_span(&expr);
            self.advance();
            let target_type = self.parse_type_annotation();
            let span = merge_spans(&start, &self.prev_span());
            expr = Expression::Cast(Box::new(CastExpr {
                expr,
                target_type,
                span,
            }));
        }
        expr
    }

    pub(super) fn parse_unary(&mut self) -> Expression {
        if check(&self.tokens, self.position, &TokenKind::Await) {
            self.advance();
            let inner = self.parse_unary();
            return Expression::Await(Box::new(inner));
        }
        if check(&self.tokens, self.position, &TokenKind::Match) {
            return self.parse_match();
        }
        if check(&self.tokens, self.position, &TokenKind::If) {
            return self.parse_if_expr();
        }
        match self.current_kind().clone() {
            TokenKind::Ampersand => {
                self.advance();
                let mutable = if check(&self.tokens, self.position, &TokenKind::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let op = if mutable {
                    UnaryOp::RefMut
                } else {
                    UnaryOp::Ref
                };
                let operand = self.parse_unary();
                self.make_unary(op, operand)
            }
            TokenKind::Star => {
                self.advance();
                let operand = self.parse_unary();
                self.make_unary(UnaryOp::Deref, operand)
            }
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary();
                self.make_unary(UnaryOp::Neg, operand)
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_unary();
                self.make_unary(UnaryOp::Not, operand)
            }
            TokenKind::Move => {
                self.advance();
                let operand = self.parse_unary();
                self.make_unary(UnaryOp::Move, operand)
            }
            TokenKind::Clone => {
                self.advance();
                let operand = self.parse_unary();
                self.make_unary(UnaryOp::Clone, operand)
            }
            _ => self.parse_primary(),
        }
    }

    pub(super) fn parse_primary(&mut self) -> Expression {
        match self.current_kind().clone() {
            TokenKind::Number(n) => {
                self.advance();
                Expression::Literal(Literal::Int(n))
            }
            TokenKind::NumberSuffix(n, k) => {
                self.advance();
                Expression::Literal(Literal::IntKind(n, k))
            }
            TokenKind::Float { bits, kind } => {
                self.advance();
                Expression::Literal(Literal::Float(f64::from_bits(bits), kind))
            }
            TokenKind::CharLit(cp) => {
                self.advance();
                Expression::Literal(Literal::Char(cp))
            }
            TokenKind::True => {
                self.advance();
                Expression::Literal(Literal::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Expression::Literal(Literal::Bool(false))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                self.parse_postfix(Expression::Literal(Literal::String(s)))
            }
            TokenKind::TemplateLit(template) => {
                let span = self.prev_span();
                self.advance();
                let mut parts = Vec::new();
                for part in template.parts {
                    match part {
                        lexer::TemplateLitPart::Text(text) => {
                            parts.push(TemplatePart::Static(text));
                        }
                        lexer::TemplateLitPart::Interp(src) => {
                            let expr = self.parse_embedded_expression(&src);
                            parts.push(TemplatePart::Interpolation(Box::new(expr)));
                        }
                    }
                }
                self.parse_postfix(Expression::TemplateLiteral(TemplateLiteralExpr {
                    parts,
                    span,
                }))
            }
            TokenKind::LBracket => {
                let start = self.current_span();
                self.advance();
                skip_newlines(&self.tokens, &mut self.position);
                if check(&self.tokens, self.position, &TokenKind::RBracket) {
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RBracket,
                        "Expected ']' after array literal",
                        &mut self.errors,
                    );
                    return self.parse_postfix(Expression::ArrayLiteral(ArrayLiteralExpr {
                        spreads: vec![],
                        elems: vec![],
                        span: merge_spans(&start, &self.prev_span()),
                    }));
                }
                let mut spreads = Vec::new();
                let mut elems = Vec::new();
                loop {
                    skip_newlines(&self.tokens, &mut self.position);
                    if check(&self.tokens, self.position, &TokenKind::RBracket) {
                        break;
                    }
                    if self.is_spread_token() {
                        self.consume_spread_token();
                        spreads.push(self.parse_expression());
                    } else {
                        let expr = self.parse_expression();
                        skip_newlines(&self.tokens, &mut self.position);
                        if spreads.is_empty()
                            && elems.is_empty()
                            && check(&self.tokens, self.position, &TokenKind::Semicolon)
                        {
                            self.advance();
                            let count_expr = self.parse_expression();
                            consume(
                                &self.tokens,
                                &mut self.position,
                                TokenKind::RBracket,
                                "Expected ']' after array repeat",
                                &mut self.errors,
                            );
                            let (count, count_from, count_expr) = match &count_expr {
                                Expression::Literal(Literal::Int(n)) if *n >= 0 => {
                                    (*n as usize, None, None)
                                }
                                Expression::Literal(Literal::IntKind(n, _)) if *n >= 0 => {
                                    (*n as usize, None, None)
                                }
                                Expression::Variable { name, .. } => (0, Some(name.clone()), None),
                                other => (0, None, Some(Box::new(other.clone()))),
                            };
                            let count_span = count_expr
                                .as_ref()
                                .map(|e| expr_span(e))
                                .unwrap_or_else(|| expr_span(&expr));
                            let span = merge_spans(&expr_span(&expr), &count_span);
                            return self.parse_postfix(Expression::ArrayRepeat {
                                element: Box::new(expr),
                                count,
                                count_from,
                                count_expr,
                                span,
                            });
                        }
                        elems.push(expr);
                    }
                    skip_newlines(&self.tokens, &mut self.position);
                    if check(&self.tokens, self.position, &TokenKind::Comma) {
                        self.advance();
                    } else if check(&self.tokens, self.position, &TokenKind::RBracket) {
                        break;
                    } else {
                        break;
                    }
                }
                skip_newlines(&self.tokens, &mut self.position);
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::RBracket,
                    "Expected ']' after array literal",
                    &mut self.errors,
                );
                self.parse_postfix(Expression::ArrayLiteral(ArrayLiteralExpr {
                    spreads,
                    elems,
                    span: merge_spans(&start, &self.prev_span()),
                }))
            }
            TokenKind::LBrace => {
                if super::diagnostics::parse_leading_brace_expr(
                    &self.tokens,
                    &mut self.position,
                    &mut self.errors,
                ) {
                    return Expression::Invalid;
                }
                if super::diagnostics::looks_like_anonymous_object_literal(
                    &self.tokens,
                    self.position,
                ) {
                    return self.parse_struct_literal(String::new());
                }
                let span = self.current_span();
                self.errors.push(
                    coded_parser_error(span, "unexpected `{` in expression")
                        .label("`{` must follow a struct type name or use `{ field: value }`")
                        .help("use `MyStruct { field: value }` or `{ field: value }`"),
                );
                let _ = super::diagnostics::skip_balanced_brace(&self.tokens, &mut self.position);
                Expression::Invalid
            }
            TokenKind::SelfKw => {
                let span = self.current_span();
                self.advance();
                self.parse_postfix(Expression::Variable {
                    name: "self".into(),
                    span,
                })
            }
            TokenKind::Identifier(name) => {
                self.advance();
                let mut name = name;
                if check(&self.tokens, self.position, &TokenKind::ColonColon) {
                    self.advance();
                    if let TokenKind::Identifier(member) = self.current_kind().clone() {
                        self.advance();
                        name = format!("{name}__{member}");
                    } else {
                        self.parse_error_here("Expected name after '::'");
                    }
                }
                if name == "comptime" {
                    skip_newlines(&self.tokens, &mut self.position);
                    if check(&self.tokens, self.position, &TokenKind::LBrace) {
                        let start = self.prev_span();
                        let body = self.parse_block();
                        let span = merge_spans(&start, &self.prev_span());
                        return self.parse_postfix(Expression::ComptimeBlock { body, span });
                    }
                }
                // `x => expr` — single inferred arrow param without parens
                if check(&self.tokens, self.position, &TokenKind::FatArrow) {
                    let start = self.prev_span();
                    self.advance();
                    let params = vec![Param {
                        name: name.clone(),
                        ty: TypeAnnotation::Generic("_".into()),
                        destructure: vec![],
                        no_escape: false,
                        mutable: false,
                    }];
                    let body = self.parse_arrow_body();
                    let span = merge_spans(&start, &self.prev_span());
                    return self.parse_postfix(Expression::ArrowFn(Box::new(ArrowFnExpr {
                        params,
                        body,
                        span,
                    })));
                }
                // Type.Variant (e.g. Color.Red) — not field access on a value (e.g. c.value).
                let is_type_name = name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase() || c == '_');
                if is_type_name && check(&self.tokens, self.position, &TokenKind::Dot) {
                    self.advance();
                    if let TokenKind::Identifier(variant) = self.current_kind().clone() {
                        self.advance();
                        let span = merge_spans(&self.prev_span(), &self.current_span());
                        let ev = Expression::EnumVariant(EnumVariantExpr {
                            enum_name: Some(name),
                            variant,
                            args: vec![],
                            span,
                        });
                        return self.parse_enum_variant_args(ev);
                    }
                }
                let type_args = if self.looks_like_generic_type_args() {
                    self.parse_call_type_args()
                } else {
                    vec![]
                };
                if check(&self.tokens, self.position, &TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    skip_newlines(&self.tokens, &mut self.position);
                    if !check(&self.tokens, self.position, &TokenKind::RParen) {
                        loop {
                            skip_newlines(&self.tokens, &mut self.position);
                            args.push(self.parse_expression());
                            skip_newlines(&self.tokens, &mut self.position);
                            if check(&self.tokens, self.position, &TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RParen,
                        "Expected ')' after arguments",
                        &mut self.errors,
                    );
                    let span = self.prev_span();
                    self.parse_postfix(Expression::Call(CallExpr {
                        callee: name,
                        type_args,
                        args,
                        span,
                    }))
                } else if check(&self.tokens, self.position, &TokenKind::LBrace)
                    && self.looks_like_struct_literal(&name)
                {
                    let lit_name = Self::mangle_instantiated_name(&name, &type_args);
                    self.parse_struct_literal(lit_name)
                } else {
                    let span = self.prev_span();
                    self.parse_postfix(Expression::Variable { name, span })
                }
            }
            TokenKind::LParen => {
                let start = self.current_span();
                self.advance();
                skip_newlines(&self.tokens, &mut self.position);
                if self.looks_like_arrow_fn() {
                    let params = self.parse_arrow_params();
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::FatArrow,
                        "Expected '=>' after arrow function parameters",
                        &mut self.errors,
                    );
                    let body = self.parse_arrow_body();
                    let span = merge_spans(&start, &self.prev_span());
                    return self.parse_postfix(Expression::ArrowFn(Box::new(ArrowFnExpr {
                        params,
                        body,
                        span,
                    })));
                }
                let first = self.parse_expression();
                if check(&self.tokens, self.position, &TokenKind::Comma) {
                    let mut elems = vec![first];
                    loop {
                        self.advance(); // comma
                        skip_newlines(&self.tokens, &mut self.position);
                        elems.push(self.parse_expression());
                        if !check(&self.tokens, self.position, &TokenKind::Comma) {
                            break;
                        }
                    }
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RParen,
                        "Expected ')' after tuple literal",
                        &mut self.errors,
                    );
                    return self.parse_postfix(Expression::TupleLiteral(elems));
                }
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::RParen,
                    "Expected ')' after expression",
                    &mut self.errors,
                );
                self.parse_postfix(Expression::Grouped(Box::new(first)))
            }
            _ => {
                self.parse_error_here(                    "Invalid expression",
                );
                if !is_at_end(&self.tokens, self.position) {
                    self.advance();
                }
                Expression::Invalid
            }
        }
    }

    pub(super) fn parse_match(&mut self) -> Expression {
        self.advance();
        let scrutinee = self.parse_expression();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{' after match scrutinee",
            &mut self.errors,
        );
        let mut arms = Vec::new();
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::RBrace) {
                break;
            }
            let pattern = self.parse_match_pattern();
            let guard = if check(&self.tokens, self.position, &TokenKind::If) {
                self.advance();
                Some(self.parse_expression())
            } else {
                None
            };
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::FatArrow,
                "Expected '=>' in match arm",
                &mut self.errors,
            );
            let body = self.parse_match_arm_body();
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' after match arms",
            &mut self.errors,
        );
        let span = merge_spans(&self.prev_span(), &expr_span(&scrutinee));
        Expression::Match(Box::new(MatchExpr {
            scrutinee: Box::new(scrutinee),
            arms,
            span,
        }))
    }

    fn parse_match_arm_body(&mut self) -> Block {
        skip_newlines(&self.tokens, &mut self.position);
        let block = if check(&self.tokens, self.position, &TokenKind::LBrace) {
            self.parse_block()
        } else {
            block_from_expr(self.parse_or())
        };
        if check(&self.tokens, self.position, &TokenKind::Comma) {
            self.advance();
        }
        block
    }

    fn is_spread_token(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::DotDot | TokenKind::DotDotDot
        )
    }

    fn consume_spread_token(&mut self) {
        if matches!(self.current_kind(), TokenKind::DotDot | TokenKind::DotDotDot) {
            self.advance();
        }
    }

    pub(super) fn parse_struct_literal(&mut self, name: String) -> Expression {
        self.advance(); // {
        let mut spreads = Vec::new();
        let mut fields = Vec::new();
        skip_newlines(&self.tokens, &mut self.position);
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            if self.is_spread_token() {
                self.consume_spread_token();
                spreads.push(self.parse_expression());
                skip_newlines(&self.tokens, &mut self.position);
                if check(&self.tokens, self.position, &TokenKind::Comma) {
                    self.advance();
                }
                continue;
            }
            let fname = match self.current_kind() {
                TokenKind::Identifier(n) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => break,
            };
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::Colon,
                "Expected ':' in struct literal",
                &mut self.errors,
            );
            let value = self.parse_expression();
            fields.push((fname, value));
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' after struct literal",
            &mut self.errors,
        );
        Expression::StructLiteral(StructLiteralExpr {
            name,
            spreads,
            fields,
            span: self.prev_span(),
        })
    }

    pub(super) fn parse_postfix(&mut self, expr: Expression) -> Expression {
        self.parse_postfix_ext(expr)
    }

    pub(super) fn make_binary(&self, left: Expression, op: BinaryOp, right: Expression) -> Expression {
        let span = merge_spans(&expr_span(&left), &expr_span(&right));
        Expression::Binary(Box::new(BinaryExpr {
            left,
            op,
            right,
            span,
        }))
    }

    pub(super) fn make_unary(&self, op: UnaryOp, operand: Expression) -> Expression {
        let span = expr_span(&operand);
        Expression::Unary(Box::new(UnaryExpr { op, operand, span }))
    }
}

#[cfg(test)]
mod expr_tests {
    use ast::{Expression, Literal};
    use lexer::Lexer;

    fn parse_expr(src: &str) -> (Expression, crate::Parser) {
        let (tokens, _) = Lexer::new(src, "t.ny").tokenize();
        let mut p = crate::Parser::new(tokens);
        let expr = p.parse_expression();
        (expr, p)
    }

    #[test]
    fn struct_literal_expression_consumes_closing_brace() {
        let (expr, p) = parse_expr(r#"NumberColor { number: 1, color: "red" }"#);
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        assert!(
            matches!(expr, Expression::StructLiteral(ref sl) if sl.name == "NumberColor"),
            "{expr:?}"
        );
    }

    #[test]
    fn array_of_struct_literals_expression_consumes_closing_bracket() {
        let src = r#"[NumberColor { number: 1, color: "red" }, NumberColor { number: 2, color: "blue" }]"#;
        let (expr, p) = parse_expr(src);
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let Expression::ArrayLiteral(al) = expr else {
            panic!("expected array, got {expr:?}");
        };
        assert_eq!(al.elems.len(), 2);
    }

    #[test]
    fn multiline_int_array_consumes_closing_bracket() {
        let src = "[\n    1,\n    2\n]";
        let (expr, p) = parse_expr(src);
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let Expression::ArrayLiteral(al) = expr else {
            panic!("expected array, got {expr:?}");
        };
        assert_eq!(al.elems.len(), 2);
        assert!(matches!(al.elems[1], Expression::Literal(Literal::Int(2))));
    }

    #[test]
    fn array_of_struct_literals_multiline_consumes_closing_bracket() {
        let src = r#"[
    NumberColor { number: 1, color: "red" },
    NumberColor { number: 2, color: "blue" }
]"#;
        let (expr, p) = parse_expr(src);
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let Expression::ArrayLiteral(al) = expr else {
            panic!("expected array, got {expr:?}");
        };
        assert_eq!(al.elems.len(), 2);
    }

    #[test]
    fn array_literal_allows_trailing_comma() {
        let (expr, p) = parse_expr("[1, 2, 3,]");
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let Expression::ArrayLiteral(al) = expr else {
            panic!("expected array, got {expr:?}");
        };
        assert_eq!(al.elems.len(), 3);
    }

    #[test]
    fn parses_array_spread_literal() {
        let (expr, p) = parse_expr("[...nums, 1, 2]");
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let Expression::ArrayLiteral(al) = expr else {
            panic!("expected array, got {expr:?}");
        };
        assert_eq!(al.spreads.len(), 1);
        assert_eq!(al.elems.len(), 2);
    }

    #[test]
    fn parses_object_spread_literal() {
        let (expr, p) = parse_expr("{ ...obj, age: 21 }");
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let Expression::StructLiteral(sl) = expr else {
            panic!("expected struct literal, got {expr:?}");
        };
        assert!(sl.name.is_empty());
        assert_eq!(sl.spreads.len(), 1);
        assert_eq!(sl.fields.len(), 1);
    }
}

