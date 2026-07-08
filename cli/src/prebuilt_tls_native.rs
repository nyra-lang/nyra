//! Prebuilt OS-native TLS client static library.
//!
//! Built from `nyra-rt-tls-native` when `tls native` is selected in `nyra.mod`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::runtime_map;

use crate::staticlib_archive;
use crate::target::TargetSpec;

const CRATE_LIB_NAME: &str = "nyra_rt_tls_native";
const CARGO_PACKAGE: &str = "nyra-rt-tls-native";

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

pub fn prebuilt_native_tls_dir(spec: &TargetSpec) -> PathBuf {
    runtime_share_root()
        .join("prebuilt")
        .join(spec.triple_for_codegen())
}

pub fn prebuilt_native_tls_archive(spec: &TargetSpec) -> PathBuf {
    prebuilt_native_tls_dir(spec).join(staticlib_archive::prebuilt_basename(CRATE_LIB_NAME))
}

pub fn ensure_prebuilt_native_tls(spec: &TargetSpec) -> Result<PathBuf, String> {
    let dest = prebuilt_native_tls_archive(spec);
    if dest.is_file() {
        return Ok(dest);
    }

    if let Some(from_cargo) = find_cargo_archive(spec) {
        fs::create_dir_all(prebuilt_native_tls_dir(spec)).map_err(|e| e.to_string())?;
        fs::copy(&from_cargo, &dest).map_err(|e| {
            format!(
                "copy {} → {}: {e}",
                from_cargo.display(),
                dest.display()
            )
        })?;
        return Ok(dest);
    }

    build_and_install(spec)
}

fn workspace_root() -> Option<PathBuf> {
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        let cli = PathBuf::from(manifest);
        if let Some(root) = cli.parent() {
            if root.join("rt-tls-native/Cargo.toml").is_file() {
                return Some(root.to_path_buf());
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            if root.join("rt-tls-native/Cargo.toml").is_file() {
                return Some(root.to_path_buf());
            }
        }
    }
    None
}

fn find_cargo_archive(spec: &TargetSpec) -> Option<PathBuf> {
    let root = workspace_root()?;
    staticlib_archive::find_cargo_build(
        &root,
        &spec.triple_for_codegen(),
        &TargetSpec::host().triple_for_codegen(),
        CRATE_LIB_NAME,
    )
}

fn build_and_install(spec: &TargetSpec) -> Result<PathBuf, String> {
    let dest_name = staticlib_archive::prebuilt_basename(CRATE_LIB_NAME);
    let root = workspace_root().ok_or_else(|| {
        format!(
            "{dest_name} not found under {} and Nyra source tree unavailable — \
             reinstall Nyra or run `cargo build -p {CARGO_PACKAGE}` in the Nyra checkout",
            prebuilt_native_tls_dir(spec).display()
        )
    })?;
    let host = TargetSpec::host().triple_for_codegen();
    let triple = spec.triple_for_codegen();
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg(CARGO_PACKAGE)
        .arg("--release")
        .current_dir(&root);
    if triple != host {
        cmd.arg("--target").arg(&triple);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run cargo build -p {CARGO_PACKAGE}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo build -p {CARGO_PACKAGE} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let built = staticlib_archive::find_cargo_build(&root, &triple, &host, CRATE_LIB_NAME)
        .ok_or_else(|| {
            format!(
                "expected {dest_name} under {}/target after cargo build -p {CARGO_PACKAGE}",
                root.display()
            )
        })?;
    let dest = prebuilt_native_tls_archive(spec);
    fs::create_dir_all(prebuilt_native_tls_dir(spec)).map_err(|e| e.to_string())?;
    fs::copy(&built, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}
