//! Incremental build fingerprint cache (source + link profiles).

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use resolve::collect_source_files;
use codegen::RuntimeProfile;

pub struct BuildFingerprint {
    pub hash: u64,
    pub source_count: usize,
}

pub fn compute_fingerprint(entry: &Path, options_key: &str) -> Result<BuildFingerprint, String> {
    let files = source_files_for_entry(entry)?;
    hash_files(&files, options_key)
}

pub fn compute_source_fingerprint(entry: &Path) -> Result<BuildFingerprint, String> {
    let files = source_files_for_entry(entry)?;
    hash_files(&files, "source")
}

fn source_files_for_entry(entry: &Path) -> Result<Vec<PathBuf>, String> {
    if entry.is_dir() {
        let main = resolve::paths::find_main_entry(entry)
            .ok_or_else(|| format!("no main.ny in {}", entry.display()))?;
        collect_source_files(&main)
    } else {
        collect_source_files(entry)
    }
}

fn hash_files(files: &[PathBuf], key: &str) -> Result<BuildFingerprint, String> {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    // Invalidate incremental cache when the compiler version changes (codegen fixes).
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    for path in files {
        path.hash(&mut hasher);
        if let Ok(bytes) = fs::read(path) {
            bytes.hash(&mut hasher);
        }
    }
    Ok(BuildFingerprint {
        hash: hasher.finish(),
        source_count: files.len(),
    })
}

pub fn cache_dir(profile_dir: &Path) -> PathBuf {
    profile_dir.join(".nyra-cache")
}

pub fn entry_cache_dir(profile_dir: &Path, entry_id: &str) -> PathBuf {
    cache_dir(profile_dir).join("entries").join(entry_id)
}

pub fn source_fingerprint_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id).join("source.fingerprint")
}

pub fn link_fingerprint_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id).join("link.fingerprint")
}

pub fn cache_key_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id).join("build.fingerprint")
}

