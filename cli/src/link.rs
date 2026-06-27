//! Native link pipeline: LLVM `opt` on IR, then `clang` with optimization / LTO / PGO.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::runtime_map::{resolve_runtime_modules_installed, RuntimeProfile};

use crate::llvm_tools;
use crate::target::{LinkTargetFlags, TargetSpec, apply_target_link_flags};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    #[default]
    O0,
    O1,
    O2,
    O3,
}

impl OptLevel {
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(Self::O0),
            1 => Some(Self::O1),
            2 => Some(Self::O2),
            3 => Some(Self::O3),
            _ => None,
        }
    }

    pub(crate) fn clang_flag(self) -> &'static str {
        match self {
            Self::O0 => "-O0",
            Self::O1 => "-O1",
            Self::O2 => "-O2",
            Self::O3 => "-O3",
        }
    }

    fn llvm_passes(self) -> &'static str {
        match self {
            Self::O0 => "default<O0>",
            Self::O1 => "default<O1>",
            Self::O2 => "default<O2>",
            Self::O3 => "default<O3>",
        }
    }

    fn legacy_opt_flag(self) -> &'static str {
        match self {
            Self::O0 => "-O0",
            Self::O1 => "-O1",
            Self::O2 => "-O2",
            Self::O3 => "-O3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LtoMode {
    #[default]
    Off,
    Thin,
    Full,
}

#[derive(Debug, Clone, Default)]
pub struct LinkProfile {
    pub opt_level: OptLevel,
    pub lto: LtoMode,
    /// Run `opt` on `.ll` before `clang` (promotes allocas, inlines, etc.).
    pub llvm_ir_opt: bool,
    pub debug_symbols: bool,
    pub cdylib: bool,
    /// Extra `-lfoo` libraries passed to clang.
    pub link_libs: Vec<String>,
    /// Extra `-Lpath` search paths for clang.
    pub link_search_paths: Vec<PathBuf>,
    /// Raw linker arguments (e.g. `-framework`, `-Wl,...`).
    pub link_args: Vec<String>,
    /// Extra C translation units (package `link-source` lines).
    pub link_sources: Vec<PathBuf>,
    pub pgo_generate: bool,
    pub pgo_use: Option<PathBuf>,
    /// `-march=native` for local max performance (not for portable CI artifacts).
    pub native_cpu: bool,
    /// `-ffreestanding -nostdlib` for kernels / bare-metal images.
    pub freestanding: bool,
    /// ThreadSanitizer (`-fsanitize=thread`) for data-race detection at runtime.
    pub race: bool,
    /// Native Nyra race runtime (`stdlib/rt/rt_race.c`) — lightweight lock-set detector.
    pub race_native: bool,
    /// AddressSanitizer (`-fsanitize=address`) for heap/stack use-after-free detection.
    pub sanitize: bool,
}

impl LinkProfile {
    /// `release` enables O3 + LLVM IR opt + thin LTO unless overridden.
    #[allow(clippy::too_many_arguments)]
    pub fn from_cli(
        release: bool,
        opt: Option<u8>,
        lto: bool,
        no_lto: bool,
        no_llvm_opt: bool,
        pgo_generate: bool,
        pgo_use: Option<PathBuf>,
        native_cpu: bool,
    ) -> Result<Self, String> {
        let opt_level = if let Some(n) = opt {
            OptLevel::from_u8(n).ok_or_else(|| "opt level must be 0, 1, 2, or 3".to_string())?
        } else if release {
            OptLevel::O3
        } else {
            OptLevel::O0
        };

        let lto = if no_lto {
            LtoMode::Off
        } else if lto || release {
            LtoMode::Thin
        } else {
            LtoMode::Off
        };

        let llvm_ir_opt = !no_llvm_opt && opt_level != OptLevel::O0;

        Ok(Self {
            opt_level,
            lto,
            llvm_ir_opt,
            debug_symbols: false,
            cdylib: false,
            link_libs: Vec::new(),
            link_search_paths: Vec::new(),
            link_args: Vec::new(),
            link_sources: Vec::new(),
            pgo_generate,
            pgo_use,
            native_cpu,
            freestanding: false,
            race: false,
            race_native: false,
            sanitize: false,
        })
    }

    pub fn with_sanitize(mut self, sanitize: bool) -> Self {
        self.sanitize = sanitize;
        self
    }

    pub fn with_race(mut self, race: bool) -> Self {
        self.race = race;
        self
    }

