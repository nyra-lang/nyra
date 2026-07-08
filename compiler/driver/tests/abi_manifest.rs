//! ABI manifest sync tests — see docs/abi-manifest.toml

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
struct AbiSymbol {
    name: String,
    c_sig: String,
    module: String,
    tier: String,
    #[allow(dead_code)]
    since: String,
}

#[derive(Debug, serde::Deserialize)]
struct AbiManifest {
    symbol: Vec<AbiSymbol>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_manifest() -> AbiManifest {
    let path = repo_root().join("docs/abi-manifest.toml");
    let text = fs::read_to_string(&path).expect("read abi-manifest.toml");
    toml::from_str(&text).expect("parse abi-manifest.toml")
}

fn rt_dir() -> PathBuf {
    repo_root().join("stdlib/rt")
}

fn symbol_source_path(module: &str) -> PathBuf {
    let root = repo_root();
    if module.starts_with("rt-tls/") || module.starts_with("rt-tls-native/") {
        root.join(module)
    } else {
        rt_dir().join(module)
    }
}

#[test]
fn manifest_symbols_exist_in_runtime_modules() {
    let manifest = load_manifest();
    for sym in &manifest.symbol {
        let path = symbol_source_path(&sym.module);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("read {}", path.display()));
        assert!(
            text.contains(&sym.name),
            "symbol {} missing in {}",
            sym.name,
            sym.module
        );
        // C: `name(` — Rust: `fn name(` or `fn name (`
        let defined = text.contains(&format!("{}(", sym.name))
            || text.contains(&format!("fn {}(", sym.name));
        assert!(
            defined,
            "definition for {} not found in {}",
            sym.name,
            sym.module
        );
    }
}

#[test]
fn runtime_map_matches_manifest() {
    let manifest = load_manifest();
    let map = codegen::runtime_map::symbol_module_map();
    let manifest_by_name: HashMap<&str, &AbiSymbol> = manifest
        .symbol
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();

    for (name, module) in &map {
        let sym = manifest_by_name
            .get(&**name)
            .unwrap_or_else(|| panic!("runtime_map symbol {name} missing from abi-manifest.toml"));
        assert_eq!(
            sym.module, *module,
            "module mismatch for {name}: manifest={} runtime_map={module}",
            sym.module
        );
    }

    for sym in &manifest.symbol {
        if sym.tier == "experimental" {
            continue;
        }
        assert!(
            map.contains_key(sym.name.as_str()),
            "stable manifest symbol {} missing from runtime_map.rs",
            sym.name
        );
    }
}

#[test]
fn nyra_rt_h_matches_manifest_stable_symbols() {
    let manifest = load_manifest();
    let header_path = repo_root().join("stdlib/nyra_rt.h");
    let header = fs::read_to_string(&header_path)
        .unwrap_or_else(|_| panic!("read {}", header_path.display()));

    for sym in manifest.symbol.iter().filter(|s| s.tier == "stable") {
        assert!(
            header.contains(&sym.c_sig),
            "nyra_rt.h missing stable signature: {}",
            sym.c_sig
        );
    }
}

#[test]
fn manifest_has_no_duplicate_names() {
    let manifest = load_manifest();
    let mut seen = HashSet::new();
    for sym in &manifest.symbol {
        assert!(
            seen.insert(sym.name.clone()),
            "duplicate symbol name: {}",
            sym.name
        );
    }
}
