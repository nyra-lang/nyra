//! Top-level declarations: struct, enum, extern, function, export.
use ast::*;
use lexer::TokenKind;
use super::recovery::{check, consume, is_at_end, is_doc_comment, merge_spans, skip_newlines, synchronize};

use super::Parser;

impl Parser {
    pub(super) fn parse_leading_doc_comments(&mut self) -> Option<String> {
        let mut lines: Vec<String> = Vec::new();
        while self.position < self.tokens.len()
            && is_doc_comment(&self.tokens[self.position].kind)
        {
            if let TokenKind::DocComment(text) = self.tokens[self.position].kind.clone() {
                if !text.is_empty() {
                    lines.push(text);
                }
            }
            self.advance();
            skip_newlines(&self.tokens, &mut self.position);
        }
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    pub(super) fn parse_struct(&mut self) -> Option<StructDef> {
        let saved = self.position;
        let doc = self.parse_leading_doc_comments();
        let public = self.parse_item_visibility();
        if !check(&self.tokens, self.position, &TokenKind::Struct) {
            self.position = saved;
            return None;
        }
        self.advance(); // struct
        let name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                self.parse_error_here(                    "Expected struct name",
                );
                return None;
            }
        };
        skip_newlines(&self.tokens, &mut self.position);
        let (_, type_params, _) = self.parse_type_params();
        let mut attrs = StructAttrs::default();
        skip_newlines(&self.tokens, &mut self.position);
        while let TokenKind::Identifier(n) = self.current_kind().clone() {
            match n.as_str() {
                "Send" => {
                    attrs.send = true;
                    self.advance();
                }
                "Sync" => {
                    attrs.sync = true;
                    self.advance();
                }
                "Copy" => {
                    attrs.copy = true;
                    self.advance();
                }
                "repr" => {
                    self.advance();
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::LParen,
                        "Expected '(' after repr",
                        &mut self.errors,
                    );
                    if let TokenKind::Identifier(n) = self.current_kind().clone() {
                        if n == "C" {
                            attrs.repr_c = true;
                        }
                        self.advance();
                    }
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RParen,
                        "Expected ')' after repr(C)",
                        &mut self.errors,
                    );
                }
                _ => break,
            }
            skip_newlines(&self.tokens, &mut self.position);
        }
        let mut fields = Vec::new();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{' after struct name",
            &mut self.errors,
        );
        skip_newlines(&self.tokens, &mut self.position);
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            let fname = self.parse_binding_name("Expected field name");
            if fname == "_invalid" {
                break;
            }
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::Colon,
                "Expected ':' after field name",
                &mut self.errors,
            );
            let ty = self.parse_type_annotation();
            fields.push(StructField { name: fname, ty });
            skip_newlines(&self.tokens, &mut self.position);
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' after struct fields",
            &mut self.errors,
        );
        attrs.copy |= self.pending_struct_attrs.copy;
        self.pending_struct_attrs = StructAttrs::default();
        Some(StructDef {
            name,
            type_params,
            attrs,
            fields,
            doc,
            public,
        })
    }

    pub(super) fn looks_like_arrow_fn(&self) -> bool {
        let mut pos = self.position;
        skip_newlines(&self.tokens, &mut pos);
        if check(&self.tokens, pos, &TokenKind::RParen) {
            pos += 1;
            skip_newlines(&self.tokens, &mut pos);
            return check(&self.tokens, pos, &TokenKind::FatArrow);
        }
        // `((a, b)) =>` tuple-destructured single param
        if check(&self.tokens, pos, &TokenKind::LParen) {
            let mut p2 = pos + 1;
            skip_newlines(&self.tokens, &mut p2);
            if matches!(
                self.tokens.get(p2).map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
            ) {
                return true;
            }
        }
        match self.tokens.get(pos).map(|t| &t.kind) {
            Some(TokenKind::Identifier(_)) | Some(TokenKind::SelfKw) => {
                pos += 1;
                skip_newlines(&self.tokens, &mut pos);
                if check(&self.tokens, pos, &TokenKind::Colon) {
                    return true;
                }
                matches!(
                    self.tokens.get(pos).map(|t| &t.kind),
                    Some(TokenKind::Comma) | Some(TokenKind::RParen)
                )
            }
            _ => false,
        }
    }

    pub(super) fn parse_arrow_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        skip_newlines(&self.tokens, &mut self.position);
        if !check(&self.tokens, self.position, &TokenKind::RParen) {
            loop {
                skip_newlines(&self.tokens, &mut self.position);
                if check(&self.tokens, self.position, &TokenKind::RParen) {
                    break;
                }
                // `((a, b))` — single tuple-destructured param
                if check(&self.tokens, self.position, &TokenKind::LParen) {
                    self.advance();
                    skip_newlines(&self.tokens, &mut self.position);
                    let mut names = Vec::new();
                    if !check(&self.tokens, self.position, &TokenKind::RParen) {
                        loop {
                            skip_newlines(&self.tokens, &mut self.position);
                            if let TokenKind::Identifier(n) = self.current_kind().clone() {
                                self.advance();
                                names.push(n);
                            } else {
                                self.parse_error_here(                                    "Expected identifier in tuple destructure",
                                );
                                break;
                            }
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
                        "Expected ')' after tuple destructure",
                        &mut self.errors,
                    );
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RParen,
                        "Expected ')' after arrow parameter",
                        &mut self.errors,
                    );
                    let ty = if names.len() > 1 {
                        TypeAnnotation::Tuple(
                            names
                                .iter()
                                .map(|_| TypeAnnotation::Generic("_".into()))
                                .collect(),
                        )
                    } else {
                        TypeAnnotation::Generic("_".into())
                    };
                    params.push(Param {
                        name: names.first().cloned().unwrap_or_else(|| "_".into()),
                        ty,
                        destructure: names,
                        no_escape: false,
                        mutable: false,
                    });
                } else {
                    let no_escape = self.take_param_no_escape();
                    let mutable = self.parse_optional_mut();
                let (param_name, ty) = match self.current_kind() {
                        TokenKind::SelfKw => {
                            self.advance();
                            let ty = if check(&self.tokens, self.position, &TokenKind::Colon) {
                                self.advance();
                                self.parse_type_annotation()
                            } else {
                                TypeAnnotation::Integer(IntKind::I32)
                            };
                            ("self".to_string(), ty)
                        }
                        TokenKind::Identifier(n) => {
                            let n = n.clone();
                            self.advance();
                            let ty = if check(&self.tokens, self.position, &TokenKind::Colon) {
                                self.advance();
                                self.parse_type_annotation()
                            } else {
                                TypeAnnotation::Generic("_".into())
                            };
                            (n, ty)
                        }
                        _ => {
                            self.parse_error_here(                                "Expected parameter name",
                            );
                            break;
                        }
                    };
                    params.push(Param {
                        name: param_name,
                        ty,
                        destructure: vec![],
                        no_escape,
                        mutable,
                    });
                }
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
            "Expected ')' after parameters",
            &mut self.errors,
        );
        params
    }

    pub(super) fn parse_arrow_body(&mut self) -> ArrowBody {
        skip_newlines(&self.tokens, &mut self.position);
        if check(&self.tokens, self.position, &TokenKind::LBrace) {
            ArrowBody::Block(self.parse_block())
        } else {
            ArrowBody::Expr(self.parse_expression())
        }
    }

    pub(super) fn parse_optional_mut(&mut self) -> bool {
        if check(&self.tokens, self.position, &TokenKind::Mut) {
            self.advance();
            return true;
        }
        false
    }

    pub(super) fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        skip_newlines(&self.tokens, &mut self.position);
        if !check(&self.tokens, self.position, &TokenKind::RParen) {
            loop {
                skip_newlines(&self.tokens, &mut self.position);
                if check(&self.tokens, self.position, &TokenKind::RParen) {
                    break;
                }
                let no_escape = self.take_param_no_escape();
                let mutable = self.parse_optional_mut();
                let (param_name, ty) = match self.current_kind() {
                    TokenKind::SelfKw => {
                        self.advance();
                        let ty = if check(&self.tokens, self.position, &TokenKind::Colon) {
                            self.advance();
                            self.parse_type_annotation()
                        } else {
                            TypeAnnotation::Integer(IntKind::I32)
                        };
                        ("self".to_string(), ty)
                    }
                    TokenKind::Identifier(n) => {
                        let n = n.clone();
                        self.advance();
                        let ty = if check(&self.tokens, self.position, &TokenKind::Colon) {
                            self.advance();
                            self.parse_type_annotation()
                        } else {
                            TypeAnnotation::Generic("_".into())
                        };
                        (n, ty)
                    }
                    _ => {
                        self.parse_error_here(                            "Expected parameter name",
                        );
                        break;
                    }
                };
                params.push(Param {
                    name: param_name,
                    ty,
                    destructure: vec![],
                    no_escape,
                    mutable,
                });
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
            "Expected ')' after parameters",
            &mut self.errors,
        );
        params
    }

    pub(super) fn parse_extern(&mut self) -> Option<ExternFn> {
        self.advance(); // extern
        if !check(&self.tokens, self.position, &TokenKind::Fn) {
            self.parse_error_here(                "Expected 'fn' after extern",
            );
            return None;
        }
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
            "Expected '(' after extern function name",
            &mut self.errors,
        );
        let params = self.parse_params();
        let return_type = if check(&self.tokens, self.position, &TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_annotation())
        } else {
            None
        };
        Some(ExternFn {
            name,
            params,
            return_type,
        })
    }

    pub(super) fn parse_enum(&mut self) -> Option<EnumDef> {
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
        let (_, type_params, _) = self.parse_type_params();
        let mut variants = Vec::new();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{' after enum name",
            &mut self.errors,
        );
        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::RBrace) {
                break;
            }
            if let TokenKind::Identifier(v) = self.current_kind().clone() {
                self.advance();
                let mut fields = Vec::new();
                if check(&self.tokens, self.position, &TokenKind::LParen) {
                    self.advance();
                    fields.push(self.parse_type_annotation_ext());
                    consume(
                        &self.tokens,
                        &mut self.position,
                        TokenKind::RParen,
                        "Expected ')' after enum variant field type",
                        &mut self.errors,
                    );
                }
                variants.push(EnumVariantDef { name: v, fields });
            } else {
                break;
            }
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            }
        }
        skip_newlines(&self.tokens, &mut self.position);
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' after enum variants",
            &mut self.errors,
        );
        Some(EnumDef {
            name,
            type_params,
            variants,
            public,
        })
    }

    pub(super) fn parse_export_instance(&mut self) -> Option<ExportInstance> {
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Export,
            "Expected 'export'",
            &mut self.errors,
        );
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Inst,
            "Expected 'inst' after 'export'",
            &mut self.errors,
        );
        let fn_name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                self.parse_error_here(                    "Expected function name after 'export inst'",
                );
                synchronize(&self.tokens, &mut self.position);
                return None;
            }
        };
        let type_args = self.parse_call_type_args();
        if type_args.is_empty() {
            self.parse_error_here(                "Expected type arguments after export inst function name (e.g. id<i32>)",
            );
        }
        Some(ExportInstance { fn_name, type_args })
    }

    pub(super) fn parse_function(&mut self) -> Option<Function> {
        let saved = self.position;
        let doc = self.parse_leading_doc_comments();
        let fn_start = self.current_span();
        let public = self.parse_item_visibility();
        let exported = if check(&self.tokens, self.position, &TokenKind::Export) {
            self.advance();
            true
        } else {
            false
        };
        let is_test = if check(&self.tokens, self.position, &TokenKind::Test) {
            self.advance();
            true
        } else {
            false
        };
        let is_async = if check(&self.tokens, self.position, &TokenKind::Async) {
            self.advance();
            true
        } else {
            false
        };
        if !check(&self.tokens, self.position, &TokenKind::Fn) {
            // `async`/`test`/`export` without `fn` must not rewind to `saved` or the
            // top-level driver loops forever on the same token.
            if self.position > saved {
                synchronize(&self.tokens, &mut self.position);
            } else {
                self.position = saved;
            }
            return None;
        }
        self.advance(); // fn

        let name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                self.parse_error_here(                    "Expected function name after 'fn'",
                );
                synchronize(&self.tokens, &mut self.position);
                return None;
            }
        };

        let (lifetime_params, type_params, type_param_bounds) = self.parse_type_params();

        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LParen,
            "Expected '(' after function name",
            &mut self.errors,
        );

        let params = self.parse_params();

        let return_type = if check(&self.tokens, self.position, &TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_annotation())
        } else {
            None
        };

        let body = if check(&self.tokens, self.position, &TokenKind::LBrace) {
            self.parse_block()
        } else {
            let expr = self.parse_expression();
            Block {
                statements: vec![Statement::Return(ReturnStmt {
                    value: Some(expr),
                })],
            }
        };
        let fn_attrs = std::mem::take(&mut self.pending_fn_attrs);
        Some(Function {
            name,
            is_test,
            ignore_test: false,
            should_fail_test: false,
            is_async,
            exported,
            public,
            span: merge_spans(&fn_start, &self.prev_span()),
            type_params,
            type_param_bounds,
            lifetime_params,
            params,
            return_type,
            body,
            inline: fn_attrs.inline,
            hot: fn_attrs.hot,
            cold: fn_attrs.cold,
            comptime: fn_attrs.comptime,
            doc,
        })
    }

    /// Parse a function in a block context; on failure advance so the caller cannot spin.
    pub(super) fn parse_function_or_sync(&mut self) -> Option<Function> {
        let before = self.position;
        if let Some(f) = self.parse_function() {
            return Some(f);
        }
        if self.position == before {
            synchronize(&self.tokens, &mut self.position);
        }
        None
    }

    pub(super) fn take_param_no_escape(&mut self) -> bool {
        if matches!(self.current_kind(), TokenKind::AttrNoEscape) {
            self.advance();
            true
        } else {
            false
        }
    }
}

