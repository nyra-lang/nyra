//! Cross-compilation target resolution: `--for`, `--os`, `--arch`, `--target`.

use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// LLVM triple passed to codegen and clang (empty = host native).
    pub triple: String,
    pub os: TargetOs,
    pub arch: TargetArch,
    pub is_cross: bool,
    pub is_wasm: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOs {
    Linux,
    MacOs,
    Windows,
    Wasm,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct TargetFlags {
    pub for_os: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub target: Option<String>,
}

impl TargetSpec {
    pub fn host() -> Self {
        let triple = detect_host_triple();
        let os = parse_os(&triple);
        let arch = parse_arch(&triple);
        Self {
            triple: String::new(),
            os,
            arch,
            is_cross: false,
            is_wasm: false,
        }
    }

    pub fn triple_for_codegen(&self) -> String {
        if self.triple.is_empty() {
            detect_host_triple()
        } else {
            self.triple.clone()
        }
    }

    pub fn is_windows(&self) -> bool {
        self.os == TargetOs::Windows
    }

    pub fn exe_extension(&self) -> &'static str {
        if self.is_wasm {
            ".wasm"
        } else if self.is_windows() {
            ".exe"
        } else {
            ""
        }
    }

    pub fn cdylib_extension(&self) -> &'static str {
        if self.is_wasm {
            ".wasm"
        } else if self.os == TargetOs::MacOs {
            ".dylib"
        } else if self.is_windows() {
            ".dll"
        } else {
            ".so"
        }
    }

    pub fn artifact_subdir(&self) -> Option<&str> {
        if self.is_cross && !self.triple.is_empty() {
            Some(&self.triple)
        } else {
            None
        }
    }
}

pub fn resolve(flags: &TargetFlags) -> Result<TargetSpec, String> {
    let host_triple = detect_host_triple();
    let host_arch = parse_arch(&host_triple);

    if let Some(ref raw) = flags.target {
        if !raw.is_empty() {
            let triple = raw.trim().to_string();
            let os = parse_os(&triple);
            let arch = parse_arch(&triple);
            let is_wasm = triple.contains("wasm");
            let is_cross = !triples_match_host(&triple, &host_triple);
            return Ok(TargetSpec {
                triple,
                os,
                arch,
                is_cross,
                is_wasm,
            });
        }
    }

    let os_name = flags
        .for_os
        .as_deref()
        .or(flags.os.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let Some(os_name) = os_name else {
        return Ok(TargetSpec::host());
    };

    if flags.for_os.is_some() && flags.os.is_some() {
        return Err("use only one of --for or --os (not both)".into());
    }

    let os = parse_os_alias(os_name)?;
    let arch = flags
        .arch
        .as_deref()
        .map(parse_arch_alias)
        .transpose()?
        .unwrap_or(host_arch);

    let triple = triple_for_os_arch(os, arch, host_arch)?;
    let is_wasm = os == TargetOs::Wasm;
    let is_cross = !triples_match_host(&triple, &host_triple);

    Ok(TargetSpec {
        triple,
        os,
        arch,
        is_cross,
        is_wasm,
    })
}

pub fn validate_native_cpu(spec: &TargetSpec, native_cpu: bool) -> Result<(), String> {
    if native_cpu && spec.is_cross {
        return Err(
            "--native-cpu cannot be used when cross-compiling (remove --for/--os/--target)".into(),
        );
    }
    Ok(())
}

/// Host triple at runtime (for codegen when `--target` is empty).
pub fn detect_host_triple() -> String {
    if let Some(t) = clang_dumpmachine() {
        return normalize_triple(&t);
    }
    triple_from_env_consts()
}

fn clang_dumpmachine() -> Option<String> {
    let clang = crate::llvm_tools::find_clang();
    let out = Command::new(&clang).arg("-dumpmachine").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn triple_from_env_consts() -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        other => other,
    };
    match std::env::consts::OS {
        "macos" => {
            if arch == "aarch64" {
                "arm64-apple-darwin".into()
            } else {
                format!("{arch}-apple-darwin")
            }
        }
        "linux" => format!("{arch}-unknown-linux-gnu"),
        "windows" => format!("{arch}-pc-windows-gnu"),
        _ => format!("{arch}-unknown-linux-gnu"),
    }
}

