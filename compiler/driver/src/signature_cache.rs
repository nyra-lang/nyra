//! Per-file public API fingerprint for incremental rebuilds.
//!
//! **Typecheck is never skipped** based on signature stability: function bodies
//! carry type errors, `unsafe` gates, and borrow-relevant changes. Skipping
//! typecheck after body-only edits let `nyra run` accept programs that
//! `nyra check` correctly rejected (e.g. raw pointer ops outside `unsafe`).
//! Speed for edit→run remains via IR unit caching / `can_skip_codegen`.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use ast::{Program, TypeAnnotation};
use resolve::parse_file_only;

use crate::cache::entry_cache_dir;
use crate::CrateManifest;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SignatureManifest {
    pub units: Vec<SignatureUnit>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SignatureUnit {
    pub path: String,
    pub content_hash: u64,
    pub api_hash: u64,
}

fn signatures_path(profile_dir: &Path, entry_id: &str) -> PathBuf {
    entry_cache_dir(profile_dir, entry_id)
        .join("crates")
        .join("signatures.json")
}

pub fn load_signatures(profile_dir: &Path, entry_id: &str) -> Option<SignatureManifest> {
    let text = fs::read_to_string(signatures_path(profile_dir, entry_id)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_signatures(
    profile_dir: &Path,
    entry_id: &str,
    manifest: &SignatureManifest,
) -> Result<(), String> {
    let path = signatures_path(profile_dir, entry_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn build_signature_manifest(manifest: &CrateManifest) -> Result<SignatureManifest, String> {
    let mut units = Vec::with_capacity(manifest.units.len());
    for unit in &manifest.units {
        let path = PathBuf::from(&unit.path);
        units.push(SignatureUnit {
            path: unit.path.clone(),
            content_hash: unit.content_hash,
            api_hash: compute_file_api_hash(&path)?,
        });
    }
    Ok(SignatureManifest { units })
}

fn type_ann_key(ann: &TypeAnnotation) -> String {
    format!("{ann:?}")
}

fn hash_program_api(program: &Program, hasher: &mut DefaultHasher) {
    let mut fns: Vec<_> = program.functions.iter().collect();
    fns.sort_by(|a, b| a.name.cmp(&b.name));
    for f in fns {
        f.name.hash(hasher);
        f.is_async.hash(hasher);
        f.public.hash(hasher);
        for p in &f.params {
            p.name.hash(hasher);
            type_ann_key(&p.ty).hash(hasher);
        }
        if let Some(ref rt) = f.return_type {
            type_ann_key(rt).hash(hasher);
        }
        for tp in &f.type_params {
            tp.hash(hasher);
        }
    }
    let mut structs: Vec<_> = program.structs.iter().collect();
    structs.sort_by(|a, b| a.name.cmp(&b.name));
    for s in structs {
        s.name.hash(hasher);
        s.public.hash(hasher);
        for field in &s.fields {
            field.name.hash(hasher);
            type_ann_key(&field.ty).hash(hasher);
        }
    }
    let mut enums: Vec<_> = program.enums.iter().collect();
    enums.sort_by(|a, b| a.name.cmp(&b.name));
    for e in enums {
        e.name.hash(hasher);
        e.public.hash(hasher);
        for v in &e.variants {
            v.name.hash(hasher);
        }
    }
    for c in &program.consts {
        c.name.hash(hasher);
        c.public.hash(hasher);
        if let Some(ref ty) = c.ty {
            type_ann_key(ty).hash(hasher);
        }
    }
    for imp in &program.impls {
        imp.type_name.hash(hasher);
        for m in &imp.methods {
            if m.public {
                m.name.hash(hasher);
            }
        }
    }
}

pub fn compute_file_api_hash(path: &Path) -> Result<u64, String> {
    let program = parse_file_only(path)?;
    let mut hasher = DefaultHasher::new();
    hash_program_api(&program, &mut hasher);
    Ok(hasher.finish())
}

pub fn file_has_async(path: &Path) -> Result<bool, String> {
    let program = parse_file_only(path)?;
    Ok(program.functions.iter().any(|f| f.is_async))
}

/// Whether dirty files may skip whole-program typecheck.
///
/// Always `false`: API-stable body edits can still introduce type/`unsafe` errors.
/// Callers keep incremental IR caching separately.
pub fn can_skip_typecheck_for_dirty(
    dirty_paths: &[String],
    previous: &SignatureManifest,
) -> Result<bool, String> {
    let _ = (dirty_paths, previous);
    Ok(false)
}

/// True when every dirty file changed only its body (public API hash unchanged,
/// no async edits). For diagnostics/tests; not used to skip typecheck.
pub fn dirty_files_are_body_only(
    dirty_paths: &[String],
    previous: &SignatureManifest,
) -> Result<bool, String> {
    if dirty_paths.is_empty() {
        return Ok(false);
    }
    let prev: HashMap<&str, &SignatureUnit> = previous
        .units
        .iter()
        .map(|u| (u.path.as_str(), u))
        .collect();
    for path in dirty_paths {
        let unit_path = PathBuf::from(path);
        let Some(prev_unit) = prev.get(path.as_str()) else {
            return Ok(false);
        };
        let current_api = compute_file_api_hash(&unit_path)?;
        if current_api != prev_unit.api_hash {
            return Ok(false);
        }
        if file_has_async(&unit_path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn api_hash_ignores_body_change() {
        let tmp = std::env::temp_dir().join(format!("nyra_sig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("m.ny");
        fs::File::create(&f)
            .unwrap()
            .write_all(b"fn main() { print(1) }\n")
            .unwrap();
        let a = compute_file_api_hash(&f).unwrap();
        fs::write(&f, "fn main() { print(2) }\n").unwrap();
        let b = compute_file_api_hash(&f).unwrap();
        assert_eq!(a, b);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn never_skip_typecheck_even_for_body_only_edits() {
        let tmp = std::env::temp_dir().join(format!("nyra_sig_skip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("main.ny");
        fs::write(
            &f,
            "fn main() {\n    mut x = 42\n    unsafe {\n        let p = &x as *i32\n        *p = 99\n        print(*p)\n    }\n}\n",
        )
        .unwrap();
        let api = compute_file_api_hash(&f).unwrap();
        let path = f.to_string_lossy().into_owned();
        let previous = SignatureManifest {
            units: vec![SignatureUnit {
                path: path.clone(),
                content_hash: 1,
                api_hash: api,
            }],
        };
        // Body-only: drop unsafe — API hash unchanged, but must still typecheck.
        fs::write(
            &f,
            "fn main() {\n    mut x = 42\n    let p = &x as *i32\n    *p = 99\n    print(*p)\n}\n",
        )
        .unwrap();
        assert_eq!(compute_file_api_hash(&f).unwrap(), api);
        assert!(dirty_files_are_body_only(&[path.clone()], &previous).unwrap());
        assert!(
            !can_skip_typecheck_for_dirty(&[path], &previous).unwrap(),
            "skipping typecheck after body-only edits is unsound"
        );
        let _ = fs::remove_dir_all(&tmp);
    }
}
