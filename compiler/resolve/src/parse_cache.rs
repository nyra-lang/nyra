//! In-process parse cache keyed by `(canonical path, content hash)`.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ast::Program;

static CACHE: Mutex<Option<ParseCache>> = Mutex::new(None);

struct ParseCache {
    entries: HashMap<(String, u64), Program>,
}

impl ParseCache {
    fn global() -> std::sync::MutexGuard<'static, Option<ParseCache>> {
        let mut guard = CACHE.lock().expect("parse cache lock");
        if guard.is_none() {
            *guard = Some(ParseCache {
                entries: HashMap::new(),
            });
        }
        guard
    }
}

pub fn content_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

pub fn hash_file(path: &Path) -> Result<u64, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    Ok(content_hash(&source))
}

pub fn get(path: &Path, hash: u64) -> Option<Program> {
    let key = path.to_string_lossy().into_owned();
    ParseCache::global()
        .as_ref()
        .and_then(|c| c.entries.get(&(key, hash)).cloned())
}

pub fn insert(path: &Path, hash: u64, program: Program) {
    let key = path.to_string_lossy().into_owned();
    if let Some(cache) = ParseCache::global().as_mut() {
        cache.entries.insert((key, hash), program);
    }
}

pub fn invalidate_path(path: &Path) {
    let key = path.to_string_lossy().into_owned();
    if let Some(cache) = ParseCache::global().as_mut() {
        cache.entries.retain(|(p, _), _| p != &key);
    }
}

pub fn clear() {
    if let Some(cache) = ParseCache::global().as_mut() {
        cache.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_stable() {
        assert_eq!(content_hash("fn main() {}"), content_hash("fn main() {}"));
        assert_ne!(content_hash("a"), content_hash("b"));
    }
}