fn normalize_triple(t: &str) -> String {
    let t = t.trim();
    if t == "arm64-apple-darwin" || t.starts_with("aarch64-apple-darwin") {
        return "arm64-apple-darwin".into();
    }
    t.to_string()
}

fn triples_match_host(triple: &str, host: &str) -> bool {
    let a = normalize_triple(triple);
    let b = normalize_triple(host);
    a == b
        || (a.contains("apple-darwin") && b.contains("apple-darwin") && same_arch(&a, &b))
        || (a.contains("linux-gnu") && b.contains("linux-gnu") && same_arch(&a, &b))
        || (a.contains("windows") && b.contains("windows") && same_arch(&a, &b))
}

fn same_arch(a: &str, b: &str) -> bool {
    parse_arch(a) == parse_arch(b)
}

fn parse_os_alias(s: &str) -> Result<TargetOs, String> {
    match s.to_ascii_lowercase().as_str() {
        "linux" => Ok(TargetOs::Linux),
        "macos" | "mac" | "darwin" | "osx" => Ok(TargetOs::MacOs),
        "windows" | "win" | "win32" => Ok(TargetOs::Windows),
        "wasm" | "wasi" => Ok(TargetOs::Wasm),
        other => Err(format!(
            "unknown OS {other:?}; use linux, macos, windows, or wasm"
        )),
    }
}

fn parse_arch_alias(s: &str) -> Result<TargetArch, String> {
    match s.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" | "x64" => Ok(TargetArch::X86_64),
        "aarch64" | "arm64" => Ok(TargetArch::Aarch64),
        other => Err(format!("unknown arch {other:?}; use x86_64 or aarch64")),
    }
}

fn triple_for_os_arch(os: TargetOs, arch: TargetArch, host_arch: TargetArch) -> Result<String, String> {
    let arch_str = match arch {
        TargetArch::X86_64 => "x86_64",
        TargetArch::Aarch64 => "aarch64",
        TargetArch::Unknown => match host_arch {
            TargetArch::X86_64 => "x86_64",
            TargetArch::Aarch64 => "aarch64",
            TargetArch::Unknown => "x86_64",
        },
    };

    Ok(match os {
        TargetOs::Linux => format!("{arch_str}-unknown-linux-gnu"),
        TargetOs::MacOs => {
            if arch == TargetArch::Aarch64 || arch_str == "aarch64" {
                "arm64-apple-darwin".into()
            } else {
                "x86_64-apple-darwin".into()
            }
        }
        TargetOs::Windows => {
            if arch == TargetArch::Aarch64 {
                format!("{arch_str}-pc-windows-gnu")
            } else {
                "x86_64-pc-windows-gnu".into()
            }
        }
        TargetOs::Wasm => "wasm32-wasip1".into(),
        TargetOs::Unknown => {
            return Err("internal: unknown target OS".into());
        }
    })
}

pub fn parse_os(triple: &str) -> TargetOs {
    let t = triple.to_ascii_lowercase();
    if t.contains("wasm") {
        TargetOs::Wasm
    } else if t.contains("windows") {
        TargetOs::Windows
    } else if t.contains("apple-darwin") || t.contains("darwin") {
        TargetOs::MacOs
    } else if t.contains("linux") {
        TargetOs::Linux
    } else {
        TargetOs::Unknown
    }
}

pub fn parse_arch(triple: &str) -> TargetArch {
    let t = triple.to_ascii_lowercase();
    if t.starts_with("aarch64") || t.starts_with("arm64") {
        TargetArch::Aarch64
    } else if t.starts_with("x86_64") || t.starts_with("amd64") {
        TargetArch::X86_64
    } else {
        TargetArch::Unknown
    }
}

#[derive(Debug, Clone, Default)]
pub struct LinkTargetFlags {
    pub needs_pthread: bool,
    pub uses_rt_os: bool,
    pub uses_rt_hw: bool,
    pub uses_rt_os_adv: bool,
    pub uses_rt_net: bool,
    pub needs_openssl: bool,
    pub needs_zlib: bool,
    pub needs_libm: bool,
}

