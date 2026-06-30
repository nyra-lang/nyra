//! Extended syntax parsing (module, const, impl, types, match patterns, etc.)

use std::collections::HashMap;

use ast::*;
use ast::expr_span;
use lexer::{Token, TokenKind};
use crate::recovery::{check, consume, is_at_end, merge_spans, skip_chain_newlines, skip_newlines};
use crate::Parser;

impl Parser {
    /// `pub` → true, `priv` → false, default true.
    pub(super) fn parse_item_visibility(&mut self) -> bool {
        if check(&self.tokens, self.position, &TokenKind::Pub) {
            self.advance();
            return true;
        }
        if check(&self.tokens, self.position, &TokenKind::Priv) {
            self.advance();
            return false;
        }
        true
    }

    pub(super) fn parse_module_decl(&mut self) -> Option<String> {
        self.advance();
        let mut path = String::new();
        while let TokenKind::Identifier(part) = self.current_kind() {
            if !path.is_empty() {
                path.push('.');
            }
            path.push_str(part);
            self.advance();
            if !check(&self.tokens, self.position, &TokenKind::Dot) {
                break;
            }
            self.advance();
        }
        if path.is_empty() {
            self.parse_error_here(                "Expected module path after 'module'",
            );
            None
        } else {
            Some(path)
        }
    }

