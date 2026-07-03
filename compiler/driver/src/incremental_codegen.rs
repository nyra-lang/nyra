//! Per-crate incremental LLVM IR cache + `llvm-link` merge (dev builds).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use ast::Program;
use codegen::Codegen;
use ownership::{DropPlan, EscapePlan};
use rayon::prelude::*;
use resolve::{assign_functions_to_units, collect_source_units, paths};

use crate::{load_unit_ir, save_unit_ir, CompileOptions, CrateManifest};

#[derive(Debug, Clone)]
pub struct IncrementalContext {
    pub profile_dir: PathBuf,
    pub entry_id: String,
    pub entry_path: PathBuf,
    pub dirty_paths: Vec<String>,
    pub manifest: CrateManifest,
}

fn is_dirty(ctx: &IncrementalContext, path: &str) -> bool {
    ctx.dirty_paths.iter().any(|p| p == path)
}

fn find_llvm_link() -> Option<String> {
    for name in ["llvm-link-21", "llvm-link-20", "llvm-link-19", "llvm-link"] {
        if Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(name.to_string());
        }
    }
    None
}

pub fn link_ir_modules(inputs: &[PathBuf], out: &Path) -> Result<(), String> {
    if inputs.is_empty() {
        return Err("no IR modules to link".into());
    }
    if inputs.len() == 1 {
        fs::copy(&inputs[0], out).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let llvm_link = find_llvm_link().ok_or_else(|| {
        "llvm-link not found (install LLVM tools for incremental multi-file builds)".to_string()
    })?;
    let mut cmd = Command::new(&llvm_link);
    for input in inputs {
        cmd.arg("-S").arg(input);
    }
    cmd.arg("-o").arg(out);
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run {llvm_link}: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("llvm-link failed: {}", stderr.trim()))
    }
}

fn merge_runtime(into: &mut codegen::RuntimeProfile, other: &codegen::RuntimeProfile) {
    into.symbols.extend(other.symbols.iter().cloned());
}

fn compile_unit_ir(
    program: &Program,
    file: &str,
    options: &CompileOptions,
    drop_plan: &DropPlan,
    escape_plan: &EscapePlan,
    fns: &HashSet<String>,
) -> (String, codegen::RuntimeProfile) {
    let mut codegen = Codegen::new(file);
    codegen.set_target(&options.target);
    codegen.set_drop_plan(drop_plan.clone());
    codegen.set_escape_plan(escape_plan.clone());
    let ir = codegen.compile_program_with_filter(program, Some(fns));
    let rt = codegen.take_runtime_profile();
    (ir, rt)
}

struct UnitIrJob {
    path: PathBuf,
    content_hash: u64,
    fns: HashSet<String>,
    from_cache: bool,
}

/// Split dev codegen: one IR module per source file, cached by content hash.
pub fn split_dev_codegen(
    program: &Program,
    file: &str,
    options: &CompileOptions,
    drop_plan: &DropPlan,
    escape_plan: &EscapePlan,
    ctx: &IncrementalContext,
) -> Result<(String, codegen::RuntimeProfile), String> {
    if ctx.manifest.units.len() <= 1 {
        return Err("split codegen requires multiple source files".into());
    }
    if !program.export_instances.is_empty() {
        return Err("split codegen unavailable with monomorph export instances".into());
    }

    let units = collect_source_units(&ctx.entry_path)?;
    let fn_map = assign_functions_to_units(&units, program);

    let work = ctx
        .profile_dir
        .join(".nyra-cache")
        .join("entries")
        .join(&ctx.entry_id)
        .join("link-units");
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;

    let mut jobs: Vec<UnitIrJob> = Vec::with_capacity(ctx.manifest.units.len());
    for unit in &ctx.manifest.units {
        let fns = fn_map.get(&unit.path).cloned().unwrap_or_default();
        let part = work.join(format!("{}.ll", unit.content_hash));
        let from_cache = !is_dirty(ctx, &unit.path)
            && load_unit_ir(&ctx.profile_dir, &ctx.entry_id, unit.content_hash).is_some();
        if from_cache {
            let cached = load_unit_ir(&ctx.profile_dir, &ctx.entry_id, unit.content_hash).unwrap();
            fs::write(&part, &cached).map_err(|e| e.to_string())?;
        }
        jobs.push(UnitIrJob {
            path: part,
            content_hash: unit.content_hash,
            fns,
            from_cache,
        });
    }

    let program = Arc::new(program.clone());
    let file = Arc::new(file.to_string());
    let options = Arc::new(options.clone());
    let drop_plan = Arc::new(drop_plan.clone());
    let escape_plan = Arc::new(escape_plan.clone());
    let profile_dir = ctx.profile_dir.clone();
    let entry_id = ctx.entry_id.clone();
    let runtime = Arc::new(Mutex::new(codegen::RuntimeProfile::default()));

    jobs.par_iter().try_for_each(|job| -> Result<(), String> {
        if job.from_cache {
            return Ok(());
        }
        let (ir, rt) = compile_unit_ir(
            &program,
            &file,
            &options,
            &drop_plan,
            &escape_plan,
            &job.fns,
        );
        save_unit_ir(&profile_dir, &entry_id, job.content_hash, &ir)?;
        fs::write(&job.path, &ir).map_err(|e| e.to_string())?;
        if let Ok(mut guard) = runtime.lock() {
            merge_runtime(&mut guard, &rt);
        }
        Ok(())
    })?;

    let merged_runtime = runtime
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();

    let ir_paths: Vec<PathBuf> = jobs.iter().map(|j| j.path.clone()).collect();
    let linked = work.join("merged.ll");
    link_ir_modules(&ir_paths, &linked)?;
    let ir = fs::read_to_string(&linked).map_err(|e| e.to_string())?;
    Ok((ir, merged_runtime))
}

impl Clone for CompileOptions {
    fn clone(&self) -> Self {
        Self {
            stop_after: self.stop_after,
            target: self.target.clone(),
            no_std: self.no_std,
            freestanding: self.freestanding,
            deny_extended: self.deny_extended,
            deny_warnings: self.deny_warnings,
            features: self.features.clone(),
            verbose_escape: self.verbose_escape,
            no_prelude: self.no_prelude,
            skip_typecheck: self.skip_typecheck,
            incremental: None,
            dev_fast: self.dev_fast,
        }
    }
}

pub fn entry_path_for_compile(path: &Path) -> PathBuf {
    if path.is_dir() {
        paths::find_main_entry(path).unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_path_for_dir_and_file() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/nyra");
        let dir = root.join("modules");
        if dir.join("main.ny").exists() {
            assert!(entry_path_for_compile(&dir).ends_with("main.ny"));
        }
    }
}
