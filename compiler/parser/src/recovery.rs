use errors::{coded_parser_error, NyraError, Span};
use lexer::{Token, TokenKind};

/// Cap parse diagnostics so malformed fuzz input cannot allocate unbounded `Vec<NyraError>`.
pub const MAX_PARSE_ERRORS: usize = 1024;

pub fn synchronize(tokens: &[Token], position: &mut usize) {
    if *position >= tokens.len() {
        return;
    }
    *position += 1;

    while *position < tokens.len() {
        match tokens[*position].kind {
            TokenKind::Newline => {
                *position += 1;
                return;
            }
            TokenKind::RBrace => return,
            TokenKind::Fn | TokenKind::Let | TokenKind::If | TokenKind::While => return,
            _ => *position += 1,
        }
    }
}

pub fn consume(
    tokens: &[Token],
    position: &mut usize,
    kind: TokenKind,
    msg: &str,
    errors: &mut Vec<NyraError>,
) -> bool {
    if *position < tokens.len() && tokens[*position].kind == kind {
        *position += 1;
        true
    } else {
        if errors.len() >= MAX_PARSE_ERRORS {
            if errors.len() == MAX_PARSE_ERRORS {
                errors.push(coded_parser_error(
                    tokens
                        .get(*position)
                        .map(|t| t.span.clone())
                        .unwrap_or_default(),
                    "too many parse errors; further errors suppressed",
                ));
            }
            synchronize(tokens, position);
            return false;
        }
        let span = tokens
            .get(*position)
            .map(|t| t.span.clone())
            .unwrap_or_default();
        errors.push(coded_parser_error(span, msg));
        synchronize(tokens, position);
        false
    }
}

pub fn check(tokens: &[Token], position: usize, kind: &TokenKind) -> bool {
    tokens.get(position).map(|t| &t.kind) == Some(kind)
}

pub fn is_doc_comment(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::DocComment(_))
}

pub fn is_at_end(tokens: &[Token], position: usize) -> bool {
    matches!(
        tokens.get(position).map(|t| &t.kind),
        Some(TokenKind::Eof) | None
    )
}

pub fn skip_newlines(tokens: &[Token], position: &mut usize) {
    while *position < tokens.len() && tokens[*position].kind == TokenKind::Newline {
        *position += 1;
    }
}

fn prev_significant_kind(tokens: &[Token], position: usize) -> Option<&TokenKind> {
    tokens[..position]
        .iter()
        .rev()
        .find(|t| !matches!(t.kind, TokenKind::Newline))
        .map(|t| &t.kind)
}

fn next_significant_kind(tokens: &[Token], position: usize) -> Option<&TokenKind> {
    let mut p = position;
    while p < tokens.len() && tokens[p].kind == TokenKind::Newline {
        p += 1;
    }
    tokens.get(p).map(|t| &t.kind)
}

fn is_chain_field_name(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier(_) | TokenKind::Clone | TokenKind::Number(_)
    )
}

/// Skip newlines that continue a dot/method chain across lines.
pub fn skip_chain_newlines(tokens: &[Token], position: &mut usize) {
    loop {
        if *position >= tokens.len() || tokens[*position].kind != TokenKind::Newline {
            break;
        }
        let prev = prev_significant_kind(tokens, *position);
        let next = next_significant_kind(tokens, *position);
        let leading_dot = matches!(next, Some(TokenKind::Dot | TokenKind::QuestionDot));
        let trailing_dot = matches!(prev, Some(TokenKind::Dot | TokenKind::QuestionDot));
        let member_after_dot = trailing_dot && next.is_some_and(is_chain_field_name);
        if leading_dot || member_after_dot {
            *position += 1;
            continue;
        }
        break;
    }
}

pub fn merge_spans(a: &Span, b: &Span) -> Span {
    let file = if !a.file.is_empty() {
        a.file.clone()
    } else {
        b.file.clone()
    };
    Span {
        file,
        start: a.start,
        end: b.end,
    }
}

/// True when `tokens[pos]` is `?` that begins a ternary (`?` then-branch `:`), not postfix try.
pub fn looks_like_ternary_question(tokens: &[Token], pos: usize) -> bool {
    if tokens.get(pos).map(|t| &t.kind) != Some(&TokenKind::Question) {
        return false;
    }
    let mut p = pos + 1;
    skip_newlines(tokens, &mut p);
    if is_at_end(tokens, p) {
        return false;
    }
    skip_ternary_then_branch(tokens, &mut p);
    check(tokens, p, &TokenKind::Colon)
}

