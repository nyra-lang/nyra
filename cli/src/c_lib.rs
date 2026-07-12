//! `nyra pkg c add|remove|list` + smart `nyra pkg add` for C libs / GitHub URLs.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bind::{bind_c, CBindOptions};
use crate::c_registry::{self, RegistryEntry};
use crate::ui::Ui;

const MANIFEST: &str = "vendor/bindings/c-libs.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledCLib {
    bindings: String,
    link: String,
    link_search: Vec<String>,
    header: String,
}

/// Options shared by `pkg c add`, smart `pkg add`, and `bind <lib>`.
#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    pub project: Option<PathBuf>,
    pub no_install: bool,
    pub yes: bool,
    /// Override header path (absolute or relative).
    pub header: Option<PathBuf>,
    /// Extra -I dirs.
    pub include: Vec<PathBuf>,
    /// Override link library names (default: registry libs).
    pub libs: Vec<String>,
}

/// True when `nyra pkg add` should handle this as a C lib / git URL (not nyrapkg).
pub fn should_handle_pkg_add(spec: &str) -> bool {
    looks_like_git_url(spec) || c_registry::is_registry_lib(spec)
}

pub fn pkg_add_smart(spec: &str, opts: AddOptions) -> Result<(), String> {
    if looks_like_git_url(spec) {
        return add_from_git(spec, opts);
    }
    c_add(spec, opts)
}

pub fn c_add(name: &str, opts: AddOptions) -> Result<(), String> {
    let root = opts.project.clone().unwrap_or_else(|| PathBuf::from("."));
    let entry = c_registry::find_entry(name)?;
    let key = entry.name.clone();

    let (header, lib_dirs, includes) = resolve_paths(&entry, &opts)?;

    if manifest_get(&root, &key)?.is_some() {
        eprintln!("{}", Ui::new().dim(&format!("refreshing {key} bindings…")));
    }

    let header_rel = entry.primary_header()?;
    let stem = Path::new(header_rel)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.name.clone());
    let bindings_rel = format!("vendor/bindings/{stem}.ny");

    let link_libs = if opts.libs.is_empty() {
        entry.libs.clone()
    } else {
        opts.libs.clone()
    };
    if link_libs.is_empty() {
        return Err(format!("no link libraries for {key}"));
    }

    let mut all_includes = includes;
    all_includes.extend(opts.include.clone());

    bind_c(CBindOptions {
        header: header.clone(),
        project: Some(root.clone()),
        link_lib: link_libs.clone(),
        include: all_includes,
        define: vec![],
        output: Some(root.join(&bindings_rel)),
        prefix: None,
        export: vec![],
        update_mod: true,
        stdout: false,
        generate_shims: false,
    })?;

    let primary = link_libs[0].as_str();
    apply_nyra_mod_links(&root, &link_libs, &lib_dirs)?;
    write_manifest_entry(
        &root,
        &key,
        InstalledCLib {
            bindings: bindings_rel.clone(),
            link: primary.to_string(),
            link_search: lib_dirs.clone(),
            header: header.display().to_string(),
        },
    )?;

    let ui = Ui::new();
    let import = bindings_rel.replace('\\', "/");
    println!("{}", ui.success(&format!("{key} ready")));
    println!("{}", ui.field("import", &format!("\"{import}\"")));
    println!("{}", ui.field("header", &header.display().to_string()));
    println!("{}", ui.field("link", &link_libs.join(", ")));
    if entry.name == "raylib" {
        println!("{}", ui.hint("nyra run .  — see examples/c_raylib/main.ny"));
    }
    Ok(())
}

/// Auto-detect an installed library and bind it (no install by default).
pub fn bind_lib(name: &str, opts: AddOptions) -> Result<(), String> {
    let mut opts = opts;
    opts.no_install = true;
    match c_add(name, opts.clone()) {
        Ok(()) => Ok(()),
        Err(e) if e.contains("not found") || e.contains("not installed") => {
            if opts.yes || prompt_yes_no(&format!("Library '{name}' not found.\n\nInstall it?"))? {
                opts.no_install = false;
                opts.yes = true;
                c_add(name, opts)
            } else {
                Err(e)
            }
        }
        Err(e) => Err(e),
    }
}

