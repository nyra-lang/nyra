//! CodeLens: Run Test above `test fn` (rust-analyzer style).

use ast::Program;
use tower_lsp::lsp_types::{Command, Position, Range};

#[derive(Debug, Clone)]
pub struct TestLens {
    pub range: Range,
    pub test_name: String,
    pub file: String,
}

pub fn collect_test_lenses(program: &Program, file: &str) -> Vec<TestLens> {
    let mut out = Vec::new();
    for f in &program.functions {
        if !(f.is_test || f.name.starts_with("test_")) {
            continue;
        }
        let line = f.span.start.line.saturating_sub(1) as u32;
        let character = f.span.start.column.saturating_sub(1) as u32;
        let end_character = f.span.end.column.saturating_sub(1) as u32;
        out.push(TestLens {
            range: Range {
                start: Position { line, character },
                end: Position {
                    line,
                    character: end_character.max(character + f.name.len() as u32),
                },
            },
            test_name: f.name.clone(),
            file: file.to_string(),
        });
    }
    out
}

pub fn test_lens_command(lens: &TestLens) -> Command {
    Command {
        title: "▶ Run Test".into(),
        command: "nyra.runTest".into(),
        arguments: Some(vec![
            serde_json::Value::String(lens.file.clone()),
            serde_json::Value::String(lens.test_name.clone()),
        ]),
    }
}
