//! Incremental `.o` cache for package `link-source` C files.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::link::{LinkProfile, LtoMode};
use crate::llvm_tools;
use crate::target::{apply_target_compile_flags, apply_windows_gcc_compile_flags, TargetSpec};

pub fn c_objects_cache_dir(work_dir: &Path) -> PathBuf {
    work_dir.join(".nyra-cache").join("c-objs")
}

fn compile_flags_key(profile: &LinkProfile, spec: &TargetSpec) -> String {
    let cc = if spec.os == crate::target::TargetOs::Windows
        && cfg!(target_os = "windows")
        && llvm_tools::find_mingw_gcc().is_some()
    {
        "gcc"
    } else {
        "clang"
    };
    format!(
        "cc={cc}|triple={}|opt={:?}|lto={:?}|dbg={}|native={}|free={}|cdylib={}|pgo_gen={}|pgo_use={}|race={}|race_native={}",
        spec.triple_for_codegen(),
        profile.opt_level,
        profile.lto,
        profile.debug_symbols,
        profile.native_cpu,
        profile.freestanding,
        profile.cdylib,
        profile.pgo_generate,
        profile
            .pgo_use
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        profile.race,
        profile.race_native,
    )
}

fn source_object_key(source: &Path, flags_key: &str) -> Result<u64, String> {
    let bytes = fs::read(source).map_err(|e| format!("read {}: {e}", source.display()))?;
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    bytes.hash(&mut hasher);
    flags_key.hash(&mut hasher);
    Ok(hasher.finish())
}

fn object_path(cache_dir: &Path, source: &Path, key: u64) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "source".into());
    cache_dir.join(format!("{stem}.{key:016x}.o"))
}

/// Compile `link-source` paths to cached object files; return paths to link.
pub fn compile_link_sources(
    sources: &[PathBuf],
    work_dir: &Path,
    profile: &LinkProfile,
    spec: &TargetSpec,
) -> Result<Vec<PathBuf>, String> {
    if sources.is_empty() {
        return Ok(Vec::new());
    }

    let cache_dir = c_objects_cache_dir(work_dir);
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let flags_key = compile_flags_key(profile, spec);
    let mut out = Vec::with_capacity(sources.len());

    for source in sources {
        if !source.is_file() {
            return Err(format!("link-source not found: {}", source.display()));
        }
        let key = source_object_key(source, &flags_key)?;
        let obj = object_path(&cache_dir, source, key);
        if obj.is_file() {
            out.push(obj);
            continue;
        }

        compile_one_source(source, &obj, profile, spec)?;
        out.push(obj);
    }

    Ok(out)
}

/// Like [`compile_link_sources`], but skips sources that fail to compile (fat prebuilt archives).
pub fn compile_link_sources_best_effort(
    sources: &[PathBuf],
    work_dir: &Path,
    profile: &LinkProfile,
    spec: &TargetSpec,
) -> Result<Vec<PathBuf>, String> {
    if sources.is_empty() {
        return Ok(Vec::new());
    }

    let cache_dir = c_objects_cache_dir(work_dir);
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let flags_key = compile_flags_key(profile, spec);
    let mut out = Vec::with_capacity(sources.len());

    for source in sources {
        if !source.is_file() {
            continue;
        }
        let key = source_object_key(source, &flags_key)?;
        let obj = object_path(&cache_dir, source, key);
        if obj.is_file() {
            out.push(obj);
            continue;
        }

        match compile_one_source(source, &obj, profile, spec) {
            Ok(()) => out.push(obj),
            Err(err) => eprintln!(
                "note: skipping {} in prebuilt runtime: {}",
                source.display(),
                err.lines().next().unwrap_or(&err)
            ),
        }
    }

    if out.is_empty() {
        return Err("no runtime objects compiled for prebuilt archive".into());
    }
    Ok(out)
}

