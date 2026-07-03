//! Per-crate (per-source-file) incremental manifest — Cargo-style dirty tracking.
//!
//! Nyra merges imports into one program before codegen, so unchanged files still
//! require a full pipeline today. This module tracks **which crates changed** and
//! stores per-unit artifact paths for split compilation (IR / object cache).

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use resolve::collect_source_files;

use crate::cache::entry_cache_dir;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CrateUnit {
    pub path: String,
    pub content_hash: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct CrateManifest {
    pub units: Vec<CrateUnit>,
}

impl CrateManifest {
    pub fn scan(entry: &Path) -> Result<Self, String> {
        let files = if entry.is_dir() {
            let main = resolve::paths::find_main_entry(entry)
                .ok_or_else(|| format!("no main.ny in {}", entry.display()))?;
            collect_source_files(&main)?
        } else {
            collect_source_files(entry)?
        };
        let mut units = Vec::new();
        for path in files {
            let content_hash = hash_file(&path)?;
            units.push(CrateUnit {
                path: path
                    .canonicalize()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned(),
                content_hash,
            });
        }
        units.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self { units })
    }

    pub fn dirty_since(&self, previous: &Self) -> Vec<String> {
        let mut prev: std::collections::HashMap<&str, u64> = previous
            .units
            .iter()
            .map(|u| (u.path.as_str(), u.content_hash))
            .collect();
        let mut dirty = Vec::new();
        for unit in &self.units {
            match prev.remove(unit.path.as_str()) {
                Some(h) if h == unit.content_hash => {}
                _ => dirty.push(unit.path.clone()),
            }
        }
        for path in prev.keys() {
            dirty.push((*path).to_string());
        }
        dirty.sort();
        dirty.dedup();
        dirty
    }

    pub fn combined_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for u in &self.units {
            u.path.hash(&mut hasher);
            u.content_hash.hash(&mut hasher);
        }
        hasher.finish()
    }
}

pub fn manifest_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id).join("crates/manifest.json")
}

pub fn load_manifest(profile_dir: &Path, entry_id: &str) -> Option<CrateManifest> {
    let text = fs::read_to_string(manifest_path(profile_dir, entry_id)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_manifest(
    profile_dir: &Path,
    entry_id: &str,
    manifest: &CrateManifest,
) -> Result<(), String> {
    let path = manifest_path(profile_dir, entry_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Per-entry directory for cached per-crate LLVM IR (split compilation).
pub fn unit_cache_dir(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id)
        .join("crates")
        .join("units")
}

pub fn unit_ir_path(profile_dir: &Path, entry_id: &str, content_hash: u64) -> PathBuf {
    unit_cache_dir(profile_dir, entry_id).join(format!("{content_hash:016x}.ll"))
}

pub fn unit_object_path(profile_dir: &Path, entry_id: &str, content_hash: u64) -> PathBuf {
    unit_cache_dir(profile_dir, entry_id).join(format!("{content_hash:016x}.o"))
}

pub fn load_unit_ir(profile_dir: &Path, entry_id: &str, content_hash: u64) -> Option<String> {
    fs::read_to_string(unit_ir_path(profile_dir, entry_id, content_hash)).ok()
}

pub fn save_unit_ir(
    profile_dir: &Path,
    entry_id: &str,
    content_hash: u64,
    ir: &str,
) -> Result<(), String> {
    let path = unit_ir_path(profile_dir, entry_id, content_hash);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, ir).map_err(|e| e.to_string())
}

fn hash_file(path: &Path) -> Result<u64, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detects_dirty_crate() {
        let tmp = std::env::temp_dir().join(format!("nyra_crate_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let main = tmp.join("main.ny");
        fs::File::create(&main)
            .unwrap()
            .write_all(b"fn main() {}\n")
            .unwrap();
        let a = CrateManifest::scan(&main).unwrap();
        fs::write(&main, "fn main() { print(1) }\n").unwrap();
        let b = CrateManifest::scan(&main).unwrap();
        assert!(!a.dirty_since(&b).is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }
}