/// macOS SDK path for Homebrew LLVM clang (no Apple sysroot by default).
fn detect_macos_sdk() -> Option<PathBuf> {
    if let Ok(sdk) = std::env::var("SDKROOT") {
        if !sdk.is_empty() {
            return Some(PathBuf::from(sdk));
        }
    }
    let output = Command::new("xcrun").args(["--show-sdk-path"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let sdk = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sdk.is_empty() {
        None
    } else {
        Some(PathBuf::from(sdk))
    }
}

/// MinGW-w64 sysroot for `*-pc-windows-gnu` (MSYS2 ucrt64/mingw64 on CI runners).
fn mingw_sysroot_prefixes() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(v) = std::env::var("NYRA_SYSROOT") {
        if !v.is_empty() {
            out.push(PathBuf::from(v));
        }
    }
    for p in [r"C:\msys64\ucrt64", r"C:\msys64\mingw64"] {
        out.push(PathBuf::from(p));
    }
    out
}

fn zlib_prefixes() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(v) = std::env::var("ZLIB_ROOT") {
        if !v.is_empty() {
            out.push(std::path::PathBuf::from(v));
        }
    }
    for p in [
        "/opt/homebrew/opt/zlib",
        "/usr/local/opt/zlib",
        r"C:\msys64\ucrt64",
        r"C:\msys64\mingw64",
        r"C:\ProgramData\chocolatey\lib\zlib\tools",
        r"C:\vcpkg\installed\x64-windows",
        r"C:\tools\zlib",
    ] {
        out.push(std::path::PathBuf::from(p));
    }
    out
}

fn openssl_prefixes() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for var in ["OPENSSL_DIR", "OPENSSL_ROOT_DIR"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                out.push(std::path::PathBuf::from(v));
            }
        }
    }
    for p in [
        "/opt/homebrew/opt/openssl@3",
        "/opt/homebrew/opt/openssl",
        "/usr/local/opt/openssl@3",
        "/usr/local/opt/openssl",
    ] {
        out.push(std::path::PathBuf::from(p));
    }
    out
}

/// WASI sysroot for wasm link (Homebrew `wasi-libc`, distro packages, or env override).
pub fn detect_wasi_sysroot() -> Option<PathBuf> {
    for p in [
        "/opt/homebrew/opt/wasi-libc/share/wasi-sysroot",
        "/usr/local/opt/wasi-libc/share/wasi-sysroot",
        "/usr/share/wasi-sysroot",
    ] {
        let root = PathBuf::from(p);
        if root.join("lib/wasm32-wasip1/crt1.o").is_file()
            || root.join("lib/wasm32-wasi/crt1.o").is_file()
        {
            return Some(root);
        }
    }
    None
}

/// Flags for `clang -c` (target triple, sysroot, include paths — no linker args).
pub fn apply_target_compile_flags(cmd: &mut Command, spec: &TargetSpec) {
    if !spec.triple.is_empty() {
        cmd.arg("-target").arg(&spec.triple);
    }

    if spec.is_wasm {
        let wasi_inc = compiler::runtime_map::wasi_rt_dir();
        if wasi_inc.is_dir() {
            cmd.arg(format!("-I{}", wasi_inc.display()));
        }
        let sysroot = std::env::var("NYRA_SYSROOT")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var("NYRA_WASI_SYSROOT").ok().filter(|s| !s.is_empty()).map(PathBuf::from))
            .or_else(detect_wasi_sysroot);
        if let Some(ref root) = sysroot {
            cmd.arg(format!("--sysroot={}", root.display()));
        }
    } else if let Ok(sysroot) = std::env::var("NYRA_SYSROOT") {
        if !sysroot.is_empty() {
            cmd.arg(format!("--sysroot={sysroot}"));
        }
    }

    if spec.os == TargetOs::MacOs {
        if let Some(sdk) = detect_macos_sdk() {
            cmd.arg(format!("-isysroot{}", sdk.display()));
        }
        for prefix in openssl_prefixes() {
            let inc = prefix.join("include");
            if inc.is_dir() {
                cmd.arg(format!("-I{}", inc.display()));
            }
        }
    }

    if spec.os == TargetOs::Windows {
        for prefix in mingw_sysroot_prefixes() {
            if prefix.join("include/stdlib.h").is_file() {
                cmd.arg(format!("--sysroot={}", prefix.display()));
                break;
            }
        }
    }

    for prefix in zlib_prefixes() {
        let inc = prefix.join("include");
        if inc.is_dir() {
            cmd.arg(format!("-I{}", inc.display()));
        }
    }
}

