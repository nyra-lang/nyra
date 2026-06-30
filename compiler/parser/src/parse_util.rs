//! Token stream helpers shared across the parser.
use errors::Span;
use lexer::TokenKind;
use super::recovery::{skip_newlines, synchronize};

use super::Parser;

impl Parser {
    pub(super) fn current_kind(&self) -> &TokenKind {
        self.tokens
            .get(self.position)
            .or_else(|| self.tokens.last())
            .map(|t| &t.kind)
            .expect("parser requires at least one token")
    }

    pub(super) fn current_span(&self) -> Span {
        self.tokens
            .get(self.position)
            .map(|t| t.span.clone())
            .unwrap_or_default()
    }

    pub(super) fn prev_span(&self) -> Span {
        if self.position == 0 {
            return self.current_span();
        }
        self.tokens[self.position - 1].span.clone()
    }

    pub(super) fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
            skip_newlines(&self.tokens, &mut self.position);
        }
    }

    /// `clone expr` requires an operand; bare `clone` in expression position is a variable name.
    pub(super) fn looks_like_clone_operand(&self) -> bool {
        let Some(next) = self.tokens.get(self.position + 1) else {
            return false;
        };
        matches!(
            next.kind,
            TokenKind::Identifier(_)
                | TokenKind::SelfKw
                | TokenKind::LParen
                | TokenKind::LBrace
                | TokenKind::Number(_)
                | TokenKind::NumberSuffix(_, _)
                | TokenKind::Float { .. }
                | TokenKind::CharLit(_)
                | TokenKind::StringLit(_)
                | TokenKind::Clone
                | TokenKind::Module
                | TokenKind::True
                | TokenKind::False
        )
    }

    /// Binding / field name: ordinary identifiers plus soft keywords `clone` and `module`.
    pub(super) fn parse_binding_name(&mut self, context: &str) -> String {
        match self.current_kind().clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            TokenKind::Clone => {
                self.advance();
                "clone".to_string()
            }
            TokenKind::Module => {
                self.advance();
                "module".to_string()
            }
            _ => {
                self.parse_error_here(context);
                synchronize(&self.tokens, &mut self.position);
                "_invalid".into()
            }
        }
    }
}

