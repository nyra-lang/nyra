mod lockfile;
mod rust_bridge;
mod semver;

pub use semver::{best_match, parse_req, parse_version, satisfies, Req, Version};

pub use lockfile::{
    cache_module_path, sha256_hex, LockEntry, LockFile, LockSource,
};
pub use rust_bridge::{
    bind_rust_crate, bind_rust_crate_with_options, build_link_crate,
    known_bridge, parse_rust_module, rust_cache_dir, BindOptions, BridgeMeta,
};

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct RequireEntry {
    pub name: String,
    pub version_req: Option<Req>,
}

#[derive(Debug, Default)]
pub struct NyraMod {
    pub module: String,
    pub version: Option<String>,
    pub requires: Vec<RequireEntry>,
    pub link_libs: Vec<String>,
    pub link_search_paths: Vec<String>,
    pub link_args: Vec<String>,
    pub link_sources: Vec<String>,
    /// Rust crate bridges (`link-crate uuid`).
    pub link_crates: Vec<String>,
    /// Args for the instrumented binary during `nyra build --pgo` (`pgo-run ...` lines).
    pub pgo_run_args: Vec<String>,
}

pub fn parse_nyra_mod(path: &Path) -> Result<NyraMod, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut module = String::new();
    let mut version = None;
    let mut requires = Vec::new();
    let mut link_libs = Vec::new();
    let mut link_search_paths = Vec::new();
    let mut link_args = Vec::new();
    let mut link_sources = Vec::new();
    let mut link_crates = Vec::new();
    let mut pgo_run_args = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("module ") {
            module = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("version ") {
            version = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("require ") {
            let req = rest.trim();
            if req == "(" || req == ")" || req.is_empty() {
                continue;
            }
            let (name, ver_spec) = req.split_once(char::is_whitespace).unwrap_or((req, ""));
            let name = name.trim();
            if name.is_empty() || name == "(" || name == ")" {
                continue;
            }
            let version_req = if ver_spec.trim().is_empty() {
                None
            } else {
                Some(parse_req(ver_spec.trim())?)
            };
            requires.push(RequireEntry {
                name: name.to_string(),
                version_req,
            });
        } else if let Some(rest) = line.strip_prefix("link-crate ") {
            let name = rest.trim();
            if !name.is_empty() && !link_crates.contains(&name.to_string()) {
                link_crates.push(name.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("link-source ") {
            let src = rest.trim();
            if !src.is_empty() {
                link_sources.push(src.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("link ") {
            let spec = rest.trim();
            if let Some(path) = spec.strip_prefix("-L") {
                link_search_paths.push(path.trim().to_string());
            } else if let Some(lib) = spec.strip_prefix("-l") {
                link_libs.push(lib.trim().to_string());
            } else if spec.starts_with('-') {
                link_args.push(spec.to_string());
            } else if !spec.is_empty() {
                link_libs.push(spec.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("link-arg ") {
            link_args.push(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("pgo-run ") {
            let args = rest.trim();
            if !args.is_empty() {
                pgo_run_args.extend(args.split_whitespace().map(str::to_string));
            }
        }
    }
    Ok(NyraMod {
        module,
        version,
        requires,
        link_libs,
        link_search_paths,
        link_args,
        link_sources,
        link_crates,
        pgo_run_args,
    })
}

/// Merge manifest link settings with CLI overrides (CLI wins on duplicates).
pub fn native_link_from_mod(path: &Path) -> Result<NyraMod, String> {
    parse_nyra_mod(path)
}

/// Collect native link settings from the project manifest and installed packages.
pub fn resolve_project_native_link(project_root: &Path) -> Result<NyraMod, String> {
    let mod_path = project_root.join("nyra.mod");
    let mut merged = if mod_path.is_file() {
        parse_nyra_mod(&mod_path)?
    } else {
        NyraMod::default()
    };

    let lock_path = project_root.join("nyra.lock");
    if lock_path.exists() {
        let lock = LockFile::read(&lock_path)?;
        for entry in &lock.require {
            let pkg_dir = project_root.join(cache_module_path(&entry.module));
            let pkg_mod = pkg_dir.join("nyra.mod");
            if pkg_mod.is_file() {
                let pkg = parse_nyra_mod(&pkg_mod)?;
                merge_link_fields(&mut merged, &pkg);
                for src in &pkg.link_sources {
                    let resolved = if Path::new(src).is_absolute() {
                        PathBuf::from(src)
                    } else {
                        pkg_dir.join(src)
                    };
                    if resolved.is_file() {
                        let s = resolved.display().to_string();
                        if !merged.link_sources.contains(&s) {
                            merged.link_sources.push(s);
                        }
                    }
                }
            }
        }
    }

    let mut normalized_sources = Vec::new();
    for src in &merged.link_sources {
        let path = if Path::new(src).is_absolute() {
            PathBuf::from(src)
        } else {
            project_root.join(src)
        };
        if !path.is_file() {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        let key = canonical.display().to_string();
        if !normalized_sources.iter().any(|s: &String| {
            Path::new(s)
                .canonicalize()
                .map(|c| c.display().to_string() == key)
                .unwrap_or(false)
                || s == &key
        }) {
            normalized_sources.push(key);
        }
    }
    merged.link_sources = normalized_sources;

    Ok(merged)
}

fn merge_link_fields(dst: &mut NyraMod, src: &NyraMod) {
    for lib in &src.link_libs {
        if !dst.link_libs.contains(lib) {
            dst.link_libs.push(lib.clone());
        }
    }
    for path in &src.link_search_paths {
        if !dst.link_search_paths.contains(path) {
            dst.link_search_paths.push(path.clone());
        }
    }
    for arg in &src.link_args {
        if !dst.link_args.contains(arg) {
            dst.link_args.push(arg.clone());
        }
    }
    for crate_name in &src.link_crates {
        if !dst.link_crates.contains(crate_name) {
            dst.link_crates.push(crate_name.clone());
        }
    }
}

pub fn verify_project(root: &Path) -> Result<(), String> {
    verify_lock_versions(root)?;
    let mod_path = root.join("nyra.mod");
    let lock_path = root.join("nyra.lock");
    let sum_path = root.join("nyra.sum");
    if lock_path.exists() && mod_path.is_file() {
        verify_requirements_match_lock(&mod_path, &lock_path)?;
    }
    if !lock_path.exists() {
        return Ok(());
    }
    let lock = LockFile::read(&lock_path)?;
    if sum_path.exists() {
        lock.verify_sum(&sum_path)?;
    }
    Ok(())
}

fn verify_requirements_match_lock(mod_path: &Path, lock_path: &Path) -> Result<(), String> {
    let nyra_mod = parse_nyra_mod(mod_path)?;
    let lock = LockFile::read(lock_path)?;
    for req in &nyra_mod.requires {
        let entry = lock
            .require
            .iter()
            .find(|e| e.module == req.name)
            .ok_or_else(|| format!("missing lock entry for require '{}'", req.name))?;
        let pinned = parse_version(&entry.version)?;
        if let Some(version_req) = &req.version_req {
            if !satisfies(version_req, &pinned) {
                return Err(format!(
                    "lock pins {} {} but nyra.mod requires {}",
                    req.name, entry.version, format_req(version_req)
                ));
            }
        }
    }
    Ok(())
}

fn format_req(req: &Req) -> String {
    match req {
        Req::Exact(v) => format!("{}.{}.{}", v.major, v.minor, v.patch),
        Req::Caret(v) => format!("^{}.{}.{}", v.major, v.minor, v.patch),
        Req::Tilde(v) => format!("~{}.{}.{}", v.major, v.minor, v.patch),
        Req::Gte(v) => format!(">={}.{}.{}", v.major, v.minor, v.patch),
    }
}

fn verify_lock_versions(root: &Path) -> Result<(), String> {
    let lock_path = root.join("nyra.lock");
    if !lock_path.exists() {
        return Ok(());
    }
    let lock = LockFile::read(&lock_path)?;
    let mut seen = HashSet::new();
    for entry in &lock.require {
        parse_version(&entry.version)?;
        if !seen.insert(entry.module.clone()) {
            return Err(format!("duplicate lock entry for '{}'", entry.module));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_roundtrip() {
        let mut lock = LockFile::new("app.example");
        lock.require.push(LockEntry {
            module: "dep.one".into(),
            version: "1.0.0".into(),
            source: LockSource::Local,
            checksum: sha256_hex(b"dep"),
        });
        let dir = std::env::temp_dir().join("nyrapkg_test_lock");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let lock_path = dir.join("nyra.lock");
        let sum_path = dir.join("nyra.sum");
        lock.write(&lock_path).unwrap();
        lock.write_sum(&sum_path).unwrap();
        let read = LockFile::read(&lock_path).unwrap();
        read.verify_sum(&sum_path).unwrap();
    }

    #[test]
    fn parse_link_lines() {
        let dir = std::env::temp_dir().join(format!("nyra_link_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nyra.mod"),
            "module test.local\nversion 1.0.0\nrequire ny-sqlite ^1.0.0\nrequire rust::uuid ^1.0.0\nlink sqlite3\nlink -L /opt/lib\nlink-arg -Wl,-rpath,/opt/lib\nlink-source rt/foo.c\nlink-crate uuid\n",
        )
        .unwrap();
        let m = parse_nyra_mod(&dir.join("nyra.mod")).unwrap();
        assert_eq!(m.version.as_deref(), Some("1.0.0"));
        assert_eq!(m.requires.len(), 2);
        assert_eq!(m.requires[0].name, "ny-sqlite");
        assert_eq!(m.requires[1].name, "rust::uuid");
        assert_eq!(m.link_libs, vec!["sqlite3"]);
        assert_eq!(m.link_search_paths, vec!["/opt/lib"]);
        assert_eq!(m.link_sources, vec!["rt/foo.c"]);
        assert_eq!(m.link_crates, vec!["uuid"]);
        assert!(m.link_args.iter().any(|a| a.contains("rpath")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_pgo_run_line() {
        let dir = std::env::temp_dir().join(format!("nyra_pgo_run_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("nyra.mod"),
            "module bench\npgo-run --benchmark-mode --quick\n",
        )
        .unwrap();
        let m = parse_nyra_mod(&dir.join("nyra.mod")).unwrap();
        assert_eq!(
            m.pgo_run_args,
            vec!["--benchmark-mode".to_string(), "--quick".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