pub fn c_remove(name: &str, project: Option<PathBuf>) -> Result<(), String> {
    let root = project.unwrap_or_else(|| PathBuf::from("."));
    let entry = c_registry::find_entry(name)?;
    let key = entry.name.clone();

    let Some(installed) = manifest_remove(&root, &key)? else {
        return Err(format!(
            "c-lib {key} is not installed in this project (no {MANIFEST} entry)"
        ));
    };

    let ui = Ui::new();
    let bindings = root.join(&installed.bindings);
    if bindings.is_file() {
        std::fs::remove_file(&bindings).map_err(|e| e.to_string())?;
        println!(
            "{}  deleted {}",
            ui.yellow("−"),
            ui.path(&bindings.display().to_string())
        );
    }

    let mut links = entry.libs.clone();
    if !links.iter().any(|l| l == &installed.link) {
        links.push(installed.link.clone());
    }
    remove_nyra_mod_links(&root, &links, &installed.link_search)?;

    println!("{}", ui.success(&format!("{key} removed from project")));
    Ok(())
}

pub fn c_list(project: Option<PathBuf>) -> Result<(), String> {
    let root = project.unwrap_or_else(|| PathBuf::from("."));
    let ui = Ui::new();
    let installed = manifest_read_all(&root)?;
    let project = root.display().to_string();

    if installed.is_empty() {
        println!("{}", ui.section("C libraries", &project));
        println!();
        println!("  {}", ui.dim("No C libraries installed"));
        println!();
        println!("  {}  {}", ui.dim("Add one"), ui.cmd("nyra pkg add gsl"));
        println!();
        println!("  {}", ui.bold("Registry"));
        for name in c_registry::list_names()? {
            if let Ok(e) = c_registry::find_entry(&name) {
                let desc = if e.description.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", e.description)
                };
                println!("    {}{}", ui.bold(&name), ui.dim(&desc));
            }
        }
        return Ok(());
    }

    println!("{}", ui.section("C libraries", &project));
    println!();
    for (name, entry) in &installed {
        println!("{}", ui.item(name));
        println!(
            "{}",
            ui.field("import", &format!("\"{}\"", entry.bindings))
        );
        println!("{}", ui.field("link", &entry.link));
        if !entry.header.is_empty() {
            println!("{}", ui.field("header", &entry.header));
        }
        println!();
    }
    let n = installed.len();
    let noun = if n == 1 { "library" } else { "libraries" };
    println!(
        "  {}  ·  add more with {}",
        ui.count(n, noun),
        ui.cmd("nyra pkg add zlib")
    );
    Ok(())
}

// ── path resolution ──────────────────────────────────────────────────────────

fn resolve_paths(
    entry: &RegistryEntry,
    opts: &AddOptions,
) -> Result<(PathBuf, Vec<String>, Vec<PathBuf>), String> {
    let header_rel = entry.primary_header()?;

    if let Some(ref custom) = opts.header {
        let header = if custom.is_absolute() {
            custom.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(custom)
        };
        if !header.is_file() {
            return Err(format!("header not found: {}", header.display()));
        }
        let include_dir = header
            .parent()
            .map(|p| {
                // If header is …/include/gsl/foo.h, prefer …/include
                if p.file_name().and_then(|n| n.to_str()) == Some("gsl")
                    || p.file_name().and_then(|n| n.to_str()) == Some("SDL2")
                    || p.file_name().and_then(|n| n.to_str()) == Some("openssl")
                    || p.file_name().and_then(|n| n.to_str()) == Some("curl")
                {
                    p.parent().unwrap_or(p).to_path_buf()
                } else {
                    p.to_path_buf()
                }
            })
            .unwrap_or_else(|| PathBuf::from("."));
        let mut includes = vec![include_dir];
        includes.extend(clang_system_includes()?);
        return Ok((header, vec![], includes));
    }

    // Prefer pkg-config when available.
    if let Some(pc) = entry.pkg_config.as_deref() {
        if let Ok(resolved) = resolve_via_pkg_config(pc, header_rel) {
            return Ok(resolved);
        }
    }

    if cfg!(target_os = "macos") {
        resolve_macos(entry, opts)
    } else if cfg!(target_os = "linux") {
        resolve_linux(entry, opts)
    } else if cfg!(windows) {
        resolve_windows(entry, opts)
    } else {
        Err("nyra pkg add (C): macOS, Linux, and Windows are supported".into())
    }
}

