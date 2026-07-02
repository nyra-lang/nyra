use std::fmt::Write as _;

use crate::color;
use crate::{display_path, NyraError, Span};

/// Append source context: line numbers, carets on their own line, label on `= label:` line.
pub fn append_source_context(out: &mut String, err: &NyraError, colored: bool) {
    if err.span.file.is_empty() {
        return;
    }
    let src = match crate::read_source(&err.span.file) {
        Some(s) => s,
        None => return,
    };

    let c = color::Colors::new();
    let start_line = err.span.start.line;
    let end_line = err.span.end.line.max(start_line);

    for line_no in start_line..=end_line {
        let Some(line) = src.lines().nth(line_no.saturating_sub(1)) else {
            continue;
        };
        if colored {
            let _ = writeln!(out, "{}", c.dim("   |"));
            let _ = writeln!(
                out,
                "{} {} {}",
                c.line_num(&format!("{line_no:>3}")),
                c.dim("|"),
                c.source(line)
            );
        } else {
            let _ = writeln!(out, "   |");
            let _ = writeln!(out, "{line_no:>3} | {line}");
        }

        if line_no == start_line {
            let col = err.span.start.column.saturating_sub(1);
            let carets = if err.span.end.line == start_line
                && err.span.end.column > err.span.start.column
            {
                "^".repeat(
                    (err.span.end.column - err.span.start.column)
                        .max(1)
                        .min(80),
                )
            } else {
                "^".to_string()
            };
            if colored {
                let _ = writeln!(out, "   | {}{}", " ".repeat(col), c.caret(&carets));
            } else {
                let _ = writeln!(out, "   | {}{}", " ".repeat(col), carets);
            }
            if let Some(label) = &err.label {
                if colored {
                    let _ = writeln!(
                        out,
                        "   {} {}",
                        c.note_label("= label:"),
                        label
                    );
                } else {
                    let _ = writeln!(out, "   = label: {label}");
                }
            }
        }
    }
}

/// Append rustc-style secondary labeled spans.
pub fn append_secondary_labels(out: &mut String, err: &NyraError, colored: bool) {
    for label in &err.labels {
        append_labeled_span(out, &label.span, &label.label, colored);
    }
}

fn append_labeled_span(out: &mut String, span: &Span, label: &str, colored: bool) {
    if span.file.is_empty() {
        return;
    }
    let src = match crate::read_source(&span.file) {
        Some(s) => s,
        None => return,
    };

    let c = color::Colors::new();
    let file = display_path(&span.file);
    let loc = format!("{}:{}:{}", file, span.start.line, span.start.column);
    if colored {
        let _ = writeln!(out, "  {} {}", c.dim("-->"), c.location(&loc));
    } else {
        let _ = writeln!(out, "  --> {loc}");
    }

    let line_no = span.start.line;
    let Some(line) = src.lines().nth(line_no.saturating_sub(1)) else {
        return;
    };
    if colored {
        let _ = writeln!(out, "{}", c.dim("   |"));
        let _ = writeln!(
            out,
            "{} {} {}",
            c.line_num(&format!("{line_no:>3}")),
            c.dim("|"),
            c.source(line)
        );
    } else {
        let _ = writeln!(out, "   |");
        let _ = writeln!(out, "{line_no:>3} | {line}");
    }

    let col = span.start.column.saturating_sub(1);
    let carets = if span.end.line == line_no && span.end.column > span.start.column {
        "^".repeat(
            (span.end.column - span.start.column)
                .max(1)
                .min(80),
        )
    } else {
        "^".to_string()
    };
    if colored {
        let _ = writeln!(
            out,
            "   | {}{} {}",
            " ".repeat(col),
            c.caret(&carets),
            label
        );
    } else {
        let _ = writeln!(out, "   | {}{} {label}", " ".repeat(col), carets);
    }
}

pub fn append_header(out: &mut String, err: &NyraError, colored: bool) {
    let file = display_path(if err.span.file.is_empty() {
        "<source>"
    } else {
        &err.span.file
    });
    let c = color::Colors::new();
    let severity_word = match err.severity {
        crate::Severity::Error => {
            if colored {
                c.error("error")
            } else {
                "error".to_string()
            }
        }
        crate::Severity::Warning => {
            if colored {
                c.warning("warning")
            } else {
                "warning".to_string()
            }
        }
    };
    if let Some(code) = &err.code {
        let _ = writeln!(
            out,
            "{}[{}]: {}",
            severity_word,
            if colored { c.code_tag(code) } else { code.clone() },
            if colored {
                c.message(&err.message)
            } else {
                err.message.clone()
            }
        );
    } else {
        let _ = writeln!(
            out,
            "{}: {}",
            severity_word,
            if colored {
                c.message(&err.message)
            } else {
                err.message.clone()
            }
        );
    }
    let loc = format!("{}:{}:{}", file, err.span.start.line, err.span.start.column);
    if colored {
        let _ = writeln!(out, "  {} {}", c.dim("-->"), c.location(&loc));
    } else {
        let _ = writeln!(out, "  --> {loc}");
    }
}

pub fn append_notes_and_helps(out: &mut String, err: &NyraError, colored: bool) {
    let c = color::Colors::new();
    for note in &err.notes {
        if colored {
            let _ = writeln!(out, "   {} {}", c.note_label("= note:"), note);
        } else {
            let _ = writeln!(out, "   = note: {note}");
        }
    }
    for help in &err.helps {
        if colored {
            let _ = writeln!(out, "   {} {}", c.help_label("= help:"), help);
        } else {
            let _ = writeln!(out, "   = help: {help}");
        }
    }
}
