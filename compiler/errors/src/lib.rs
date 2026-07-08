mod color;
mod codes;
mod const_eval_diag;
mod display;
mod explain;
mod json;
mod lexer_diag;
mod paths;
mod parser_diag;
mod render;
mod sources;
mod suggest;

use std::fmt;

pub use codes::*;
pub use color::{set_color_choice, ColorChoice};
pub use display::print_diagnostics;
pub use explain::{explain, format_explain, list_codes, ExplainEntry};
pub use json::{diagnostics_to_json, DiagnosticJson};
pub use const_eval_diag::coded_comptime_error;
pub use lexer_diag::coded_lexer_error;
pub use parser_diag::{
    coded_parser_error, finalize_lexer_diagnostics, finalize_parser_diagnostics,
    is_parser_cascade, parallel_prefer_max, parallel_prefer_max_threads,
};
pub use paths::{clear_diagnostic_root, display_path, set_diagnostic_root};
pub use sources::{clear_sources, read_source, register_source};
pub use suggest::did_you_mean;

/// Returned when diagnostics were already printed to stderr.
pub const COMPILE_FAILED: &str = "compilation failed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub file: String,
    pub start: Position,
    pub end: Position,
}
impl Span {
    pub fn new(file: impl Into<String>, start: Position, end: Position) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A labeled source range attached to a diagnostic (secondary location).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Lexer,
    Parser,
    Type,
    NameResolution,
    BorrowCheck,
    Runtime,
    Internal,
    Lint,
    /// Compile-time evaluation (`comptime` modules and constants).
    ConstEval,
}

#[derive(Debug, Clone)]
pub struct NyraError {
    pub code: Option<String>,
    pub severity: Severity,
    pub kind: ErrorKind,
    pub span: Span,
    pub message: String,
    /// Short hint shown on the underline line (after `^`).
    pub label: Option<String>,
    pub notes: Vec<String>,
    /// Actionable fix suggestions (`help:` lines).
    pub helps: Vec<String>,
    /// Additional labeled spans (e.g. move origin, borrow site).
    pub labels: Vec<DiagnosticLabel>,
}

impl NyraError {
    pub fn new(kind: ErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            code: None,
            severity: Severity::Error,
            kind,
            span,
            message: message.into(),
            label: None,
            notes: vec![],
            helps: vec![],
            labels: vec![],
        }
    }

    pub fn warning(kind: ErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            code: None,
            severity: Severity::Warning,
            kind,
            span,
            message: message.into(),
            label: None,
            notes: vec![],
            helps: vec![],
            labels: vec![],
        }
    }

    pub fn coded(code: &'static str, kind: ErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.to_string()),
            severity: Severity::Error,
            kind,
            span,
            message: message.into(),
            label: None,
            notes: vec![],
            helps: vec![],
            labels: vec![],
        }
    }

    pub fn coded_warning(
        code: &'static str,
        kind: ErrorKind,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: Some(code.to_string()),
            severity: Severity::Warning,
            kind,
            span,
            message: message.into(),
            label: None,
            notes: vec![],
            helps: vec![],
            labels: vec![],
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Attach a secondary labeled span (e.g. where a value was moved).
    pub fn label_span(mut self, span: Span, label: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            label: label.into(),
        });
        self
    }

    pub fn format_colored(&self) -> String {
        let mut out = String::new();
        render::append_header(&mut out, self, true);
        render::append_source_context(&mut out, self, true);
        render::append_secondary_labels(&mut out, self, true);
        render::append_notes_and_helps(&mut out, self, true);
        if out.ends_with('\n') {
            out.pop();
        }
        out
    }
}

impl fmt::Display for NyraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if color::colors_enabled() {
            write!(f, "{}", self.format_colored())
        } else {
            write!(f, "{}", self.format_plain())
        }
    }
}

