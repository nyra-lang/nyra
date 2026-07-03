//! Locate LLVM toolchain binaries (`clang`, `opt`, `lld`, …).
//!
//! Discovery order (first match wins):
//! 1. `$NYRA_LLVM_BIN` / `$NYRA_HOME/lib/llvm/bin` / install-relative `../lib/llvm/bin`
//! 2. Same directory as discovered `opt`
//! 3. `PATH`, Homebrew `llvm`, `xcrun`, fixed prefixes

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static BREW_LLVM_PREFIX: OnceLock<Option<PathBuf>> = OnceLock::new();
static LLVM_TOOL_CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);
static CLANG_CACHE: OnceLock<String> = OnceLock::new();
static DISK_CACHE_LOADED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct DiskToolCache {
    brew_llvm_prefix: Option<String>,
    tools: HashMap<String, Option<String>>,
}

fn disk_cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nyra")
        .join("cache")
        .join("llvm-tools.json")
}

fn load_disk_cache() -> DiskToolCache {
    let path = disk_cache_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return DiskToolCache::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_disk_cache(cache: &DiskToolCache) {
    let path = disk_cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(path, json);
    }
}

fn ensure_disk_cache_loaded() {
    DISK_CACHE_LOADED.get_or_init(|| {
        let disk = load_disk_cache();
        if let Some(prefix) = disk.brew_llvm_prefix {
            let _ = BREW_LLVM_PREFIX.set(Some(PathBuf::from(prefix)));
        }
        let mut guard = LLVM_TOOL_CACHE.lock().expect("llvm tool cache");
        *guard = Some(disk.tools);
    });
}

fn persist_tool_cache() {
    let brew = BREW_LLVM_PREFIX.get().and_then(|p| p.as_ref().map(|b| b.to_string_lossy().into_owned()));
    let tools = LLVM_TOOL_CACHE
        .lock()
        .expect("llvm tool cache")
        .clone()
        .unwrap_or_default();
    save_disk_cache(&DiskToolCache {
        brew_llvm_prefix: brew,
        tools,
    });
}

#[derive(Debug, Clone, Default)]
pub struct ToolchainInfo {
    pub clang: String,
    pub opt: Option<String>,
    pub llvm_profdata: Option<String>,
    pub lld: Option<String>,
    pub wasm_ld: Option<String>,
    pub llvm_bin_dir: Option<PathBuf>,
}

fn tool_candidates(base: &str) -> Vec<String> {
    let mut names = vec![base.to_string()];
    for v in ["21", "20", "19", "18", "17", "16", "15"] {
        names.push(format!("{base}-{v}"));
    }
    names
}

