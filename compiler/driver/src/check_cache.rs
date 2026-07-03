//! Incremental cache for `nyra check` — skip the full frontend when sources and
//! check options are unchanged since the last successful check.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::cache::entry_cache_dir;

pub fn check_cache_key(deny_extended: bool, deny_warnings: bool) -> u64 {
    let mut hasher = DefaultHasher::new();
    "check".hash(&mut hasher);
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    deny_extended.hash(&mut hasher);
    deny_warnings.hash(&mut hasher);
    hasher.finish()
}

fn check_cache_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id).join("check.fingerprint")
}

fn read_check_fp(path: &Path) -> Option<(u64, u64)> {
    let text = fs::read_to_string(path).ok()?;
    let mut parts = text.split_whitespace();
    let source = parts.next()?.parse().ok()?;
    let check = parts.next()?.parse().ok()?;
    Some((source, check))
}

/// Last successful check matches current sources and check flags.
pub fn is_check_cache_hit(
    profile_dir: &Path,
    entry_id: &str,
    source_hash: u64,
    check_key: u64,
) -> bool {
    read_check_fp(&check_cache_path(profile_dir, entry_id))
        == Some((source_hash, check_key))
}

pub fn write_check_cache(
    profile_dir: &Path,
    entry_id: &str,
    source_hash: u64,
    check_key: u64,
) -> Result<(), String> {
    let path = check_cache_path(profile_dir, entry_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, format!("{source_hash} {check_key}")).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_cache_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("nyra_check_cache_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let entry = "main";
        assert!(!is_check_cache_hit(&tmp, entry, 1, 2));
        write_check_cache(&tmp, entry, 1, 2).unwrap();
        assert!(is_check_cache_hit(&tmp, entry, 1, 2));
        assert!(!is_check_cache_hit(&tmp, entry, 2, 2));
        let _ = fs::remove_dir_all(&tmp);
    }
}
