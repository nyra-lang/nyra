//! File-based Nyra compile test harness (pass / fail / run).

mod directives;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Once, OnceLock};

use compiler::{CompileOptions, CompileOutput, Compiler};
use directives::{parse_directives, stderr_path, DiagKind, FileDirectives};
use errors::{set_color_choice, ColorChoice};
use regex::Regex;
use walkdir::WalkDir;

pub use directives::{ExpectedDiag, FileDirectives as Directives};

static PLAIN_DIAGNOSTICS: Once = Once::new();

fn ensure_plain_diagnostics() {
    PLAIN_DIAGNOSTICS.call_once(|| set_color_choice(ColorChoice::Never));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteMode {
    Pass,
    Fail,
    Run,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Ignored,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub path: PathBuf,
    pub status: TestStatus,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SuiteResult {
    pub mode: SuiteMode,
    pub results: Vec<TestResult>,
}

impl SuiteResult {
    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Passed)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count()
    }

    pub fn ignored(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Ignored)
            .count()
    }

    pub fn assert_all_passed(&self) {
        let failures: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .collect();
        if failures.is_empty() {
            return;
        }
        let mut msg = format!(
            "suite {:?}: {} failed / {} total\n",
            self.mode,
            failures.len(),
            self.results.len()
        );
        for f in failures {
            msg.push_str(&format!("  {} — {}\n", f.path.display(), f.message));
        }
        panic!("{msg}");
    }
}

/// Collect `.ny` files under `root`, optionally filtered by substring.
pub fn collect_tests(root: &Path, filter: &str) -> Vec<PathBuf> {
    collect_tests_excluding(root, filter, &[])
}

/// Like [`collect_tests`] but skips paths containing any of `exclude_components`
/// (e.g. `generated` so umbrella suites do not overlap shard targets).
pub fn collect_tests_excluding(
    root: &Path,
    filter: &str,
    exclude_components: &[&str],
) -> Vec<PathBuf> {
    if root.is_file() {
        if is_ny_test(root)
            && matches_filter(root, filter)
            && !is_excluded(root, exclude_components)
        {
            return vec![root.to_path_buf()];
        }
        return vec![];
    }
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            is_ny_test(p)
                && matches_filter(p, filter)
                && !is_project_aux_file(p)
                && !is_excluded(p, exclude_components)
        })
        .collect();
    files.sort();
    files
}

fn is_excluded(path: &Path, exclude_components: &[&str]) -> bool {
    exclude_components.iter().any(|component| {
        path.components()
            .any(|c| c.as_os_str() == std::ffi::OsStr::new(component))
    })
}

fn is_ny_test(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "ny")
}

/// Any non-`main.ny` source under `projects/` is loaded via the project entry.
fn is_project_aux_file(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n == "main.ny") {
        return false;
    }
    path.components().any(|c| c.as_os_str() == "projects")
}

fn matches_filter(path: &Path, filter: &str) -> bool {
    filter.is_empty() || path.to_string_lossy().contains(filter)
}

/// Run all tests under `root` for the given mode.
pub fn run_suite(root: &Path, mode: SuiteMode, filter: &str, update: bool) -> SuiteResult {
    run_suite_labeled(root, mode, filter, update, "")
}

/// Like [`run_suite`] but skips paths under excluded directory components.
pub fn run_suite_excluding(
    root: &Path,
    mode: SuiteMode,
    filter: &str,
    update: bool,
    exclude_components: &[&str],
) -> SuiteResult {
    run_suite_labeled_excluding(root, mode, filter, update, "", exclude_components)
}

/// Like [`run_suite`] but prints per-test progress when not in quiet mode.
pub fn run_suite_labeled(
    root: &Path,
    mode: SuiteMode,
    filter: &str,
    update: bool,
    label: &str,
) -> SuiteResult {
    run_suite_labeled_excluding(root, mode, filter, update, label, &[])
}

/// Like [`run_suite_labeled`] but skips paths under excluded directory components.
pub fn run_suite_labeled_excluding(
    root: &Path,
    mode: SuiteMode,
    filter: &str,
    update: bool,
    label: &str,
    exclude_components: &[&str],
) -> SuiteResult {
    ensure_plain_diagnostics();
    let files = collect_tests_excluding(root, filter, exclude_components);
    let total = files.len();
    let verbose = progress_enabled();
    let opts = CompileOptions::default();

    if verbose && total > 0 {
        let tag = suite_tag(label, mode);
        eprintln!("{tag}: starting {total} tests");
    }

    let mut results = Vec::with_capacity(total);
    for (idx, path) in files.iter().enumerate() {
        let result = run_one(path, mode, &opts, update);
        if verbose {
            print_progress(label, mode, idx + 1, total, path, &result);
        }
        results.push(result);
    }

    if verbose && total > 0 {
        let passed = results.iter().filter(|r| r.status == TestStatus::Passed).count();
        let failed = results.iter().filter(|r| r.status == TestStatus::Failed).count();
        let ignored = results.iter().filter(|r| r.status == TestStatus::Ignored).count();
        let tag = suite_tag(label, mode);
        eprintln!(
            "{tag}: done — {passed} passed, {failed} failed, {ignored} ignored / {total} total"
        );
    }

    SuiteResult { mode, results }
}