    pub fn with_race_native(mut self, race_native: bool) -> Self {
        self.race_native = race_native;
        self
    }

    pub fn with_freestanding(mut self, freestanding: bool) -> Self {
        self.freestanding = freestanding;
        self
    }

    pub fn with_native_link(
        mut self,
        libs: Vec<String>,
        search_paths: Vec<PathBuf>,
        args: Vec<String>,
        sources: Vec<PathBuf>,
    ) -> Self {
        self.link_libs = libs;
        self.link_search_paths = search_paths;
        self.link_args = args;
        self.link_sources = sources;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug_symbols = debug;
        self
    }

    pub fn with_cdylib(mut self, cdylib: bool) -> Self {
        self.cdylib = cdylib;
        self
    }
}

/// Legacy unified runtime paths (backward compatibility).
#[allow(dead_code)]
pub fn runtime_c_path_for_target(target: &str) -> PathBuf {
    if target.contains("wasm") {
        compiler::runtime_map::wasi_runtime_path()
    } else {
        compiler::runtime_map::legacy_runtime_path()
    }
}

#[allow(dead_code)]
pub fn runtime_c_path() -> PathBuf {
    compiler::runtime_map::legacy_runtime_path()
}

/// `.../nyra/bin/nyra` → `.../nyra/share/stdlib/nyra_rt.c`
#[allow(dead_code)]
pub fn runtime_from_exe(exe: &Path) -> Option<PathBuf> {
    let bin_dir = exe.parent()?;
    let install_root = bin_dir.parent()?;
    Some(install_root.join("share/stdlib/nyra_rt.c"))
}

fn find_llvm_opt() -> Option<String> {
    llvm_tools::find_llvm_opt()
}

