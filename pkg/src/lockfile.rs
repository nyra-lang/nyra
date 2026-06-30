use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockFile {
    pub version: u32,
    pub module: String,
    pub require: Vec<LockEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockEntry {
    pub module: String,
    pub version: String,
    #[serde(default)]
    pub source: LockSource,
    pub checksum: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LockSource {
    #[default]
    Local,
    Git { url: String, rev: String },
}

impl LockFile {
    pub fn new(module: impl Into<String>) -> Self {
        Self {
            version: 1,
            module: module.into(),
            require: vec![],
        }
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("invalid nyra.lock: {e}"))
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn write_sum(&self, path: &Path) -> Result<(), String> {
        let mut lines = Vec::new();
        for e in &self.require {
            lines.push(format!("{} {}", e.checksum, e.module));
        }
        fs::write(path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn verify_sum(&self, path: &Path) -> Result<(), String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut found: HashMap<String, String> = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (hash, module) = line
                .split_once(char::is_whitespace)
                .ok_or_else(|| format!("bad nyra.sum line: {line}"))?;
            found.insert(module.to_string(), hash.to_string());
        }
        for e in &self.require {
            match found.get(&e.module) {
                Some(h) if h == &e.checksum => {}
                _ => {
                    return Err(format!(
                        "checksum mismatch or missing entry for '{}' in nyra.sum",
                        e.module
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

pub fn cache_module_path(module: &str) -> PathBuf {
    PathBuf::from(".nyra/cache").join(module.replace('.', "/"))
}
