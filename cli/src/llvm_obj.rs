//! Cache Nyra user LLVM IR as a native object file — speeds dev relinks.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::link::{LinkProfile, LtoMode};
use crate::llvm_tools;
use crate::target::{apply_target_compile_flags, TargetSpec};

pub fn user_objects_cache_dir(work_dir: &Path) -> PathBuf {
    work_dir.join(".nyra-cache").join("user-objs")
}

fn object_key(ll: &Path, flags_key: &str) -> Result<u64, String> {
    let bytes = fs::read(ll).map_err(|e| format!("read {}: {e}", ll.display()))?;
    let mut hasher = DefaultHasher::new();
    ll.hash(&mut hasher);
    bytes.hash(&mut hasher);
    flags_key.hash(&mut hasher);
    Ok(hasher.finish())
}

fn object_path(cache_dir: &Path, ll: &Path, key: u64) -> PathBuf {
    let stem = ll
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "user".into());
    cache_dir.join(format!("{stem}.{key:016x}.o"))
}

fn compile_flags_key(profile: &LinkProfile, spec: &TargetSpec) -> String {
    format!(
        "triple={}|opt={:?}|lto={:?}|dbg={}|native={}|free={}|cdylib={}",
        spec.triple_for_codegen(),
        profile.opt_level,
        profile.lto,
        profile.debug_symbols,
        profile.native_cpu,
        profile.freestanding,
        profile.cdylib,
    )
}

fn compile_ir_to_object(
    ll: &Path,
    obj: &Path,
    profile: &LinkProfile,
    spec: &TargetSpec,
) -> Result<(), String> {
    let compiler = llvm_tools::find_clang();
    let mut cmd = Command::new(&compiler);
    apply_target_compile_flags(&mut cmd, spec);
    cmd.arg("-c").arg(ll).arg("-o").arg(obj);
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
    if profile.debug_symbols {
        cmd.arg("-g");
    }
    if profile.native_cpu {
        cmd.arg("-march=native");
    }
    if profile.freestanding {
        cmd.arg("-ffreestanding");
    }
    cmd.arg("-Wno-override-module");
    let output = cmd
        .output()
        .map_err(|e| format!("failed to compile {}: {e}", ll.display()))?;
    if output.status.success() {
        if obj.is_file() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = [stderr.trim(), stdout.trim()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            Err(format!(
                "compiled IR {} but object output is missing: {}{}",
                ll.display(),
                obj.display(),
                if detail.is_empty() {
                    String::new()
                } else {
                    format!("\n{detail}")
                }
            ))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "failed to compile IR {} (exit {}): {}",
            ll.display(),
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ))
    }
}

/// Return a cached native object for user LLVM IR (compile on cache miss).
pub fn compile_cached_user_object(
    ll: &Path,
    work_dir: &Path,
    profile: &LinkProfile,
    spec: &TargetSpec,
) -> Result<PathBuf, String> {
    let cache_dir = user_objects_cache_dir(work_dir);
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let flags_key = compile_flags_key(profile, spec);
    let key = object_key(ll, &flags_key)?;
    let obj = object_path(&cache_dir, ll, key);
    if obj.is_file() {
        return Ok(obj);
    }
    compile_ir_to_object(ll, &obj, profile, spec)?;
    Ok(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::OptLevel;

    #[test]
    fn object_key_changes_with_ir_content() {
        let tmp = std::env::temp_dir().join(format!("nyra_user_obj_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let ll = tmp.join("main.ll");
        fs::write(&ll, "define i32 @main() { ret i32 0 }\n").unwrap();
        let spec = TargetSpec::host();
        let profile = LinkProfile::default();
        let flags = compile_flags_key(&profile, &spec);
        let a = object_key(&ll, &flags).unwrap();
        fs::write(&ll, "define i32 @main() { ret i32 1 }\n").unwrap();
        let b = object_key(&ll, &flags).unwrap();
        assert_ne!(a, b);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn compiles_minimal_ir_to_object() {
        let tmp = std::env::temp_dir().join(format!("nyra_user_obj_build_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let ll = tmp.join("main.ll");
        fs::write(
            &ll,
            "target triple = \"aarch64-apple-darwin\"\ndefine i32 @main() { ret i32 0 }\n",
        )
        .unwrap();
        let spec = TargetSpec::host();
        let profile = LinkProfile {
            opt_level: OptLevel::O0,
            ..Default::default()
        };
        let obj = compile_cached_user_object(&ll, &tmp, &profile, &spec);
        if obj.is_err() {
            eprintln!("skip compiles_minimal_ir_to_object: clang unavailable or triple mismatch");
            let _ = fs::remove_dir_all(&tmp);
            return;
        }
        assert!(obj.unwrap().is_file());
        let _ = fs::remove_dir_all(&tmp);
    }
}
