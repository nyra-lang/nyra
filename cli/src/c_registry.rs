//! Built-in + user C library registry (`registry/c/*.toml`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One registry library (system package → headers + link names).
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub headers: Vec<String>,
    #[serde(default)]
    pub libs: Vec<String>,
    #[serde(default)]
    pub pkg_config: Option<String>,
    #[serde(default)]
    pub brew: Option<String>,
    #[serde(default)]
    pub apt: Option<String>,
    #[serde(default)]
    pub pacman: Option<String>,
    #[serde(default)]
    pub dnf: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl RegistryEntry {
    pub fn primary_header(&self) -> Result<&str, String> {
        self.headers
            .first()
            .map(String::as_str)
            .ok_or_else(|| format!("registry entry '{}': headers must be non-empty", self.name))
    }

    pub fn primary_link(&self) -> Result<&str, String> {
        self.libs
            .first()
            .map(String::as_str)
            .ok_or_else(|| format!("registry entry '{}': libs must be non-empty", self.name))
    }

    pub fn brew_formula(&self) -> &str {
        self.brew.as_deref().unwrap_or(self.name.as_str())
    }
}

/// Optional `nyra.toml` inside a third-party C repo.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NyraToml {
    #[serde(default)]
    pub c: Option<NyraTomlC>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NyraTomlC {
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub libraries: Vec<String>,
    #[serde(default)]
    pub include_dirs: Vec<String>,
    #[serde(default)]
    pub link_dirs: Vec<String>,
}

const BUILTIN: &[(&str, &str)] = &[
    ("zlib", include_str!("../../registry/c/zlib.toml")),
    ("sqlite3", include_str!("../../registry/c/sqlite3.toml")),
    ("raylib", include_str!("../../registry/c/raylib.toml")),
    ("sdl2", include_str!("../../registry/c/sdl2.toml")),
    ("raygui", include_str!("../../registry/c/raygui.toml")),
    ("gsl", include_str!("../../registry/c/gsl.toml")),
    ("openssl", include_str!("../../registry/c/openssl.toml")),
    ("libpng", include_str!("../../registry/c/libpng.toml")),
    ("curl", include_str!("../../registry/c/curl.toml")),
];

pub fn load_registry() -> Result<BTreeMap<String, RegistryEntry>, String> {
    let mut map = BTreeMap::new();
    for (name, text) in BUILTIN {
        let entry: RegistryEntry = toml::from_str(text)
            .map_err(|e| format!("builtin registry/{name}.toml: {e}"))?;
        if entry.name != *name && entry.name.to_ascii_lowercase() != *name {
            // allow; key by file stem
        }
        map.insert(entry.name.clone(), entry);
    }

    // User / local overrides win.
    for dir in registry_search_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for ent in entries.filter_map(|e| e.ok()) {
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("README.md") {
                continue;
            }
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let entry: RegistryEntry = toml::from_str(&text)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            map.insert(entry.name.clone(), entry);
        }
    }
    Ok(map)
}

pub fn find_entry(name: &str) -> Result<RegistryEntry, String> {
    let key = name.trim().to_ascii_lowercase();
    let map = load_registry()?;
    if let Some(e) = map.get(&key) {
        return Ok(e.clone());
    }
    for e in map.values() {
        if e.name.eq_ignore_ascii_case(&key) {
            return Ok(e.clone());
        }
        if e.aliases.iter().any(|a| a.eq_ignore_ascii_case(&key)) {
            return Ok(e.clone());
        }
        if e.libs.iter().any(|l| l.eq_ignore_ascii_case(&key)) {
            return Ok(e.clone());
        }
        if e.brew
            .as_ref()
            .is_some_and(|b| b.eq_ignore_ascii_case(&key))
        {
            return Ok(e.clone());
        }
    }
    let known: Vec<_> = map.keys().cloned().collect();
    Err(format!(
        "unknown c-lib '{name}' — known: {}\n  tip: nyra pkg add https://github.com/org/repo  ·  or nyra bind c HEADER.h --lib NAME",
        known.join(", ")
    ))
}

pub fn is_registry_lib(name: &str) -> bool {
    find_entry(name).is_ok()
}

pub fn list_names() -> Result<Vec<String>, String> {
    Ok(load_registry()?.keys().cloned().collect())
}

pub fn parse_nyra_toml(text: &str) -> Result<NyraToml, String> {
    toml::from_str(text).map_err(|e| format!("nyra.toml: {e}"))
}

fn registry_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(extra) = std::env::var("NYRA_C_REGISTRY") {
        dirs.push(PathBuf::from(extra));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".nyra/registry/c"));
    }
    // Dev checkout: repo registry next to cwd or CARGO_MANIFEST_DIR equivalent at runtime.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    dirs.push(cwd.join("registry/c"));
    if let Ok(root) = std::env::var("NYRA_HOME") {
        dirs.push(Path::new(&root).join("registry/c"));
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_builtin_gsl() {
        let e = find_entry("gsl").unwrap();
        assert_eq!(e.name, "gsl");
        assert_eq!(e.primary_header().unwrap(), "gsl/gsl_sf.h");
        assert_eq!(e.libs[0], "gsl");
    }

    #[test]
    fn resolves_aliases() {
        assert_eq!(find_entry("sqlite").unwrap().name, "sqlite3");
        assert_eq!(find_entry("z").unwrap().name, "zlib");
        assert_eq!(find_entry("libcurl").unwrap().name, "curl");
    }

    #[test]
    fn parses_project_nyra_toml() {
        let t = parse_nyra_toml(
            r#"
[c]
headers = ["include/cool.h"]
libraries = ["cool"]
include_dirs = ["include"]
"#,
        )
        .unwrap();
        let c = t.c.unwrap();
        assert_eq!(c.headers, vec!["include/cool.h"]);
        assert_eq!(c.libraries, vec!["cool"]);
    }
}