    pub(super) fn parse_const_def(&mut self) -> Option<ConstDef> {
        let public = self.parse_item_visibility();
        self.advance();
        let name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return None,
        };
        let ty = if check(&self.tokens, self.position, &TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_annotation())
        } else {
            None
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Equal,
            "Expected '=' in const",
            &mut self.errors,
        );
        let value = self.parse_expression();
        Some(ConstDef {
            name,
            ty,
            value,
            public,
        })
    }

    pub(super) fn parse_impl(&mut self) -> Option<ImplDef> {
        self.advance();
        let type_name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return None,
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{' after impl type",
            &mut self.errors,
        );
        skip_newlines(&self.tokens, &mut self.position);
        let mut methods = Vec::new();
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            if let Some(mut f) = self.parse_function_or_sync() {
                for p in &mut f.params {
                    if p.name == "self" && matches!(p.ty, TypeAnnotation::Integer(IntKind::I32)) {
                        p.ty = TypeAnnotation::Struct(type_name.clone());
                    }
                }
                f.name = format!("{type_name}_{}", f.name);
                methods.push(f);
            }
            skip_newlines(&self.tokens, &mut self.position);
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' after impl block",
            &mut self.errors,
        );
        Some(ImplDef { type_name, methods })
    }

    pub(super) fn parse_type_params(&mut self) -> (Vec<String>, Vec<String>, HashMap<String, Vec<String>>) {
        let mut lifetimes = Vec::new();
        let mut types = Vec::new();
        let mut bounds = HashMap::new();
        if !check(&self.tokens, self.position, &TokenKind::Less) {
            return (lifetimes, types, bounds);
        }
        self.advance();
        loop {
            match self.current_kind().clone() {
                TokenKind::Lifetime(lt) => {
                    lifetimes.push(lt);
                    self.advance();
                }
                TokenKind::Identifier(n) => {
                    types.push(n.clone());
                    self.advance();
                    if check(&self.tokens, self.position, &TokenKind::Colon) {
                        self.advance();
                        let mut trait_bounds = Vec::new();
                        loop {
                            match self.current_kind().clone() {
                                TokenKind::Identifier(tr) => {
                                    trait_bounds.push(tr);
                                    self.advance();
                                }
                                _ => break,
                            }
                            if check(&self.tokens, self.position, &TokenKind::Plus) {
                                self.advance();
                                continue;
                            }
                            break;
                        }
                        if !trait_bounds.is_empty() {
                            bounds.insert(n, trait_bounds);
                        }
                    }
                }
                _ => break,
            }
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Greater,
            "Expected '>' after type parameters",
            &mut self.errors,
        );
        (lifetimes, types, bounds)
    }

    fn parse_fn_ptr_type(&mut self) -> TypeAnnotation {
        self.advance(); // fn
        let (lifetime_params, _, _) = self.parse_type_params();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LParen,
            "Expected '(' after fn type",
            &mut self.errors,
        );
        let mut params = Vec::new();
        skip_newlines(&self.tokens, &mut self.position);
        while !check(&self.tokens, self.position, &TokenKind::RParen)
            && !is_at_end(&self.tokens, self.position)
        {
            params.push(self.parse_type_annotation_ext());
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
            skip_newlines(&self.tokens, &mut self.position);
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RParen,
            "Expected ')' after fn type params",
            &mut self.errors,
        );
        let return_type = if check(&self.tokens, self.position, &TokenKind::Arrow) {
            self.advance();
            Some(Box::new(self.parse_type_annotation_ext()))
        } else {
            None
        };
        TypeAnnotation::FnPtr {
            lifetime_params,
            params,
            return_type,
        }
    }

    pub(super) fn parse_type_annotation_ext(&mut self) -> TypeAnnotation {
        if check(&self.tokens, self.position, &TokenKind::Dyn) {
            self.advance();
            let trait_name = match self.current_kind() {
                TokenKind::Identifier(n) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => {
                    self.parse_error_here("Expected trait name after 'dyn'");
                    return TypeAnnotation::Void;
                }
            };
            let mut bounds = Vec::new();
            while check(&self.tokens, self.position, &TokenKind::Plus) {
                self.advance();
                if let TokenKind::Identifier(b) = self.current_kind().clone() {
                    bounds.push(b);
                    self.advance();
                } else {
                    break;
                }
            }
            return TypeAnnotation::DynTrait { trait_name, bounds };
        }
        if check(&self.tokens, self.position, &TokenKind::For) {
            self.advance();
            let (lifetimes, _, _) = self.parse_type_params();
            let inner = Box::new(self.parse_type_annotation_ext());
            return TypeAnnotation::ForAll { lifetimes, inner };
        }
        if check(&self.tokens, self.position, &TokenKind::Fn) {
            return self.parse_fn_ptr_type();
        }
        if check(&self.tokens, self.position, &TokenKind::LParen) {
            self.advance();
            let mut elems = Vec::new();
            skip_newlines(&self.tokens, &mut self.position);
            if !check(&self.tokens, self.position, &TokenKind::RParen) {
                loop {
                    elems.push(self.parse_type_annotation_ext());
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
                "Expected ')' after tuple type",
                &mut self.errors,
            );
            return TypeAnnotation::Tuple(elems);
        }
        if check(&self.tokens, self.position, &TokenKind::LBracket) {
            self.advance();
            let elem = Box::new(self.parse_type_annotation_ext());
            if check(&self.tokens, self.position, &TokenKind::Colon)
                || check(&self.tokens, self.position, &TokenKind::Semicolon)
            {
                self.advance();
                let len = if let TokenKind::Number(n) = self.current_kind().clone() {
                    self.advance();
                    Some(n as usize)
                } else {
                    None
                };
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::RBracket,
                    "Expected ']' after [T; N]",
                    &mut self.errors,
                );
                return TypeAnnotation::Array { elem, len };
            }
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::RBracket,
                "Expected ']' after [T]",
                &mut self.errors,
            );
            return TypeAnnotation::Array { elem, len: None };
        }
        if check(&self.tokens, self.position, &TokenKind::Star) {
            self.advance();
            // Rust-style `*const T` / `*mut T` — both lower to typed raw `*T`.
            if check(&self.tokens, self.position, &TokenKind::Const) {
                self.advance();
            } else if check(&self.tokens, self.position, &TokenKind::Mut) {
                self.advance();
            }
            let inner = Box::new(self.parse_type_annotation_ext());
            return TypeAnnotation::RawPtr { inner };
        }
        if check(&self.tokens, self.position, &TokenKind::Ampersand) {
            self.advance();
            let lifetime = if let TokenKind::Lifetime(lt) = self.current_kind().clone() {
                self.advance();
                Some(lt)
            } else {
                None
            };
            let mutable = if check(&self.tokens, self.position, &TokenKind::Mut) {
                self.advance();
                true
            } else {
                false
            };
            let inner = Box::new(self.parse_type_annotation_ext());
            return TypeAnnotation::Ref {
                inner,
                mutable,
                lifetime,
            };
        }
        if check(&self.tokens, self.position, &TokenKind::Less) {
            self.advance();
            if let TokenKind::Identifier(n) = self.current_kind().clone() {
                self.advance();
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::Greater,
                    "Expected '>' after generic type param",
                    &mut self.errors,
                );
                return TypeAnnotation::Generic(n);
            }
        }
        self.parse_type_annotation_base()
    }

    pub(super) fn parse_type_annotation_base(&mut self) -> TypeAnnotation {
        match self.current_kind() {
            TokenKind::TypeInt(k) => {
                let k = *k;
                self.advance();
                TypeAnnotation::Integer(k)
            }
            TokenKind::TypeF32 => {
                self.advance();
                TypeAnnotation::F32
            }
            TokenKind::TypeF64 => {
                self.advance();
                TypeAnnotation::F64
            }
            TokenKind::TypeChar => {
                self.advance();
                TypeAnnotation::Char
            }
            TokenKind::TypeBool => {
                self.advance();
                TypeAnnotation::Bool
            }
            TokenKind::TypeString => {
                self.advance();
                TypeAnnotation::String
            }
            TokenKind::TypePtr => {
                self.advance();
                TypeAnnotation::Ptr
            }
            TokenKind::TypeVoid => {
                self.advance();
                TypeAnnotation::Void
            }
            TokenKind::Identifier(n) => {
                let name = n.clone();
                self.advance();
                if name == "VecStr" {
                    return TypeAnnotation::VecStr;
                }
                if check(&self.tokens, self.position, &TokenKind::Less) {
                    return self.parse_applied_type(name);
                }
                if self.enums_contains(&name)
                    || matches!(
                        name.as_str(),
                        "option" | "Option" | "result" | "Result"
                    )
                {
                    TypeAnnotation::Enum(name)
                } else {
                    TypeAnnotation::Struct(name)
                }
            }
            _ => {
                self.parse_error_here(                    "Expected type annotation",
                );
                TypeAnnotation::Integer(IntKind::I32)
            }
        }
    }

    fn parse_applied_type(&mut self, base: String) -> TypeAnnotation {
        self.advance(); // <
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type_annotation_ext());
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.consume_generic_close("Expected '>' after generic type args");
        TypeAnnotation::Applied { base, args }
    }

    /// Accept `>` or split `>>` (`Shr`) for nested generics like `Vec<Vec<i32>>`.
    fn consume_generic_close(&mut self, msg: &str) {
        if check(&self.tokens, self.position, &TokenKind::Greater) {
            self.advance();
            return;
        }
        if check(&self.tokens, self.position, &TokenKind::Shr) {
            let span = self.tokens[self.position].span.clone();
            self.tokens[self.position].kind = TokenKind::Greater;
            self.tokens.insert(
                self.position + 1,
                Token {
                    kind: TokenKind::Greater,
                    span,
                },
            );
            self.advance();
            return;
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Greater,
            msg,
            &mut self.errors,
        );
    }

    fn enums_contains(&self, name: &str) -> bool {
        self.parsed_enum_names.iter().any(|e| e == name)
    }

    pub(super) fn structs_contains(&self, name: &str) -> bool {
        self.parsed_struct_names.iter().any(|s| s == name)
    }

    pub(super) fn mangle_instantiated_name(base: &str, type_args: &[TypeAnnotation]) -> String {
        if type_args.is_empty() {
            return base.to_string();
        }
        let suffix: String = type_args
            .iter()
            .map(Self::mangle_type_ann_for_inst)
            .collect::<Vec<_>>()
            .join("_");
        format!("{base}__{suffix}")
    }

    fn mangle_type_ann_for_inst(t: &TypeAnnotation) -> String {
        match t {
            TypeAnnotation::Integer(k) => k.name().into(),
            TypeAnnotation::F32 => "f32".into(),
            TypeAnnotation::F64 => "f64".into(),
            TypeAnnotation::Char => "char".into(),
            TypeAnnotation::Bool => "bool".into(),
            TypeAnnotation::String => "string".into(),
            TypeAnnotation::VecStr => "vec_str".into(),
            TypeAnnotation::Ptr => "ptr".into(),
            TypeAnnotation::Void => "void".into(),
            TypeAnnotation::Struct(n) | TypeAnnotation::Enum(n) => n.clone(),
            TypeAnnotation::Applied { base, args } => Self::mangle_instantiated_name(base, args),
            TypeAnnotation::Generic(n) => n.clone(),
            _ => "unknown".into(),
        }
    }

    /// `Name { field: expr }` but not `Name { return ... }` (if/while block).
    pub(super) fn looks_like_struct_literal(&self, name: &str) -> bool {
        if self.enums_contains(name) {
            return false;
        }
        if !name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        {
            return false;
        }
        if !check(&self.tokens, self.position, &TokenKind::LBrace) {
            return false;
        }
        let mut p = self.position + 1;
        skip_newlines(&self.tokens, &mut p);
        match self.tokens.get(p).map(|t| &t.kind) {
            Some(TokenKind::DotDot) | Some(TokenKind::DotDotDot) => true,
            Some(TokenKind::Identifier(_))
            | Some(TokenKind::Module)
            | Some(TokenKind::Clone) => self
                .tokens
                .get(p + 1)
                .map(|t| &t.kind)
                == Some(&TokenKind::Colon),
            _ => false,
        }
    }

    pub(super) fn parse_match_pattern(&mut self) -> MatchPattern {
        let first = self.parse_match_pattern_atom();
        let mut patterns = vec![first];
        while matches!(self.current_kind(), TokenKind::BitOr) {
            self.advance();
            patterns.push(self.parse_match_pattern_atom());
        }
        if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            MatchPattern::Or(patterns)
        }
    }

    fn parse_match_pattern_atom(&mut self) -> MatchPattern {
        if check(&self.tokens, self.position, &TokenKind::LParen) {
            self.advance();
            let mut elems = vec![self.parse_match_payload_pattern()];
            while check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
                elems.push(self.parse_match_payload_pattern());
            }
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::RParen,
                "Expected ')' after tuple match pattern",
                &mut self.errors,
            );
            return MatchPattern::Tuple(elems);
        }
        if let TokenKind::StringLit(lit) = self.current_kind().clone() {
            self.advance();
            return MatchPattern::Literal(lit);
        }
        if let TokenKind::Number(n) | TokenKind::NumberSuffix(n, _) = self.current_kind().clone() {
            self.advance();
            return MatchPattern::Variant(n.to_string());
        }
        if matches!(self.current_kind(), TokenKind::True) {
            self.advance();
            return MatchPattern::Variant("true".into());
        }
        if matches!(self.current_kind(), TokenKind::False) {
            self.advance();
            return MatchPattern::Variant("false".into());
        }
        if let TokenKind::Identifier(p) = self.current_kind().clone() {
            if p == "_" {
                self.advance();
                return MatchPattern::Wildcard;
            }
            self.advance();
            if check(&self.tokens, self.position, &TokenKind::LBrace) {
                self.advance();
                let fields = self.parse_struct_match_fields();
                return MatchPattern::Struct(p, fields);
            }
            if check(&self.tokens, self.position, &TokenKind::Dot) {
                self.advance();
                if let TokenKind::Identifier(v) = self.current_kind().clone() {
                    self.advance();
                    if check(&self.tokens, self.position, &TokenKind::LParen) {
                        self.advance();
                        let payload = self.parse_match_payload_pattern();
                        consume(
                            &self.tokens,
                            &mut self.position,
                            TokenKind::RParen,
                            "Expected ')' after match bind",
                            &mut self.errors,
                        );
                        return MatchPattern::QualifiedBind(p, v, payload);
                    }
                    return MatchPattern::Qualified(p, v);
                }
            }
            return MatchPattern::Variant(p);
        }
        MatchPattern::Wildcard
    }

    fn parse_match_payload_pattern(&mut self) -> MatchPayloadPattern {
        if let TokenKind::Identifier(p) = self.current_kind().clone() {
            if p == "_" {
                self.advance();
                return MatchPayloadPattern::Wildcard;
            }
            self.advance();
            if check(&self.tokens, self.position, &TokenKind::Dot) {
                self.advance();
                if let TokenKind::Identifier(v) = self.current_kind().clone() {
                    self.advance();
                    if check(&self.tokens, self.position, &TokenKind::LParen) {
                        self.advance();
                        let inner = self.parse_match_payload_pattern();
                        consume(
                            &self.tokens,
                            &mut self.position,
                            TokenKind::RParen,
                            "Expected ')' after nested match pattern",
                            &mut self.errors,
                        );
                        return MatchPayloadPattern::Nested(Box::new(MatchPattern::QualifiedBind(
                            p, v, inner,
                        )));
                    }
                    return MatchPayloadPattern::Nested(Box::new(MatchPattern::Qualified(p, v)));
                }
            }
            if check(&self.tokens, self.position, &TokenKind::LParen) {
                self.advance();
                let inner = self.parse_match_payload_pattern();
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::RParen,
                    "Expected ')' after nested match pattern",
                    &mut self.errors,
                );
                return MatchPayloadPattern::Nested(Box::new(MatchPattern::QualifiedBind(
                    String::new(),
                    p,
                    inner,
                )));
            }
            return MatchPayloadPattern::Bind(p);
        }
        MatchPayloadPattern::Wildcard
    }

    fn parse_struct_match_fields(&mut self) -> Vec<StructMatchField> {
        let mut fields = Vec::new();
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            if let TokenKind::Identifier(field) = self.current_kind().clone() {
                self.advance();
                let bind = if check(&self.tokens, self.position, &TokenKind::Colon) {
                    self.advance();
                    if let TokenKind::Identifier(b) = self.current_kind().clone() {
                        self.advance();
                        Some(b)
                    } else {
                        self.parse_error_here("Expected bind name after ':' in struct pattern");
                        Some("_".into())
                    }
                } else {
                    None
                };
                fields.push(StructMatchField { field, bind });
                if check(&self.tokens, self.position, &TokenKind::Comma) {
                    self.advance();
                }
            } else {
                break;
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' after struct match pattern",
            &mut self.errors,
        );
        fields
    }

    pub(super) fn parse_enum_variant_args(&mut self, mut expr: Expression) -> Expression {
        if let Expression::EnumVariant(ref mut ev) = expr {
            if check(&self.tokens, self.position, &TokenKind::LParen) {
                self.advance();
                skip_newlines(&self.tokens, &mut self.position);
                if !check(&self.tokens, self.position, &TokenKind::RParen) {
                    ev.args.push(self.parse_expression());
                    skip_newlines(&self.tokens, &mut self.position);
                }
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::RParen,
                    "Expected ')' after enum variant args",
                    &mut self.errors,
                );
            }
        }
        self.parse_postfix_ext(expr)
    }

    pub(super) fn parse_if_expr(&mut self) -> Expression {
        self.advance();
        let condition = self.parse_expression();
        let then_block = self.parse_block();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Else,
            "Expected 'else' in if expression",
            &mut self.errors,
        );
        let else_block = self.parse_block();
        let span = merge_spans(
            &expr_span(&condition),
            &then_block
                .statements
                .last()
                .map(stmt_span)
                .unwrap_or_else(|| expr_span(&condition)),
        );
        Expression::If(Box::new(IfExpr {
            condition,
            then_block,
            else_block,
            span,
        }))
    }

    pub(super) fn parse_braced_expr(&mut self) -> Expression {
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{'",
            &mut self.errors,
        );
        skip_newlines(&self.tokens, &mut self.position);
        let expr = self.parse_expression();
        skip_newlines(&self.tokens, &mut self.position);
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}'",
            &mut self.errors,
        );
        expr
    }

    fn is_type_arg_start(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::TypeInt(_)
                | TokenKind::TypeBool
                | TokenKind::TypeString
                | TokenKind::TypePtr
                | TokenKind::TypeVoid
                | TokenKind::Identifier(_)
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::Fn
        )
    }

    pub(super) fn looks_like_generic_type_args(&self) -> bool {
        let mut p = self.position;
        if !matches!(self.tokens.get(p).map(|t| &t.kind), Some(TokenKind::Less)) {
            return false;
        }
        p += 1;
        if !self
            .tokens
            .get(p)
            .map(|t| Self::is_type_arg_start(&t.kind))
            .unwrap_or(false)
        {
            return false;
        }
        p += 1;
        if !matches!(self.tokens.get(p).map(|t| &t.kind), Some(TokenKind::Greater)) {
            return false;
        }
        p += 1;
        matches!(
            self.tokens.get(p).map(|t| &t.kind),
            Some(TokenKind::LParen) | Some(TokenKind::LBrace)
        )
    }

    pub(super) fn parse_call_type_args(&mut self) -> Vec<TypeAnnotation> {
        let mut args = Vec::new();
        if !check(&self.tokens, self.position, &TokenKind::Less) {
            return args;
        }
        self.advance();
        loop {
            args.push(self.parse_type_annotation_ext());
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.consume_generic_close("Expected '>' after type arguments");
        args
    }

    pub(super) fn parse_field_after_dot(&mut self) -> Option<String> {
        skip_chain_newlines(&self.tokens, &mut self.position);
        match self.current_kind().clone() {
            TokenKind::Identifier(field) => {
                self.advance();
                Some(field)
            }
            TokenKind::Clone => {
                self.advance();
                Some("clone".into())
            }
            TokenKind::Module => {
                self.advance();
                Some("module".into())
            }
            TokenKind::Number(n) => {
                self.advance();
                Some(n.to_string())
            }
            _ => None,
        }
    }

    pub(super) fn parse_postfix_ext(&mut self, mut expr: Expression) -> Expression {
        loop {
            if self.errors_over_limit() {
                break;
            }
            skip_chain_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::LBracket) {
                self.advance();
                let index = self.parse_expression();
                if !consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::RBracket,
                    "Expected ']'",
                    &mut self.errors,
                ) {
                    break;
                }
                let span = self.prev_span();
                expr = Expression::Index(Box::new(IndexExpr {
                    object: expr,
                    index,
                    span,
                }));
                continue;
            }
            if check(&self.tokens, self.position, &TokenKind::QuestionDot) {
                self.advance();
                let field = if let Some(field) = self.parse_field_after_dot() {
                    field
                } else {
                    break;
                };
                if check(&self.tokens, self.position, &TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    skip_newlines(&self.tokens, &mut self.position);
                    if !check(&self.tokens, self.position, &TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expression());
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
                        "Expected ')'",
                        &mut self.errors,
                    );
                    expr = Expression::MethodCall(Box::new(MethodCallExpr {
                        object: expr,
                        method: field,
                        span: self.prev_span(),
                        args,
                        optional: true,
                    }));
                } else {
                    expr = Expression::FieldAccess(Box::new(FieldAccessExpr {
                        object: expr,
                        field,
                        optional: true,
                        span: self.prev_span(),
                    }));
                }
                continue;
            }
            if check(&self.tokens, self.position, &TokenKind::Question) {
                if super::recovery::looks_like_ternary_question(&self.tokens, self.position) {
                    break;
                }
                self.advance();
                let span = self.prev_span();
                expr = Expression::Unary(Box::new(UnaryExpr {
                    op: UnaryOp::Try,
                    operand: expr,
                    span,
                }));
                continue;
            }
            if check(&self.tokens, self.position, &TokenKind::Dot) {
                self.advance();
                let field = if let Some(field) = self.parse_field_after_dot() {
                    field
                } else {
                    break;
                };
                if check(&self.tokens, self.position, &TokenKind::LParen) {
                        self.advance();
                        let mut args = Vec::new();
                        skip_newlines(&self.tokens, &mut self.position);
                        if !check(&self.tokens, self.position, &TokenKind::RParen) {
                            loop {
                                args.push(self.parse_expression());
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
                            "Expected ')'",
                            &mut self.errors,
                        );
                        expr = Expression::MethodCall(Box::new(MethodCallExpr {
                            object: expr,
                            method: field,
                            span: self.prev_span(),
                            args,
                            optional: false,
                        }));
                    } else {
                        expr = Expression::FieldAccess(Box::new(FieldAccessExpr {
                            object: expr,
                            field,
                            optional: false,
                            span: self.prev_span(),
                        }));
                    }
                continue;
            }
            break;
        }
        expr
    }

    pub(super) fn parse_trait(&mut self) -> Option<TraitDef> {
        self.advance();
        let name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return None,
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{' after trait name",
            &mut self.errors,
        );
        let mut methods = Vec::new();
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::Fn) {
                self.advance();
                let mname = match self.current_kind() {
                    TokenKind::Identifier(n) => {
                        let n = n.clone();
                        self.advance();
                        n
                    }
                    _ => continue,
                };
                consume(
                    &self.tokens,
                    &mut self.position,
                    TokenKind::LParen,
                    "Expected '('",
                    &mut self.errors,
                );
                let params = self.parse_params();
                let return_type = if check(&self.tokens, self.position, &TokenKind::Arrow) {
                    self.advance();
                    Some(self.parse_type_annotation())
                } else {
                    None
                };
                if check(&self.tokens, self.position, &TokenKind::Semicolon) {
                    self.advance();
                } else if check(&self.tokens, self.position, &TokenKind::LBrace) {
                    self.advance();
                    while !check(&self.tokens, self.position, &TokenKind::RBrace)
                        && !is_at_end(&self.tokens, self.position)
                    {
                        self.advance();
                    }
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RBrace,
                        "Expected '}'",
                        &mut self.errors,
                    );
                }
                methods.push(TraitMethodSig {
                    name: mname,
                    params,
                    return_type,
                });
            } else {
                self.advance();
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}'",
            &mut self.errors,
        );
        Some(TraitDef { name, methods })
    }

    pub(super) fn parse_macro_def(&mut self) -> Option<MacroDef> {
        self.advance();
        let name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return None,
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LParen,
            "Expected '(' after macro name",
            &mut self.errors,
        );
        let mut params = Vec::new();
        while let TokenKind::Identifier(p) = self.current_kind().clone() {
            params.push(p);
            self.advance();
            if matches!(self.current_kind(), TokenKind::Comma) {
                self.advance();
                continue;
            }
            break;
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RParen,
            "Expected ')'",
            &mut self.errors,
        );
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{'",
            &mut self.errors,
        );
        let body = self.parse_expression();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}'",
            &mut self.errors,
        );
        Some(MacroDef { name, params, body })
    }

    /// `impl Trait for Type { ... }` — returns `Some` or `None` if plain `impl`.
    pub(super) fn parse_trait_impl(&mut self) -> Option<TraitImpl> {
        if !check(&self.tokens, self.position, &TokenKind::Impl) {
            return None;
        }
        let saved = self.position;
        self.advance();
        let first = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                self.position = saved;
                return None;
            }
        };
        if !check(&self.tokens, self.position, &TokenKind::For) {
            self.position = saved;
            return None;
        }
        self.advance();
        let second = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                self.position = saved;
                return None;
            }
        };
        let (trait_name, type_name) = if self.structs_contains(&first) || self.enums_contains(&first) {
            (second, first)
        } else {
            (first, second)
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{'",
            &mut self.errors,
        );
        skip_newlines(&self.tokens, &mut self.position);
        let mut methods = Vec::new();
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            if let Some(mut f) = self.parse_function_or_sync() {
                for p in &mut f.params {
                    if p.name == "self" {
                        p.ty = TypeAnnotation::Struct(type_name.clone());
                    }
                }
                f.name = format!("{trait_name}_{type_name}_{}", f.name);
                methods.push(f);
            }
            skip_newlines(&self.tokens, &mut self.position);
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}'",
            &mut self.errors,
        );
        Some(TraitImpl {
            type_name,
            trait_name,
            methods,
        })
    }
}