fn sanitize_ir_file(path: &Path) -> Result<(), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let cleaned = llvm_tools::sanitize_ir_for_clang(&raw);
    if cleaned != raw {
        std::fs::write(path, cleaned).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Optimize textual IR; returns path to use for linking (may equal input if opt skipped).
pub fn optimize_llvm_ir(
    ll_in: &Path,
    work_dir: &Path,
    profile: &LinkProfile,
    target_triple: &str,
) -> Result<PathBuf, String> {
    if profile.pgo_generate {
        // Match release CFG (O3) so profdata applies without hash-mismatch discards.
        let passes = format!("{},pgo-instr-gen", OptLevel::O3.llvm_passes());
        return run_llvm_opt_passes(
            ll_in,
            work_dir,
            &passes,
            target_triple,
            None,
            "out.instr.ll",
        );
    }

    if !profile.llvm_ir_opt {
        return Ok(ll_in.to_path_buf());
    }

    let require_opt = profile.opt_level != OptLevel::O0
        || profile.pgo_use.is_some();

    let opt_bin = if require_opt {
        llvm_tools::require_llvm_opt()?
    } else {
        match find_llvm_opt() {
            Some(bin) => bin,
            None => {
                eprintln!(
                    "note: llvm `opt` not found; linking unoptimized IR (install LLVM for faster debug builds)"
                );
                return Ok(ll_in.to_path_buf());
            }
        }
    };

    let ll_out = work_dir.join("out.opt.ll");
    let passes = if let Some(ref prof) = profile.pgo_use {
        format!("{},pgo-instr-use", profile.opt_level.llvm_passes())
    } else {
        profile.opt_level.llvm_passes().to_string()
    };

    let mut new_cmd = Command::new(&opt_bin);
    new_cmd
        .arg("-S")
        .arg(format!("-passes={passes}"));
    if let Some(ref prof) = profile.pgo_use {
        new_cmd.arg(format!("-profile-file={}", prof.display()));
    }
    if !target_triple.is_empty() {
        new_cmd.arg(format!("-mtriple={target_triple}"));
    }
    let new_style = new_cmd
        .arg(ll_in)
        .arg("-o")
        .arg(&ll_out)
        .status();

    let ok = match new_style {
        Ok(s) if s.success() => true,
        _ => {
            let mut legacy = Command::new(&opt_bin);
            legacy
                .arg("-S")
                .arg(profile.opt_level.legacy_opt_flag())
                .arg(ll_in)
                .arg("-o")
                .arg(&ll_out);
            if let Some(ref prof) = profile.pgo_use {
                legacy.arg(format!("-profile-file={}", prof.display()));
            }
            legacy
                .status()
                .map_err(|e| format!("Failed to run {opt_bin}: {e}"))?
                .success()
        }
    };

    if ok {
        sanitize_ir_file(&ll_out)?;
        Ok(ll_out)
    } else if require_opt {
        Err(format!("llvm `opt` failed — cannot continue release/PGO build"))
    } else {
        eprintln!("note: `opt` failed; linking original IR");
        Ok(ll_in.to_path_buf())
    }
}

fn run_llvm_opt_passes(
    ll_in: &Path,
    work_dir: &Path,
    passes: &str,
    target_triple: &str,
    profile_file: Option<&Path>,
    out_name: &str,
) -> Result<PathBuf, String> {
    let opt_bin = llvm_tools::require_llvm_opt()?;
    let ll_out = work_dir.join(out_name);
    let mut cmd = Command::new(&opt_bin);
    cmd.arg("-S").arg(format!("-passes={passes}"));
    if let Some(prof) = profile_file {
        cmd.arg(format!("-profile-file={}", prof.display()));
    }
    if !target_triple.is_empty() {
        cmd.arg(format!("-mtriple={target_triple}"));
    }
    let status = cmd
        .arg(ll_in)
        .arg("-o")
        .arg(&ll_out)
        .status()
        .map_err(|e| format!("Failed to run {opt_bin}: {e}"))?;
    if !status.success() {
        return Err(format!("llvm `opt` failed — cannot continue release/PGO build"));
    }
    sanitize_ir_file(&ll_out)?;
    Ok(ll_out)
}

fn link_target_spec(target: &str) -> TargetSpec {
    if target.is_empty() {
        TargetSpec::host()
    } else {
        use crate::target::{TargetFlags, parse_arch, parse_os, resolve};
        resolve(&TargetFlags {
            target: Some(target.to_string()),
            ..Default::default()
        })
        .unwrap_or_else(|_| TargetSpec {
            triple: target.to_string(),
            os: parse_os(target),
            arch: parse_arch(target),
            is_cross: true,
            is_wasm: target.contains("wasm"),
        })
    }
}

pub fn link_binary(
    ll_path: &Path,
    bin_path: &Path,
    profile: &LinkProfile,
    work_dir: &Path,
    target: &str,
    runtime_profile: &RuntimeProfile,
) -> Result<(), String> {
    let spec = link_target_spec(target);
    let triple_for_opt = if target.is_empty() {
        String::new()
    } else {
        target.to_string()
    };
    let ll_link = optimize_llvm_ir(ll_path, work_dir, profile, &triple_for_opt)?;
    let mut rt_modules = resolve_runtime_modules_installed(runtime_profile, target)?;

    if profile.cdylib {
        let alloc = compiler::runtime_map::stdlib_rt_dir().join("rt_alloc.c");
        if alloc.is_file() && !rt_modules.iter().any(|p| p.ends_with("rt_alloc.c")) {
            rt_modules.push(alloc);
        }
    }

    if profile.race_native {
        let race = compiler::runtime_map::stdlib_rt_dir().join("rt_race.c");
        if race.is_file() && !rt_modules.iter().any(|p| p.ends_with("rt_race.c")) {
            rt_modules.push(race);
        }
    }

    let link_objects = crate::c_cache::compile_link_sources(
        &profile.link_sources,
        work_dir,
        profile,
        &spec,
    )?;
    rt_modules = filter_runtime_modules_superseded_by_link_objects(rt_modules, &link_objects)?;

    let clang = llvm_tools::find_clang();
    let mut cmd = Command::new(&clang);
    cmd.arg(&ll_link);
    for rt in &rt_modules {
        cmd.arg(rt);
    }
    for obj in &link_objects {
        cmd.arg(obj);
    }
    let rt_flags = LinkTargetFlags {
        needs_pthread: runtime_profile.needs_pthread(),
        uses_rt_os: runtime_profile.modules().contains("rt_os.c"),
        uses_rt_hw: runtime_profile.modules().contains("rt_hw.c"),
        uses_rt_os_adv: runtime_profile.modules().contains("rt_os_adv.c"),
        uses_rt_net: runtime_profile.modules().contains("rt_net.c"),
        needs_openssl: runtime_profile.needs_openssl(),
        needs_zlib: runtime_profile.needs_zlib(),
        needs_libm: runtime_profile.needs_libm(),
    };
    apply_target_link_flags(&mut cmd, &spec, &rt_flags);

    cmd.arg(profile.opt_level.clang_flag());

    match profile.lto {
        LtoMode::Off => {}
        LtoMode::Thin => {
            cmd.arg("-flto=thin");
        }
        LtoMode::Full => {
            cmd.arg("-flto");
        }
    }

    if profile.pgo_generate {
        cmd.arg("-fprofile-instr-generate");
    }
    if let Some(ref prof) = profile.pgo_use {
        let flag = format!("-fprofile-instr-use={}", prof.display());
        cmd.arg(flag);
    }

    if profile.debug_symbols {
        cmd.arg("-g");
    }
    if profile.cdylib {
        cmd.arg("-shared");
        if spec.os != crate::target::TargetOs::Windows {
            cmd.arg("-fPIC");
        }
        if spec.os == crate::target::TargetOs::MacOs {
            let install_name = bin_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("invalid cdylib path {}", bin_path.display()))?;
            cmd.arg(format!("-Wl,-install_name,@rpath/{install_name}"));
        }
    }
    if profile.native_cpu {
        cmd.arg("-march=native");
    }

    if profile.freestanding {
        cmd.arg("-ffreestanding");
        cmd.arg("-nostdlib");
    }

    if profile.race {
        cmd.arg("-fsanitize=thread");
        cmd.arg("-fno-omit-frame-pointer");
        if !profile.debug_symbols {
            cmd.arg("-g");
        }
    }
    if profile.sanitize {
        cmd.arg("-fsanitize=address");
        cmd.arg("-fno-omit-frame-pointer");
        if !profile.debug_symbols {
            cmd.arg("-g");
        }
    }
    if profile.race_native {
        cmd.arg("-DNYRA_RACE_NATIVE_BUILD");
    }

    if runtime_profile_needs_compiler_ffi(runtime_profile) {
        if let Some(dir) = compiler_ffi_lib_dir() {
            cmd.arg(format!("-L{}", dir.display()));
            #[cfg(target_os = "macos")]
            cmd.arg(format!("-Wl,-rpath,{}", dir.display()));
            cmd.arg("-lnyra_compiler");
        }
    }

    let link_tmp = bin_path.with_extension("nyra-link-tmp");
    if link_tmp.exists() {
        let _ = fs::remove_file(&link_tmp);
    }

    cmd.arg("-o").arg(&link_tmp).arg("-Wno-override-module");

    for path in &profile.link_search_paths {
        cmd.arg(format!("-L{}", path.display()));
    }
    for lib in &profile.link_libs {
        if lib.starts_with('-') {
            cmd.arg(lib);
        } else {
            cmd.arg(format!("-l{lib}"));
        }
    }
    for arg in &profile.link_args {
        cmd.arg(arg);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to invoke clang: {e}"))?;

    if !status.success() {
        let _ = fs::remove_file(&link_tmp);
        return Err(format!("clang failed to link LLVM IR ({clang})"));
    }

    fs::rename(&link_tmp, bin_path).map_err(|e| {
        let _ = fs::remove_file(&link_tmp);
        format!("failed to install {}: {e}", bin_path.display())
    })?;
    if profile.cdylib {
        ensure_macos_cdylib_install_name(bin_path, &spec)?;
    }
    Ok(())
}

/// macOS dylibs are linked via a temp path first; fix LC_ID_DYLIB for `@rpath` loading.
pub fn ensure_macos_cdylib_install_name(
    path: &Path,
    spec: &TargetSpec,
) -> Result<(), String> {
    if spec.os != crate::target::TargetOs::MacOs {
        return Ok(());
    }
    if path.extension().and_then(|e| e.to_str()) != Some("dylib") {
        return Ok(());
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("invalid dylib path {}", path.display()))?;
    let id = format!("@rpath/{file_name}");
    let status = Command::new("install_name_tool")
        .args(["-id", &id, path.to_str().ok_or("non-utf8 dylib path")?])
        .status()
        .map_err(|e| format!("install_name_tool: {e}"))?;
    if !status.success() {
        return Err(format!(
            "install_name_tool -id failed for {}",
            path.display()
        ));
    }
    Ok(())
}

/// Drop stdlib `rt/*.c` units when package `link-source` objects already define the same symbols.
fn filter_runtime_modules_superseded_by_link_objects(
    rt_modules: Vec<PathBuf>,
    link_objects: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    if link_objects.is_empty() {
        return Ok(rt_modules);
    }

    let mut link_syms = HashSet::new();
    for obj in link_objects {
        link_syms.extend(object_exported_symbols(obj)?);
    }
    if link_syms.is_empty() {
        return Ok(rt_modules);
    }

    Ok(rt_modules
        .into_iter()
        .filter(|rt| {
            let Ok(rt_syms) = c_source_exported_symbols(rt) else {
                return true;
            };
            !rt_syms.iter().any(|sym| link_syms.contains(sym))
        })
        .collect())
}

fn demangle_linker_symbol(sym: &str) -> String {
    sym.strip_prefix('_').unwrap_or(sym).to_string()
}

fn object_exported_symbols(path: &Path) -> Result<HashSet<String>, String> {
    let output = if cfg!(target_os = "macos") {
        Command::new("nm")
            .args(["-gU", path.to_str().ok_or("non-utf8 object path")?])
            .output()
    } else {
        Command::new("nm")
            .args([
                "-g",
                "--defined-only",
                path.to_str().ok_or("non-utf8 object path")?,
            ])
            .output()
    }
    .map_err(|e| format!("nm {}: {e}", path.display()))?;

    if !output.status.success() {
        return Err(format!(
            "nm failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut out = HashSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1] == "T" {
            out.insert(demangle_linker_symbol(parts[2]));
        }
    }
    Ok(out)
}

fn c_source_exported_symbols(path: &Path) -> Result<HashSet<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
        {
            continue;
        }
        let Some((head, _)) = line.split_once('(') else {
            continue;
        };
        let Some(name) = head.split_whitespace().last() else {
            continue;
        };
        if name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !matches!(name, "if" | "for" | "while" | "switch" | "return")
        {
            out.insert(name.to_string());
        }
    }
    Ok(out)
}

