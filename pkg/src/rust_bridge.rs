//! Rust crate bridge: generate C-ABI wrappers and Nyra bindings for crates.io crates.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use nyra_bindgen::bindgen_crate;

use crate::semver::{parse_req, parse_version, best_match, Req, Version};

const CRATES_IO: &str = "https://crates.io/api/v1/crates";
const CRATES_IO_USER_AGENT: &str = "nyra/1.0 (https://github.com/nyra-lang/nyra)";

fn crates_io_get(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-sf", "-A", CRATES_IO_USER_AGENT, url])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;
    if !output.status.success() {
        return Err(format!("crates.io request failed: {url}"));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

/// `rust::uuid` → crate name `uuid`.
pub fn parse_rust_module(name: &str) -> Option<&str> {
    name.strip_prefix("rust::")
}

pub fn rust_cache_dir(project_root: &Path, crate_name: &str) -> PathBuf {
    project_root
        .join(".nyra/cache/rust")
        .join(crate_name)
}

pub fn wrapper_dir(project_root: &Path, crate_name: &str) -> PathBuf {
    rust_cache_dir(project_root, crate_name).join("wrapper")
}

#[derive(Debug, Clone, Default)]
pub struct BindOptions {
    /// Only bind these symbols (`Regex::new`, `is_match`, …).
    pub export_filter: Option<Vec<String>>,
    /// Use hand-written template even if bindgen is available.
    pub force_template: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMeta {
    pub crate_name: String,
    pub version: String,
    pub lib_name: String,
    #[serde(default)]
    pub mode: String,
}

impl BridgeMeta {
    pub fn read(dir: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(dir.join("bridge.json")).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn write(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("bridge.json"), text).map_err(|e| e.to_string())
    }
}

fn lib_name_for(crate_name: &str) -> String {
    format!("bridge_{}", crate_name.replace('-', "_"))
}

/// Resolve a semver requirement against crates.io.
pub fn resolve_crates_io_version(crate_name: &str, req: Option<&Req>) -> Result<String, String> {
    let body = crates_io_get(&format!("{CRATES_IO}/{crate_name}"))?;
    let parsed: CratesIoResponse =
        serde_json::from_str(&body).map_err(|e| format!("crates.io parse error: {e}"))?;
    if parsed.versions.is_empty() {
        return Err(format!("crate '{crate_name}' has no published versions"));
    }
    let versions: Vec<Version> = parsed
        .versions
        .iter()
        .filter(|v| !v.yanked && !v.num.contains('-'))
        .filter_map(|v| parse_version(&v.num).ok())
        .collect();
    if let Some(req) = req {
        let best = best_match(req, versions.iter())
            .ok_or_else(|| format!("no version of '{crate_name}' satisfies requirement"))?;
        Ok(format!("{}.{}.{}", best.major, best.minor, best.patch))
    } else {
        let latest = versions
            .iter()
            .max_by(|a, b| a.compare(b))
            .ok_or_else(|| format!("no versions for '{crate_name}'"))?;
        Ok(format!(
            "{}.{}.{}",
            latest.major, latest.minor, latest.patch
        ))
    }
}

#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    versions: Vec<CratesIoVersion>,
}

#[derive(Debug, Deserialize)]
struct CratesIoVersion {
    num: String,
    #[serde(default)]
    yanked: bool,
}

/// Known bridge profiles (MVP). Extend as more crates are validated.
pub fn known_bridge(crate_name: &str) -> Option<BridgeTemplate> {
    match crate_name {
        "uuid" => Some(BridgeTemplate::uuid()),
        "serde_json" => Some(BridgeTemplate::serde_json()),
        "toml" => Some(BridgeTemplate::toml()),
        _ => None,
    }
}

pub struct BridgeTemplate {
    pub crate_name: &'static str,
    pub dep_line: fn(&str) -> String,
    pub wrapper_rs: &'static str,
    pub bindings_ny: &'static str,
}

impl BridgeTemplate {
    fn uuid() -> Self {
        Self {
            crate_name: "uuid",
            dep_line: |version| format!(r#"uuid = {{ version = "{version}", features = ["v4"] }}"#),
            wrapper_rs: r#"use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn uuid_new_v4() -> *mut c_char {
    let s = uuid::Uuid::new_v4().to_string();
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn uuid_parse(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let Ok(text) = cstr.to_str() else {
        return std::ptr::null_mut();
    };
    match uuid::Uuid::parse_str(text) {
        Ok(u) => match CString::new(u.to_string()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}
"#,
            bindings_ny: r#"// Generated by `nyra bind rust uuid` — do not edit by hand.
extern fn uuid_new_v4() -> string
extern fn uuid_parse(input: string) -> string

fn new_v4() -> string {
    return uuid_new_v4()
}

fn parse(input: string) -> string {
    return uuid_parse(input)
}
"#,
        }
    }

    fn serde_json() -> Self {
        Self {
            crate_name: "serde_json",
            dep_line: |version| format!(r#"serde_json = "{version}""#),
            wrapper_rs: r#"use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn serde_json_parse(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let Ok(text) = cstr.to_str() else {
        return std::ptr::null_mut();
    };
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(v) => match CString::new(v.to_string()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn serde_json_stringify(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let Ok(text) = cstr.to_str() else {
        return std::ptr::null_mut();
    };
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(v) => match CString::new(serde_json::to_string(&v).unwrap_or_default()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}
"#,
            bindings_ny: r#"// Generated by `nyra bind rust serde_json` — do not edit by hand.
extern fn serde_json_parse(input: string) -> string
extern fn serde_json_stringify(input: string) -> string

fn parse(input: string) -> string {
    return serde_json_parse(input)
}

fn stringify(input: string) -> string {
    return serde_json_stringify(input)
}
"#,
        }
    }

    fn toml() -> Self {
        Self {
            crate_name: "toml",
            dep_line: |version| format!(r#"toml = "{version}""#),
            wrapper_rs: r#"use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn toml_parse(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let Ok(text) = cstr.to_str() else {
        return std::ptr::null_mut();
    };
    match toml::from_str::<toml::Value>(text) {
        Ok(v) => match CString::new(toml::to_string(&v).unwrap_or_default()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn toml_stringify(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let Ok(text) = cstr.to_str() else {
        return std::ptr::null_mut();
    };
    match toml::from_str::<toml::Value>(text) {
        Ok(v) => match CString::new(toml::to_string(&v).unwrap_or_default()) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}
"#,
            bindings_ny: r#"// Generated by `nyra bind rust toml` — do not edit by hand.
extern fn toml_parse(input: string) -> string
extern fn toml_stringify(input: string) -> string

fn parse(input: string) -> string {
    return toml_parse(input)
}

fn stringify(input: string) -> string {
    return toml_stringify(input)
}
"#,
        }
    }
}

/// Generate wrapper crate + Nyra bindings under `.nyra/cache/rust/<name>/`.
pub fn bind_rust_crate(
    project_root: &Path,
    crate_name: &str,
    version_req: Option<&Req>,
) -> Result<BridgeMeta, String> {
    bind_rust_crate_with_options(project_root, crate_name, version_req, &BindOptions::default())
}

pub fn bind_rust_crate_with_options(
    project_root: &Path,
    crate_name: &str,
    version_req: Option<&Req>,
    options: &BindOptions,
) -> Result<BridgeMeta, String> {
    let version = resolve_crates_io_version(crate_name, version_req)?;
    let cache = rust_cache_dir(project_root, crate_name);
    let wrap = wrapper_dir(project_root, crate_name);
    std::fs::create_dir_all(&wrap).map_err(|e| e.to_string())?;

    let lib_name = lib_name_for(crate_name);
    let (wrapper_rs, bindings_ny, dep_line, mode) =
        if options.force_template {
            template_artifacts(crate_name, &version)?
        } else {
            match bindgen_crate(
                crate_name,
                &version,
                &wrap,
                options
                    .export_filter
                    .as_ref()
                    .map(|v| v.as_slice()),
            ) {
                Ok(generated) => (
                    generated.wrapper_rs,
                    generated.bindings_ny,
                    generated.dep_line,
                    "bindgen".to_string(),
                ),
                Err(bindgen_err) => {
                    if let Some((wrapper_rs, bindings_ny, dep_line, mode)) =
                        template_artifacts(crate_name, &version).ok()
                    {
                        eprintln!("note: bindgen failed ({bindgen_err}); using template fallback");
                        (wrapper_rs, bindings_ny, dep_line, mode)
                    } else {
                        return Err(bindgen_err);
                    }
                }
            }
        };

    write_wrapper_project(&wrap, crate_name, &lib_name, &dep_line, &wrapper_rs)?;
    std::fs::write(cache.join("bindings.ny"), bindings_ny).map_err(|e| e.to_string())?;

    let meta = BridgeMeta {
        crate_name: crate_name.to_string(),
        version: version.clone(),
        lib_name: lib_name.clone(),
        mode,
    };
    meta.write(&cache)?;

    eprintln!(
        "bound rust::{crate_name} {version} ({}) → {}",
        meta.mode,
        cache.join("bindings.ny").display()
    );
    Ok(meta)
}

fn template_artifacts(
    crate_name: &str,
    version: &str,
) -> Result<(String, String, String, String), String> {
    let template = known_bridge(crate_name).ok_or_else(|| {
        format!("no bridge template for crate '{crate_name}'")
    })?;
    Ok((
        template.wrapper_rs.to_string(),
        template.bindings_ny.to_string(),
        (template.dep_line)(version),
        "template".to_string(),
    ))
}

fn write_wrapper_project(
    wrap: &Path,
    crate_name: &str,
    lib_name: &str,
    dep_line: &str,
    wrapper_rs: &str,
) -> Result<(), String> {
    let cargo_toml = format!(
        r#"[package]
name = "nyra-bridge-{crate_name}"
version = "0.1.0"
edition = "2021"
publish = false

[workspace]

[lib]
crate-type = ["staticlib"]
name = "{lib_name}"

[dependencies]
{dep_line}
"#
    );
    std::fs::create_dir_all(wrap.join("src")).map_err(|e| e.to_string())?;
    std::fs::write(wrap.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;
    std::fs::write(wrap.join("src/lib.rs"), wrapper_rs).map_err(|e| e.to_string())?;
    Ok(())
}

/// Build a link-crate wrapper and return (lib_name, search_path).
pub fn build_link_crate(project_root: &Path, crate_name: &str) -> Result<(String, PathBuf), String> {
    let wrap = wrapper_dir(project_root, crate_name);
    if !wrap.join("Cargo.toml").is_file() {
        return Err(format!(
            "link-crate '{crate_name}' not bound — run `nyra bind rust {crate_name}` or `nyra add rust::{crate_name}`"
        ));
    }
    let meta = BridgeMeta::read(&rust_cache_dir(project_root, crate_name))?;
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(wrap.join("Cargo.toml"))
        .status()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build failed for link-crate '{crate_name}'"));
    }
    let search = wrap.join("target/release");
    Ok((meta.lib_name, search))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rust_module_prefix() {
        assert_eq!(parse_rust_module("rust::uuid"), Some("uuid"));
        assert_eq!(parse_rust_module("ny-sqlite"), None);
    }

    #[test]
    fn lib_name_sanitizes_dashes() {
        assert_eq!(lib_name_for("serde-json"), "bridge_serde_json");
    }

    #[test]
    fn known_bridge_includes_uuid() {
        assert!(known_bridge("uuid").is_some());
        assert!(known_bridge("serde_json").is_some());
        assert!(known_bridge("toml").is_some());
        assert!(known_bridge("unknown-crate").is_none());
    }
}
