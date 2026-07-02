//! Table-driven tests over `tests/corpus/manifest.toml`.

mod common;

use std::process::Command;
use std::path::{PathBuf};

use common::{assert_ir_patterns, normalize_process_stdout_trimmed, nyra_bin, workspace_root};
use compiler::{CompileOptions, Compiler};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Manifest {
    case: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
struct CorpusCase {
    id: String,
    path: String,
    mode: String,
    tier: String,
    #[serde(default = "default_true")]
    expect_compile: bool,
    #[serde(default)]
    run: bool,
    #[serde(default)]
    expect_stdout: Option<String>,
    #[serde(default)]
    nyra_home_empty: bool,
    #[serde(default)]
    ir_contains: Vec<String>,
    #[serde(default)]
    ir_must_not: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn load_manifest() -> Manifest {
    let path = workspace_root().join("tests/corpus/manifest.toml");
    let text = std::fs::read_to_string(&path).expect("manifest.toml");
    toml::from_str(&text).expect("parse manifest")
}

fn compile_case(case: &CorpusCase) -> Result<compiler::CompileOutput, String> {
    match case.mode.as_str() {
        "file" => {
            let path = workspace_root().join(&case.path);
            Compiler::compile_file(&path, &CompileOptions::default())
        }
        "project" => {
            let path = workspace_root().join(&case.path);
            Compiler::compile_project(&path, &CompileOptions::default())
        }
        other => panic!("unknown mode {other} for {}", case.id),
    }
}

#[test]
fn corpus_all_examples_compile() {
    let manifest = load_manifest();
    assert!(
        manifest.case.len() >= 40,
        "expected large corpus, got {}",
        manifest.case.len()
    );

    for case in &manifest.case {
        let result = compile_case(case);
        if !case.expect_compile {
            assert!(
                result.is_err() || result.as_ref().map(|o| o.llvm_ir.is_none()).unwrap_or(true),
                "{}: expected compile failure but succeeded",
                case.id
            );
            continue;
        }
        let out = result.unwrap_or_else(|e| panic!("{}: compile error: {e}", case.id));
        assert!(
            out.lexer_errors.is_empty(),
            "{}: lexer errors: {:?}",
            case.id,
            out.lexer_errors
        );
        assert!(
            out.parser_errors.is_empty(),
            "{}: parser errors: {:?}",
            case.id,
            out.parser_errors
        );
        assert!(
            out.type_errors.is_empty(),
            "{}: type errors: {:?}",
            case.id,
            out.type_errors
        );
        assert!(
            out.borrow_errors.is_empty(),
            "{}: borrow errors: {:?}",
            case.id,
            out.borrow_errors
        );
        assert!(
            out.llvm_ir.is_some(),
            "{}: missing LLVM IR",
            case.id
        );

        if !case.ir_contains.is_empty() || !case.ir_must_not.is_empty() {
            let ir = out.llvm_ir.as_ref().unwrap();
            let must: Vec<&str> = case.ir_contains.iter().map(String::as_str).collect();
            let must_not: Vec<&str> = case.ir_must_not.iter().map(String::as_str).collect();
            assert_ir_patterns(ir, &must, &must_not);
        }
    }
}

#[test]
fn corpus_e2e_stdout() {
    let manifest = load_manifest();
    for case in manifest.case.iter().filter(|c| c.run) {
        let path = workspace_root().join(&case.path);
        let mut cmd = Command::new(nyra_bin());
        cmd.arg("run").arg(&path);
        cmd.env_remove("NYRA_HOME");
        if case.nyra_home_empty {
            cmd.env("NYRA_HOME", "");
        }
        let output = cmd.output().expect("run nyra");
        assert!(
            output.status.success(),
            "{}: stderr={}",
            case.id,
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(expected) = &case.expect_stdout {
            let stdout = normalize_process_stdout_trimmed(&output.stdout);
            assert_eq!(
                stdout, *expected,
                "{}: stdout mismatch",
                case.id
            );
        }
    }
}

#[test]
fn corpus_tier_labels_valid() {
    for case in &load_manifest().case {
        assert!(
            case.tier == "core" || case.tier == "extended",
            "{}: invalid tier {}",
            case.id,
            case.tier
        );
    }
}

#[test]
fn corpus_paths_exist() {
    for case in &load_manifest().case {
        let path = workspace_root().join(&case.path);
        assert!(path.exists(), "{}: path missing: {}", case.id, path.display());
    }
}

fn walk_examples_dir(dir: &std::path::PathBuf, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect(&format!("read_dir {}", dir.display()));
    for ent in entries {
        let ent = ent.expect("read_dir entry");
        let path = ent.path();
        if path.is_dir() {
            walk_examples_dir(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
#[ignore = "dev helper: cargo test -p compiler --test examples_corpus scan_entrypoint_failures -- --ignored --nocapture"]
fn scan_entrypoint_failures() {
    let manifest = load_manifest();
    let mut skip_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut skip_project_prefixes: Vec<PathBuf> = Vec::new();
    for case in &manifest.case {
        let abs = workspace_root().join(&case.path);
        if case.mode == "file" {
            skip_files.insert(abs);
        } else if case.mode == "project" {
            skip_project_prefixes.push(abs);
        }
    }
    let examples_root = workspace_root().join("examples");
    let mut all_paths = Vec::new();
    walk_examples_dir(&examples_root, &mut all_paths);
    for path in all_paths {
        if path.extension().and_then(|e| e.to_str()) != Some("ny") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        if !src.contains("fn main") {
            continue;
        }
        if skip_files.contains(&path) {
            continue;
        }
        if skip_project_prefixes.iter().any(|p| path.starts_with(p)) {
            continue;
        }
        let rel = path.strip_prefix(workspace_root()).unwrap();
        let out = match Compiler::compile_file(&path, &CompileOptions::default()) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("ERR {rel:?}: {e}");
                continue;
            }
        };
        if !out.type_errors.is_empty()
            || !out.parser_errors.is_empty()
            || !out.lexer_errors.is_empty()
            || !out.borrow_errors.is_empty()
            || out.llvm_ir.is_none()
        {
            eprintln!(
                "FAIL {rel:?}: lex={} par={} type={} borrow={} ir={}",
                out.lexer_errors.len(),
                out.parser_errors.len(),
                out.type_errors.len(),
                out.borrow_errors.len(),
                out.llvm_ir.is_some()
            );
            for e in &out.type_errors {
                eprintln!("  type: {}", e.message);
            }
        }
    }
}

#[test]
fn corpus_compile_all_example_entrypoints() {
    let manifest = load_manifest();

    // Skip anything already represented in the manifest (including entire project dirs).
    let mut skip_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut skip_project_prefixes: Vec<PathBuf> = Vec::new();
    for case in &manifest.case {
        let abs = workspace_root().join(&case.path);
        if case.mode == "file" {
            skip_files.insert(abs);
        } else if case.mode == "project" {
            skip_project_prefixes.push(abs);
        }
    }

    let examples_root = workspace_root().join("examples");
    let mut all_paths = Vec::new();
    walk_examples_dir(&examples_root, &mut all_paths);

    for path in all_paths {
        if path.extension().and_then(|e| e.to_str()) != Some("ny") {
            continue;
        }

        // Only treat actual entrypoints: `fn main` exists in the file.
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        if !src.contains("fn main") {
            continue;
        }

        if skip_files.contains(&path) {
            continue;
        }
        if skip_project_prefixes.iter().any(|p| path.starts_with(p)) {
            continue;
        }

        let rel = path.strip_prefix(workspace_root()).unwrap();
        let id = format!("all_examples_compile:{}", rel.display());

        let out = Compiler::compile_file(&path, &CompileOptions::default())
            .unwrap_or_else(|e| panic!("{id}: compile error: {e}"));

        assert!(out.lexer_errors.is_empty(), "{id}: lexer errors: {:?}", out.lexer_errors);
        assert!(out.parser_errors.is_empty(), "{id}: parser errors: {:?}", out.parser_errors);
        assert!(out.type_errors.is_empty(), "{id}: type errors: {:?}", out.type_errors);
        assert!(out.borrow_errors.is_empty(), "{id}: borrow errors: {:?}", out.borrow_errors);
        assert!(out.llvm_ir.is_some(), "{id}: missing LLVM IR");
    }
}