pub fn apply_target_link_flags(cmd: &mut Command, spec: &TargetSpec, rt: &LinkTargetFlags) {
    apply_target_compile_flags(cmd, spec);

    if spec.is_wasm {
        let sysroot = std::env::var("NYRA_SYSROOT")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var("NYRA_WASI_SYSROOT").ok().filter(|s| !s.is_empty()).map(PathBuf::from))
            .or_else(detect_wasi_sysroot);
        if sysroot.is_some() {
            cmd.arg("-nodefaultlibs");
            cmd.arg("-lc");
        }
    }

    match spec.os {
        TargetOs::MacOs => {
            cmd.arg("-Wl,-dead_strip");
            if rt.uses_rt_os || rt.uses_rt_hw {
                cmd.arg("-framework").arg("IOKit");
                cmd.arg("-framework").arg("CoreFoundation");
            }
            if rt.uses_rt_hw {
                cmd.arg("-framework").arg("CoreGraphics");
            }
            if rt.needs_openssl {
                for prefix in openssl_prefixes() {
                    let lib = prefix.join("lib");
                    if lib.is_dir() {
                        cmd.arg(format!("-L{}", lib.display()));
                    }
                }
                cmd.arg("-lssl").arg("-lcrypto");
            }
            if rt.needs_zlib {
                cmd.arg("-lz");
            }
            if rt.needs_libm {
                cmd.arg("-lm");
            }
        }
        TargetOs::Linux => {
            cmd.arg("-Wl,--gc-sections");
            if rt.needs_pthread {
                cmd.arg("-lpthread");
            }
            if rt.uses_rt_os_adv {
                cmd.arg("-lrt");
            }
            if rt.needs_openssl {
                cmd.arg("-lssl").arg("-lcrypto");
            }
            if rt.needs_zlib {
                cmd.arg("-lz");
            }
            if rt.needs_libm {
                cmd.arg("-lm");
            }
        }
        TargetOs::Windows => {
            // Windows rt modules use native CRITICAL_SECTION / Win32 threads, not pthread.
            if rt.uses_rt_net {
                cmd.arg("-lws2_32");
            }
            if rt.uses_rt_hw {
                cmd.arg("-liphlpapi");
            }
            if rt.uses_rt_os_adv {
                cmd.arg("-lbcrypt");
                cmd.arg("-lsetupapi");
            }
            if rt.needs_zlib {
                for prefix in zlib_prefixes() {
                    let lib = prefix.join("lib");
                    if lib.is_dir() {
                        cmd.arg(format!("-L{}", lib.display()));
                    }
                }
                cmd.arg("-lz");
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_linux_alias() {
        let spec = resolve(&TargetFlags {
            for_os: Some("linux".into()),
            ..Default::default()
        })
        .unwrap();
        assert!(spec.triple.contains("linux-gnu"));
        assert_eq!(spec.os, TargetOs::Linux);
    }

    #[test]
    fn resolve_windows_for() {
        let spec = resolve(&TargetFlags {
            for_os: Some("windows".into()),
            ..Default::default()
        })
        .unwrap();
        assert!(spec.triple.contains("windows"));
        assert_eq!(spec.exe_extension(), ".exe");
    }

    #[test]
    fn resolve_wasm() {
        let spec = resolve(&TargetFlags {
            for_os: Some("wasm".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(spec.triple, "wasm32-wasip1");
        assert!(spec.is_wasm);
    }

    #[test]
    fn explicit_target_wins() {
        let spec = resolve(&TargetFlags {
            for_os: Some("linux".into()),
            target: Some("wasm32-wasi".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(spec.triple, "wasm32-wasi");
    }

    #[test]
    fn for_and_os_conflict() {
        let err = resolve(&TargetFlags {
            for_os: Some("linux".into()),
            os: Some("linux".into()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(err.contains("--for"));
    }

    #[test]
    fn macos_aarch64_triple() {
        let triple = triple_for_os_arch(TargetOs::MacOs, TargetArch::Aarch64, TargetArch::X86_64).unwrap();
        assert_eq!(triple, "arm64-apple-darwin");
    }

    #[test]
    fn host_has_empty_triple() {
        let spec = TargetSpec::host();
        assert!(spec.triple.is_empty());
        assert!(!spec.is_cross);
    }

    #[test]
    fn parse_os_from_triple() {
        assert_eq!(parse_os("x86_64-pc-windows-gnu"), TargetOs::Windows);
        assert_eq!(parse_os("aarch64-unknown-linux-gnu"), TargetOs::Linux);
        assert_eq!(parse_os("arm64-apple-darwin"), TargetOs::MacOs);
    }
}
