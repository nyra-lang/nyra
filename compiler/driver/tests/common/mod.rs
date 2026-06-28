//! Shared helpers for compiler integration tests.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Once, OnceLock};

use compiler::{CompileOptions, CompileOutput, CompileStage, Compiler};
use errors::{set_color_choice, ColorChoice};

static PLAIN_DIAGNOSTICS: Once = Once::new();

/// Snapshot and integration tests must not embed ANSI color codes.
pub fn ensure_plain_diagnostics() {
    PLAIN_DIAGNOSTICS.call_once(|| set_color_choice(ColorChoice::Never));
}

/// Workspace root (`NyraProgrammingLangauge/`).
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn examples_dir() -> PathBuf {
    workspace_root().join("examples")
}

pub fn tests_dir() -> PathBuf {
    workspace_root().join("tests")
}

pub fn corpus_dir() -> PathBuf {
    tests_dir().join("corpus")
}

/// Compile inline source with default options.
pub fn compile(src: &str) -> CompileOutput {
    ensure_plain_diagnostics();
    Compiler::compile_source(src, "test.ny", &CompileOptions::default()).unwrap()
}

/// Compile inline source with custom options.
pub fn compile_with(src: &str, file: &str, opts: &CompileOptions) -> CompileOutput {
    ensure_plain_diagnostics();
    Compiler::compile_source(src, file, opts).unwrap()
}

/// Compile source stopping at a pipeline stage.
pub fn compile_stage(src: &str, stage: CompileStage) -> CompileOutput {
    let opts = CompileOptions {
        stop_after: Some(stage),
        ..Default::default()
    };
    compile_with(src, "test.ny", &opts)
}

/// Compile a file relative to workspace root (e.g. `examples/syntax/math.ny`).
pub fn compile_file_rel(rel: &str) -> CompileOutput {
    let path = workspace_root().join(rel);
    Compiler::compile_file(&path, &CompileOptions::default()).unwrap()
}

/// Compile an example by path relative to `examples/`.
pub fn compile_example(rel: &str) -> CompileOutput {
    let path = examples_dir().join(rel);
    Compiler::compile_file(&path, &CompileOptions::default()).unwrap()
}

/// Compile a project directory relative to workspace root.
pub fn compile_project_rel(rel: &str) -> CompileOutput {
    let path = workspace_root().join(rel);
    Compiler::compile_project(&path, &CompileOptions::default()).unwrap()
}

/// Assert compile output has no errors at any stage.
pub fn assert_clean(out: &CompileOutput) {
    assert!(out.lexer_errors.is_empty(), "lexer: {:?}", out.lexer_errors);
    assert!(out.parser_errors.is_empty(), "parser: {:?}", out.parser_errors);
    assert!(out.type_errors.is_empty(), "type: {:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "borrow: {:?}", out.borrow_errors);
}

/// Assert IR contains required patterns and excludes forbidden ones.
pub fn assert_ir_patterns(ir: &str, must: &[&str], must_not: &[&str]) {
    for pat in must {
        assert!(ir.contains(pat), "IR must contain `{pat}`:\n{ir}");
    }
    for pat in must_not {
        assert!(!ir.contains(pat), "IR must not contain `{pat}`:\n{ir}");
    }
}

fn nyra_bin_once() -> &'static PathBuf {
    static NYRA: OnceLock<PathBuf> = OnceLock::new();
    NYRA.get_or_init(|| {
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_nyra") {
            return PathBuf::from(path);
        }
        let path = workspace_root().join("target/debug/nyra");
        if !path.exists() {
            let status = Command::new("cargo")
                .args(["build", "-p", "cli", "--quiet"])
                .current_dir(workspace_root())
                .status()
                .expect("cargo build -p cli");
            assert!(status.success(), "failed to build nyra CLI");
        }
        assert!(
            path.exists(),
            "nyra binary missing at {} — run `cargo build -p cli`",
            path.display()
        );
        path
    })
}