fn skip_ternary_then_branch(tokens: &[Token], p: &mut usize) {
    let mut ternary_depth = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;

    while *p < tokens.len() {
        match &tokens[*p].kind {
            TokenKind::Eof => return,
            TokenKind::LParen => paren += 1,
            TokenKind::RParen if paren > 0 => paren -= 1,
            TokenKind::LBracket => bracket += 1,
            TokenKind::RBracket if bracket > 0 => bracket -= 1,
            TokenKind::LBrace => brace += 1,
            TokenKind::RBrace if brace > 0 => brace -= 1,
            TokenKind::Question if paren == 0 && bracket == 0 && brace == 0 => {
                if looks_like_ternary_question(tokens, *p) {
                    ternary_depth += 1;
                }
            }
            TokenKind::Colon if paren == 0 && bracket == 0 && brace == 0 => {
                if ternary_depth == 0 {
                    return;
                }
                ternary_depth -= 1;
            }
            _ => {}
        }
        *p += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::Lexer;

    fn parse_tokens(src: &str) -> Vec<Token> {
        Lexer::new(src, "test.ny").tokenize().0
    }

    #[test]
    fn fuzz_slow_unit_caps_parse_errors() {
        // libFuzzer slow-unit / OOM reproducer (fuzz_parser, 487 bytes).
        const FUZZ_SLOW_UNIT: &[u8] = &[
            102, 110, 32, 109, 97, 105, 102, 110, 32, 109, 101, 93, 111, 10, 117, 49, 54, 10, 64,
            0, 0, 0, 123, 56, 110, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 56, 10, 0, 0, 61,
            108, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 48, 48, 1, 105, 0, 61, 62, 123, 64,
            36, 56, 10, 0, 0, 61, 108, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 56, 48, 1, 1,
            102, 95, 46, 208, 42, 48, 1, 46, 56, 48, 251, 56, 46, 105, 0, 61, 62, 123, 64, 46,
            56, 10, 0, 0, 61, 110, 105, 91, 10, 42, 101, 93, 111, 10, 91, 105, 95, 10, 64, 0, 0,
            0, 123, 56, 110, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 60, 10, 0, 0, 61, 108,
            102, 40, 41, 96, 123, 96, 165, 105, 102, 110, 32, 109, 101, 93, 111, 10, 91, 105, 95,
            10, 64, 0, 0, 0, 123, 56, 110, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 56, 0, 61,
            108, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 56, 48, 1, 1, 102, 95, 46, 208, 42,
            48, 1, 46, 56, 48, 251, 56, 46, 105, 0, 61, 62, 123, 64, 46, 56, 10, 0, 0, 61, 110,
            105, 91, 10, 42, 101, 93, 111, 10, 91, 105, 95, 10, 64, 0, 0, 0, 123, 56, 110, 116,
            10, 91, 105, 0, 61, 62, 123, 64, 46, 60, 10, 0, 0, 61, 108, 102, 40, 41, 96, 123,
            96, 165, 105, 102, 110, 32, 109, 101, 93, 111, 10, 91, 105, 95, 10, 64, 0, 0, 0,
            123, 56, 110, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 56, 10, 0, 0, 61, 108, 116,
            10, 91, 105, 0, 61, 62, 123, 64, 46, 48, 48, 1, 105, 0, 61, 62, 123, 64, 124, 255,
            40, 165, 124, 124, 124, 165, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 48, 48, 1,
            105, 0, 61, 62, 123, 64, 46, 56, 10, 0, 56, 10, 0, 0, 61, 108, 116, 10, 91, 105, 0,
            61, 62, 123, 64, 46, 48, 48, 1, 105, 0, 61, 62, 123, 64, 124, 255, 40, 165, 124, 124,
            124, 165, 116, 10, 91, 105, 0, 61, 62, 123, 64, 46, 48, 48, 1, 105, 0, 61, 62, 123,
            64, 46, 56, 10, 0, 56, 116, 110, 116, 10, 91, 105, 0, 61, 62, 123, 64, 105, 102, 32,
            40, 40, 40, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105,
            105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 105, 40, 119, 104,
            105, 108, 101, 0, 61, 110, 116, 10, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
            43, 91, 105, 0, 61, 62, 10, 124, 91, 105,
        ];
        let s = String::from_utf8_lossy(FUZZ_SLOW_UNIT);
        let (tokens, _) = Lexer::new(&s, "fuzz.ny").tokenize();
        let (_program, errs) = crate::Parser::new(tokens).parse();
        assert!(
            errs.len() <= MAX_PARSE_ERRORS + 1,
            "too many errors: {}",
            errs.len()
        );
    }

    #[test]
    fn ternary_question_vs_postfix_try() {
        let ternary = parse_tokens("a == b ? c : d");
        let q = ternary.iter().position(|t| matches!(t.kind, TokenKind::Question)).unwrap();
        assert!(looks_like_ternary_question(&ternary, q));

        let try_only = parse_tokens("a?");
        let q = try_only.iter().position(|t| matches!(t.kind, TokenKind::Question)).unwrap();
        assert!(!looks_like_ternary_question(&try_only, q));

        let try_then_ternary = parse_tokens("a()? ? 1 : 0");
        let qs: Vec<_> = try_then_ternary
            .iter()
            .enumerate()
            .filter(|(_, t)| matches!(t.kind, TokenKind::Question))
            .map(|(i, _)| i)
            .collect();
        assert!(!looks_like_ternary_question(&try_then_ternary, qs[0]));
        assert!(looks_like_ternary_question(&try_then_ternary, qs[1]));
    }

    #[test]
    fn skip_chain_newlines_across_leading_dot() {
        let tokens = parse_tokens("foo\n.push()");
        let mut pos = tokens
            .iter()
            .position(|t| matches!(&t.kind, TokenKind::Identifier(s) if s == "foo"))
            .unwrap()
            + 1;
        skip_chain_newlines(&tokens, &mut pos);
        assert!(matches!(tokens.get(pos).map(|t| &t.kind), Some(TokenKind::Dot)));
    }

    #[test]
    fn skip_chain_newlines_across_trailing_dot() {
        let tokens = parse_tokens("foo.\npush()");
        let dot = tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Dot))
            .unwrap()
            + 1;
        let mut pos = dot;
        skip_chain_newlines(&tokens, &mut pos);
        assert!(matches!(
            tokens.get(pos).map(|t| &t.kind),
            Some(TokenKind::Identifier(s)) if s == "push"
        ));
    }
}