fn read_fp(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn binary_looks_runnable(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.len() < 64 {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn runtime_cache_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id).join("runtime.symbols")
}

pub fn write_runtime_cache(
    profile_dir: &Path,
    entry_id: &str,
    profile: &RuntimeProfile,
) -> Result<(), String> {
    let dir = entry_cache_dir(profile_dir, entry_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let body = profile.symbols.iter().map(String::as_str).collect::<Vec<_>>().join("\n");
    fs::write(runtime_cache_path(profile_dir, entry_id), body).map_err(|e| e.to_string())
}

pub fn read_runtime_cache(profile_dir: &Path, entry_id: &str) -> Option<RuntimeProfile> {
    let text = fs::read_to_string(runtime_cache_path(profile_dir, entry_id)).ok()?;
    let symbols = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    Some(RuntimeProfile { symbols })
}

pub fn mix_crate_manifest(fp: BuildFingerprint, crate_hash: u64) -> BuildFingerprint {
    let mut hasher = DefaultHasher::new();
    fp.hash.hash(&mut hasher);
    crate_hash.hash(&mut hasher);
    BuildFingerprint {
        hash: hasher.finish(),
        source_count: fp.source_count,
    }
}

pub fn write_cached_fingerprint(
    profile_dir: &Path,
    entry_id: &str,
    source: u64,
    link: u64,
) -> Result<(), String> {
    let dir = entry_cache_dir(profile_dir, entry_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(source_fingerprint_path(profile_dir, entry_id), source.to_string())
        .map_err(|e| e.to_string())?;
    fs::write(link_fingerprint_path(profile_dir, entry_id), link.to_string())
        .map_err(|e| e.to_string())?;
    let combined = {
        let mut h = DefaultHasher::new();
        source.hash(&mut h);
        link.hash(&mut h);
        h.finish()
    };
    fs::write(cache_key_path(profile_dir, entry_id), combined.to_string())
        .map_err(|e| e.to_string())
}

/// Sources and compile/link options unchanged and LLVM IR present — skip codegen.
pub fn can_skip_codegen(
    profile_dir: &Path,
    entry_id: &str,
    ll_path: &Path,
    source: &BuildFingerprint,
    link_hash: u64,
) -> bool {
    read_fp(&source_fingerprint_path(profile_dir, entry_id)) == Some(source.hash)
        && read_fp(&link_fingerprint_path(profile_dir, entry_id)) == Some(link_hash)
        && ll_path.exists()
}

/// Full hit: sources + link profile unchanged and binary exists.
pub fn is_incremental_hit(
    profile_dir: &Path,
    entry_id: &str,
    ll_path: &Path,
    bin_path: &Path,
    source: &BuildFingerprint,
    link_hash: u64,
) -> bool {
    if read_fp(&source_fingerprint_path(profile_dir, entry_id)) != Some(source.hash) {
        return false;
    }
    if read_fp(&link_fingerprint_path(profile_dir, entry_id)) != Some(link_hash) {
        return false;
    }
    ll_path.exists() && bin_path.exists() && binary_looks_runnable(bin_path)
}

pub fn options_cache_key(
    target: &str,
    release: bool,
    no_std: bool,
    freestanding: bool,
    deny_extended: bool,
    no_prelude: bool,
) -> String {
    format!(
        "t={target}|r={release}|ns={no_std}|fs={freestanding}|de={deny_extended}|np={no_prelude}"
    )
}

pub fn link_cache_key(
    base: &str,
    debug_symbols: bool,
    cdylib: bool,
    link_libs: &[String],
    link_args: &[String],
    link_sources: &[PathBuf],
    tls_backend: &str,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    base.hash(&mut hasher);
    debug_symbols.hash(&mut hasher);
    cdylib.hash(&mut hasher);
    if cdylib {
        // Bumped when macOS cdylib link metadata changes (e.g. @rpath install name).
        "macos_cdylib_id@rpath".hash(&mut hasher);
    }
    for lib in link_libs {
        lib.hash(&mut hasher);
    }
    for arg in link_args {
        arg.hash(&mut hasher);
    }
    for src in link_sources {
        src.hash(&mut hasher);
        if let Ok(bytes) = fs::read(src) {
            bytes.hash(&mut hasher);
        }
    }
    tls_backend.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn fingerprint_includes_compiler_version_key() {
        let tmp = std::env::temp_dir().join(format!("nyra_fp_ver_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let main = tmp.join("main.ny");
        fs::write(&main, "fn main() { print(1) }\n").unwrap();
        let a = compute_source_fingerprint(&main).unwrap();
        let b = compute_fingerprint(&main, "other-key").unwrap();
        // Version is hashed for all fingerprint keys; same sources differ only by key suffix.
        assert_ne!(a.hash, b.hash);
        let c = compute_source_fingerprint(&main).unwrap();
        assert_eq!(a.hash, c.hash);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn fingerprint_changes_when_source_changes() {
        let tmp = std::env::temp_dir().join(format!("nyra_fp_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let main = tmp.join("main.ny");
        fs::File::create(&main)
            .unwrap()
            .write_all(b"fn main() { print(1) }\n")
            .unwrap();
        let a = compute_source_fingerprint(&main).unwrap();
        fs::write(&main, "fn main() { print(2) }\n").unwrap();
        let b = compute_source_fingerprint(&main).unwrap();
        assert_ne!(a.hash, b.hash);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn codegen_skip_requires_link_hash() {
        let tmp = std::env::temp_dir().join(format!("nyra_codegen_skip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let entry = "bench";
        let ll = tmp.join("bench.ll");
        fs::write(&ll, "define i32 @main() { ret i32 0 }").unwrap();
        let source = BuildFingerprint {
            hash: 42,
            source_count: 1,
        };
        write_cached_fingerprint(&tmp, entry, source.hash, 100).unwrap();
        assert!(can_skip_codegen(&tmp, entry, &ll, &source, 100));
        assert!(!can_skip_codegen(&tmp, entry, &ll, &source, 101));
        let _ = fs::remove_dir_all(&tmp);
    }
}