/// Path to the `nyra` CLI binary (builds `cli` crate on first use if needed).
pub fn nyra_bin() -> PathBuf {
    nyra_bin_once().clone()
}

/// Run `nyra` with arguments; builds the CLI if missing.
pub fn run_nyra(args: &[&str]) -> Output {
    let mut cmd = Command::new(nyra_bin_once());
    cmd.args(args);
    cmd.env_remove("NYRA_HOME");
    cmd.output().expect("run nyra")
}

/// Run `nyra run` on a path and return stdout trimmed.
pub fn run_nyra_file(path: &Path) -> String {
    let output = run_nyra(&["run", &path.to_string_lossy()]);
    assert!(
        output.status.success(),
        "nyra run failed for {}: stderr={}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Format all errors from compile output for snapshot tests.
pub fn format_all_errors(out: &CompileOutput) -> String {
    ensure_plain_diagnostics();
    let mut lines = Vec::new();
    for e in &out.load_errors {
        lines.push(format!("{e}"));
    }
    for e in &out.lexer_errors {
        lines.push(format!("{e}"));
    }
    for e in &out.parser_errors {
        lines.push(format!("{e}"));
    }
    for e in &out.type_errors {
        lines.push(format!("{e}"));
    }
    for e in &out.borrow_errors {
        lines.push(format!("{e}"));
    }
    for e in &out.warnings {
        lines.push(format!("{e}"));
    }
    lines.join("\n")
}

/// True when IR defines a program entry (`main` lowers to C `argc`/`argv` when applicable).
pub fn ir_defines_main(ir: &str) -> bool {
    ir.contains("define void @main(") || ir.contains("define i32 @main(")
}

/// Normalize LLVM IR for snapshot comparison (strip non-deterministic bits).
pub fn normalize_ir(ir: &str) -> String {
    let filtered: Vec<String> = ir
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.starts_with(';')
                && !t.starts_with("declare void @rt_args_init")
                && !t.starts_with("call void @rt_args_init")
        })
        .map(normalize_ir_line)
        .collect();

    let stripped = strip_stdlib_serde_functions(&filtered.join("\n"));
    let ssa_norm = normalize_ssa_names(&stripped);
    sort_ir_sections(&normalize_string_constants(&ssa_norm))
}

fn normalize_ir_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with("target triple = ") {
        return "target triple = \"nyra-snapshot-host\"".to_string();
    }
    if trimmed.starts_with("define i32 @main(") {
        return "define i32 @main() {".to_string();
    }
    line.to_string()
}

/// Drop lazy-prelude struct JSON/clone helpers — not under test in codegen snapshots.
fn strip_stdlib_serde_functions(ir: &str) -> String {
    const STDLIB: &[&str] = &[
        "CalendarDate",
        "DateTime",
        "HttpUrl",
        "HttpRequest",
        "HttpResponse",
        "RequestContext",
        "Server",
        "Client",
        "TcpListener",
        "TcpStream",
        "Promise_i32",
        "StrVec",
    ];

    let lines: Vec<&str> = ir.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if trimmed.starts_with("define ") && is_stdlib_serde_define(trimmed, STDLIB) {
            i += 1;
            while i < lines.len() && lines[i].trim() != "}" {
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            continue;
        }
        out.push(line.to_string());
        i += 1;
    }
    out.join("\n")
}

fn is_stdlib_serde_define(line: &str, stdlib: &[&str]) -> bool {
    for name in stdlib {
        if line.contains(&format!("@{name}_json_")) || line.contains(&format!("@{name}_clone")) {
            return true;
        }
    }
    false
}