fn tool_runs(path: &Path) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn find_on_path(names: &[String]) -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        for name in names {
            let path = dir.join(name);
            if path.is_file() && tool_runs(&path) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Extra search paths for a bundled or user-configured LLVM install (zig-cc-style layout).
pub fn llvm_bin_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(p) = std::env::var("NYRA_LLVM_BIN") {
        let p = p.trim();
        if !p.is_empty() {
            paths.push(PathBuf::from(p));
        }
    }
    if let Ok(home) = std::env::var("NYRA_HOME") {
        let home = home.trim();
        if !home.is_empty() {
            paths.push(PathBuf::from(home).join("lib/llvm/bin"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            paths.push(bin_dir.join("../lib/llvm/bin"));
        }
    }
    paths
}

fn find_in_dirs(base: &str, dirs: &[PathBuf]) -> Option<String> {
    let names = tool_candidates(base);
    for dir in dirs {
        for name in &names {
            let path = dir.join(name);
            if path.is_file() && tool_runs(&path) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn brew_llvm_prefix() -> Option<PathBuf> {
    ensure_disk_cache_loaded();
    BREW_LLVM_PREFIX
        .get_or_init(|| {
            let output = Command::new("brew")
                .args(["--prefix", "llvm"])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if prefix.is_empty() {
                None
            } else {
                Some(PathBuf::from(prefix))
            }
        })
        .clone()
}

fn xcrun_tool(tool: &str) -> Option<PathBuf> {
    let output = Command::new("xcrun")
        .args(["-find", tool])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn find_llvm_tool_uncached(base: &str) -> Option<String> {
    let names = tool_candidates(base);
    let bundled_dirs = llvm_bin_search_paths();
    if let Some(found) = find_in_dirs(base, &bundled_dirs) {
        return Some(found);
    }
    if let Some(found) = find_on_path(&names) {
        return Some(found);
    }
    if let Some(prefix) = brew_llvm_prefix() {
        let bin = prefix.join("bin");
        for name in &names {
            let path = bin.join(name);
            if path.is_file() && tool_runs(&path) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    for name in &names {
        if let Some(path) = xcrun_tool(name) {
            if tool_runs(&path) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    for prefix in ["/opt/homebrew/opt/llvm", "/usr/local/opt/llvm"] {
        for name in &names {
            let path = PathBuf::from(prefix).join("bin").join(name);
            if tool_runs(&path) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn find_llvm_tool(base: &str) -> Option<String> {
    ensure_disk_cache_loaded();
    let mut guard = LLVM_TOOL_CACHE.lock().expect("llvm tool cache");
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    let cache = guard.as_mut().expect("llvm tool cache init");
    if let Some(hit) = cache.get(base) {
        return hit.clone();
    }
    drop(guard);
    let found = find_llvm_tool_uncached(base);
    let mut guard = LLVM_TOOL_CACHE.lock().expect("llvm tool cache");
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
        .as_mut()
        .expect("llvm tool cache init")
        .insert(base.to_string(), found.clone());
    drop(guard);
    persist_tool_cache();
    found
}

pub fn find_llvm_opt() -> Option<String> {
    find_llvm_tool("opt")
        .or_else(|| find_llvm_tool("llvm-opt"))
}

pub fn find_llvm_profdata() -> Option<String> {
    find_llvm_tool("llvm-profdata")
}

pub fn find_lld() -> Option<String> {
    find_llvm_tool("lld")
}

pub fn find_llvm_nm() -> Option<String> {
    find_llvm_tool("llvm-nm")
}

pub fn find_ar() -> PathBuf {
    find_llvm_tool("llvm-ar")
        .or_else(|| find_llvm_tool("ar"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ar"))
}

pub fn find_wasm_ld() -> Option<String> {
    find_llvm_tool("wasm-ld").or_else(|| find_lld())
}

/// Prefer `clang` from the same LLVM install as `opt` (avoids Apple Clang choking on
/// LLVM 21+ IR attributes such as `captures(none)` after `opt -O3`).
fn resolve_executable(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name);
    if path.is_absolute() && path.is_file() {
        return Some(path);
    }
    if name.contains('/') {
        return path.canonicalize().ok().filter(|p| p.is_file());
    }
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

pub fn find_clang() -> String {
    CLANG_CACHE
        .get_or_init(|| {
            let bundled_dirs = llvm_bin_search_paths();
            if let Some(found) = find_in_dirs("clang", &bundled_dirs) {
                return found;
            }
            if let Some(opt) = find_llvm_opt() {
                let opt_path = PathBuf::from(&opt);
                if let Some(bin_dir) = opt_path.parent() {
                    for name in ["clang", "clang-21", "clang-20", "clang-19", "clang-18"] {
                        let candidate = bin_dir.join(name);
                        if tool_runs(&candidate) {
                            return candidate.to_string_lossy().into_owned();
                        }
                    }
                }
            }
            find_llvm_tool("clang").unwrap_or_else(|| "clang".into())
        })
        .clone()
}

fn sanitize_env_path(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('\r').to_string();
    if s.starts_with('\u{feff}') {
        s = s.trim_start_matches('\u{feff}').to_string();
    }
    s
}

fn find_mingw_gcc_in_prefix(prefix: &Path) -> Option<String> {
    for name in ["gcc.exe", "x86_64-w64-mingw32-gcc.exe"] {
        let path = prefix.join("bin").join(name);
        if path.is_file() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    None
}

/// MSYS2 ucrt64/mingw64 gcc for compiling rt `.c` on Windows (LLVM clang mishandles MinGW `-isystem` headers).
pub fn find_mingw_gcc() -> Option<String> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let mut prefixes = Vec::new();
    if let Ok(v) = std::env::var("NYRA_SYSROOT") {
        let v = sanitize_env_path(&v);
        if !v.is_empty() {
            prefixes.push(PathBuf::from(v));
        }
    }
    for p in [r"C:\msys64\ucrt64", r"C:\msys64\mingw64"] {
        prefixes.push(PathBuf::from(p));
    }
    for prefix in prefixes {
        if let Some(gcc) = find_mingw_gcc_in_prefix(&prefix) {
            return Some(gcc);
        }
    }
    // Fallback: gcc may be on PATH (CI adds ucrt64/bin) even when sysroot layout differs.
    if let Ok(output) = Command::new("where").arg("gcc.exe").output() {
        if output.status.success() {
            let first = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .map(sanitize_env_path)
                .filter(|s| !s.is_empty());
            if let Some(path) = first {
                let p = PathBuf::from(&path);
                if p.is_file() {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// MinGW `ld` for linking gnu objects on a Windows host (avoid LLVM `lld-link`/MSVC).
pub fn find_mingw_ld() -> Option<String> {
    let gcc = find_mingw_gcc()?;
    let gcc_path = PathBuf::from(&gcc);
    let bin = gcc_path.parent()?;
    for name in ["ld.exe", "x86_64-w64-mingw32-ld.exe"] {
        let path = bin.join(name);
        if path.is_file() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    None
}

pub fn toolchain_info() -> ToolchainInfo {
    let clang = find_clang();
    let clang_path = resolve_executable(&clang).unwrap_or_else(|| PathBuf::from(&clang));
    let opt = find_llvm_opt();
    let llvm_bin_dir = clang_path
        .parent()
        .filter(|p| p.is_dir())
        .map(Path::to_path_buf)
        .or_else(|| {
            opt.as_ref()
                .and_then(|o| resolve_executable(o))
                .and_then(|p| p.parent().map(Path::to_path_buf))
        });
    ToolchainInfo {
        clang: clang_path.to_string_lossy().into_owned(),
        opt,
        llvm_profdata: find_llvm_profdata(),
        lld: find_lld(),
        wasm_ld: find_wasm_ld(),
        llvm_bin_dir,
    }
}

pub fn print_toolchain_info() {
    let t = toolchain_info();
    println!("nyra native toolchain (LLVM/clang driver)");
    println!("  clang:          {}", t.clang);
    if let Some(ref opt) = t.opt {
        println!("  opt:            {opt}");
    } else {
        println!("  opt:            (not found — release/PGO needs full LLVM)");
    }
    if let Some(ref p) = t.llvm_profdata {
        println!("  llvm-profdata:  {p}");
    }
    if let Some(ref l) = t.lld {
        println!("  lld:            {l}");
    }
    if let Some(ref w) = t.wasm_ld {
        println!("  wasm-ld:        {w}");
    }
    if let Some(ref d) = t.llvm_bin_dir {
        println!("  llvm bin dir:   {}", d.display());
    }
    if !llvm_bin_search_paths().is_empty() {
        println!("  search paths:");
        for p in llvm_bin_search_paths() {
            println!("    {}", p.display());
        }
    }
    println!();
    println!("Set NYRA_LLVM_BIN or NYRA_HOME/lib/llvm/bin for a bundled toolchain.");
    println!("Use as CC/CXX: export CC=\"nyra cc\" CXX=\"nyra cc\"");
}

/// Strip LLVM IR parameter attrs that older `clang` frontends reject (e.g. Apple Clang vs
/// Homebrew `opt` output). Safe to apply unconditionally — semantics unchanged.
pub fn sanitize_ir_for_clang(content: &str) -> String {
    let mut out = content.replace(" captures(none)", "");
    out = out.replace(" captures(all)", "");
    out = out.replace(" captures(ret)", "");
    for (from, to) in [
        ("ptr 0,", "ptr %0,"),
        ("ptr 0)", "ptr %0)"),
        ("phi ptr [0,", "phi ptr [%0,"),
    ] {
        if out.contains(from) {
            out = out.replace(from, to);
        }
    }
    // Codegen occasionally double-prefixes SSA names (%%1) when a register already includes '%'.
    let mut fixed = String::with_capacity(out.len());
    let mut chars = out.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' && chars.peek() == Some(&'%') {
            chars.next();
            fixed.push('%');
        } else {
            fixed.push(c);
        }
    }
    fixed
}

pub fn require_llvm_opt() -> Result<String, String> {
    find_llvm_opt().ok_or_else(|| {
        "llvm `opt` not found — Nyra release/PGO builds need the full LLVM toolchain \
         (not Apple Clang alone).\n\
         Install: brew install llvm  (macOS)  |  dnf install llvm  (Fedora)  |  apt install llvm\n\
         Then: export NYRA_LLVM_BIN=\"$(brew --prefix llvm)/bin\"  or  export PATH=\"$(brew --prefix llvm)/bin:$PATH\""
            .to_string()
    })
}

pub fn require_llvm_profdata() -> Result<String, String> {
    find_llvm_profdata().ok_or_else(|| {
        "llvm-profdata not found — install the full LLVM toolchain (same package as `opt`)"
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_or_profdata_lookup_does_not_panic() {
        let _ = find_llvm_opt();
        let _ = find_llvm_profdata();
    }

    #[test]
    fn sanitize_strips_captures_attrs() {
        let raw = "declare i32 @printf(ptr noundef readonly captures(none), ...)";
        let cleaned = sanitize_ir_for_clang(raw);
        assert!(!cleaned.contains("captures("));
        assert!(cleaned.contains("readonly"));
    }

    #[test]
    fn sanitize_fixes_opaque_ptr_zero_operands() {
        let raw = "  %call = call i32 @find_host_end(ptr 0, i32 %x, i32 %y)";
        let cleaned = sanitize_ir_for_clang(raw);
        assert!(cleaned.contains("ptr %0,"));
        assert!(!cleaned.contains("ptr 0,"));
    }

    #[test]
    fn sanitize_fixes_double_percent_ssa() {
        let raw = "  store i32 %%1, i32* %closure.gep.91";
        let cleaned = sanitize_ir_for_clang(raw);
        assert!(cleaned.contains("store i32 %1,"));
        assert!(!cleaned.contains("%%"));
    }

    #[test]
    fn sanitize_env_path_strips_bom_and_cr() {
        assert_eq!(sanitize_env_path("\u{feff}C:\\msys64\\ucrt64\r"), "C:\\msys64\\ucrt64");
        assert_eq!(sanitize_env_path("  C:\\foo  "), "C:\\foo");
    }

    #[test]
    fn toolchain_info_has_clang() {
        let t = toolchain_info();
        assert!(!t.clang.is_empty());
    }
}