fn resolve_via_pkg_config(
    pc: &str,
    header_rel: &str,
) -> Result<(PathBuf, Vec<String>, Vec<PathBuf>), String> {
    let cflags = pkg_config_output(&["--cflags-only-I", pc])?;
    let libs = pkg_config_output(&["--libs-only-L", pc])?;

    let mut includes: Vec<PathBuf> = cflags
        .split_whitespace()
        .filter_map(|f| f.strip_prefix("-I").map(PathBuf::from))
        .collect();
    let lib_dirs: Vec<String> = libs
        .split_whitespace()
        .filter_map(|f| f.strip_prefix("-L").map(str::to_string))
        .collect();

    let header = includes
        .iter()
        .map(|d| d.join(header_rel))
        .find(|p| p.is_file())
        .ok_or_else(|| format!("pkg-config {pc}: header {header_rel} not found"))?;

    includes.extend(clang_system_includes()?);
    Ok((header, lib_dirs, includes))
}

fn pkg_config_output(args: &[&str]) -> Result<String, String> {
    let out = Command::new("pkg-config")
        .args(args)
        .output()
        .map_err(|e| format!("pkg-config: {e}"))?;
    if !out.status.success() {
        return Err("pkg-config failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn resolve_macos(
    entry: &RegistryEntry,
    opts: &AddOptions,
) -> Result<(PathBuf, Vec<String>, Vec<PathBuf>), String> {
    let formula = entry.brew_formula();
    let header_rel = entry.primary_header()?;

    let prefix = match brew_prefix(formula) {
        Ok(p) => p,
        Err(_) => {
            ensure_installed(entry, opts)?;
            brew_prefix(formula)?
        }
    };

    let include_dir = prefix.join("include");
    let header = include_dir.join(header_rel);
    if !header.is_file() {
        // Fall back to common search roots.
        if let Some(found) = find_header_in_roots(header_rel, &macos_include_roots()) {
            let mut includes = macos_include_roots();
            includes.extend(clang_system_includes()?);
            let lib_dirs = vec![prefix.join("lib").display().to_string()];
            return Ok((found, lib_dirs, includes));
        }
        return Err(format!(
            "header not found: {} (brew --prefix {})",
            header.display(),
            formula
        ));
    }
    let lib_dirs = vec![prefix.join("lib").display().to_string()];
    let mut includes = vec![include_dir];
    includes.extend(clang_system_includes()?);
    Ok((header, lib_dirs, includes))
}

fn resolve_linux(
    entry: &RegistryEntry,
    opts: &AddOptions,
) -> Result<(PathBuf, Vec<String>, Vec<PathBuf>), String> {
    let header_rel = entry.primary_header()?;
    let roots = linux_include_roots();

    let header = match find_header_in_roots(header_rel, &roots) {
        Some(h) => h,
        None => {
            ensure_installed(entry, opts)?;
            find_header_in_roots(header_rel, &linux_include_roots()).ok_or_else(|| {
                format!(
                    "header not found for {} after install — expected {}",
                    entry.name, header_rel
                )
            })?
        }
    };

    let mut lib_dirs = Vec::new();
    for cand in [
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib64",
        "/usr/lib",
        "/usr/local/lib",
    ] {
        if Path::new(cand).is_dir() {
            lib_dirs.push(cand.to_string());
            break;
        }
    }

    let mut includes = linux_include_roots();
    includes.extend(clang_system_includes()?);
    Ok((header, lib_dirs, includes))
}

fn resolve_windows(
    entry: &RegistryEntry,
    opts: &AddOptions,
) -> Result<(PathBuf, Vec<String>, Vec<PathBuf>), String> {
    let header_rel = entry.primary_header()?;
    let roots = windows_include_roots();
    let header = match find_header_in_roots(header_rel, &roots) {
        Some(h) => h,
        None => {
            let hint = entry
                .apt
                .as_deref()
                .map(|_| "install via vcpkg or MSYS2".to_string())
                .unwrap_or_else(|| format!("install {} development files", entry.name));
            if opts.no_install {
                return Err(format!("header not found for {}; {hint}", entry.name));
            }
            return Err(format!(
                "Library '{}' not found on Windows.\n  {hint}\n  Or: nyra bind c PATH\\TO\\header.h --lib {} --update-mod",
                entry.name,
                entry.primary_link().unwrap_or("lib")
            ));
        }
    };
    let includes = roots;
    let lib_dirs = windows_lib_roots()
        .into_iter()
        .map(|p| p.display().to_string())
        .collect();
    Ok((header, lib_dirs, includes))
}

fn macos_include_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/opt/homebrew/include"),
        PathBuf::from("/usr/local/include"),
    ];
    if let Ok(p) = Command::new("brew").args(["--prefix"]).output() {
        if p.status.success() {
            let prefix = String::from_utf8_lossy(&p.stdout).trim().to_string();
            if !prefix.is_empty() {
                roots.insert(0, PathBuf::from(&prefix).join("include"));
            }
        }
    }
    roots
}

