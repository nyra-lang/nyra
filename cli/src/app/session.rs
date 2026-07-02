//! Build, run, link, and PGO session logic.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use compiler::{
    can_skip_codegen, compute_source_fingerprint, is_incremental_hit, link_cache_key,
    load_manifest, mix_crate_manifest, options_cache_key, read_runtime_cache, save_manifest,
    write_cached_fingerprint, write_runtime_cache, CompileOptions, Compiler, CrateManifest,
};
use pkg::{build_link_crate, resolve_project_native_link};

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::artifacts::{self, profile_name};
use crate::link::{self, LinkProfile};
use crate::pgo;
use crate::target::{TargetSpec, validate_native_cpu};
use crate::ui::{format_build_elapsed, build_profile_detail, Ui};

pub(crate) fn apply_lto_full(mut profile: LinkProfile, lto_full: bool) -> LinkProfile {
    if lto_full {
        profile.lto = link::LtoMode::Full;
    }
    profile
}

/// Overrides for multi-phase links (PGO instrument / optimize).
#[derive(Default)]
pub(crate) struct CompileLinkConfig {
    bin_path: Option<PathBuf>,
    force_rebuild: bool,
    link_profile: Option<LinkProfile>,
    /// Use `target/release` layout even when `opt.release` is false (`nyra build --pgo`).
    release_layout: bool,
}

pub(crate) fn base_link_profile(
    opt: &OptFlags,
    release: bool,
    lto_full: bool,
    debug_symbols: bool,
    cdylib: bool,
    freestanding: bool,
    path: &Path,
    spec: &TargetSpec,
) -> Result<LinkProfile, String> {
    let mut flags = opt.clone();
    flags.release = release;
    flags.pgo = false;
    let (link_libs, link_search_paths, link_args, link_sources) = resolve_native_link(path, &flags)?;
    Ok(apply_lto_full(
        flags
            .link_profile(spec.is_cross)?
            .with_debug(debug_symbols)
            .with_cdylib(cdylib)
            .with_freestanding(freestanding)
            .with_native_link(link_libs, link_search_paths, link_args, link_sources),
        lto_full,
    ))
}

