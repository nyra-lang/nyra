use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ast::Program;
use errors::{NyraError, W002_UNUSED_IMPORT, W003_UNUSED_VARIABLE};

use crate::spans::prefix_binding_on_line;
use crate::{check_unused_imports, check_unused_variables};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PruneAction {
    RemoveImportLine {
        file: PathBuf,
        line: usize,
    },
    PrefixUnusedVar {
        file: PathBuf,
        line: usize,
        column: usize,
        name: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrunePlan {
    pub actions: Vec<PruneAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PruneResult {
    pub files_changed: usize,
    pub imports_removed: usize,
    pub vars_prefixed: usize,
}

/// Collect automatic fixes for unused imports and unused locals.
pub fn plan_prune(entry: &Path, program: &Program) -> PrunePlan {
    let mut actions = Vec::new();
    for w in check_unused_imports(entry, Some(program)) {
        if let Some(action) = import_warning_to_action(&w) {
            actions.push(action);
        }
    }
    for w in check_unused_variables(program) {
        if let Some(action) = var_warning_to_action(&w) {
            actions.push(action);
        }
    }
    dedupe_actions(actions)
}

pub fn apply_prune(plan: &PrunePlan, dry_run: bool) -> Result<PruneResult, String> {
    let mut by_file: BTreeMap<PathBuf, Vec<PruneAction>> = BTreeMap::new();
    for action in &plan.actions {
        let file = match action {
            PruneAction::RemoveImportLine { file, .. }
            | PruneAction::PrefixUnusedVar { file, .. } => file.clone(),
        };
        by_file.entry(file).or_default().push(action.clone());
    }

    let mut result = PruneResult::default();
    for (file, actions) in by_file {
        let original = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let updated = apply_file_edits(&original, &actions);
        if updated == original {
            continue;
        }
        for action in &actions {
            match action {
                PruneAction::RemoveImportLine { .. } => result.imports_removed += 1,
                PruneAction::PrefixUnusedVar { .. } => result.vars_prefixed += 1,
            }
        }
        result.files_changed += 1;
        if !dry_run {
            std::fs::write(&file, updated).map_err(|e| e.to_string())?;
        }
    }
    Ok(result)
}

fn apply_file_edits(source: &str, actions: &[PruneAction]) -> String {
    let mut remove_lines = BTreeSet::new();
    let mut prefix_ops = Vec::new();
    for action in actions {
        match action {
            PruneAction::RemoveImportLine { line, .. } => {
                remove_lines.insert(*line);
            }
            PruneAction::PrefixUnusedVar { line, column, .. } => {
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
            if let Some(new) = prefix_binding_on_line(&line_str, *pc) {
                line_str = new;
            }
        }
        out.push_str(&line_str);
        out.push('\n');
    }
    out
}

fn import_warning_to_action(w: &NyraError) -> Option<PruneAction> {
    if w.code.as_deref() != Some(W002_UNUSED_IMPORT) {
        return None;
    }
    // Selective `import { name }` unused binding — do not delete the whole line.
    if w
        .labels
        .iter()
        .any(|l| l.label.contains("is imported but never used"))
    {
        return None;
    }
    Some(PruneAction::RemoveImportLine {
        file: PathBuf::from(&w.span.file),
        line: w.span.start.line,
    })
}

fn var_warning_to_action(w: &NyraError) -> Option<PruneAction> {
    if w.code.as_deref() != Some(W003_UNUSED_VARIABLE) {
        return None;
    }
    let name = w
        .message
        .strip_prefix("unused variable `")?
        .strip_suffix('`')?
        .to_string();
    if name.starts_with('_') {
        return None;
    }
    Some(PruneAction::PrefixUnusedVar {
        file: PathBuf::from(&w.span.file),
        line: w.span.start.line,
        column: w.span.start.column,
        name,
    })
}

fn dedupe_actions(actions: Vec<PruneAction>) -> PrunePlan {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for action in actions {
        let key = match &action {
            PruneAction::RemoveImportLine { file, line } => {
                format!("import:{}:{}", file.display(), line)
            }
            PruneAction::PrefixUnusedVar {
                file, line, name, ..
            } => format!("var:{}:{}:{}", file.display(), line, name),
        };
        if seen.insert(key) {
            out.push(action);
        }
    }
    PrunePlan { actions: out }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resolve::load_program;

    fn write_temp_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::write(
            root.join("main.ny"),
            r#"import "src/unused.ny"

fn main() {
    let dead = 99
    print("ok")
}
"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/unused.ny"),
            "fn never_called() {\n    print(\"x\")\n}\n",
        )
        .unwrap();
        (dir, root)
    }

    #[test]
    fn plan_finds_unused_import_and_var() {
        let (_guard, root) = write_temp_project();
        let main = root.join("main.ny");
        let loaded = load_program(&main).expect("load");
        let plan = plan_prune(&main, &loaded.program);
        assert!(
            plan.actions.iter().any(|a| matches!(
                a,
                PruneAction::RemoveImportLine { line: 1, .. }
            )),
            "expected unused import on line 1, got {:?}",
            plan.actions
        );
        assert!(
            plan.actions.iter().any(|a| matches!(
                a,
                PruneAction::PrefixUnusedVar { name, .. } if name == "dead"
            )),
            "expected unused var dead, got {:?}",
            plan.actions
        );
    }

    #[test]
    fn apply_removes_import_and_prefixes_var() {
        let (_guard, root) = write_temp_project();
        let main = root.join("main.ny");
        let loaded = load_program(&main).expect("load");
        let plan = plan_prune(&main, &loaded.program);
        let result = apply_prune(&plan, false).expect("apply");
        assert!(result.imports_removed >= 1);
        assert!(result.vars_prefixed >= 1);
        let text = std::fs::read_to_string(&main).unwrap();
        assert!(!text.contains("import \"src/unused.ny\""));
        assert!(text.contains("let _dead = 99"));
    }

    #[test]
    fn dry_run_does_not_write() {
        let (_guard, root) = write_temp_project();
        let main = root.join("main.ny");
        let before = std::fs::read_to_string(&main).unwrap();
        let loaded = load_program(&main).expect("load");
        let plan = plan_prune(&main, &loaded.program);
        apply_prune(&plan, true).expect("dry run");
        let after = std::fs::read_to_string(&main).unwrap();
        assert_eq!(before, after);
    }
}