fn runtime_profile_needs_compiler_ffi(profile: &RuntimeProfile) -> bool {
    profile.symbols.iter().any(|s| {
        s.starts_with("nyra_check")
            || s.starts_with("nyra_diag_json")
            || s == "nyra_compiler_free"
    })
}

fn compiler_ffi_lib_dir() -> Option<PathBuf> {
    if let Ok(root) = std::env::var("NYRA_ROOT") {
        let dir = PathBuf::from(root).join("target/debug");
        if dir.join(compiler_ffi_lib_name()).is_file() {
            return Some(dir);
        }
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.join(compiler_ffi_lib_name()).is_file() {
        return Some(exe_dir);
    }
    None
}

fn compiler_ffi_lib_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "libnyra_compiler.dylib"
    } else if cfg!(target_os = "windows") {
        "nyra_compiler.dll"
    } else {
        "libnyra_compiler.so"
    }
}

#[cfg(test)]
mod link_source_filter_tests {
    use super::*;
    use std::process::Command as Proc;

    #[test]
    fn c_source_exported_symbols_finds_functions() {
        let dir = std::env::temp_dir().join(format!("nyra_link_filter_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let c = dir.join("shim.c");
        fs::write(
            &c,
            "int sqlite_open(const char *path) { return 0; }\nvoid sqlite_close(int h) {}\n",
        )
        .unwrap();
        let syms = c_source_exported_symbols(&c).unwrap();
        assert!(syms.contains("sqlite_open"));
        assert!(syms.contains("sqlite_close"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn filter_drops_runtime_module_when_link_object_defines_same_symbol() {
        let dir = std::env::temp_dir().join(format!("nyra_link_filter2_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let rt = dir.join("rt_sqlite.c");
        fs::write(&rt, "int sqlite_open(const char *p) { return 1; }\n").unwrap();
        let src = dir.join("sqlite.c");
        fs::write(&src, "int sqlite_open(const char *p) { return 2; }\n").unwrap();
        let obj = dir.join("sqlite.o");
        let clang = llvm_tools::find_clang();
        assert!(
            Proc::new(&clang)
                .args(["-c", src.to_str().unwrap(), "-o", obj.to_str().unwrap()])
                .status()
                .unwrap()
                .success()
        );

        let filtered = filter_runtime_modules_superseded_by_link_objects(vec![rt.clone()], &[obj])
            .unwrap();
        assert!(filtered.is_empty(), "expected rt_sqlite.c to be skipped");

        let kept =
            filter_runtime_modules_superseded_by_link_objects(vec![rt], &[]).unwrap();
        assert_eq!(kept.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_defaults_to_o3_thin_lto() {
        let p = LinkProfile::from_cli(true, None, false, false, false, false, None, false).unwrap();
        assert_eq!(p.opt_level, OptLevel::O3);
        assert_eq!(p.lto, LtoMode::Thin);
        assert!(p.llvm_ir_opt);
    }

    #[test]
    fn explicit_o2_no_release() {
        let p = LinkProfile::from_cli(false, Some(2), false, false, false, false, None, false).unwrap();
        assert_eq!(p.opt_level, OptLevel::O2);
        assert_eq!(p.lto, LtoMode::Off);
        assert!(p.llvm_ir_opt);
    }

    #[test]
    fn runtime_from_exe_resolves_share_stdlib() {
        let tmp = std::env::temp_dir().join(format!("nyra_rt_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("bin")).unwrap();
        std::fs::create_dir_all(tmp.join("share/stdlib")).unwrap();
        std::fs::write(tmp.join("share/stdlib/nyra_rt.c"), "// test\n").unwrap();
        let exe = tmp.join("bin/nyra");
        let rt = runtime_from_exe(&exe).expect("layout .../bin/nyra");
        assert!(rt.ends_with("share/stdlib/nyra_rt.c"));
        assert!(rt.is_file());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn empty_profile_links_no_runtime_modules() {
        let profile = RuntimeProfile::default();
        let mods = resolve_runtime_modules_installed(&profile, "").unwrap();
        assert!(mods.is_empty());
    }

    #[test]
    fn input_profile_links_rt_io() {
        use compiler::{CompileOptions, Compiler};
        use std::io::Write;

        let src = r#"fn main() {
    let n = input("> ")
    print(n)
}"#;
        let out = Compiler::compile_source(src, "input.ny", &CompileOptions::default()).unwrap();
        assert!(
            out.runtime_profile.symbols.contains("stdin_read_line"),
            "{:?}",
            out.runtime_profile.symbols
        );
        let mods = resolve_runtime_modules_installed(&out.runtime_profile, "").unwrap();
        assert!(
            mods.iter().any(|p| p.ends_with("rt_io.c")),
            "modules: {:?}",
            mods
        );

        let work = std::env::temp_dir().join(format!("nyra_link_input_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        let ll = work.join("input.ll");
        let bin = work.join("input");
        std::fs::write(&ll, out.llvm_ir.unwrap()).unwrap();
        let profile = LinkProfile::default();
        link_binary(&ll, &bin, &profile, &work, "", &out.runtime_profile).unwrap();
        assert!(bin.is_file());

        let mut child = std::process::Command::new(&bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(b"fast\n").unwrap();
        }
        let run = child.wait_with_output().unwrap();
        assert!(run.status.success());
        let out = String::from_utf8_lossy(&run.stdout);
        assert!(out.contains("fast"), "stdout: {out}");
        let _ = std::fs::remove_dir_all(&work);
    }

    #[test]
    fn async_state_machine_string_links_rt_async() {
        use compiler::{
            load_program_with_options, parse_source, set_diagnostic_root, LoadOptions,
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/nyra/async_state_machine_string_test.ny");
        let compile_opts = compiler::CompileOptions::default();
        let loaded = load_program_with_options(&path, LoadOptions { auto_prelude: true })
            .unwrap();
        let mut program = loaded.program;
        program.functions.retain(|f| f.name != "main");
        let harness_main = parse_source(
            "fn main() {\n    test_state_machine_string_return()\n}",
            "harness.ny",
        )
        .unwrap();
        program.functions.extend(harness_main.functions);
        set_diagnostic_root(path.parent().unwrap());
        let out = compiler::Compiler::compile_program(
            &program,
            &path.to_string_lossy(),
            &compile_opts,
            Some(&path),
            loaded.errors,
        )
        .unwrap();
        assert!(
            out.runtime_profile.symbols.contains("async_future_done"),
            "{:?}",
            out.runtime_profile.symbols
        );
        let work = std::env::temp_dir().join(format!("nyra_link_async_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        let ll = work.join("async.ll");
        let bin = work.join("async");
        std::fs::write(&ll, out.llvm_ir.unwrap()).unwrap();
        let profile = LinkProfile::default();
        link_binary(&ll, &bin, &profile, &work, "", &out.runtime_profile).unwrap();
        assert!(bin.is_file());
        let run = std::process::Command::new(&bin).output().unwrap();
        assert!(run.status.success(), "stderr: {}", String::from_utf8_lossy(&run.stderr));
        let _ = std::fs::remove_dir_all(&work);
    }
}
