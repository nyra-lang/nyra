use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::{load_program_with_options, parse_file, paths, set_diagnostic_root, Compiler, LoadOptions, parse_source};

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::app::session::{compile_options, resolve_native_link};
use crate::link;
use crate::link::LinkProfile;
use crate::target::validate_native_cpu;

#[derive(Debug, Clone)]
pub struct TestListEntry {
    pub file: String,
    pub name: String,
    pub line: usize,
}

pub(crate) fn test_dir(
    path: &Path,
    target_args: &TargetArgs,
    opt: &OptFlags,
    list_json: bool,
    filter: Option<&str>,
) -> Result<(), String> {
    if list_json {
        let entries = discover_tests(path)?;
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "file": e.file,
                    "name": e.name,
                    "line": e.line,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    run_tests(path, target_args, opt, filter)
}

fn discover_tests(path: &Path) -> Result<Vec<TestListEntry>, String> {
    let mut entries = Vec::new();
    for entry in walkdir_ny_files(path)? {
        let program = parse_file(&entry)?;
        let file = entry.to_string_lossy().into_owned();
        let mut found = false;
        for f in &program.functions {
            if f.is_test || f.name.starts_with("test_") {
                entries.push(TestListEntry {
                    file: file.clone(),
                    name: f.name.clone(),
                    line: f.span.start.line.max(1),
                });
                found = true;
            }
        }
        if !found && is_legacy_test_file(&entry) {
            entries.push(TestListEntry {
                file,
                name: "main".into(),
                line: 1,
            });
        }
    }
    entries.sort_by(|a, b| a.file.cmp(&b.file).then(a.name.cmp(&b.name)));
    Ok(entries)
}

fn run_tests(
    path: &Path,
    target_args: &TargetArgs,
    opt: &OptFlags,
    filter: Option<&str>,
) -> Result<(), String> {
    let spec = target_args.resolve()?;
    validate_native_cpu(&spec, opt.native_cpu)?;
    if spec.is_cross {
        return Err(format!(
            "nyra test does not run cross-compiled binaries (target {}); build on the target host or omit --for/--target",
            spec.triple_for_codegen()
        ));
    }
    let mut ran = 0;
    let mut failed = 0;
    let base_profile = LinkProfile::from_cli(false, None, false, false, false, false, None, false)?;
    let compile_opts = compile_options(&spec, false, false, false, &StabilityFlags::default(), false);
    for entry in walkdir_ny_files(path)? {
        let src = std::fs::read_to_string(&entry).map_err(|e| e.to_string())?;
        let program = parse_file(&entry)?;
        let mut tests: Vec<String> = program
            .functions
            .iter()
            .filter(|f| f.is_test || f.name.starts_with("test_"))
            .map(|f| f.name.clone())
            .collect();
        if tests.is_empty() && is_legacy_test_file(&entry) {
            tests.push("main".into());
        }
        if tests.is_empty() {
            continue;
        }
        for test_name in tests {
            if let Some(f) = filter {
                if !test_name.contains(f) {
                    continue;
                }
            }
            ran += 1;
            let label = format!("{}::{}", entry.display(), test_name);
            let output = compile_test_case(&entry, &src, &test_name, &compile_opts)?;
            if Compiler::report_errors(&output) {
                failed += 1;
                eprintln!("FAIL compile: {label}");
                continue;
            }
            let ir = output.llvm_ir.unwrap();
            let out_dir = std::env::temp_dir().join(format!(
                "nyra_test_{}_{}_{}",
                entry.file_stem().unwrap().to_string_lossy(),
                test_name,
                std::process::id()
            ));
            std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
            let ll = out_dir.join("out.ll");
            let bin = out_dir.join(format!("test_bin{}", spec.exe_extension()));
            std::fs::write(&ll, ir).map_err(|e| e.to_string())?;
            let mut profile = base_profile.clone();
            let (link_libs, link_search_paths, link_args, link_sources) =
                resolve_native_link(&entry, opt)?;
            profile.link_libs.extend(link_libs);
            profile.link_search_paths.extend(link_search_paths);
            profile.link_args.extend(link_args);
            profile.link_sources.extend(link_sources);
            link::link_binary(&ll, &bin, &profile, &out_dir, "", &output.runtime_profile)?;
            let status = Command::new(&bin).status().map_err(|e| e.to_string())?;
            let _ = std::fs::remove_dir_all(&out_dir);
            if status.success() {
                println!("PASS {label}");
            } else {
                failed += 1;
                eprintln!("FAIL run: {label}");
            }
        }
    }
    if ran == 0 {
        return Err(format!(
            "no tests found under {} (use `test fn`, test_*, or *_test.ny / *_test.nyra)",
            path.display()
        ));
    }
    if failed > 0 {
        return Err(format!("{failed}/{ran} tests failed"));
    }
    println!("{ran} tests passed");
    Ok(())
}

fn compile_test_case(
    entry: &Path,
    _src: &str,
    test_name: &str,
    compile_opts: &compiler::CompileOptions,
) -> Result<compiler::CompileOutput, String> {
    if test_name == "main" && is_legacy_test_file(entry) {
        return Compiler::compile_file(entry, compile_opts);
    }
    let load_opts = LoadOptions {
        auto_prelude: !compile_opts.no_std && !compile_opts.freestanding && !compile_opts.no_prelude,
    };
    let loaded = load_program_with_options(entry, load_opts)?;
    let mut program = loaded.program;
    program.functions.retain(|f| f.name != "main");
    let harness_main = parse_source(
        &format!("fn main() {{\n    {test_name}()\n}}"),
        "harness.ny",
    )?;
    program.functions.extend(harness_main.functions);
    let file = entry.to_string_lossy().into_owned();
    set_diagnostic_root(entry.parent().unwrap_or(entry));
    Compiler::compile_program(
        &program,
        &file,
        compile_opts,
        Some(entry),
        loaded.errors,
    )
}

pub(crate) fn is_legacy_test_file(p: &Path) -> bool {
    paths::is_legacy_test_file(p)
}

fn is_test_harness_artifact(p: &Path) -> bool {
    p.components()
        .any(|c| c.as_os_str() == std::ffi::OsStr::new(".nyra-test"))
}

/// CONF-LANG layout: `fail/` and `fixtures/` are not `nyra test` targets.
fn should_skip_test_dir(dir: &Path) -> bool {
    let name = match dir.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    if name != "fail" && name != "fixtures" {
        return false;
    }
    dir.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some("conformance")
}

pub(crate) fn walkdir_ny_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if root.is_file() && paths::is_nyra_source(root) && !is_test_harness_artifact(root) {
        files.push(root.to_path_buf());
        return Ok(files);
    }
    if !root.is_dir() {
        return Err(format!("not found: {}", root.display()));
    }
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.is_dir() {
            if should_skip_test_dir(&p) {
                continue;
            }
            files.extend(walkdir_ny_files(&p)?);
        } else if paths::is_nyra_source(&p) && !is_test_harness_artifact(&p) {
            files.push(p);
        }
    }
    files.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    Ok(files)
}