fn linux_include_roots() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/include"),
        PathBuf::from("/usr/local/include"),
    ]
}

fn windows_include_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(vcpkg) = std::env::var("VCPKG_ROOT") {
        roots.push(PathBuf::from(vcpkg).join("installed/x64-windows/include"));
    }
    roots.push(PathBuf::from(r"C:\msys64\mingw64\include"));
    roots.push(PathBuf::from(r"C:\Program Files"));
    roots
}

fn windows_lib_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(vcpkg) = std::env::var("VCPKG_ROOT") {
        roots.push(PathBuf::from(vcpkg).join("installed/x64-windows/lib"));
    }
    roots.push(PathBuf::from(r"C:\msys64\mingw64\lib"));
    roots
}

fn find_header_in_roots(header_rel: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    for root in roots {
        let p = root.join(header_rel);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

// ── install backends ─────────────────────────────────────────────────────────

fn ensure_installed(entry: &RegistryEntry, opts: &AddOptions) -> Result<(), String> {
    if opts.no_install {
        return Err(missing_install_hint(entry));
    }

    let ui = Ui::new();
    let cmd = install_command(entry)?;
    eprintln!();
    eprintln!("{}", ui.bold(&format!("Library '{}' not found.", entry.name)));
    eprintln!();
    eprintln!("  Install with: {}", ui.cmd(&cmd.join(" ")));
    eprintln!();

    let do_install = opts.yes || prompt_yes_no("Install it?")?;
    if !do_install {
        return Err(format!(
            "aborted — install manually: {}",
            cmd.join(" ")
        ));
    }

    eprintln!("{}", ui.dim(&format!("running {} …", cmd.join(" "))));
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .map_err(|e| format!("failed to run {}: {e}", cmd[0]))?;
    if !status.success() {
        return Err(format!("{} failed", cmd.join(" ")));
    }
    Ok(())
}

fn missing_install_hint(entry: &RegistryEntry) -> String {
    install_command(entry)
        .map(|c| format!("{} not installed — run: {}", entry.name, c.join(" ")))
        .unwrap_or_else(|_| format!("{} not installed", entry.name))
}

fn install_command(entry: &RegistryEntry) -> Result<Vec<String>, String> {
    if cfg!(target_os = "macos") {
        return Ok(vec![
            "brew".into(),
            "install".into(),
            entry.brew_formula().into(),
        ]);
    }
    if cfg!(target_os = "linux") {
        if which("apt-get") || which("apt") {
            let pkg = entry
                .apt
                .clone()
                .ok_or_else(|| format!("no apt package mapped for {}", entry.name))?;
            let apt = if which("apt") { "apt" } else { "apt-get" };
            return Ok(vec![
                "sudo".into(),
                apt.into(),
                "install".into(),
                "-y".into(),
                pkg,
            ]);
        }
        if which("pacman") {
            let pkg = entry
                .pacman
                .clone()
                .unwrap_or_else(|| entry.name.clone());
            return Ok(vec![
                "sudo".into(),
                "pacman".into(),
                "-S".into(),
                "--noconfirm".into(),
                pkg,
            ]);
        }
        if which("dnf") {
            let pkg = entry
                .dnf
                .clone()
                .unwrap_or_else(|| format!("{}-devel", entry.name));
            return Ok(vec![
                "sudo".into(),
                "dnf".into(),
                "install".into(),
                "-y".into(),
                pkg,
            ]);
        }
        return Err(format!(
            "no supported package manager found; install {} manually",
            entry.name
        ));
    }
    Err("automatic install is not supported on this OS".into())
}

fn which(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn brew_prefix(formula: &str) -> Result<PathBuf, String> {
    let out = Command::new("brew")
        .args(["--prefix", formula])
        .output()
        .map_err(|e| format!("brew not found: {e}"))?;
    if !out.status.success() {
        return Err(format!("brew --prefix {formula} failed"));
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        return Err(format!("empty prefix for {formula}"));
    }
    // `brew --prefix` succeeds even when the formula is not installed.
    if !Path::new(&path).join("include").is_dir() {
        return Err(format!("{formula} not installed"));
    }
    Ok(PathBuf::from(path))
}

fn prompt_yes_no(question: &str) -> Result<bool, String> {
    if !io::stdin().is_terminal() {
        return Ok(false);
    }
    eprint!("{question} [Y/n] ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("stdin: {e}"))?;
    let t = line.trim().to_ascii_lowercase();
    Ok(t.is_empty() || t == "y" || t == "yes")
}

fn clang_system_includes() -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    if cfg!(target_os = "macos") {
        if let Ok(out) = Command::new("xcrun").args(["--show-sdk-path"]).output() {
            if out.status.success() {
                let sdk = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !sdk.is_empty() {
                    paths.push(PathBuf::from(sdk).join("usr/include"));
                }
            }
        }
    }
    if let Ok(out) = Command::new("brew").args(["--prefix", "llvm"]).output() {
        if out.status.success() {
            let llvm = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !llvm.is_empty() {
                let clang_dir = PathBuf::from(&llvm).join("lib/clang");
                if let Ok(entries) = std::fs::read_dir(&clang_dir) {
                    let mut vers: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                    vers.sort_by_key(|e| e.file_name());
                    if let Some(last) = vers.last() {
                        paths.push(last.path().join("include"));
                    }
                }
            }
        }
    }
    Ok(paths)
}

// ── GitHub / git URL ─────────────────────────────────────────────────────────

pub fn looks_like_git_url(spec: &str) -> bool {
    let s = spec.trim();
    s.starts_with("https://")
        || s.starts_with("http://")
        || s.starts_with("git@")
        || s.starts_with("ssh://")
        || s.ends_with(".git")
}

fn add_from_git(url: &str, opts: AddOptions) -> Result<(), String> {
    let root = opts.project.clone().unwrap_or_else(|| PathBuf::from("."));
    let ui = Ui::new();
    let name = repo_name_from_url(url);
    let src_dir = root.join("vendor/c-src").join(&name);

    if src_dir.is_dir() {
        eprintln!("{}", ui.dim(&format!("using existing {}", src_dir.display())));
    } else {
        eprintln!("{}", ui.dim(&format!("cloning {url} …")));
        std::fs::create_dir_all(src_dir.parent().unwrap()).map_err(|e| e.to_string())?;
        let status = Command::new("git")
            .args(["clone", "--depth", "1", url])
            .arg(&src_dir)
            .status()
            .map_err(|e| format!("git clone failed: {e}"))?;
        if !status.success() {
            return Err(format!("git clone {url} failed"));
        }
    }

    // Prefer nyra.toml in the cloned repo.
    let nyra_toml = src_dir.join("nyra.toml");
    if nyra_toml.is_file() {
        let text = std::fs::read_to_string(&nyra_toml).map_err(|e| e.to_string())?;
        let doc = c_registry::parse_nyra_toml(&text)?;
        if let Some(c) = doc.c {
            return bind_from_nyra_toml(&root, &src_dir, &name, &c, &opts);
        }
    }

    // Heuristic discovery.
    match discover_and_bind(&root, &src_dir, &name, &opts) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!();
            eprintln!("{}", ui.bold("Couldn't determine how to generate bindings."));
            eprintln!();
            eprintln!("  Please specify:");
            eprintln!("    Header:  include/{name}.h   (example)");
            eprintln!("    Library: {name}");
            eprintln!();
            eprintln!(
                "  {}",
                ui.cmd(&format!(
                    "nyra bind c vendor/c-src/{name}/include/{name}.h --lib {name} --update-mod"
                ))
            );
            eprintln!();
            eprintln!(
                "  Or add a {} to the upstream repo:",
                ui.path("nyra.toml")
            );
            eprintln!("    [c]");
            eprintln!("    headers = [\"include/{name}.h\"]");
            eprintln!("    libraries = [\"{name}\"]");
            eprintln!("    include_dirs = [\"include\"]");
            Err(e)
        }
    }
}

fn bind_from_nyra_toml(
    root: &Path,
    src: &Path,
    name: &str,
    c: &c_registry::NyraTomlC,
    _opts: &AddOptions,
) -> Result<(), String> {
    if c.headers.is_empty() || c.libraries.is_empty() {
        return Err("nyra.toml [c] needs headers and libraries".into());
    }
    let header_rel = &c.headers[0];
    let header = src.join(header_rel);
    if !header.is_file() {
        return Err(format!("nyra.toml header not found: {}", header.display()));
    }

    let mut includes: Vec<PathBuf> = c
        .include_dirs
        .iter()
        .map(|d| src.join(d))
        .collect();
    if includes.is_empty() {
        if let Some(parent) = header.parent() {
            includes.push(parent.to_path_buf());
        }
    }
    includes.extend(clang_system_includes()?);

    // Try to build if a build system is present.
    let prefix = root.join("vendor/c-prefix").join(name);
    let _ = try_build_c_project(src, &prefix);

    let mut lib_dirs: Vec<String> = c
        .link_dirs
        .iter()
        .map(|d| src.join(d).display().to_string())
        .collect();
    let built_lib = prefix.join("lib");
    if built_lib.is_dir() {
        lib_dirs.push(built_lib.display().to_string());
    }

    let stem = Path::new(header_rel)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let bindings_rel = format!("vendor/bindings/{stem}.ny");

    bind_c(CBindOptions {
        header: header.clone(),
        project: Some(root.to_path_buf()),
        link_lib: c.libraries.clone(),
        include: includes,
        define: vec![],
        output: Some(root.join(&bindings_rel)),
        prefix: None,
        export: vec![],
        update_mod: true,
        stdout: false,
        generate_shims: false,
    })?;

    apply_nyra_mod_links(root, &c.libraries, &lib_dirs)?;
    write_manifest_entry(
        root,
        name,
        InstalledCLib {
            bindings: bindings_rel.clone(),
            link: c.libraries[0].clone(),
            link_search: lib_dirs,
            header: header.display().to_string(),
        },
    )?;

    let ui = Ui::new();
    println!("{}", ui.success(&format!("{name} ready (from nyra.toml)")));
    println!(
        "{}",
        ui.field("import", &format!("\"{}\"", bindings_rel.replace('\\', "/")))
    );
    Ok(())
}

fn discover_and_bind(
    root: &Path,
    src: &Path,
    name: &str,
    _opts: &AddOptions,
) -> Result<(), String> {
    let prefix = root.join("vendor/c-prefix").join(name);
    try_build_c_project(src, &prefix)?;

    let search_roots = [
        src.join("include"),
        src.to_path_buf(),
        src.join("src"),
        prefix.join("include"),
    ];

    let header = search_roots
        .iter()
        .flat_map(|r| {
            [
                r.join(format!("{name}.h")),
                r.join(format!("lib{name}.h")),
                r.join(name).join(format!("{name}.h")),
            ]
        })
        .find(|p| p.is_file())
        .ok_or_else(|| format!("could not find a header for '{name}' under {}", src.display()))?;

    let include_dir = header.parent().unwrap_or(src).to_path_buf();
    let mut includes = vec![include_dir];
    if src.join("include").is_dir() {
        includes.push(src.join("include"));
    }
    includes.extend(clang_system_includes()?);

    let mut lib_dirs = Vec::new();
    for d in [prefix.join("lib"), src.join("build"), src.join("lib")] {
        if d.is_dir() {
            lib_dirs.push(d.display().to_string());
        }
    }

    let stem = header
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let bindings_rel = format!("vendor/bindings/{stem}.ny");
    let libs = vec![name.to_string()];

    bind_c(CBindOptions {
        header: header.clone(),
        project: Some(root.to_path_buf()),
        link_lib: libs.clone(),
        include: includes,
        define: vec![],
        output: Some(root.join(&bindings_rel)),
        prefix: None,
        export: vec![],
        update_mod: true,
        stdout: false,
        generate_shims: false,
    })?;

    apply_nyra_mod_links(root, &libs, &lib_dirs)?;
    write_manifest_entry(
        root,
        name,
        InstalledCLib {
            bindings: bindings_rel.clone(),
            link: name.to_string(),
            link_search: lib_dirs,
            header: header.display().to_string(),
        },
    )?;

    let ui = Ui::new();
    println!("{}", ui.success(&format!("{name} ready")));
    println!(
        "{}",
        ui.field("import", &format!("\"{}\"", bindings_rel.replace('\\', "/")))
    );
    Ok(())
}

fn try_build_c_project(src: &Path, prefix: &Path) -> Result<(), String> {
    let ui = Ui::new();
    std::fs::create_dir_all(prefix).ok();

    if src.join("CMakeLists.txt").is_file() {
        eprintln!("{}", ui.dim("detected CMake — configuring…"));
        let build = src.join("build-nyra");
        std::fs::create_dir_all(&build).ok();
        let conf = Command::new("cmake")
            .args([
                "-S",
                &src.display().to_string(),
                "-B",
                &build.display().to_string(),
                &format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()),
            ])
            .status();
        if conf.map(|s| s.success()).unwrap_or(false) {
            let _ = Command::new("cmake")
                .args(["--build", &build.display().to_string(), "-j"])
                .status();
            let _ = Command::new("cmake")
                .args([
                    "--install",
                    &build.display().to_string(),
                ])
                .status();
        }
        return Ok(());
    }

    if src.join("meson.build").is_file() && which("meson") {
        eprintln!("{}", ui.dim("detected Meson — configuring…"));
        let build = src.join("build-nyra");
        let _ = Command::new("meson")
            .args([
                "setup",
                &build.display().to_string(),
                &format!("--prefix={}", prefix.display()),
            ])
            .current_dir(src)
            .status();
        let _ = Command::new("meson")
            .args(["compile", "-C", &build.display().to_string()])
            .status();
        let _ = Command::new("meson")
            .args(["install", "-C", &build.display().to_string()])
            .status();
        return Ok(());
    }

    if src.join("configure").is_file() {
        eprintln!("{}", ui.dim("detected ./configure — building…"));
        let _ = Command::new("sh")
            .arg("./configure")
            .arg(format!("--prefix={}", prefix.display()))
            .current_dir(src)
            .status();
        let _ = Command::new("make").arg("-j").current_dir(src).status();
        let _ = Command::new("make").arg("install").current_dir(src).status();
        return Ok(());
    }

    if src.join("Makefile").is_file() || src.join("makefile").is_file() {
        eprintln!("{}", ui.dim("detected Makefile — building…"));
        let _ = Command::new("make").arg("-j").current_dir(src).status();
        return Ok(());
    }

    // Single-header / header-only — nothing to build.
    Ok(())
}

