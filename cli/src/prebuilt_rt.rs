//! Prebuilt dev runtime static library — compile all `stdlib/rt/*.c` once at O0 and
//! link against `libnyra_rt_dev.a` instead of recompiling runtime modules every build.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::runtime_map;

use crate::c_cache;
use crate::link::{LinkProfile, OptLevel};
use crate::llvm_tools;
use crate::target::TargetSpec;

const ARCHIVE_NAME: &str = "libnyra_rt_dev.a";
const STAMP_NAME: &str = "rt-sources.stamp";

pub fn prebuilt_rt_dir(spec: &TargetSpec) -> PathBuf {
    runtime_share_root().join("prebuilt").join(spec.triple_for_codegen())
}

pub fn prebuilt_rt_archive(spec: &TargetSpec) -> PathBuf {
    prebuilt_rt_dir(spec).join(ARCHIVE_NAME)
}

fn runtime_share_root() -> PathBuf {
    if let Some(rt) = runtime_map::stdlib_rt_dir().parent() {
        if rt.is_dir() {
            return rt.to_path_buf();
        }
    }
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".nyra/share/stdlib");
        if p.is_dir() {
            return p;
        }
    }
    runtime_map::stdlib_rt_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn rt_sources_dir() -> PathBuf {
    runtime_map::stdlib_rt_dir()
}

fn compute_rt_sources_stamp() -> Result<u64, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let rt_dir = rt_sources_dir();
    let mut entries: Vec<_> = fs::read_dir(&rt_dir)
        .map_err(|e| format!("read {}: {e}", rt_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|x| x.to_str()) == Some("c")
                && !p.to_string_lossy().contains(".inc.")
        })
        .collect();
    entries.sort();
    let mut hasher = DefaultHasher::new();
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    for path in entries {
        path.hash(&mut hasher);
        let bytes = fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        bytes.hash(&mut hasher);
    }
    Ok(hasher.finish())
}

fn stamp_path(spec: &TargetSpec) -> PathBuf {
    prebuilt_rt_dir(spec).join(STAMP_NAME)
}

pub fn is_prebuilt_stale(spec: &TargetSpec) -> bool {
    let archive = prebuilt_rt_archive(spec);
    let stamp = stamp_path(spec);
    if !archive.is_file() || !stamp.is_file() {
        return true;
    }
    let Ok(want) = compute_rt_sources_stamp() else {
        return true;
    };
    fs::read_to_string(&stamp)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        != Some(want)
}

pub fn ensure_prebuilt_runtime(spec: &TargetSpec) -> Result<PathBuf, String> {
    if !is_prebuilt_stale(spec) {
        return Ok(prebuilt_rt_archive(spec));
    }
    build_prebuilt_runtime(spec)
}

fn build_prebuilt_runtime(spec: &TargetSpec) -> Result<PathBuf, String> {
    let out_dir = prebuilt_rt_dir(spec);
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let rt_dir = rt_sources_dir();
    let mut sources: Vec<PathBuf> = fs::read_dir(&rt_dir)
        .map_err(|e| format!("read {}: {e}", rt_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|x| x.to_str()) == Some("c")
                && !p.to_string_lossy().contains(".inc.")
        })
        .collect();
    sources.sort();

    let profile = LinkProfile {
        opt_level: OptLevel::O0,
        ..Default::default()
    };
    let work = out_dir.join(".build");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;

    let objects = c_cache::compile_link_sources_best_effort(&sources, &work, &profile, spec)?;
    if objects.is_empty() {
        return Err(format!(
            "no runtime objects compiled from {}",
            rt_dir.display()
        ));
    }

    let archive = prebuilt_rt_archive(spec);
    let ar = llvm_tools::find_ar();
    let mut cmd = Command::new(&ar);
    if archive.is_file() {
        cmd.arg("rcs").arg(&archive);
    } else {
        cmd.arg("rcs").arg(&archive);
    }
    for obj in &objects {
        cmd.arg(obj);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run `{}`: {e}", ar.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "failed to build {}: {}",
            archive.display(),
            stderr.trim()
        ));
    }

    let stamp = compute_rt_sources_stamp()?;
    fs::write(stamp_path(spec), stamp.to_string()).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&work);
    Ok(archive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::LtoMode;

    #[test]
    fn dev_profile_allows_prebuilt_on_host() {
        let spec = TargetSpec::host();
        let profile = LinkProfile::default();
        assert!(profile.can_use_prebuilt_runtime(&spec));
    }

    #[test]
    fn release_profile_disallows_prebuilt() {
        let spec = TargetSpec::host();
        let profile = LinkProfile {
            opt_level: OptLevel::O3,
            lto: LtoMode::Thin,
            ..Default::default()
        };
        assert!(!profile.can_use_prebuilt_runtime(&spec));
    }
}
