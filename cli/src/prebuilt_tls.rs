//! Prebuilt rustls TLS client static library (`libnyra_rt_tls.a`).
//!
//! Built from the `nyra-rt-tls` crate and placed under
//! `stdlib/prebuilt/<triple>/` so end users need neither Rust nor OpenSSL for HTTPS.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::runtime_map;

use crate::target::TargetSpec;

const ARCHIVE_NAME: &str = "libnyra_rt_tls.a";

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

pub fn prebuilt_tls_dir(spec: &TargetSpec) -> PathBuf {
    runtime_share_root()
        .join("prebuilt")
        .join(spec.triple_for_codegen())
}

pub fn prebuilt_tls_archive(spec: &TargetSpec) -> PathBuf {
    prebuilt_tls_dir(spec).join(ARCHIVE_NAME)
}

/// Prefer shipped/prebuilt archive; otherwise build with cargo from the Nyra workspace.
pub fn ensure_prebuilt_tls(spec: &TargetSpec) -> Result<PathBuf, String> {
    let dest = prebuilt_tls_archive(spec);
    if dest.is_file() {
        return Ok(dest);
    }

    // Dev: cargo target/<profile>/libnyra_rt_tls.a next to the workspace.
    if let Some(from_cargo) = find_cargo_tls_archive(spec) {
        fs::create_dir_all(prebuilt_tls_dir(spec)).map_err(|e| e.to_string())?;
        fs::copy(&from_cargo, &dest).map_err(|e| {
            format!(
                "copy {} → {}: {e}",
                from_cargo.display(),
                dest.display()
            )
        })?;
        return Ok(dest);
    }

    build_and_install_tls(spec)
}

fn workspace_root() -> Option<PathBuf> {
    // Compile-time path to the cli crate → parent is the Nyra workspace.
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        let cli = PathBuf::from(manifest);
        if let Some(root) = cli.parent() {
            if root.join("rt-tls/Cargo.toml").is_file() {
                return Some(root.to_path_buf());
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        // target/{debug,release}/nyra → workspace root
        if let Some(root) = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            if root.join("rt-tls/Cargo.toml").is_file() {
                return Some(root.to_path_buf());
            }
        }
    }
    None
}

fn find_cargo_tls_archive(spec: &TargetSpec) -> Option<PathBuf> {
    let root = workspace_root()?;
    let triple = spec.triple_for_codegen();
    let host = TargetSpec::host().triple_for_codegen();
    let candidates = if triple == host {
        vec![
            root.join("target/release").join(ARCHIVE_NAME),
            root.join("target/debug").join(ARCHIVE_NAME),
        ]
    } else {
        vec![
            root.join("target").join(&triple).join("release").join(ARCHIVE_NAME),
            root.join("target").join(&triple).join("debug").join(ARCHIVE_NAME),
        ]
    };
    candidates.into_iter().find(|p| p.is_file())
}

fn build_and_install_tls(spec: &TargetSpec) -> Result<PathBuf, String> {
    let root = workspace_root().ok_or_else(|| {
        format!(
            "libnyra_rt_tls.a not found under {} and Nyra source tree unavailable — \
             reinstall Nyra or run `cargo build -p nyra-rt-tls` in the Nyra checkout",
            prebuilt_tls_dir(spec).display()
        )
    })?;
    let host = TargetSpec::host().triple_for_codegen();
    let triple = spec.triple_for_codegen();
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("nyra-rt-tls")
        .arg("--release")
        .current_dir(&root);
    if triple != host {
        cmd.arg("--target").arg(&triple);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run cargo build -p nyra-rt-tls: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo build -p nyra-rt-tls failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let built = if triple == host {
        root.join("target/release").join(ARCHIVE_NAME)
    } else {
        root.join("target")
            .join(&triple)
            .join("release")
            .join(ARCHIVE_NAME)
    };
    if !built.is_file() {
        return Err(format!("expected {} after cargo build", built.display()));
    }
    let dest = prebuilt_tls_archive(spec);
    fs::create_dir_all(prebuilt_tls_dir(spec)).map_err(|e| e.to_string())?;
    fs::copy(&built, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}