fn repo_name_from_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".git");
    trimmed
        .rsplit('/')
        .next()
        .unwrap_or("c-lib")
        .to_string()
}

// ── manifest + nyra.mod ──────────────────────────────────────────────────────

fn manifest_path(root: &Path) -> PathBuf {
    root.join(MANIFEST)
}

fn manifest_read_all(root: &Path) -> Result<BTreeMap<String, InstalledCLib>, String> {
    let path = manifest_path(root);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    parse_manifest(&text)
}

fn manifest_get(root: &Path, name: &str) -> Result<Option<InstalledCLib>, String> {
    Ok(manifest_read_all(root)?.get(name).cloned())
}

fn write_manifest_entry(root: &Path, name: &str, entry: InstalledCLib) -> Result<(), String> {
    let mut all = manifest_read_all(root)?;
    all.insert(name.to_string(), entry);
    let path = manifest_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, render_manifest(&all)).map_err(|e| e.to_string())?;
    Ok(())
}

fn manifest_remove(root: &Path, name: &str) -> Result<Option<InstalledCLib>, String> {
    let mut all = manifest_read_all(root)?;
    let removed = all.remove(name);
    let path = manifest_path(root);
    if all.is_empty() {
        if path.is_file() {
            std::fs::remove_file(&path).ok();
        }
    } else {
        std::fs::write(&path, render_manifest(&all)).map_err(|e| e.to_string())?;
    }
    Ok(removed)
}