pub(crate) fn compile_options(
    spec: &TargetSpec,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
    stability: &StabilityFlags,
    verbose_escape: bool,
) -> CompileOptions {
    CompileOptions {
        target: spec.triple_for_codegen(),
        no_std,
        freestanding,
        no_prelude,
        deny_extended: stability.deny_extended,
        deny_warnings: stability.deny_warnings,
        verbose_escape,
        ..CompileOptions::default()
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build(
    path: &Path,
    output: Option<&str>,
    opt: &OptFlags,
    debug_symbols: bool,
    cdylib: bool,
    lto_full: bool,
    target_args: &TargetArgs,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
) -> Result<(), String> {
    let spec = target_args.resolve()?;
    if spec.is_wasm {
        crate::wasm_toolchain::prepare_wasm_toolchain()?;
    }
    validate_native_cpu(
        &spec,
        opt.native_cpu || (opt.release && !spec.is_cross && !opt.no_native_cpu),
    )?;
    if opt.pgo {
        pgo::validate_pgo_build(&spec, cdylib)?;
    }
    let bin_path = if opt.pgo {
        build_with_pgo(
            path,
            opt,
            debug_symbols,
            cdylib,
            lto_full,
            &spec,
            output,
            stability,
            no_std,
            freestanding,
            no_prelude,
        )?
    } else {
        compile_and_link(
            path,
            opt,
            debug_symbols,
            cdylib,
            lto_full,
            &spec,
            output,
            stability,
            no_std,
            freestanding,
            no_prelude,
            None,
        )?
    };
    if spec.is_cross {
        println!(
            "cross-compiled for {} → {}",
            spec.triple_for_codegen(),
            bin_path.display()
        );
    } else {
        println!("built: {}", bin_path.display());
    }
    Ok(())
}

pub(crate) fn run_file(
    path: &Path,
    opt: &OptFlags,
    target_args: &TargetArgs,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
) -> Result<(), String> {
    let spec = target_args.resolve()?;
    if spec.is_wasm {
        crate::wasm_toolchain::prepare_wasm_toolchain()?;
    }
    validate_native_cpu(
        &spec,
        opt.native_cpu || (opt.release && !spec.is_cross && !opt.no_native_cpu),
    )?;
    if opt.pgo {
        return Err("nyra run does not support --pgo (use nyra build --pgo, then run the binary)".into());
    }
    let bin_path = compile_and_link(
        path,
        opt,
        false,
        false,
        false,
        &spec,
        None,
        stability,
        no_std,
        freestanding,
        no_prelude,
        None,
    )?;

    if spec.is_cross {
        return Err(format!(
            "cannot run cross-compiled binary for {} on this host; built: {}",
            spec.triple_for_codegen(),
            bin_path.display()
        ));
    }

    let status = Command::new(&bin_path)
        .status()
        .map_err(|e| format!("Failed to run {}: {e}", bin_path.display()))?;

    if !status.success() {
        return Err(format!(
            "program exited with status {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compile_pgo_test_harness(
    harness: &str,
    label: &str,
    bin_path: &Path,
    instr_profile: &LinkProfile,
    spec: &TargetSpec,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
    work_dir: &Path,
) -> Result<(), String> {
    let options = compile_options(spec, no_std, freestanding, no_prelude, stability, false);
    let output = Compiler::compile_source(harness, label, &options)?;
    if Compiler::report_errors(&output) {
        return Err(format!("PGO test harness compile failed: {label}"));
    }
    let safe = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let ll = work_dir.join(format!("{safe}.ll"));
    std::fs::write(&ll, output.llvm_ir.unwrap()).map_err(|e| e.to_string())?;
    link::link_binary(
        &ll,
        bin_path,
        instr_profile,
        work_dir,
        "",
        &output.runtime_profile,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_with_pgo(
    path: &Path,
    opt: &OptFlags,
    debug_symbols: bool,
    cdylib: bool,
    lto_full: bool,
    spec: &TargetSpec,
    output: Option<&str>,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
) -> Result<PathBuf, String> {
    let layout = artifacts::layout(path, true, output, spec, cdylib);
    let pgo_layout = pgo::PgoLayout::new(&layout.profile_dir, spec);
    std::fs::create_dir_all(&pgo_layout.dir).map_err(|e| e.to_string())?;

    let source_fp = compute_source_fingerprint(path)?;
    let (link_libs, link_search_paths, link_args, link_sources) = {
        let mut flags = opt.clone();
        flags.release = true;
        flags.pgo = false;
        resolve_native_link(path, &flags)?
    };
    let training_args = pgo::resolve_training_args(path, &opt.pgo_arg);
    let training = pgo::PgoTrainingConfig {
        args: training_args.clone(),
        timeout: Duration::from_secs(opt.pgo_timeout),
    };
    let options_hash = pgo::pgo_options_hash(
        lto_full,
        debug_symbols,
        opt.native_cpu,
        freestanding,
        &link_libs,
        &link_args,
        &training_args,
        opt.pgo_timeout,
        pgo::comparison_training_fingerprint(path),
    );
    let cache_key = pgo::PgoCacheKey {
        source_hash: source_fp.hash,
        options_hash,
    };

    let profile_ready = pgo::profile_cache_hit(&pgo_layout, &cache_key);
    if profile_ready {
        eprintln!(
            "PGO: cache hit — source unchanged, {} valid → skipping instrument/train/merge",
            pgo_layout.profdata.display()
        );
        if layout.bin_path.is_file() {
            eprintln!(
                "PGO: phase 5/5 — binary up to date at {}",
                layout.bin_path.display()
            );
            return Ok(layout.bin_path);
        }
        eprintln!("PGO: phase 4/5 — optimized rebuild (binary missing, profile cached)");
        let base = base_link_profile(
            opt,
            true,
            lto_full,
            debug_symbols,
            cdylib,
            freestanding,
            path,
            spec,
        )?;
        let opt_profile = pgo::optimized_link_profile(
            base,
            pgo_layout.profdata.clone(),
            debug_symbols,
            cdylib,
            freestanding,
            link_libs,
            link_search_paths,
            link_args,
            link_sources,
        );
        return compile_and_link(
            path,
            opt,
            debug_symbols,
            cdylib,
            lto_full,
            spec,
            output,
            stability,
            no_std,
            freestanding,
            no_prelude,
            Some(CompileLinkConfig {
                bin_path: None,
                force_rebuild: false,
                link_profile: Some(opt_profile),
                release_layout: true,
            }),
        );
    }

    let base = base_link_profile(
        opt,
        true,
        lto_full,
        debug_symbols,
        cdylib,
        freestanding,
        path,
        spec,
    )?;
    let instr_profile = pgo::instrumented_link_profile(
        base.clone(),
        debug_symbols,
        cdylib,
        freestanding,
        link_libs.clone(),
        link_search_paths.clone(),
        link_args.clone(),
        link_sources.clone(),
    );

    eprintln!(
        "PGO: phase 1/5 — instrumented build (-fprofile-instr-generate) → {}",
        pgo_layout.instrumented_bin.display()
    );
    compile_and_link(
        path,
        opt,
        debug_symbols,
        cdylib,
        lto_full,
        spec,
        output,
        stability,
        no_std,
        freestanding,
        no_prelude,
        Some(CompileLinkConfig {
            bin_path: Some(pgo_layout.instrumented_bin.clone()),
            force_rebuild: true,
            link_profile: Some(instr_profile.clone()),
            release_layout: true,
        }),
    )?;

    let test_cases = pgo::discover_training_cases(path, &pgo_layout, spec)?;
    let mut test_bins = Vec::new();
    if !test_cases.is_empty() {
        eprintln!(
            "PGO: building {} instrumented test harness(es) for training",
            test_cases.len()
        );
        for case in &test_cases {
            compile_pgo_test_harness(
                &case.harness_source,
                &case.label,
                &case.instrumented_bin,
                &instr_profile,
                spec,
                stability,
                no_std,
                freestanding,
                no_prelude,
                &pgo_layout.dir,
            )?;
            test_bins.push(case.instrumented_bin.clone());
        }
    }

    eprintln!("PGO: phase 2/5 — automated training run");
    if test_bins.is_empty() {
        eprintln!("PGO:   no tests found — profiling main only");
    } else {
        eprintln!(
            "PGO:   main + {} test harness(es) (like `nyra test` discovery)",
            test_bins.len()
        );
    }
    pgo::run_training(
        &pgo_layout,
        &pgo_layout.instrumented_bin,
        &test_bins,
        &training,
    )?;

    eprintln!(
        "PGO: phase 3/5 — llvm-profdata merge → {}",
        pgo_layout.profdata.display()
    );
    pgo::merge_profdata(&pgo_layout)?;
    cache_key.write(&pgo_layout)?;

    eprintln!(
        "PGO: phase 4/5 — optimized rebuild (-fprofile-instr-use + LTO) → {}",
        layout.bin_path.display()
    );
    let opt_profile = pgo::optimized_link_profile(
        base,
        pgo_layout.profdata.clone(),
        debug_symbols,
        cdylib,
        freestanding,
        link_libs,
        link_search_paths,
        link_args,
        link_sources,
    );
    let final_bin = compile_and_link(
        path,
        opt,
        debug_symbols,
        cdylib,
        lto_full,
        spec,
        output,
        stability,
        no_std,
        freestanding,
        no_prelude,
        Some(CompileLinkConfig {
            bin_path: None,
            force_rebuild: true,
            link_profile: Some(opt_profile),
            release_layout: true,
        }),
    )?;

    eprintln!(
        "PGO: phase 5/5 — profile cached at {} (re-run after source changes)",
        pgo_layout.profdata.display()
    );
    Ok(final_bin)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_and_link(
    path: &Path,
    opt: &OptFlags,
    debug_symbols: bool,
    cdylib: bool,
    lto_full: bool,
    spec: &TargetSpec,
    output: Option<&str>,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
    config: Option<CompileLinkConfig>,
) -> Result<PathBuf, String> {
    let cfg = config.unwrap_or_default();
    let release = cfg.release_layout || opt.release;
    let link_target = spec.triple.clone();
    let layout = artifacts::layout(path, release, output, spec, cdylib);
    let bin_path = cfg.bin_path.clone().unwrap_or_else(|| layout.bin_path.clone());
    let entry_id = artifacts::entry_cache_id(&layout);
    std::fs::create_dir_all(&layout.profile_dir).map_err(|e| e.to_string())?;

    let options_key = options_cache_key(
        &spec.triple_for_codegen(),
        release,
        no_std,
        freestanding,
        stability.deny_extended,
        no_prelude,
    );
    let current_crates = CrateManifest::scan(path)?;
    let source_fp = mix_crate_manifest(
        compute_source_fingerprint(path)?,
        current_crates.combined_hash(),
    );
    let previous_crates = load_manifest(&layout.profile_dir, &entry_id);
    let dirty_paths: Vec<String> = previous_crates
        .as_ref()
        .map(|prev| current_crates.dirty_since(prev))
        .unwrap_or_default();
    let (link_libs, link_search_paths, link_args, link_sources) = resolve_native_link(path, opt)?;
    let link_hash = link_cache_key(
        &options_key,
        debug_symbols,
        cdylib,
        &link_libs,
        &link_args,
        &link_sources,
    );

    if !cfg.force_rebuild
        && is_incremental_hit(
            &layout.profile_dir,
            &entry_id,
            &layout.ll_path,
            &bin_path,
            &source_fp,
            link_hash,
        )
    {
        if cdylib {
            link::ensure_macos_cdylib_install_name(&bin_path, spec)?;
        }
        return Ok(bin_path);
    }

    let skipped_codegen = can_skip_codegen(
        &layout.profile_dir,
        &entry_id,
        &layout.ll_path,
        &source_fp,
        link_hash,
    );

    let build_started = Instant::now();
    let ui = Ui::new();
    let profile_label = profile_name(release);
    let profile_detail = build_profile_detail(release, debug_symbols);

    if !skipped_codegen {
        if dirty_paths.is_empty() {
            let label = artifacts::entry_stem(path);
            let root = project_root(path);
            eprintln!(
                "{}",
                ui.compiling(&label, &root.display().to_string())
            );
        } else {
            for dirty in &dirty_paths {
                let label = Path::new(dirty)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                eprintln!("{}", ui.compiling(&label, dirty));
            }
        }
    }

    let runtime_profile = if skipped_codegen {
        read_runtime_cache(&layout.profile_dir, &entry_id).unwrap_or_default()
    } else {
        let (ir, runtime_profile) =
            compile_to_ir(path, spec, stability, no_std, freestanding, no_prelude, opt.verbose)?;
        std::fs::write(&layout.ll_path, &ir).map_err(|e| e.to_string())?;
        write_runtime_cache(&layout.profile_dir, &entry_id, &runtime_profile)?;
        runtime_profile
    };

    let profile = if let Some(p) = cfg.link_profile {
        p
    } else {
        apply_lto_full(
            opt.link_profile(spec.is_cross)?
                .with_debug(debug_symbols)
                .with_cdylib(cdylib)
                .with_freestanding(freestanding)
                .with_native_link(link_libs, link_search_paths, link_args, link_sources),
            lto_full,
        )
    };
    let link_work = artifacts::entry_link_work_dir(&layout);
    std::fs::create_dir_all(&link_work).map_err(|e| e.to_string())?;
    link::link_binary(
        &layout.ll_path,
        &bin_path,
        &profile,
        &link_work,
        &link_target,
        &runtime_profile,
    )?;
    write_cached_fingerprint(&layout.profile_dir, &entry_id, source_fp.hash, link_hash)?;
    save_manifest(&layout.profile_dir, &entry_id, &current_crates)?;

    let elapsed = format_build_elapsed(build_started.elapsed());
    eprintln!(
        "{}",
        ui.finished(profile_label, profile_detail, &elapsed)
    );
    Ok(bin_path)
}

pub(crate) fn project_root(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or(path).to_path_buf()
    }
}

pub(crate) fn resolve_native_link(
    path: &Path,
    opt: &OptFlags,
) -> Result<(Vec<String>, Vec<PathBuf>, Vec<String>, Vec<PathBuf>), String> {
    let mut libs = Vec::new();
    let mut search_paths = Vec::new();
    let mut args = Vec::new();
    let mut sources = Vec::new();

    let root = project_root(path);
    if let Ok(nyra_mod) = resolve_project_native_link(&root) {
        libs.extend(nyra_mod.link_libs);
        search_paths.extend(
            nyra_mod
                .link_search_paths
                .into_iter()
                .map(PathBuf::from),
        );
        args.extend(nyra_mod.link_args);
        sources.extend(nyra_mod.link_sources.into_iter().map(PathBuf::from));

        for crate_name in &nyra_mod.link_crates {
            let (lib, search) = build_link_crate(&root, crate_name)?;
            if !search_paths.iter().any(|p| p == &search) {
                search_paths.push(search);
            }
            if !libs.contains(&lib) {
                libs.push(lib);
            }
        }
    }

    libs.extend(opt.link_lib.clone());
    search_paths.extend(opt.link_search_path.clone());
    args.extend(opt.link_arg.clone());

    Ok((libs, search_paths, args, sources))
}

pub(crate) fn compile_to_output(
    path: &Path,
    spec: &TargetSpec,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
    verbose_escape: bool,
) -> Result<compiler::CompileOutput, String> {
    let options = compile_options(
        spec,
        no_std,
        freestanding,
        no_prelude,
        stability,
        verbose_escape,
    );
    let output = if path.is_dir() {
        Compiler::compile_project(path, &options)?
    } else {
        Compiler::compile_file(path, &options)?
    };
    if Compiler::report_errors(&output) {
        return Err("compilation failed".into());
    }
    Ok(output)
}

pub(crate) fn compile_to_ir(
    path: &Path,
    spec: &TargetSpec,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
    verbose_escape: bool,
) -> Result<(String, compiler::RuntimeProfile), String> {
    let output = compile_to_output(
        path,
        spec,
        stability,
        no_std,
        freestanding,
        no_prelude,
        verbose_escape,
    )?;
    let ir = output
        .llvm_ir
        .ok_or_else(|| "no LLVM IR generated".to_string())?;
    Ok((ir, output.runtime_profile))
}