/// Remap `%foo.123` / `while.body.9:` suffixes to stable per-prefix indices (sorted discovery).
fn normalize_ssa_names(ir: &str) -> String {
    use std::collections::{BTreeSet, HashMap};

    fn collect_ssa_token(token: &str, out: &mut BTreeSet<String>) {
        if let Some(dot) = token.rfind('.') {
            let suffix = &token[dot + 1..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                let prefix = &token[..dot];
                if !prefix.is_empty() {
                    out.insert(token.to_string());
                }
            }
        }
    }

    fn scan_ssa_tokens(input: &str, out: &mut BTreeSet<String>) {
        let bytes = input.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let ch = bytes[i] as char;
            if ch == '%' {
                let sym_start = i + 1;
                let mut j = sym_start;
                while j < bytes.len() {
                    let c = bytes[j] as char;
                    if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                        j += 1;
                    } else {
                        break;
                    }
                }
                collect_ssa_token(&input[sym_start..j], out);
                i = j;
                continue;
            }
            if ch.is_ascii_alphabetic() {
                let start = i;
                let mut j = i;
                while j < bytes.len() {
                    let c = bytes[j] as char;
                    if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                        j += 1;
                    } else {
                        break;
                    }
                }
                if j < bytes.len() && bytes[j] == b':' {
                    collect_ssa_token(&input[start..j], out);
                    i = j + 1;
                    continue;
                }
            }
            i += 1;
        }
    }

    let mut tokens = BTreeSet::new();
    scan_ssa_tokens(ir, &mut tokens);

    let mut prefix_next: HashMap<String, usize> = HashMap::new();
    let mut remap: HashMap<String, String> = HashMap::new();
    for full in &tokens {
        let (prefix, _) = full.rsplit_once('.').unwrap_or((full.as_str(), "0"));
        let n = *prefix_next.entry(prefix.to_string()).or_insert(0);
        prefix_next.insert(prefix.to_string(), n + 1);
        remap.insert(full.clone(), format!("{prefix}.{n}"));
    }

    let mut old_keys: Vec<String> = remap.keys().cloned().collect();
    old_keys.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));

    let mut out = String::new();
    let bytes = ir.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '%' {
            let sym_start = i + 1;
            let mut j = sym_start;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                    j += 1;
                } else {
                    break;
                }
            }
            let sym = &ir[sym_start..j];
            out.push('%');
            out.push_str(remap.get(sym).map(String::as_str).unwrap_or(sym));
            i = j;
            continue;
        }
        if ch == '@' {
            let sym_start = i + 1;
            let mut j = sym_start;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                    j += 1;
                } else {
                    break;
                }
            }
            out.push('@');
            out.push_str(&ir[sym_start..j]);
            i = j;
            continue;
        }
        if ch.is_ascii_alphabetic() {
            let start = i;
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j < bytes.len() && bytes[j] == b':' {
                let sym = &ir[start..j];
                if remap.contains_key(sym) {
                    out.push_str(remap.get(sym).unwrap());
                    out.push(':');
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn str_literal_key(line: &str) -> String {
    line.find(" c\"")
        .map(|i| line[i..].to_string())
        .unwrap_or_else(|| line.to_string())
}

fn str_def_index(line: &str) -> Option<String> {
    let t = line.trim();
    t.strip_prefix("@.str.")
        .and_then(|r| r.split('=').next())
        .map(str::trim)
        .map(str::to_string)
}

/// Sort `@.str.N` globals by literal bytes and renumber references deterministically.
fn normalize_string_constants(ir: &str) -> String {
    use std::collections::{BTreeMap, HashMap};

    let mut defs: Vec<(String, String)> = Vec::new();
    for line in ir.lines() {
        let t = line.trim();
        if t.starts_with("@.str.") && t.contains(" = private unnamed_addr constant") {
            if let Some(idx) = str_def_index(line) {
                defs.push((idx, line.to_string()));
            }
        }
    }

    let mut by_content: BTreeMap<String, String> = BTreeMap::new();
    for (_, line) in &defs {
        let key = str_literal_key(line);
        by_content.entry(key).or_insert_with(|| line.clone());
    }

    let mut content_to_new: HashMap<String, String> = HashMap::new();
    for (new_i, key) in by_content.keys().enumerate() {
        content_to_new.insert(key.clone(), new_i.to_string());
    }

    let mut remap: HashMap<String, String> = HashMap::new();
    for (old_idx, line) in &defs {
        let key = str_literal_key(line);
        if let Some(new_idx) = content_to_new.get(&key) {
            remap.insert(old_idx.clone(), new_idx.clone());
        }
    }

    let mut old_keys: Vec<String> = remap.keys().cloned().collect();
    old_keys.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));

    let remapped = ir
        .lines()
        .filter(|line| {
            let t = line.trim();
            !(t.starts_with("@.str.") && t.contains(" = private unnamed_addr constant"))
        })
        .map(|line| {
            let mut s = line.to_string();
            for old in &old_keys {
                if let Some(new) = remap.get(old) {
                    s = s.replace(&format!("@.str.{old}"), &format!("@.str.{new}"));
                }
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut canon_defs: Vec<String> = Vec::new();
    for (new_i, (_key, template)) in by_content.iter().enumerate() {
        if let Some(rest) = template.find(" = private unnamed_addr constant") {
            let suffix = &template[rest..];
            canon_defs.push(format!("@.str.{new_i}{suffix}"));
        }
    }

    let mut out: Vec<String> = remapped.lines().map(str::to_string).collect();
    if !canon_defs.is_empty() {
        // Insert canonical string globals after `target triple` header.
        let insert_at = out
            .iter()
            .position(|l| l.starts_with("target triple = "))
            .map(|i| i + 1)
            .unwrap_or(out.len());
        if insert_at < out.len() && !out[insert_at].is_empty() {
            out.insert(insert_at, String::new());
        }
        for (i, def) in canon_defs.iter().enumerate() {
            out.insert(insert_at + 1 + i, def.clone());
        }
    }
    out.join("\n")
}

/// Canonical section order for snapshot tests (immune to codegen emission order).
fn sort_ir_sections(ir: &str) -> String {
    let mut meta: Vec<String> = Vec::new();
    let mut strs: Vec<String> = Vec::new();
    let mut types: Vec<String> = Vec::new();
    let mut declares: Vec<String> = Vec::new();
    let mut defines: Vec<String> = Vec::new();
    let mut other: Vec<String> = Vec::new();

    let mut current_define: Vec<String> = Vec::new();
    let mut in_define = false;

    for line in ir.lines() {
        let t = line.trim();
        if in_define {
            current_define.push(line.to_string());
            if t == "}" {
                defines.push(current_define.join("\n"));
                current_define.clear();
                in_define = false;
            }
            continue;
        }
        if t.starts_with("source_filename") || t.starts_with("target triple") {
            meta.push(line.to_string());
        } else if t.starts_with("@.str.") && t.contains(" = private unnamed_addr constant") {
            strs.push(line.to_string());
        } else if t.starts_with('%') && t.contains(" = type {") {
            types.push(line.to_string());
        } else if t.starts_with("declare ") {
            declares.push(line.to_string());
        } else if t.starts_with("define ") {
            in_define = true;
            current_define.push(line.to_string());
        } else if t.starts_with("attributes #") {
            other.push(line.to_string());
        } else if t.is_empty() {
            // skip — we re-insert blank lines between sections
        } else {
            other.push(line.to_string());
        }
    }

    strs.sort();
    types.sort();
    declares.sort();
    defines.sort();
    other.sort();

    let mut out = meta;
    if !strs.is_empty() {
        out.push(String::new());
        out.extend(strs);
    }
    if !types.is_empty() {
        out.push(String::new());
        out.extend(types);
    }
    if !declares.is_empty() {
        out.push(String::new());
        out.extend(declares);
    }
    if !other.is_empty() {
        out.push(String::new());
        out.extend(other);
    }
    if !defines.is_empty() {
        out.push(String::new());
        out.extend(defines);
    }
    out.join("\n")
}