fn compile_one_source(
    source: &Path,
    obj: &Path,
    profile: &LinkProfile,
    spec: &TargetSpec,
) -> Result<(), String> {
    let use_mingw_gcc = spec.os == crate::target::TargetOs::Windows
        && cfg!(target_os = "windows")
        && llvm_tools::find_mingw_gcc().is_some();
    let compiler = if use_mingw_gcc {
        llvm_tools::find_mingw_gcc().unwrap()
    } else {
        llvm_tools::find_clang()
    };
    let mut cmd = Command::new(&compiler);
    if use_mingw_gcc {
        apply_windows_gcc_compile_flags(&mut cmd);
    } else {
        apply_target_compile_flags(&mut cmd, spec);
    }
    cmd.arg("-c").arg(source).arg("-o").arg(obj);
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
        cmd.arg(format!("-fprofile-instr-use={}", prof.display()));
    }
    if profile.debug_symbols {
        cmd.arg("-g");
    }
    if profile.cdylib && spec.os != crate::target::TargetOs::Windows {
        cmd.arg("-fPIC");
    }
    if profile.native_cpu {
        cmd.arg("-march=native");
    }
    if profile.freestanding {
        cmd.arg("-ffreestanding");
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

    for path in &profile.link_search_paths {
        cmd.arg(format!("-I{}", path.display()));
    }

    if !use_mingw_gcc {
        cmd.arg("-Wno-override-module");
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to compile {}: {e}", source.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(format!(
            "nyra cc: failed to compile link-source {} (exit {}): {}",
            source.display(),
            output.status.code().unwrap_or(-1),
            detail
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::OptLevel;
    use crate::link::LinkProfile;
    use crate::target::TargetSpec;

    #[test]
    fn object_key_changes_with_content() {
        let tmp = std::env::temp_dir().join(format!("nyra_cobj_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("shim.c");
        fs::write(&src, "void f() {}\n").unwrap();
        let spec = TargetSpec::host();
        let profile = LinkProfile::default();
        let flags = compile_flags_key(&profile, &spec);
        let a = source_object_key(&src, &flags).unwrap();
        fs::write(&src, "void f() { return; }\n").unwrap();
        let b = source_object_key(&src, &flags).unwrap();
        assert_ne!(a, b);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn opt_level_affects_flags_key() {
        let spec = TargetSpec::host();
        let mut p0 = LinkProfile::default();
        p0.opt_level = OptLevel::O0;
        let mut p3 = LinkProfile::default();
        p3.opt_level = OptLevel::O3;
        assert_ne!(
            compile_flags_key(&p0, &spec),
            compile_flags_key(&p3, &spec)
        );
    }

    #[test]
    fn compiles_ny_sqlite_shim_to_object() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/packages/ny-sqlite");
        let m = pkg::resolve_project_native_link(&root).expect("resolve");
        let src = PathBuf::from(&m.link_sources[0]);
        if !sqlite_headers_available() {
            eprintln!("skip compiles_ny_sqlite_shim_to_object — install libsqlite3 dev headers");
            return;
        }
        let tmp = std::env::temp_dir().join(format!("nyra_cobj_build_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let spec = TargetSpec::host();
        let profile = LinkProfile::default();
        let objs = compile_link_sources(
            &[src],
            &tmp,
            &profile,
            &spec,
        )
        .expect("compile");
        assert_eq!(objs.len(), 1);
        assert!(objs[0].is_file());
        let _ = fs::remove_dir_all(&tmp);
    }

    fn sqlite_headers_available() -> bool {
        let clang = llvm_tools::find_clang();
        let out = std::process::Command::new(&clang)
            .args(["-E", "-x", "c", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        let Ok(mut child) = out else {
            return false;
        };
        use std::io::Write;
        if child.stdin.take().and_then(|mut s| s.write_all(b"#include <sqlite3.h>\n").ok()).is_none() {
            return false;
        }
        child.wait().map(|s| s.success()).unwrap_or(false)
    }
}