impl NyraError {
    fn format_plain(&self) -> String {
        let mut out = String::new();
        render::append_header(&mut out, self, false);
        render::append_source_context(&mut out, self, false);
        render::append_secondary_labels(&mut out, self, false);
        render::append_notes_and_helps(&mut out, self, false);
        if out.ends_with('\n') {
            out.pop();
        }
        out
    }
}

pub struct ErrorReporter {
    pub errors: Vec<NyraError>,
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Print diagnostics to stderr (grouped by file + summary).
pub fn eprint_diagnostics(errors: &[NyraError]) {
    print_diagnostics(errors, 0);
}

/// Print diagnostics including a suppressed-error count from cascade filtering.
pub fn eprint_diagnostics_suppressed(errors: &[NyraError], suppressed: usize) {
    print_diagnostics(errors, suppressed);
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    pub fn report(&mut self, error: NyraError) {
        self.errors.push(error);
    }

    pub fn report_all(&mut self, items: impl IntoIterator<Item = NyraError>) {
        self.errors.extend(items);
    }

    pub fn has_errors(&self) -> bool {
        self.errors.iter().any(|e| e.is_error())
    }

    pub fn has_warnings(&self) -> bool {
        self.errors.iter().any(|e| !e.is_error())
    }

    pub fn warning_count(&self) -> usize {
        self.errors.iter().filter(|e| !e.is_error()).count()
    }

    pub fn error_count(&self) -> usize {
        self.errors.iter().filter(|e| e.is_error()).count()
    }

    pub fn print_all(&self) {
        print_diagnostics(&self.errors, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static PLAIN_DIAGNOSTICS: Once = Once::new();

    fn ensure_plain_diagnostics() {
        PLAIN_DIAGNOSTICS.call_once(|| set_color_choice(ColorChoice::Never));
    }

    #[test]
    fn display_error_includes_location() {
        ensure_plain_diagnostics();
        let err = NyraError::new(
            ErrorKind::Type,
            Span::new("test.ny", Position { line: 2, column: 5 }, Position { line: 2, column: 6 }),
            "undefined variable `x`",
        );
        let msg = format!("{err}");
        assert!(msg.contains("error:"));
        assert!(msg.contains("undefined variable"));
        assert!(msg.contains("test.ny:2:5"));
    }

    #[test]
    fn display_warning_with_code() {
        ensure_plain_diagnostics();
        let err = NyraError::coded_warning(
            "W001",
            ErrorKind::Type,
            Span::default(),
            "Extended tier feature",
        );
        let msg = format!("{err}");
        assert!(msg.contains("warning[W001]"));
    }

    #[test]
    fn display_error_with_note() {
        ensure_plain_diagnostics();
        let err = NyraError::new(
            ErrorKind::BorrowCheck,
            Span::default(),
            "use after move",
        )
        .note("value was moved here");
        let msg = format!("{err}");
        assert!(msg.contains("= note: value was moved here"));
    }

    #[test]
    fn display_error_with_help_and_label() {
        ensure_plain_diagnostics();
        let err = NyraError::coded(
            "P001",
            ErrorKind::Parser,
            Span::new("test.ny", Position { line: 2, column: 15 }, Position { line: 4, column: 5 }),
            "anonymous object literals are not supported",
        )
        .label("expected a struct type name before `{`")
        .note("JavaScript `{ key: value }` syntax is not valid in Nyra")
        .help("declare `struct Person { name: string }` then use `Person { name: \"Hamdy\" }`");
        let msg = format!("{err}");
        assert!(msg.contains("error[P001]:"));
        assert!(msg.contains("anonymous object literals"));
        assert!(msg.contains("= help:"));
        assert!(msg.contains("= note:"));
    }

    #[test]
    fn reporter_tracks_errors_and_warnings() {
        let mut rep = ErrorReporter::new();
        rep.report(NyraError::new(ErrorKind::Lexer, Span::default(), "bad token"));
        rep.report(NyraError::warning(ErrorKind::Type, Span::default(), "unused"));
        assert!(rep.has_errors());
        assert!(rep.has_warnings());
    }
}

pub type NyraResult<T> = Result<T, NyraError>;
