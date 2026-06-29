//! Cargo-style output layout: `{project_root}/target/{debug|release}/` or
//! `{project_root}/target/{triple}/{debug|release}/` when cross-compiling.

use std::path::{Path, PathBuf};

use compiler::paths;

use crate::target::TargetSpec;

pub struct ArtifactLayout {
    pub profile_dir: PathBuf,
    pub ll_path: PathBuf,
    pub bin_path: PathBuf,
}

/// Cache key for incremental metadata when multiple entry files share one `profile_dir`.
pub fn entry_cache_id(layout: &ArtifactLayout) -> String {
    layout
        .ll_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main")
        .to_string()
}

/// Per-entry scratch dir for LLVM opt + link (avoids shared `out.opt.ll` races).
pub fn entry_link_work_dir(layout: &ArtifactLayout) -> PathBuf {
    layout
        .profile_dir
        .join(".nyra-cache")
        .join("entries")
        .join(entry_cache_id(layout))
        .join("link")
}

pub fn profile_name(release: bool) -> &'static str {
    if release {
        "release"
    } else {
        "debug"
    }
}

/// Directory that owns `target/` (project root or parent of a single-file build).
pub fn project_root(input: &Path) -> PathBuf {
    if input.is_dir() {
        input.to_path_buf()
    } else {
        input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Default binary stem: `main` for projects, otherwise the source file stem.
pub fn entry_stem(input: &Path) -> String {
    if input.is_dir() {
        paths::find_main_entry(input)
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "main".into())
    } else {
        input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "program".into())
    }
}

/// Resolve `-o` / `--output` into a file name inside `target/{profile}/`.
fn output_file_name(output: Option<&str>, default_stem: &str, spec: &TargetSpec) -> String {
    let Some(raw) = output else {
        if spec.is_wasm {
            return format!("{default_stem}.wasm");
        }
        return format!("{default_stem}{}", spec.exe_extension());
    };

    let path = Path::new(raw);
    if path.extension().is_some() {
        return path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| raw.to_string());
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(raw);
    if spec.is_wasm {
        format!("{stem}.wasm")
    } else {
        format!("{stem}{}", spec.exe_extension())
    }
}

pub fn layout(
    input: &Path,
    release: bool,
    output: Option<&str>,
    spec: &TargetSpec,
    cdylib: bool,
) -> ArtifactLayout {
    let root = project_root(input);
    let profile = profile_name(release);
    let profile_dir = match spec.artifact_subdir() {
        Some(triple) => root.join("target").join(triple).join(profile),
        None => root.join("target").join(profile),
    };
    let stem = entry_stem(input);
    let mut bin_name = output_file_name(output, &stem, spec);
    if cdylib && !spec.is_wasm {
        let ext = spec.cdylib_extension();
        if !bin_name.ends_with(ext) {
            bin_name.push_str(ext);
        }
    }
    let ll_stem = Path::new(&bin_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&stem);
    ArtifactLayout {
        profile_dir: profile_dir.clone(),
        ll_path: profile_dir.join(format!("{ll_stem}.ll")),
        bin_path: profile_dir.join(bin_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::{TargetArch, TargetFlags, TargetOs, TargetSpec, resolve};
    use std::fs;

    #[test]
    fn project_uses_main_stem() {
        let tmp = std::env::temp_dir().join(format!("nyra_layout_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("main.ny"), "fn main() {}\n").unwrap();

        let spec = TargetSpec::host();
        let layout = layout(&tmp, false, None, &spec, false);
        assert_eq!(layout.profile_dir, tmp.join("target/debug"));
        assert_eq!(layout.ll_path, tmp.join("target/debug/main.ll"));
        assert!(layout.bin_path.ends_with("main") || layout.bin_path.ends_with("main.exe"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn release_profile_dir() {
        let spec = TargetSpec::host();
        let layout = layout(Path::new("."), true, None, &spec, false);
        assert!(layout.profile_dir.ends_with("target/release"));
    }

    #[test]
    fn cross_windows_uses_triple_subdir_and_exe() {
        // Opposite arch from host so layout is cross on every CI runner (incl. Windows ARM64/MSVC
        // where gnu/msvc same-arch triples would otherwise look native).
        let host = TargetSpec::host();
        let (triple, arch) = match host.arch {
            TargetArch::Aarch64 => ("x86_64-pc-windows-msvc", TargetArch::X86_64),
            _ => ("aarch64-pc-windows-msvc", TargetArch::Aarch64),
        };
        let spec = TargetSpec {
            triple: triple.into(),
            os: TargetOs::Windows,
            arch,
            is_cross: true,
            is_wasm: false,
        };
        let layout = layout(Path::new("."), true, None, &spec, false);
        assert_eq!(
            layout.profile_dir,
            project_root(Path::new(".")).join(format!("target/{triple}/release"))
        );
        assert!(layout.bin_path.to_string_lossy().ends_with("main.exe"));
    }

    #[test]
    fn wasm_output_extension() {
        let spec = resolve(&TargetFlags {
            target: Some("wasm32-wasi".into()),
            ..Default::default()
        })
        .unwrap();
        let layout = layout(Path::new("."), false, Some("app.wasm"), &spec, false);
        assert_eq!(
            layout.bin_path,
            project_root(Path::new(".")).join("target/wasm32-wasi/debug/app.wasm")
        );
    }

    #[test]
    fn cross_linux_triple_subdir() {
        let spec = TargetSpec {
            triple: "aarch64-unknown-linux-gnu".into(),
            os: TargetOs::Linux,
            arch: TargetArch::Aarch64,
            is_cross: true,
            is_wasm: false,
        };
        let layout = layout(Path::new("."), false, None, &spec, false);
        assert_eq!(
            layout.profile_dir,
            project_root(Path::new(".")).join("target/aarch64-unknown-linux-gnu/debug")
        );
    }
}