fn parse_manifest(text: &str) -> Result<BTreeMap<String, InstalledCLib>, String> {
    let doc: toml::Table = toml::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
    let mut out = BTreeMap::new();
    for (name, val) in &doc {
        if name == "libs" {
            continue;
        }
        let t = val
            .as_table()
            .ok_or_else(|| format!("{MANIFEST}: [{name}] must be a table"))?;
        let bindings = t
            .get("bindings")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{MANIFEST}: [{name}.bindings]"))?
            .to_string();
        let link = t
            .get("link")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{MANIFEST}: [{name}.link]"))?
            .to_string();
        let header = t
            .get("header")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let link_search = t
            .get("link_search")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        out.insert(
            name.clone(),
            InstalledCLib {
                bindings,
                link,
                link_search,
                header,
            },
        );
    }
    Ok(out)
}

fn render_manifest(libs: &BTreeMap<String, InstalledCLib>) -> String {
    let mut out = String::from(
        "# Managed by `nyra pkg add` / `nyra pkg c add|remove` — system C libraries.\n",
    );
    for (name, e) in libs {
        out.push_str(&format!("\n[{name}]\n"));
        out.push_str(&format!("bindings = \"{}\"\n", e.bindings));
        out.push_str(&format!("link = \"{}\"\n", e.link));
        out.push_str(&format!("header = \"{}\"\n", e.header));
        if !e.link_search.is_empty() {
            out.push_str("link_search = [");
            for (i, p) in e.link_search.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("\"{p}\""));
            }
            out.push_str("]\n");
        }
    }
    out
}

