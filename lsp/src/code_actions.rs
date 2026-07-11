//! Quick-fix and source-fix code actions (rust-analyzer-style assists).

use std::collections::HashMap;
use std::path::Path;

use errors::NyraError;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, Position, Range, TextEdit, Url,
    WorkspaceEdit,
};

/// Build code actions for a document, optionally filtered by selection range and requested kinds.
pub fn code_actions_for_document(
    uri: &Url,
    source: &str,
    path: &str,
    errors: &[NyraError],
    selection: Option<Range>,
    only: Option<&[CodeActionKind]>,
) -> Vec<CodeActionOrCommand> {
    let mut out = Vec::new();

    let wants_quickfix = only.map(|ks| ks.iter().any(is_quickfix_kind)).unwrap_or(true);
    let wants_source = only.map(|ks| ks.iter().any(is_source_kind)).unwrap_or(true);

    if wants_quickfix {
        use crate::diagnostics::diagnostic_from_error;
        let diags: Vec<Diagnostic> = errors
            .iter()
            .map(|e| diagnostic_from_error(e, tower_lsp::lsp_types::DiagnosticSeverity::ERROR))
            .collect();
        for diag in &diags {
            if let Some(sel) = selection {
                if !ranges_overlap(diag.range, sel) {
                    continue;
                }
            }
            let Some(related) = &diag.related_information else {
                continue;
            };
            for info in related {
                if let Some(action) = help_to_action(uri, diag.range, &info.message) {
                    out.push(CodeActionOrCommand::CodeAction(action));
                }
            }
            // Also try the main diagnostic message for "did you mean".
            if let Some(action) = did_you_mean_action(uri, diag.range, &diag.message) {
                out.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    if wants_source {
        out.extend(source_fix_actions(uri, source, path));
    }

    out
}

pub fn code_actions_from_errors(uri: &Url, errors: &[NyraError]) -> Vec<CodeActionOrCommand> {
    code_actions_for_document(uri, "", "", errors, None, None)
}

fn is_quickfix_kind(k: &CodeActionKind) -> bool {
    *k == CodeActionKind::QUICKFIX
        || k.as_str() == "quickfix"
        || k.as_str().starts_with("quickfix.")
}

fn is_source_kind(k: &CodeActionKind) -> bool {
    *k == CodeActionKind::SOURCE_FIX_ALL
        || k.as_str() == "source.fixAll"
        || k.as_str().starts_with("source.")
        || *k == CodeActionKind::SOURCE
}

fn ranges_overlap(a: Range, b: Range) -> bool {
    !(position_lt(a.end, b.start) || position_lt(b.end, a.start))
}

fn position_lt(a: Position, b: Position) -> bool {
    a.line < b.line || (a.line == b.line && a.character < b.character)
}

fn help_to_action(uri: &Url, range: Range, message: &str) -> Option<CodeAction> {
    let help = message.strip_prefix("help: ").unwrap_or(message);
    let (title, edit_text) = parse_help_fix(help)?;
    Some(make_edit_action(title, CodeActionKind::QUICKFIX, uri, range, edit_text))
}

fn did_you_mean_action(uri: &Url, range: Range, message: &str) -> Option<CodeAction> {
    // Patterns: did you mean `foo`? / Did you mean `bar`
    let lower = message.to_ascii_lowercase();
    let idx = lower.find("did you mean")?;
    let after = &message[idx..];
    let start = after.find('`')? + 1;
    let rest = &after[start..];
    let end = rest.find('`')?;
    let suggestion = &rest[..end];
    if suggestion.is_empty() {
        return None;
    }
    Some(make_edit_action(
        format!("Change to `{suggestion}`"),
        CodeActionKind::QUICKFIX,
        uri,
        range,
        suggestion.to_string(),
    ))
}

fn parse_help_fix(help: &str) -> Option<(String, String)> {
    if let Some(rest) = help.strip_prefix("borrow instead: ") {
        return Some(("Apply borrow suggestion".into(), rest.to_string()));
    }
    if let Some(rest) = help.strip_prefix("or duplicate: ") {
        return Some(("Apply clone suggestion".into(), rest.to_string()));
    }
    if let Some(rest) = help.strip_prefix("use `") {
        if let Some(code) = rest.strip_suffix('`') {
            return Some(("Apply suggested fix".into(), code.to_string()));
        }
        if let Some((code, _)) = rest.split_once('`') {
            return Some(("Apply suggested fix".into(), code.to_string()));
        }
    }
    if help.contains("clone") && help.contains(':') {
        if let Some((_, code)) = help.split_once(':') {
            let code = code.trim();
            if !code.is_empty() {
                return Some(("Apply clone suggestion".into(), code.to_string()));
            }
        }
    }
    None
}

fn make_edit_action(
    title: String,
    kind: CodeActionKind,
    uri: &Url,
    range: Range,
    new_text: String,
) -> CodeAction {
    CodeAction {
        title,
        kind: Some(kind),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit { range, new_text }],
            )])),
            ..Default::default()
        }),
        is_preferred: Some(true),
        ..Default::default()
    }
}

