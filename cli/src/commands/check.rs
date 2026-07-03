use std::path::{Path, PathBuf};
use std::time::Instant;

use compiler::{
    check_cache_key, compute_source_fingerprint, is_check_cache_hit, mix_crate_manifest,
    write_check_cache, CompileOptions, CompileStage, Compiler, CrateManifest,
};
use errors::diagnostics_to_json;

use crate::app::args::{OptFlags, StabilityFlags};
use crate::artifacts;
use crate::target::TargetSpec;
use crate::ui::{format_build_elapsed, Ui};

pub(crate) fn path_or_file(p: &Path) -> PathBuf {
    p.to_path_buf()
}

fn check_layout(path: &Path) -> (artifacts::ArtifactLayout, String) {
    let spec = TargetSpec::host();
    let layout = artifacts::layout(path, false, None, &spec, false);
    let entry_id = artifacts::entry_cache_id(&layout);
    (layout, entry_id)
}

pub(crate) fn diag(path: &Path, json: bool, stability: &StabilityFlags) -> Result<(), String> {
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        deny_extended: stability.deny_extended,
        deny_warnings: stability.deny_warnings,
        ..CompileOptions::default()
    };
    let output = if path.is_dir() {
        Compiler::compile_project(path, &options)?
    } else {
        Compiler::compile_file(path, &options)?
    };
    if json {
        let mut all = Vec::new();
        for e in output
            .warnings
            .iter()
            .chain(&output.load_errors)
            .chain(&output.lexer_errors)
            .chain(&output.parser_errors)
            .chain(&output.type_errors)
            .chain(&output.borrow_errors)
        {
            all.push(e);
        }
        let owned: Vec<_> = all.into_iter().cloned().collect();
        println!(
            "{}",
            diagnostics_to_json(&owned).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if Compiler::report_errors(&output) {
        return Err("diagnostics found".into());
    }
    println!("diag: {} — ok", path.display());
    Ok(())
}

pub(crate) fn check(path: &Path, stability: &StabilityFlags) -> Result<(), String> {
    check_with_opt(path, stability, &OptFlags::default())
}

pub(crate) fn check_with_opt(
    path: &Path,
    stability: &StabilityFlags,
    opt: &OptFlags,
) -> Result<(), String> {
    if let Some(result) = crate::daemon::try_dispatch_check(path, opt, stability)? {
        return result;
    }
    let started = Instant::now();
    let check_key = check_cache_key(stability.deny_extended, stability.deny_warnings);
    let manifest = CrateManifest::scan(path)?;
    let source = compute_source_fingerprint(path)?;
    let source_fp = mix_crate_manifest(source, manifest.combined_hash());
    let (layout, entry_id) = check_layout(path);
    std::fs::create_dir_all(&layout.profile_dir).map_err(|e| e.to_string())?;

    if is_check_cache_hit(
        &layout.profile_dir,
        &entry_id,
        source_fp.hash,
        check_key,
    ) {
        let ui = Ui::new();
        eprintln!(
            "{}",
            ui.finished("check", "", &format_build_elapsed(started.elapsed()))
        );
        println!("check: {} — ok", path.display());
        return Ok(());
    }

    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        deny_extended: stability.deny_extended,
        deny_warnings: stability.deny_warnings,
        ..CompileOptions::default()
    };
    let output = if path.is_dir() {
        Compiler::compile_project(path, &options)?
    } else {
        Compiler::compile_file(path, &options)?
    };
    if Compiler::report_errors(&output) {
        return Err("check failed".into());
    }

    write_check_cache(
        &layout.profile_dir,
        &entry_id,
        source_fp.hash,
        check_key,
    )?;
    compiler::save_manifest(&layout.profile_dir, &entry_id, &manifest)?;

    let ui = Ui::new();
    eprintln!(
        "{}",
        ui.finished("check", "", &format_build_elapsed(started.elapsed()))
    );
    println!("check: {} — ok", path.display());
    Ok(())
}
