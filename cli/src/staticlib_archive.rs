//! Locate Rust `staticlib` crate outputs (Unix `lib*.a` vs Windows MSVC `*.lib`).

use std::path::{Path, PathBuf};

/// Installed basename under `stdlib/prebuilt/<triple>/`.
pub fn prebuilt_basename(crate_lib_name: &str) -> String {
    if cfg!(windows) {
        format!("{crate_lib_name}.lib")
    } else {
        format!("lib{crate_lib_name}.a")
    }
}

fn cargo_output_basenames(crate_lib_name: &str) -> [String; 2] {
    [
        format!("lib{crate_lib_name}.a"),
        format!("{crate_lib_name}.lib"),
    ]
}

pub fn find_in_dir(dir: &Path, crate_lib_name: &str) -> Option<PathBuf> {
    for name in cargo_output_basenames(crate_lib_name) {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub fn find_cargo_build(
    root: &Path,
    spec_triple: &str,
    host_triple: &str,
    crate_lib_name: &str,
) -> Option<PathBuf> {
    let profiles = ["release", "debug"];
    let dirs: Vec<PathBuf> = if spec_triple == host_triple {
        profiles
            .iter()
            .map(|profile| root.join("target").join(profile))
            .collect()
    } else {
        profiles
            .iter()
            .map(|profile| root.join("target").join(spec_triple).join(profile))
            .collect()
    };
    for dir in dirs {
        if let Some(found) = find_in_dir(&dir, crate_lib_name) {
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
            assert_eq!(prebuilt_basename("nyra_rt_tls"), "libnyra_rt_tls.a");
        }
    }

    #[test]
    fn prebuilt_basename_msvc_style_on_windows() {
        if cfg!(windows) {
            assert_eq!(prebuilt_basename("nyra_rt_tls"), "nyra_rt_tls.lib");
        }
    }
}