fn source_fix_actions(uri: &Url, source: &str, path: &str) -> Vec<CodeActionOrCommand> {
    if source.is_empty() {
        return vec![];
    }
    let path_buf = Path::new(path);
    let Ok((program, parse_errs)) = parse_program(source, path) else {
        return vec![];
    };
    if !parse_errs.is_empty() {
        return vec![];
    }
    let plan = lint::plan_prune(path_buf, &program);
    if plan.actions.is_empty() {
        return vec![];
    }

    let mut individual = Vec::new();
    let mut all_edits = Vec::new();

    for action in &plan.actions {
        let Some((title, edits)) = prune_action_to_edits(source, action) else {
            continue;
        };
        all_edits.extend(edits.clone());
        individual.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: title.clone(),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(uri.clone(), edits)])),
                ..Default::default()
            }),
            is_preferred: Some(false),
            ..Default::default()
        }));
    }

    let mut out = Vec::new();
    if !all_edits.is_empty() {
        // Merge overlapping edits by applying prune to get a single full-file replace
        // when multiple line removals conflict with simple TextEdits.
        let fixed = apply_prune_to_source(source, &plan);
        if fixed != source {
            let end = full_range(source);
            out.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Fix all unused imports/variables".into(),
                kind: Some(CodeActionKind::SOURCE_FIX_ALL),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(
                        uri.clone(),
                        vec![TextEdit {
                            range: end,
                            new_text: fixed,
                        }],
                    )])),
                    ..Default::default()
                }),
                is_preferred: Some(true),
                ..Default::default()
            }));
        }
    }
    out.extend(individual);
    out
}

fn parse_program(source: &str, path: &str) -> Result<(ast::Program, Vec<NyraError>), ()> {
    let (tokens, lex_errs) = lexer::Lexer::new(source, path).tokenize();
    if !lex_errs.is_empty() {
        return Err(());
    }
    let (program, parse_errs) = parser::Parser::new(tokens).parse();
    Ok((program, parse_errs))
}

fn apply_prune_to_source(source: &str, plan: &lint::PrunePlan) -> String {
    // Reuse lint's apply logic via a temporary single-file plan with dummy path.
    let mut remove_lines = std::collections::BTreeSet::new();
    let mut prefix_ops = Vec::new();
    for action in &plan.actions {
        match action {
            lint::PruneAction::RemoveImportLine { line, .. } => {
                remove_lines.insert(*line);
            }
            lint::PruneAction::PrefixUnusedVar { line, column, .. } => {
                prefix_ops.push((*line, *column));
            }
        }
    }
    let mut out = String::new();
    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;
        if remove_lines.contains(&line_num) {
            continue;
        }
        let mut line_str = line.to_string();
        for (_, pc) in prefix_ops.iter().filter(|(pl, _)| *pl == line_num) {
            if let Some(new) = lint_prefix_binding(&line_str, *pc) {
                line_str = new;
            }
        }
        out.push_str(&line_str);
        out.push('\n');
    }
    // Preserve lack of trailing newline if original had none.
    if !source.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn lint_prefix_binding(line: &str, col: usize) -> Option<String> {
    // Mirror lint::spans::prefix_binding_on_line without needing pub re-export.
    let idx = col.saturating_sub(1);
    if idx >= line.len() {
        return None;
    }
    let rest = &line[idx..];
    if !rest
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        return None;
    }
    if idx > 0 && line.as_bytes()[idx - 1] == b'_' {
        return None;
    }
    let mut out = String::new();
    out.push_str(&line[..idx]);
    out.push('_');
    out.push_str(rest);
    Some(out)
}

fn prune_action_to_edits(
    source: &str,
    action: &lint::PruneAction,
) -> Option<(String, Vec<TextEdit>)> {
    match action {
        lint::PruneAction::RemoveImportLine { line, .. } => {
            let range = line_range(source, *line)?;
            Some((
                format!("Remove unused import (line {line})"),
                vec![TextEdit {
                    range,
                    new_text: String::new(),
                }],
            ))
        }
        lint::PruneAction::PrefixUnusedVar {
            line, column, name, ..
        } => {
            let lines: Vec<&str> = source.lines().collect();
            let idx = line.saturating_sub(1);
            let text = *lines.get(idx)?;
            let new_line = lint_prefix_binding(text, *column)?;
            let range = line_content_range(source, *line)?;
            Some((
                format!("Prefix unused variable `{name}` with `_`"),
                vec![TextEdit {
                    range,
                    new_text: new_line,
                }],
            ))
        }
    }
}

fn line_range(source: &str, line_1: usize) -> Option<Range> {
    let start_line = (line_1.saturating_sub(1)) as u32;
    let lines: Vec<&str> = source.lines().collect();
    if line_1 == 0 || line_1 > lines.len() {
        return None;
    }
    Some(Range {
        start: Position {
            line: start_line,
            character: 0,
        },
        end: Position {
            line: start_line + 1,
            character: 0,
        },
    })
}

fn line_content_range(source: &str, line_1: usize) -> Option<Range> {
    let start_line = (line_1.saturating_sub(1)) as u32;
    let lines: Vec<&str> = source.lines().collect();
    let text = lines.get(line_1.saturating_sub(1))?;
    Some(Range {
        start: Position {
            line: start_line,
            character: 0,
        },
        end: Position {
            line: start_line,
            character: text.len() as u32,
        },
    })
}

fn full_range(source: &str) -> Range {
    let lines: Vec<&str> = source.lines().collect();
    let last = lines.len().saturating_sub(1) as u32;
    let last_len = lines.last().map(|l| l.len() as u32).unwrap_or(0);
    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: last,
            character: last_len,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_borrow_help() {
        let (title, text) = parse_help_fix("borrow instead: save(&user)").unwrap();
        assert!(title.contains("borrow"));
        assert_eq!(text, "save(&user)");
    }

    #[test]
    fn parses_did_you_mean() {
        let uri = Url::parse("file:///t.ny").unwrap();
        let range = Range::default();
        let action =
            did_you_mean_action(&uri, range, "unknown function `ad`; did you mean `add`?").unwrap();
        assert!(action.title.contains("add"));
    }
}