fn progress_enabled() -> bool {
    !std::env::var("NYRA_SUITE_QUIET").is_ok()
}

fn suite_tag(label: &str, mode: SuiteMode) -> String {
    if label.is_empty() {
        format!("suite {mode:?}")
    } else {
        format!("suite {label}")
    }
}

fn short_test_path(path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(default_suite_root()) {
        return rel.display().to_string();
    }
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn print_progress(
    label: &str,
    mode: SuiteMode,
    current: usize,
    total: usize,
    path: &Path,
    result: &TestResult,
) {
    let remaining = total.saturating_sub(current);
    let tag = suite_tag(label, mode);
    let short = short_test_path(path);
    match result.status {
        TestStatus::Passed => {
            eprintln!("[{current}/{total}] PASS {short} ({remaining} left) — {tag}");
        }
        TestStatus::Ignored => {
            eprintln!("[{current}/{total}] SKIP {short} ({remaining} left) — {tag}");
        }
        TestStatus::Failed => {
            eprintln!("[{current}/{total}] FAIL {short} ({remaining} left) — {tag}");
            eprintln!("  {}", result.message.replace('\n', "\n  "));
        }
    }
}

fn run_one(path: &Path, mode: SuiteMode, opts: &CompileOptions, update: bool) -> TestResult {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                path: path.to_path_buf(),
                status: TestStatus::Failed,
                message: format!("read failed: {e}"),
            };
        }
    };
    let directives = parse_directives(&source);
    if directives.ignore {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Ignored,
            message: "ignored".into(),
        };
    }

    match mode {
        SuiteMode::Pass => run_pass(path, opts),
        SuiteMode::Fail => run_fail(path, &source, &directives, opts, update),
        SuiteMode::Run => run_run(path, &directives),
    }
}

fn run_pass(path: &Path, opts: &CompileOptions) -> TestResult {
    let out = match compile_test_file(path, opts) {
        Ok(o) => o,
        Err(e) => {
            return TestResult {
                path: path.to_path_buf(),
                status: TestStatus::Failed,
                message: format!("compile error: {e}"),
            };
        }
    };
    if has_errors(&out) {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: format!("expected clean compile, got:\n{}", format_all_errors(&out)),
        };
    }
    TestResult {
        path: path.to_path_buf(),
        status: TestStatus::Passed,
        message: "ok".into(),
    }
}

fn run_fail(
    path: &Path,
    source: &str,
    directives: &FileDirectives,
    opts: &CompileOptions,
    update: bool,
) -> TestResult {
    let out = match compile_test_file(path, opts) {
        Ok(o) => o,
        Err(e) => {
            return TestResult {
                path: path.to_path_buf(),
                status: TestStatus::Failed,
                message: format!("compile error: {e}"),
            };
        }
    };
    if !has_errors(&out) {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: "expected compile failure, but succeeded".into(),
        };
    }

    let formatted = format_all_errors(&out);
    let stderr_file = stderr_path(path);

    if stderr_file.exists() || update {
        return check_stderr_file(path, &formatted, update);
    }

    if directives.expected.is_empty() {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: "fail test needs //~ ERROR directives or a .stderr file".into(),
        };
    }

    match check_directives(source, directives, &formatted) {
        Ok(()) => TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Passed,
            message: "ok".into(),
        },
        Err(msg) => TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: msg,
        },
    }
}

fn check_stderr_file(path: &Path, actual: &str, update: bool) -> TestResult {
    let stderr_file = stderr_path(path);
    let normalized = normalize_diagnostics(actual);
    if update {
        if let Err(e) = std::fs::write(&stderr_file, &normalized) {
            return TestResult {
                path: path.to_path_buf(),
                status: TestStatus::Failed,
                message: format!("failed to write {}: {e}", stderr_file.display()),
            };
        }
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Passed,
            message: format!("updated {}", stderr_file.display()),
        };
    }
    let expected = match std::fs::read_to_string(&stderr_file) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                path: path.to_path_buf(),
                status: TestStatus::Failed,
                message: format!("missing {}: {e}", stderr_file.display()),
            };
        }
    };
    let expected_norm = normalize_diagnostics(&expected);
    if expected_norm == normalized {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Passed,
            message: "ok".into(),
        };
    }
    if stderr_lines_match(&expected_norm, &normalized) {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Passed,
            message: "ok".into(),
        };
    }
    TestResult {
        path: path.to_path_buf(),
        status: TestStatus::Failed,
        message: format!(
            "stderr mismatch for {}\n--- expected ({})\n{}\n--- actual\n{}",
            path.display(),
            stderr_file.display(),
            expected.trim_end(),
            normalized.trim_end()
        ),
    }
}

/// Each non-empty, non-`#` line in `.stderr` must appear in the actual output.
fn stderr_lines_match(expected: &str, actual: &str) -> bool {
    let lines: Vec<_> = expected
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if lines.is_empty() {
        return false;
    }
    lines.iter().all(|line| actual.contains(line))
}

