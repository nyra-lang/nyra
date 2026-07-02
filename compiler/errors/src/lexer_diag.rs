use crate::{ErrorKind, NyraError, Span};
use crate::codes::*;

/// Build a coded lexer diagnostic from a message.
pub fn coded_lexer_error(span: Span, message: impl AsRef<str>) -> NyraError {
    let msg = message.as_ref();
    let (code, label, help) = classify_lexer_message(msg);
    let mut err = NyraError::coded(code, ErrorKind::Lexer, span, msg);
    if let Some(l) = label {
        err = err.label(l);
    }
    if let Some(h) = help {
        err = err.help(h);
    }
    err
}

fn classify_lexer_message(msg: &str) -> (&'static str, Option<&'static str>, Option<&'static str>) {
    match msg {
        m if m.starts_with("Invalid character") => (
            L001_INVALID_TOKEN,
            Some("this character is not valid in Nyra source"),
            None,
        ),
        "Expected identifier after '@'" => (
            L004_INVALID_ATTRIBUTE,
            Some("`@` must be followed by an identifier"),
            Some("example: `@derive(Clone)` or `@inline`"),
        ),
        "Expected '[' after '#'" => (
            L004_INVALID_ATTRIBUTE,
            Some("attributes use `#[name]` syntax"),
            Some("example: `#[derive(Clone)]` or `#[inline]`"),
        ),
        "Expected '(' after derive" => (
            L004_INVALID_ATTRIBUTE,
            Some("`#[derive(...)]` requires a parenthesized trait list"),
            Some("example: `#[derive(Clone, Debug)]`"),
        ),
        m if m.starts_with("Unknown attribute") => (
            L004_INVALID_ATTRIBUTE,
            Some("this attribute is not recognized"),
            None,
        ),
        "unclosed block comment" => (
            L002_UNCLOSED,
            Some("block comment opened with `/*` was never closed"),
            Some("add `*/` to close the comment"),
        ),
        "Character literal must end with a single closing quote" | "Invalid character literal" => (
            L002_UNCLOSED,
            Some("character literal is not properly closed"),
            Some("example: `'a'` or `'\\n'`"),
        ),
        "Integer literal overflow" => (
            L003_INVALID_NUMBER,
            Some("this integer literal exceeds the maximum value"),
            Some("use a smaller literal or a wider type suffix"),
        ),
        m if m.contains("Numeric separators") => (
            L003_INVALID_NUMBER,
            Some("`_` may only appear between digits"),
            Some("example: `1_000_000` or `0xFF_FF`"),
        ),
        "Expected hex digits after 0x" => (
            L003_INVALID_NUMBER,
            Some("hex literals need digits after `0x`"),
            Some("example: `0xFF` or `0x1A2B`"),
        ),
        _ => (L001_INVALID_TOKEN, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_invalid_character() {
        let err = coded_lexer_error(
            Span::default(),
            "Invalid character '@'",
        );
        assert_eq!(err.code.as_deref(), Some(L001_INVALID_TOKEN));
        assert!(err.label.is_some());
    }

    #[test]
    fn classifies_unclosed_comment() {
        let err = coded_lexer_error(Span::default(), "unclosed block comment");
        assert_eq!(err.code.as_deref(), Some(L002_UNCLOSED));
        assert!(!err.helps.is_empty());
    }

    #[test]
    fn classifies_derive_attribute() {
        let err = coded_lexer_error(Span::default(), "Expected '(' after derive");
        assert_eq!(err.code.as_deref(), Some(L004_INVALID_ATTRIBUTE));
    }
}
