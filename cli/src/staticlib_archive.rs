//! Locate Rust `staticlib` crate outputs (Unix `lib*.a` vs Windows MSVC `*.lib`).
//!
//! Nyra links user programs with MinGW on Windows (`*-pc-windows-gnu`), even when
//! `rustc` reports `*-pc-windows-msvc` as the host. TLS staticlibs must therefore
//! be cross-compiled for the GNU triple and installed as `lib*.a`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Installed basename under `stdlib/prebuilt/<triple>/`.
pub fn prebuilt_basename(crate_lib_name: &str, link_triple: &str) -> String {
    if is_windows_gnu_link_triple(link_triple) {
        format!("lib{crate_lib_name}.a")
    } else if cfg!(windows) {
        format!("{crate_lib_name}.lib")
    } else {
        format!("lib{crate_lib_name}.a")
    }
}

pub fn is_windows_gnu_link_triple(triple: &str) -> bool {
    triple.contains("windows-gnu")
}

fn cargo_output_basenames(crate_lib_name: &str) -> [String; 2] {
    [
        format!("lib{crate_lib_name}.a"),
        format!("{crate_lib_name}.lib"),
    ]
}

pub fn find_in_dir(dir: &Path, crate_lib_name: &str, link_triple: &str) -> Option<PathBuf> {
    let names: Vec<String> = if is_windows_gnu_link_triple(link_triple) {
        vec![format!("lib{crate_lib_name}.a")]
    } else {
        cargo_output_basenames(crate_lib_name).into()
    };
    for name in names {
        let path = dir.join(&name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// `rustc -vV` host triple (not Nyra-normalized).
fn rustc_raw_host_triple() -> Option<String> {
    let out = Command::new("rustc").args(["-vV"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::trim).map(str::to_string))
}

/// Whether `cargo build` needs `--target <link_triple>` for this staticlib.
pub fn needs_cargo_cross_target(link_triple: &str, nyra_host_triple: &str) -> bool {
    if is_windows_gnu_link_triple(link_triple) {
        return !rustc_raw_host_triple()
            .is_some_and(|h| h.contains("windows-gnu"));
    }
    link_triple != nyra_host_triple
}

pub fn ensure_rust_target(link_triple: &str) -> Result<(), String> {
    if !needs_cargo_cross_target(link_triple, link_triple) {
        return Ok(());
    }
    let output = Command::new("rustup")
        .args(["target", "add", link_triple])
        .output()
        .map_err(|e| format!("failed to run rustup target add {link_triple}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustup target add {link_triple} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

pub fn append_cargo_target_args(cmd: &mut Command, link_triple: &str, nyra_host_triple: &str) {
    if needs_cargo_cross_target(link_triple, nyra_host_triple) {
        cmd.arg("--target").arg(link_triple);
    }
}

pub fn find_cargo_build(
    root: &Path,
    link_triple: &str,
    nyra_host_triple: &str,
    crate_lib_name: &str,
) -> Option<PathBuf> {
    let profiles = ["release", "debug"];
    let cross = needs_cargo_cross_target(link_triple, nyra_host_triple);
    let dirs: Vec<PathBuf> = if cross {
        profiles
            .iter()
            .map(|profile| root.join("target").join(link_triple).join(profile))
            .collect()
    } else {
        profiles
            .iter()
            .map(|profile| root.join("target").join(profile))
            .collect()
    };
    for dir in dirs {
        if let Some(found) = find_in_dir(&dir, crate_lib_name, link_triple) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prebuilt_basename_unix_style_on_non_windows() {
        if !cfg!(windows) {
            assert_eq!(
                prebuilt_basename("nyra_rt_tls", "x86_64-unknown-linux-gnu"),
                "libnyra_rt_tls.a"
            );
        }
    }

    #[test]
    fn prebuilt_basename_gnu_windows_uses_ar_archive() {
        assert_eq!(
            prebuilt_basename("nyra_rt_tls", "x86_64-pc-windows-gnu"),
            "libnyra_rt_tls.a"
        );
    }

    #[test]
    fn windows_gnu_always_needs_cross_from_msvc_host() {
        if cfg!(windows) {
            let host = rustc_raw_host_triple().unwrap_or_default();
            if host.contains("windows-msvc") {
                assert!(needs_cargo_cross_target(
                    "x86_64-pc-windows-gnu",
                    "x86_64-pc-windows-gnu"
                ));
            }
        }
    }
}