fn check_directives(source: &str, directives: &FileDirectives, formatted: &str) -> Result<(), String> {
    for exp in &directives.expected {
        let pattern = &exp.pattern;
        let haystack = match exp.kind {
            DiagKind::Error => formatted,
            DiagKind::Warning => formatted,
        };
        if !pattern.is_empty() && !haystack.contains(pattern) {
            return Err(format!(
                "expected {} matching `{pattern}` not found in diagnostics:\n{haystack}",
                kind_label(exp.kind)
            ));
        }
        if pattern.is_empty() && !has_error_lines(haystack) {
            return Err(format!(
                "expected at least one {} but diagnostics were empty or warning-only:\n{haystack}",
                kind_label(exp.kind)
            ));
        }
        // Optional line check when directive is on the same line as code.
        if let Some(line_text) = source.lines().nth(exp.line.saturating_sub(1)) {
            let code = line_text.split("//~").next().unwrap_or(line_text).trim();
            if !code.is_empty() && exp.line > 0 {
                let _ = code; // line association reserved for future strict mode
            }
        }
    }
    Ok(())
}

fn compile_test_file(path: &Path, opts: &CompileOptions) -> Result<CompileOutput, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().into_owned();
    let use_file_loader = path.file_name().is_some_and(|n| n == "main.ny")
        || source.lines().any(|line| {
            let t = line.trim();
            t.starts_with("import \"") || t.starts_with("import {")
        });
    if use_file_loader {
        return Compiler::compile_file(path, opts);
    }
    Compiler::compile_source(&source, &file, opts)
}

fn has_error_lines(formatted: &str) -> bool {
    formatted.lines().any(|l| {
        l.starts_with("error") || l.contains("error[") || l.starts_with("error:")
    })
}

fn kind_label(kind: DiagKind) -> &'static str {
    match kind {
        DiagKind::Error => "ERROR",
        DiagKind::Warning => "WARNING",
    }
}

fn run_run(path: &Path, directives: &FileDirectives) -> TestResult {
    let Some(expected) = directives.run_stdout.as_ref() else {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: "run test needs `// run-stdout:` directive".into(),
        };
    };
    let nyra = nyra_bin();
    let output = Command::new(&nyra)
        .args(["run", &path.to_string_lossy()])
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", nyra.display()));
    if !output.status.success() {
        return TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: format!(
                "nyra run failed (exit {}):\nstdout={}\nstderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        };
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout == *expected {
        TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Passed,
            message: "ok".into(),
        }
    } else {
        TestResult {
            path: path.to_path_buf(),
            status: TestStatus::Failed,
            message: format!("stdout mismatch: expected `{expected}`, got `{stdout}`"),
        }
    }
}

pub fn has_errors(out: &CompileOutput) -> bool {
    !out.load_errors.is_empty()
        || !out.lexer_errors.is_empty()
        || !out.parser_errors.is_empty()
        || !out.type_errors.is_empty()
        || !out.borrow_errors.is_empty()
}

/// Format all compile diagnostics (plain text, no ANSI).
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
    for w in &out.warnings {
        lines.push(format!("{w}"));
    }
    lines.join("\n")
}

/// Strip volatile paths/timestamps for golden `.stderr` comparison.
pub fn normalize_diagnostics(text: &str) -> String {
    let re_path = Regex::new(r"--> [^\n]+").unwrap();
    let re = re_path.replace_all(text, "--> TESTFILE");
    re.to_string()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workspace_debug_nyra_bin() -> PathBuf {
    let root = workspace_root();
    let base = root.join("target/debug/nyra");
    if base.is_file() {
        return base;
    }
    let with_exe = root.join(format!("target/debug/nyra{}", std::env::consts::EXE_SUFFIX));
    if with_exe.is_file() {
        return with_exe;
    }
    if std::env::consts::EXE_SUFFIX.is_empty() {
        base
    } else {
        with_exe
    }
}

fn nyra_bin_once() -> &'static PathBuf {
    static NYRA: OnceLock<PathBuf> = OnceLock::new();
    NYRA.get_or_init(|| {
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_nyra") {
            return PathBuf::from(path);
        }
        if !workspace_debug_nyra_bin().is_file() {
            let status = Command::new("cargo")
                .args(["build", "-p", "cli", "--quiet"])
                .current_dir(workspace_root())
                .status()
                .expect("cargo build -p cli");
            assert!(status.success(), "failed to build nyra CLI");
        }
        workspace_debug_nyra_bin()
    })
}

pub fn nyra_bin() -> PathBuf {
    nyra_bin_once().clone()
}

/// Compile `path` and return formatted diagnostics (for `.stderr` capture).
pub fn capture_errors(path: &Path) -> Result<String, String> {
    ensure_plain_diagnostics();
    let out = Compiler::compile_file(path, &CompileOptions::default())?;
    Ok(format_all_errors(&out))
}

pub fn default_suite_root() -> PathBuf {
    workspace_root().join("tests/suite")
}
