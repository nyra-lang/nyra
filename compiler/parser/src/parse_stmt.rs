//! Statements, blocks, and control-flow parsing.
use ast::*;
use ast::expr_span;
use lexer::TokenKind;
use super::recovery::{check, consume, is_at_end, merge_spans, skip_newlines, synchronize};

use super::Parser;

impl Parser {
    pub(super) fn parse_block(&mut self) -> Block {
        let mut statements = Vec::new();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LBrace,
            "Expected '{' to start block",
            &mut self.errors,
        );
        skip_newlines(&self.tokens, &mut self.position);

        while !check(&self.tokens, self.position, &TokenKind::RBrace)
            && !is_at_end(&self.tokens, self.position)
        {
            let pos_before = self.position;
            match self.parse_statement() {
                Some(stmt) => statements.push(stmt),
                None => break,
            }
            skip_newlines(&self.tokens, &mut self.position);
            if self.position == pos_before {
                self.parse_error_here(                    "Stuck parsing block; skipping token",
                );
                self.advance();
            }
        }

        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RBrace,
            "Expected '}' to close block",
            &mut self.errors,
        );

        Block { statements }
    }

    pub(super) fn parse_statement(&mut self) -> Option<Statement> {
        if self.errors_over_limit() {
            return None;
        }
        skip_newlines(&self.tokens, &mut self.position);
        match self.current_kind().clone() {
            TokenKind::RBrace | TokenKind::Eof => None,
            TokenKind::Newline => {
                self.advance();
                None
            }
            TokenKind::Let => Some(self.parse_let()),
            TokenKind::Const => Some(self.parse_const_stmt()),
            TokenKind::Mut => Some(self.parse_mut_decl()),
            TokenKind::Return => Some(self.parse_return()),
            TokenKind::If => Some(self.parse_if()),
            TokenKind::While => Some(self.parse_while()),
            TokenKind::Parallel => Some(self.parse_parallel_for()),
            TokenKind::Progress => Some(self.parse_progress_for()),
            TokenKind::For => Some(self.parse_for()),
            TokenKind::Break => {
                let span = self.current_span();
                self.advance();
                Some(Statement::Break { span })
            }
            TokenKind::Continue => {
                let span = self.current_span();
                self.advance();
                Some(Statement::Continue { span })
            }
            TokenKind::Spawn => Some(self.parse_spawn()),
            TokenKind::Benchmark => Some(self.parse_benchmark()),
            TokenKind::Unsafe => Some(self.parse_unsafe()),
            TokenKind::Asm => Some(self.parse_asm()),
            TokenKind::Defer => {
                self.advance();
                let expr = self.parse_expression();
                Some(Statement::Defer(expr))
            }
            TokenKind::Print => {
                self.advance();
                Some(Statement::Print(self.parse_print_arg_list()))
            }
            TokenKind::Identifier(_) | TokenKind::Star | TokenKind::LParen | TokenKind::SelfKw => {
                Some(self.parse_assign_or_expression())
            }
            _ => {
                let pos = self.position;
                let expr = self.parse_expression();
                if self.position == pos {
                    return None;
                }
                Some(Statement::Expression(expr))
            }
        }
    }

    /// `mut x = expr` — shorthand for `let mut x = expr`
    pub(super) fn parse_mut_decl(&mut self) -> Statement {
        self.advance(); // mut
        let name = self.parse_binding_name("Expected variable name after 'mut'");

        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::Equal,
            "Expected '=' after variable name",
            &mut self.errors,
        );

        let value = self.parse_expression();
        Statement::Let(LetStmt {
            mutable: true,
            name,
            destructure: vec![],
            span: merge_spans(&self.prev_span(), &expr_span(&value)),
            ty: None,
            value,
        })
    }

    /// `lvalue = expr` or a bare expression statement
    pub(super) fn parse_assign_or_expression(&mut self) -> Statement {
        let saved = self.position;
        let target = self.parse_lvalue();
        if self.check_assign_equals() {
            let target_span = expr_span(&target);
            self.advance(); // consume '='
            let value = self.parse_expression();
            return Statement::Assign(AssignStmt {
                target,
                span: merge_spans(&target_span, &expr_span(&value)),
                value,
            });
        }
        self.position = saved;
        Statement::Expression(self.parse_expression())
    }

    /// Assignment target: variable, `*ptr`, field, or index (no binary operators).
    pub(super) fn parse_lvalue(&mut self) -> Expression {
        let expr = self.parse_unary();
        self.parse_postfix(expr)
    }

    pub(super) fn check_assign_equals(&self) -> bool {
        matches!(self.tokens.get(self.position).map(|t| &t.kind), Some(TokenKind::Equal))
    }

    pub(super) fn parse_print_arg_list(&mut self) -> PrintStmt {
        if !check(&self.tokens, self.position, &TokenKind::LParen) {
            return PrintStmt {
                args: vec![self.parse_expression()],
                color: None,
            };
        }
        self.advance();
        let mut args = Vec::new();
        let mut color = None;
        skip_newlines(&self.tokens, &mut self.position);
        while !check(&self.tokens, self.position, &TokenKind::RParen) {
            skip_newlines(&self.tokens, &mut self.position);
            if let TokenKind::Identifier(name) = &self.current_kind().clone() {
                if name == "color"
                    && self
                        .tokens
                        .get(self.position + 1)
                        .is_some_and(|t| matches!(t.kind, TokenKind::Colon))
                {
                    self.advance(); // color
                    self.advance(); // :
                    color = Some(self.parse_expression());
                    skip_newlines(&self.tokens, &mut self.position);
                    if check(&self.tokens, self.position, &TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
            args.push(self.parse_expression());
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RParen,
            "Expected ')' after print arguments",
            &mut self.errors,
        );
        PrintStmt { args, color }
    }

    pub(super) fn parse_embedded_expression(&mut self, src: &str) -> Expression {
        let (tokens, mut lex_errors) = lexer::Lexer::new(src, "<template>").tokenize();
        self.errors.append(&mut lex_errors);
        let mut embedded = Parser::new(tokens);
        let expr = embedded.parse_expression();
        self.errors.extend(embedded.errors);
        expr
    }

    pub(super) fn parse_let(&mut self) -> Statement {
        self.advance(); // let
        let mutable = if check(&self.tokens, self.position, &TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let (name, destructure, name_span) = if check(&self.tokens, self.position, &TokenKind::LParen) {
            self.advance();
            let mut names = Vec::new();
            skip_newlines(&self.tokens, &mut self.position);
            if !check(&self.tokens, self.position, &TokenKind::RParen) {
                loop {
                    if let TokenKind::Identifier(n) = self.current_kind().clone() {
                        self.advance();
                        names.push(n);
                    } else {
                        self.parse_error_here(                            "Expected identifier in destructure pattern",
                        );
                        break;
                    }
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
                "Expected ')' after destructure pattern",
                &mut self.errors,
            );
            let span = self.prev_span();
            (
                names.first().cloned().unwrap_or_else(|| "_tuple".into()),
                names,
                span,
            )
        } else {
            let n = self.parse_binding_name("Expected variable name after 'let'");
            (n.clone(), vec![], self.prev_span())
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
            "Expected '=' after variable name",
            &mut self.errors,
        );

        let value = self.parse_expression();
        Statement::Let(LetStmt {
            mutable,
            name,
            destructure,
            span: merge_spans(&name_span, &expr_span(&value)),
            ty,
            value,
        })
    }

    pub(super) fn parse_return(&mut self) -> Statement {
        self.advance();
        skip_newlines(&self.tokens, &mut self.position);
        let value = if matches!(
            self.current_kind(),
            TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        ) {
            None
        } else {
            Some(self.parse_expression())
        };
        Statement::Return(ReturnStmt { value })
    }

    pub(super) fn parse_if(&mut self) -> Statement {
        self.advance();
        skip_newlines(&self.tokens, &mut self.position);
        let condition = self.parse_expression();
        skip_newlines(&self.tokens, &mut self.position);
        let then_block = self.parse_block();
        skip_newlines(&self.tokens, &mut self.position);
        let else_block = if check(&self.tokens, self.position, &TokenKind::Else) {
            self.advance();
            skip_newlines(&self.tokens, &mut self.position);
            if check(&self.tokens, self.position, &TokenKind::If) {
                Some(Block {
                    statements: vec![self.parse_if()],
                })
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };
        Statement::If(IfStmt {
            condition,
            then_block,
            else_block,
        })
    }

    pub(super) fn parse_while(&mut self) -> Statement {
        self.advance();
        skip_newlines(&self.tokens, &mut self.position);
        let condition = self.parse_expression();
        skip_newlines(&self.tokens, &mut self.position);
        let body = self.parse_block();
        Statement::While(WhileStmt { condition, body })
    }

    pub(super) fn parse_spawn_kind(&mut self) -> SpawnKind {
        self.parse_optional_spawn_kind().unwrap_or(SpawnKind::Task)
    }

    /// `spawn:task` / `parallel:thread` suffix — `None` when no `:kind` is present.
    pub(super) fn parse_optional_spawn_kind(&mut self) -> Option<SpawnKind> {
        if !check(&self.tokens, self.position, &TokenKind::Colon) {
            return None;
        }
        self.advance();
        match self.current_kind() {
            TokenKind::Identifier(name) => {
                let kind = match name.as_str() {
                    "task" => SpawnKind::Task,
                    "thread" => SpawnKind::Thread,
                    other => {
                        self.parse_error_here(format!(
                            "expected `task` or `thread` after `:`, found `{other}`"
                        ));
                        SpawnKind::Task
                    }
                };
                self.advance();
                Some(kind)
            }
            _ => {
                self.parse_error_here("expected `task` or `thread` after `:`");
                Some(SpawnKind::Task)
            }
        }
    }

    pub(super) fn parse_spawn(&mut self) -> Statement {
        self.advance();
        let kind = self.parse_spawn_kind();
        let body = self.parse_block();
        Statement::Spawn(SpawnStmt { kind, body })
    }

    pub(super) fn parse_benchmark(&mut self) -> Statement {
        self.advance();
        let body = self.parse_block();
        Statement::Benchmark(body)
    }

    pub(super) fn parse_unsafe(&mut self) -> Statement {
        self.advance();
        let body = self.parse_block();
        Statement::Unsafe(body)
    }

    pub(super) fn parse_asm(&mut self) -> Statement {
        let start = self.current_span();
        self.advance();
        skip_newlines(&self.tokens, &mut self.position);
        let template = match self.current_kind() {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                self.parse_error_here(                    "Expected string literal after asm (e.g. asm \"nop\")",
                );
                String::new()
            }
        };
        Statement::Asm {
            template,
            span: start,
        }
    }

    pub(super) fn parse_parallel_for(&mut self) -> Statement {
        self.advance(); // parallel
        let suffix_kind = self.parse_optional_spawn_kind();
        let mut config = if matches!(self.current_kind(), TokenKind::LParen) {
            self.parse_parallel_config()
        } else {
            ParallelConfig::default()
        };
        if let Some(kind) = suffix_kind {
            config.kind = kind;
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::For,
            "Expected 'for' after 'parallel'",
            &mut self.errors,
        );
        self.parse_for_inner(Some(config), None)
    }

    pub(super) fn parse_progress_for(&mut self) -> Statement {
        self.advance(); // progress
        let config = if matches!(self.current_kind(), TokenKind::LParen) {
            self.parse_progress_config()
        } else {
            ProgressConfig::default()
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::For,
            "Expected 'for' after 'progress'",
            &mut self.errors,
        );
        self.parse_for_inner(None, Some(config))
    }

    fn parse_progress_config(&mut self) -> ProgressConfig {
        let mut config = ProgressConfig::default();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LParen,
            "Expected '(' after 'progress'",
            &mut self.errors,
        );
        if matches!(self.current_kind(), TokenKind::RParen) {
            self.advance();
            return config;
        }
        loop {
            let key = match self.current_kind() {
                TokenKind::Identifier(n) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => {
                    self.parse_error_here("Expected progress option name (e.g. label)");
                    break;
                }
            };
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::Equal,
                "Expected '=' after progress option",
                &mut self.errors,
            );
            match key.as_str() {
                "label" | "msg" | "message" => {
                    config.label = Some(self.parse_expression());
                }
                other => {
                    self.parse_error_here(&format!(
                        "Unknown progress option '{other}' (label)"
                    ));
                    let _ = self.parse_expression();
                }
            }
            skip_newlines(&self.tokens, &mut self.position);
            match self.current_kind() {
                TokenKind::Comma => self.advance(),
                TokenKind::RParen => break,
                _ => {
                    self.parse_error_here("Expected ',' or ')' in progress options");
                    break;
                }
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RParen,
            "Expected ')' to close progress options",
            &mut self.errors,
        );
        config
    }

    fn parse_parallel_config(&mut self) -> ParallelConfig {
        let mut config = ParallelConfig::default();
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::LParen,
            "Expected '(' after 'parallel'",
            &mut self.errors,
        );
        if matches!(self.current_kind(), TokenKind::RParen) {
            self.advance();
            return config;
        }
        loop {
            let key = match self.current_kind() {
                TokenKind::Identifier(n) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => {
                    self.parse_error_here("Expected parallel option name (e.g. max, mode)");
                    break;
                }
            };
            consume(
                &self.tokens,
                &mut self.position,
                TokenKind::Equal,
                "Expected '=' after parallel option",
                &mut self.errors,
            );
            match key.as_str() {
                "backend" | "kind" => {
                    let backend_name = match self.current_kind() {
                        TokenKind::Identifier(n) => {
                            let n = n.clone();
                            self.advance();
                            n
                        }
                        _ => {
                            self.parse_error_here(
                                "Expected backend: task, tasks, thread, or threads",
                            );
                            "task".into()
                        }
                    };
                    config.kind = match backend_name.as_str() {
                        "task" | "tasks" => SpawnKind::Task,
                        "thread" | "threads" => SpawnKind::Thread,
                        other => {
                            self.parse_error_here(&format!(
                                "Unknown parallel backend '{other}' (use task, tasks, thread, or threads)"
                            ));
                            SpawnKind::Task
                        }
                    };
                }
                "mode" => {
                    let mode_name = match self.current_kind() {
                        TokenKind::Identifier(n) => {
                            let n = n.clone();
                            self.advance();
                            n
                        }
                        _ => {
                            self.parse_error_here(
                                "Expected mode: auto, balanced, max_performance, or background",
                            );
                            "auto".into()
                        }
                    };
                    config.mode = match mode_name.as_str() {
                        "auto" => ParallelMode::Auto,
                        "balanced" => ParallelMode::Balanced,
                        "max_performance" => ParallelMode::MaxPerformance,
                        "background" => ParallelMode::Background,
                        _ => {
                            self.parse_error_here(&format!(
                                "Unknown parallel mode '{mode_name}' (use auto, balanced, max_performance, background)"
                            ));
                            ParallelMode::Auto
                        }
                    };
                }
                "max" | "max_threads" | "max_workers" | "cores" => {
                    let expr = self.parse_expression();
                    config.threads = ParallelThreads::Max(expr);
                    if key != "max" {
                        self.errors.push(errors::parallel_prefer_max(
                            self.current_span(),
                            &key,
                        ));
                    }
                }
                "threads" | "workers" => {
                    config.threads = ParallelThreads::Exact(self.parse_expression());
                }
                "cpu" => {
                    let expr = self.parse_term_no_mod();
                    if matches!(self.current_kind(), TokenKind::Percent) {
                        self.advance();
                        config.threads = ParallelThreads::CpuPercent(expr);
                    } else {
                        self.parse_error_here("Expected '%' after cpu value (e.g. cpu = 80%)");
                    }
                }
                other => {
                    self.parse_error_here(&format!(
                        "Unknown parallel option '{other}' (backend, mode, max, threads, cpu)"
                    ));
                    let _ = self.parse_expression();
                }
            }
            skip_newlines(&self.tokens, &mut self.position);
            match self.current_kind() {
                TokenKind::Comma => self.advance(),
                TokenKind::RParen => break,
                _ => {
                    self.parse_error_here("Expected ',' or ')' in parallel options");
                    break;
                }
            }
        }
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::RParen,
            "Expected ')' to close parallel options",
            &mut self.errors,
        );
        config
    }

    pub(super) fn parse_for(&mut self) -> Statement {
        self.advance(); // for
        self.parse_for_inner(None, None)
    }

    fn parse_for_inner(
        &mut self,
        parallel: Option<ParallelConfig>,
        progress: Option<ProgressConfig>,
    ) -> Statement {
        let var = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                self.parse_error_here(                    "Expected loop variable after for",
                );
                "_".into()
            }
        };
        consume(
            &self.tokens,
            &mut self.position,
            TokenKind::In,
            "Expected 'in' after for variable",
            &mut self.errors,
        );
        let first = self.parse_expression();
        let kind = if matches!(self.current_kind(), TokenKind::DotDot) {
            self.advance();
            let end = self.parse_expression();
            ForKind::Range {
                start: first,
                end,
            }
        } else {
            ForKind::Iterable { iterable: first }
        };
        skip_newlines(&self.tokens, &mut self.position);
        let body = self.parse_block();
        Statement::For(ForStmt {
            var,
            kind,
            body,
            parallel,
            progress,
        })
    }

    pub(super) fn parse_const_stmt(&mut self) -> Statement {
        self.advance();
        let name = match self.current_kind() {
            TokenKind::Identifier(n) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => "_".into(),
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
        Statement::Const(LetStmt {
            mutable: false,
            name,
            destructure: vec![],
            span: merge_spans(&self.prev_span(), &expr_span(&value)),
            ty,
            value,
        })
    }

    pub(super) fn parse_type_annotation(&mut self) -> TypeAnnotation {
        self.parse_type_annotation_ext()
    }
}

