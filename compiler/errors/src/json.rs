//! JSON serialization for `nyra diag --json` and editor integrations.

use serde::Serialize;

use crate::{DiagnosticLabel, NyraError, Severity};

/// Structured diagnostic for machine output (`nyra diag --json`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagnosticJson {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub helps: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<DiagnosticLabelJson>,
    pub kind: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagnosticLabelJson {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub label: String,
}

impl From<&DiagnosticLabel> for DiagnosticLabelJson {
    fn from(l: &DiagnosticLabel) -> Self {
        DiagnosticLabelJson {
            file: l.span.file.clone(),
            line: l.span.start.line,
            column: l.span.start.column,
            end_line: l.span.end.line,
            end_column: l.span.end.column,
            label: l.label.clone(),
        }
    }
}

impl From<&NyraError> for DiagnosticJson {
    fn from(e: &NyraError) -> Self {
        DiagnosticJson {
            file: e.span.file.clone(),
            line: e.span.start.line,
            column: e.span.start.column,
            end_line: e.span.end.line,
            end_column: e.span.end.column,
            message: e.message.clone(),
            code: e.code.clone(),
            label: e.label.clone(),
            notes: e.notes.clone(),
            helps: e.helps.clone(),
            labels: e.labels.iter().map(DiagnosticLabelJson::from).collect(),
            kind: format!("{:?}", e.kind),
            severity: match e.severity {
                Severity::Error => "Error".into(),
                Severity::Warning => "Warning".into(),
            },
        }
    }
}

/// Serialize diagnostics as pretty-printed JSON.
pub fn diagnostics_to_json(errors: &[NyraError]) -> Result<String, serde_json::Error> {
    let items: Vec<DiagnosticJson> = errors.iter().map(DiagnosticJson::from).collect();
    serde_json::to_string_pretty(&items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorKind, Position, Span};

    #[test]
    fn json_includes_code_and_helps() {
        let err = NyraError::coded(
            "E003",
            ErrorKind::Type,
            Span::new("main.ny", Position { line: 2, column: 5 }, Position { line: 2, column: 10 }),
            "type mismatch",
        )
        .label("expected i32")
        .help("pass an integer literal");
        let json: DiagnosticJson = (&err).into();
        assert_eq!(json.code.as_deref(), Some("E003"));
        assert_eq!(json.label.as_deref(), Some("expected i32"));
        assert_eq!(json.helps, vec!["pass an integer literal"]);
        assert_eq!(json.end_line, 2);
        assert_eq!(json.end_column, 10);
    }
}