fn apply_nyra_mod_links(root: &Path, links: &[String], search: &[String]) -> Result<(), String> {
    let mod_path = root.join("nyra.mod");
    let mut text = if mod_path.is_file() {
        std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?
    } else {
        "module example.local\n\n".into()
    };

    strip_wrong_include_link_lines(&mut text);

    for link in links {
        let link_line = format!("link {link}");
        if !text.lines().any(|l| l.trim() == link_line) {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&link_line);
            text.push('\n');
        }
    }
    for dir in search {
        let line = format!("link -L {dir}");
        if !text.lines().any(|l| l.trim() == line) {
            text.push_str(&line);
            text.push('\n');
        }
    }

    std::fs::write(&mod_path, text).map_err(|e| e.to_string())?;
    Ok(())
}

fn remove_nyra_mod_links(root: &Path, links: &[String], search: &[String]) -> Result<(), String> {
    let mod_path = root.join("nyra.mod");
    if !mod_path.is_file() {
        return Ok(());
    }
    let mut lines: Vec<String> = std::fs::read_to_string(&mod_path)
        .map_err(|e| e.to_string())?
        .lines()
        .map(str::to_string)
        .collect();

    for link in links {
        let link_line = format!("link {link}");
        lines.retain(|l| l.trim() != link_line);
    }
    for dir in search {
        let line = format!("link -L {dir}");
        lines.retain(|l| l.trim() != line);
    }

    let mut text = lines.join("\n");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    std::fs::write(&mod_path, text).map_err(|e| e.to_string())?;
    Ok(())
}

