//! Convert Nyra diagnostics to LSP `Diagnostic` values.

use errors::NyraError;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Position,
    Range, Url,
};

pub fn diagnostic_from_error(
    e: &NyraError,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    let primary_range = span_to_lsp_range(&e.span);
    let primary_uri = file_uri(&e.span.file);

    let mut related = Vec::new();
    for label in &e.labels {
        push_related(
            &mut related,
            file_uri(&label.span.file),
            span_to_lsp_range(&label.span),
            label.label.clone(),
        );
    }
    if let Some(label) = &e.label {
        push_related(&mut related, primary_uri.clone(), primary_range, format!("= {label}"));
    }
    for note in &e.notes {
        push_related(&mut related, primary_uri.clone(), primary_range, format!("note: {note}"));
    }
    for help in &e.helps {
        push_related(&mut related, primary_uri.clone(), primary_range, format!("help: {help}"));
    }

    Diagnostic {
        range: primary_range,
        severity: Some(severity),
        message: e.message.clone(),
        code: e.code.clone().map(NumberOrString::String),
        source: Some("nyra".into()),
        related_information: if related.is_empty() {
            None
        } else {
            Some(related)
        },
        ..Default::default()
    }
}

fn push_related(
    related: &mut Vec<DiagnosticRelatedInformation>,
    uri: Url,
    range: Range,
    message: String,
) {
    related.push(DiagnosticRelatedInformation {
        location: Location { uri, range },
        message,
    });
}

fn file_uri(path: &str) -> Url {
    Url::from_file_path(path).unwrap_or_else(|_| {
        Url::parse(&format!("file:///{path}"))
            .unwrap_or_else(|_| Url::parse("file:///unknown.ny").expect("valid url"))
    })
}

fn span_to_lsp_range(span: &errors::Span) -> Range {
    Range {
        start: Position {
            line: (span.start.line.saturating_sub(1)) as u32,
            character: (span.start.column.saturating_sub(1)) as u32,
        },
        end: Position {
            line: (span.end.line.saturating_sub(1)) as u32,
            character: (span.end.column.saturating_sub(1)) as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use errors::{ErrorKind, NyraError, Position, Span};

    #[test]
    fn related_uses_real_file_uri() {
        let err = NyraError::coded(
            "E003",
            ErrorKind::Type,
            Span::new("/tmp/main.ny", Position { line: 2, column: 5 }, Position { line: 2, column: 8 }),
            "type mismatch",
        )
        .label("expected i32")
        .note("function expects integer")
        .help("pass a number literal");
        let diag = diagnostic_from_error(&err, DiagnosticSeverity::ERROR);
        let related = diag.related_information.expect("related");
        assert_eq!(related.len(), 3);
        assert!(related[0].location.uri.path().ends_with("main.ny"));
        assert!(related[0].message.contains("expected i32"));
        assert!(related[1].message.starts_with("note:"));
        assert!(related[2].message.starts_with("help:"));
    }
}
