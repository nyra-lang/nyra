use std::collections::HashSet;

use crate::{ErrorKind, NyraError, Span};
use crate::codes::*;

/// Build a coded parser diagnostic from a message (maps known messages to `P00x`).
pub fn coded_parser_error(span: Span, message: impl AsRef<str>) -> NyraError {
    let msg = message.as_ref();
    let (code, label, help) = classify_parser_message(msg);
    let mut err = NyraError::coded(code, ErrorKind::Parser, span, msg);
    if let Some(l) = label {
        err = err.label(l);
    }
    if let Some(h) = help {
        err = err.help(h);
    }
    err
}

/// Deprecated parallel keys — prefer `max = N`.
pub fn parallel_prefer_max(span: Span, key: &str) -> NyraError {
    NyraError::coded(
        P015_PARALLEL_PREFER_THREADS,
        ErrorKind::Parser,
        span,
        format!("prefer `max` over `{key}` in `parallel(...)`"),
    )
    .note("caps worker count; use `parallel:task` / `parallel:thread` to pick the backend")
}

/// Back-compat alias for [`parallel_prefer_max`].
pub fn parallel_prefer_max_threads(span: Span) -> NyraError {
    parallel_prefer_max(span, "cores")
}

fn classify_parser_message(msg: &str) -> (&'static str, Option<&'static str>, Option<&'static str>) {
    match msg {
        "Expected parameter name" => (
            P003_EXPECTED_PARAM_NAME,
            Some("parameters look like `(name: type, ...)`"),
            None,
        ),
        "Expected ')' after parameters" => (
            P004_EXPECTED_CLOSE_PAREN_PARAMS,
            Some("close the parameter list with `)`"),
            None,
        ),
        "Expected '(' after function name" | "Expected '(' after extern function name" => (
            P005_EXPECTED_OPEN_PAREN_FN,
            Some("function parameters start with `(`"),
            None,
        ),
        "Invalid expression" => (P006_INVALID_EXPRESSION, None, None),
        "unexpected `{` in expression" => (
            P007_UNEXPECTED_LBRACE_EXPR,
            Some("`{` after a type name starts a struct literal"),
            Some("use `MyStruct { field: value }`, not `{ field: value }`"),
        ),
        "Expected '}' to close block" | "Expected '}'" => (
            P008_EXPECTED_CLOSE_BRACE,
            Some("add `}` to close this block"),
            None,
        ),
        "Expected '{' to start block" | "Expected '{'" | "Expected '{' after struct name"
        | "Expected '{' after enum name" | "Expected '{' after impl type"
        | "Expected '{' after trait name" | "Expected '{' after match scrutinee" => (
            P009_EXPECTED_OPEN_BRACE,
            Some("a `{` opens a block or struct body"),
            None,
        ),
        "Expected 'fn' at top level" => (P010_EXPECTED_TOP_LEVEL_ITEM, None, None),
        "Expected ')' after arguments" | "Expected ')' after expression"
        | "Expected ')' after tuple literal" | "Expected ')' after array literal"
        | "Expected ')' after match bind" | "Expected ')' after enum variant args"
        | "Expected ')' after destructure pattern" | "Expected ')' after tuple destructure"
        | "Expected ')' after arrow parameter" => (
            P011_EXPECTED_CLOSE_PAREN_ARGS,
            Some("close the argument list with `)`"),
            None,
        ),
        "Expected '=>' after arrow function parameters" => (
            P012_EXPECTED_ARROW_FAT_ARROW,
            Some("arrow functions use `|x| => expr` or `(x) => expr`"),
            None,
        ),
        "Expected '=>' in match arm" => (
            P012_EXPECTED_ARROW_FAT_ARROW,
            Some("match arms use `pattern => body`"),
            None,
        ),
        "Expected ']' after array literal" | "Expected ']'"
        | "Expected ']' after [T]" | "Expected ']' after [T; N]" => (
            P014_EXPECTED_CLOSE_BRACKET,
            Some("close with `]`"),
            None,
        ),
        "Expected string path after import" => (
            P099_UNEXPECTED,
            Some("import paths are string literals"),
            Some(r#"example: `import "src/helper.ny"`"#),
        ),
        _ if msg.starts_with("Expected ')'") => (
            P013_EXPECTED_CLOSE_PAREN,
            Some("add a closing `)`"),
            None,
        ),
        _ => (P099_UNEXPECTED, None, None),
    }
}

/// True when a parser error is likely fallout from an earlier syntax error.
pub fn is_parser_cascade(err: &NyraError) -> bool {
    if err.kind != ErrorKind::Parser {
        return false;
    }
    matches!(
        err.code.as_deref(),
        Some(P006_INVALID_EXPRESSION)
            | Some(P007_UNEXPECTED_LBRACE_EXPR)
            | Some(P010_EXPECTED_TOP_LEVEL_ITEM)
            | Some(P012_EXPECTED_ARROW_FAT_ARROW)
    )
}

fn dedupe_same_location(errors: Vec<NyraError>) -> (Vec<NyraError>, usize) {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut hidden = 0usize;
    for e in errors {
        let key = (e.span.file.clone(), e.span.start.line, e.span.start.column);
        if seen.insert(key) {
            out.push(e);
        } else {
            hidden += 1;
        }
    }
    (out, hidden)
}

/// Limit parser cascade noise; returns (shown, suppressed_count).
pub fn finalize_parser_diagnostics(errors: Vec<NyraError>) -> (Vec<NyraError>, usize) {
    const MAX_PRIMARY: usize = 12;
    const MAX_CASCADE: usize = 2;

    if errors.is_empty() {
        return (errors, 0);
    }

    let mut sorted = errors;
    sorted.retain(|e| !e.span.file.is_empty());
    sorted.sort_by(|a, b| {
        a.span
            .start
            .line
            .cmp(&b.span.start.line)
            .then(a.span.start.column.cmp(&b.span.start.column))
    });

    if sorted.is_empty() {
        return (vec![], 0);
    }

    let (primary, location_hidden) = dedupe_same_location(sorted);
    let (primary, cascade): (Vec<_>, Vec<_>) =
        primary.into_iter().partition(|e| !is_parser_cascade(e));

    let first_line = primary
        .first()
        .map(|e| e.span.start.line)
        .unwrap_or(1);

    let primary_hidden = primary.len().saturating_sub(MAX_PRIMARY);
    let cascade_take = cascade.len().min(MAX_CASCADE);
    let cascade_hidden = cascade.len().saturating_sub(cascade_take);

    let mut out: Vec<NyraError> = primary.into_iter().take(MAX_PRIMARY).collect();
    for mut e in cascade.into_iter().take(cascade_take) {
        if !e
            .notes
            .iter()
            .any(|n| n.contains("caused by an earlier error"))
        {
            e = e.note(format!(
                "this may be caused by an earlier error on line {first_line}"
            ));
        }
        out.push(e);
    }

    out.sort_by(|a, b| {
        a.span
            .start
            .line
            .cmp(&b.span.start.line)
            .then(a.span.start.column.cmp(&b.span.start.column))
    });

    (
        out,
        location_hidden + primary_hidden + cascade_hidden,
    )
}

/// Collapse duplicate lexer errors on the same line (e.g. `@@@`).
pub fn finalize_lexer_diagnostics(errors: Vec<NyraError>) -> (Vec<NyraError>, usize) {
    const MAX_LEXER: usize = 8;

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut hidden = 0usize;
    for e in errors {
        let key = (e.span.start.line, e.message.clone());
        if seen.insert(key) {
            out.push(e);
        } else {
            hidden += 1;
        }
    }

    if out.len() > MAX_LEXER {
        hidden += out.len() - MAX_LEXER;
        out.truncate(MAX_LEXER);
    }
    (out, hidden)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;

    #[test]
    fn suppresses_parser_cascade_after_primary() {
        let file = "t.ny";
        let primary = coded_parser_error(
            Span::new(file, Position { line: 14, column: 26 }, Position { line: 14, column: 27 }),
            "Expected parameter name",
        );
        let cascade = coded_parser_error(
            Span::new(file, Position { line: 43, column: 18 }, Position { line: 43, column: 19 }),
            "unexpected `{` in expression",
        );
        let mut errors = vec![primary];
        for line in 50..60 {
            errors.push(coded_parser_error(
                Span::new(file, Position { line, column: 1 }, Position { line, column: 2 }),
                "Invalid expression",
            ));
        }
        errors.push(cascade);
        let (shown, hidden) = finalize_parser_diagnostics(errors);
        assert!(hidden >= 1, "hidden={hidden} shown={}", shown.len());
        assert!(shown.len() < 15);
        assert!(shown.iter().any(|e| e.code.as_deref() == Some(P003_EXPECTED_PARAM_NAME)));
    }

    #[test]
    fn dedupes_same_location() {
        let file = "t.ny";
        let span = Span::new(file, Position { line: 14, column: 26 }, Position { line: 14, column: 27 });
        let e1 = coded_parser_error(span.clone(), "Expected parameter name");
        let e2 = coded_parser_error(span, "Expected ')' after parameters");
        let (shown, hidden) = finalize_parser_diagnostics(vec![e1, e2]);
        assert_eq!(shown.len(), 1);
        assert_eq!(hidden, 1);
    }

    #[test]
    fn classifies_arrow_fat_arrow() {
        let err = coded_parser_error(Span::default(), "Expected '=>' after arrow function parameters");
        assert_eq!(err.code.as_deref(), Some(P012_EXPECTED_ARROW_FAT_ARROW));
    }
}