fn strip_wrong_include_link_lines(text: &mut String) {
    let mut kept = Vec::new();
    for line in text.lines() {
        let trim = line.trim();
        if trim.starts_with("link -L ") {
            let path = trim.trim_start_matches("link -L ").trim();
            if path.contains("/include")
                || path.contains("MacOSX.sdk")
                || path.contains("/lib/clang/")
            {
                continue;
            }
        }
        kept.push(line);
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    *text = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let mut m = BTreeMap::new();
        m.insert(
            "raylib".into(),
            InstalledCLib {
                bindings: "vendor/bindings/raylib.ny".into(),
                link: "raylib".into(),
                link_search: vec!["/opt/homebrew/opt/raylib/lib".into()],
                header: "/opt/homebrew/opt/raylib/include/raylib.h".into(),
            },
        );
        let text = render_manifest(&m);
        let back = parse_manifest(&text).unwrap();
        assert_eq!(back.get("raylib"), m.get("raylib"));
    }

    #[test]
    fn strips_include_link_lines() {
        let mut text = "link raylib\nlink -L /opt/homebrew/opt/raylib/lib\nlink -L /opt/homebrew/opt/raylib/include\n".into();
        strip_wrong_include_link_lines(&mut text);
        assert!(text.contains("link raylib"));
        assert!(text.contains("raylib/lib"));
        assert!(!text.contains("raylib/include"));
    }

    #[test]
    fn detects_git_urls() {
        assert!(looks_like_git_url("https://github.com/someone/cool-library"));
        assert!(looks_like_git_url("git@github.com:org/repo.git"));
        assert!(!looks_like_git_url("gsl"));
        assert!(!looks_like_git_url("github.com/org/repo"));
    }

    #[test]
    fn repo_name_parsing() {
        assert_eq!(
            repo_name_from_url("https://github.com/someone/cool-library"),
            "cool-library"
        );
        assert_eq!(
            repo_name_from_url("https://github.com/someone/cool-library.git"),
            "cool-library"
        );
    }

    #[test]
    fn smart_pkg_add_routing() {
        assert!(should_handle_pkg_add("gsl"));
        assert!(should_handle_pkg_add("https://github.com/a/b"));
        assert!(!should_handle_pkg_add("some.nyra.package"));
    }
}
