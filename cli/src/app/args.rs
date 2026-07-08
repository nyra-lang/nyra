//! Clap argument definitions for the `nyra` CLI.

use std::path::PathBuf;

use clap::{Args, Parser as ClapParser, Subcommand};
use compiler::NYRA_VERSION;
use crate::link::LinkProfile;
use crate::target::{TargetFlags, TargetSpec, resolve};

#[derive(Args, Clone, Default)]
pub(crate) struct ColorArgs {
    /// When to colorize diagnostics: auto, always, or never (also respects NO_COLOR).
    #[arg(long, value_name = "WHEN", default_value = "auto", global = true)]
    pub(crate) color: String,
}

#[derive(ClapParser)]
#[command(name = "nyra", about = "Nyra programming language toolchain", version = NYRA_VERSION)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) color: ColorArgs,
    #[command(subcommand)]
    pub(crate) command: Commands,
}

/// Optimization flags shared by `build` and `run`.
#[derive(Args, Clone, Default)]
pub(crate) struct OptFlags {
    /// Full automated PGO: instrument → train (main + tests) → merge → optimized rebuild.
    /// Caches `target/release/pgo/nyra.profdata` when sources are unchanged. Implies `--release`.
    #[arg(long, conflicts_with_all = ["pgo_generate", "pgo_use"])]
    pub(crate) pgo: bool,
    /// Argument passed to the instrumented binary during PGO training (repeatable).
    #[arg(long = "pgo-arg", value_name = "ARG", requires = "pgo")]
    pub(crate) pgo_arg: Vec<String>,
    /// Max seconds for each PGO training run before aborting (default 120).
    #[arg(long = "pgo-timeout", value_name = "SECS", default_value_t = 120, requires = "pgo")]
    pub(crate) pgo_timeout: u64,
    /// Release build: -O3, LLVM IR opt, thin LTO (overrides default -O0).
    #[arg(long)]
    pub(crate) release: bool,
    /// Clang optimization level 0–3 (default: 3 with --release, else 0).
    #[arg(long, value_name = "LEVEL")]
    pub(crate) opt: Option<u8>,
    /// Enable thin link-time optimization (-flto=thin).
    #[arg(long)]
    pub(crate) lto: bool,
    /// Disable LTO even with --release.
    #[arg(long)]
    pub(crate) no_lto: bool,
    /// Skip the LLVM `opt` pass on generated IR.
    #[arg(long)]
    pub(crate) no_llvm_opt: bool,
    /// Clang PGO: generate profile data in this binary.
    #[arg(long, conflicts_with = "pgo")]
    pub(crate) pgo_generate: bool,
    /// Clang PGO: use profile from this file (e.g. default.profdata).
    #[arg(long, value_name = "FILE", conflicts_with = "pgo")]
    pub(crate) pgo_use: Option<PathBuf>,
    /// Tune for the host CPU (-march=native; not portable). Default on `--release` for host builds.
    #[arg(long)]
    pub(crate) native_cpu: bool,
    /// Disable `-march=native` even with `--release` (portable release artifacts).
    #[arg(long)]
    pub(crate) no_native_cpu: bool,
    /// Print compiler diagnostics including escape analysis (`Variable x → NoEscape`).
    #[arg(long, short = 'v')]
    pub(crate) verbose: bool,
    /// Print analysis / codegen / link timings after build or run.
    #[arg(long)]
    pub(crate) timings: bool,
    /// Prefer a running `nyra internal daemon` when its socket is present.
    #[arg(long)]
    pub(crate) use_daemon: bool,
    /// Do not connect to a running compiler daemon.
    #[arg(long, conflicts_with = "use_daemon")]
    pub(crate) no_daemon: bool,
    /// Link native library `-lfoo` (repeatable).
    #[arg(long = "link-lib", value_name = "LIB")]
    pub(crate) link_lib: Vec<String>,
    /// Library search path `-Ldir` (repeatable).
    #[arg(long = "link-search-path", value_name = "DIR")]
    pub(crate) link_search_path: Vec<PathBuf>,
    /// Raw linker argument passed to clang (repeatable).
    #[arg(long = "link-arg", value_name = "ARG")]
    pub(crate) link_arg: Vec<String>,
    /// Enable ThreadSanitizer for data-race detection (`-fsanitize=thread`).
    #[arg(long, conflicts_with_all = ["race_native", "sanitize"])]
    pub(crate) race: bool,
    /// Enable native Nyra race runtime (`stdlib/rt/rt_race.c`) — lightweight lock-set detector.
    #[arg(long, conflicts_with_all = ["race", "sanitize"])]
    pub(crate) race_native: bool,
    /// Enable AddressSanitizer (`-fsanitize=address`) for heap corruption detection.
    #[arg(long, conflicts_with_all = ["race", "race_native"])]
    pub(crate) sanitize: bool,
}


impl OptFlags {
    pub(crate) fn link_profile(&self, is_cross: bool) -> Result<LinkProfile, String> {
        let native_cpu = self.native_cpu || (self.release && !is_cross && !self.no_native_cpu);
        Ok(LinkProfile::from_cli(
            self.release,
            self.opt,
            self.lto,
            self.no_lto,
            self.no_llvm_opt,
            self.pgo_generate,
            self.pgo_use.clone(),
            native_cpu,
        )?
        .with_race(self.race)
        .with_race_native(self.race_native)
        .with_sanitize(self.sanitize))
    }
}

/// Cross-compilation target selection (`--for`, `--os`, `--arch`, `--target`).
#[derive(Args, Clone, Default)]
pub(crate) struct TargetArgs {
    /// Build for OS: windows, linux, macos, wasm (easy alias).
    #[arg(long = "for", value_name = "OS")]
    pub(crate) for_os: Option<String>,
    /// Target OS (same as --for, for explicit use with --arch).
    #[arg(long, value_name = "OS")]
    pub(crate) os: Option<String>,
    /// Target CPU: x86_64 or aarch64 (default: host arch).
    #[arg(long, value_name = "ARCH")]
    pub(crate) arch: Option<String>,
    /// Full LLVM target triple (overrides --for / --os).
    #[arg(long, default_value = "")]
    pub(crate) target: String,
}


impl TargetArgs {
    pub(crate) fn resolve(&self) -> Result<TargetSpec, String> {
        resolve(&TargetFlags {
            for_os: self.for_os.clone(),
            os: self.os.clone(),
            arch: self.arch.clone(),
            target: if self.target.is_empty() {
                None
            } else {
                Some(self.target.clone())
            },
        })
    }
}

/// Stability flags (Nyra v1.0 Core vs Extended policy).
#[derive(Args, Clone, Default)]
pub(crate) struct StabilityFlags {
    /// Reject Extended-tier features (async, traits, macros, spawn, defer, explicit lifetimes).
    #[arg(long)]
    pub(crate) deny_extended: bool,
    /// Treat warnings as errors (unused imports, unused variables, etc.).
    #[arg(long)]
    pub(crate) deny_warnings: bool,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    Build {
        #[arg(default_value = ".")]
        file: PathBuf,
        /// Binary name (or `name.wasm`) under `target/debug` or `target/release`.
        #[arg(short, long)]
        output: Option<String>,
        #[command(flatten)]
        opt: OptFlags,
        #[arg(long)]
        debug_symbols: bool,
        #[arg(long)]
        cdylib: bool,
        /// Full LTO instead of thin (slower link, sometimes faster binary).
        #[arg(long)]
        lto_full: bool,
        #[command(flatten)]
        target_args: TargetArgs,
        #[command(flatten)]
        stability: StabilityFlags,
        /// no_std mode: no runtime stdlib (use with --freestanding for kernels/embedded).
        #[arg(long)]
        no_std: bool,
        /// Freestanding link: skip nyra_rt, pass -ffreestanding -nostdlib to clang.
        #[arg(long)]
        freestanding: bool,
        /// Skip auto-merging the full stdlib prelude (smaller/faster builds; add explicit imports as needed).
        #[arg(long)]
        no_prelude: bool,
    },
    Run {
        #[arg(default_value = ".")]
        file: PathBuf,
        #[command(flatten)]
        opt: OptFlags,
        #[command(flatten)]
        target_args: TargetArgs,
        #[command(flatten)]
        stability: StabilityFlags,
        #[arg(long)]
        no_std: bool,
        #[arg(long)]
        freestanding: bool,
        #[arg(long)]
        no_prelude: bool,
    },
    Check {
        #[arg(default_value = ".")]
        file: PathBuf,
        #[command(flatten)]
        stability: StabilityFlags,
        /// Print per-binding ownership summary at function exit (compile-time).
        #[arg(long = "ownership-verbose")]
        ownership_verbose: bool,
    },
    /// Print compiler diagnostics (for editors; JSON with --json).
    Diag {
        #[arg(default_value = ".")]
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        stability: StabilityFlags,
    },
    /// Inspect compile-time ownership of a binding at a source location.
    Inspect {
        /// Binding name to inspect (e.g. `name`).
        name: String,
        /// Source location as `file:line` (e.g. `main.ny:42`).
        #[arg(long, value_name = "FILE:LINE")]
        at: String,
        #[arg(default_value = ".")]
        project: PathBuf,
        #[command(flatten)]
        stability: StabilityFlags,
    },
    /// Explain a stable diagnostic code (e.g. E003, P001).
    Explain {
        /// Diagnostic code (E001, P001, W002, L001, …).
        code: Option<String>,
        /// List all known diagnostic codes.
        #[arg(long)]
        list: bool,
    },
    Test {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// List discovered tests as JSON (for IDE test explorer).
        #[arg(long = "list-json")]
        list_json: bool,
        /// Run only tests whose name contains this substring.
        #[arg(long)]
        filter: Option<String>,
        #[command(flatten)]
        target_args: TargetArgs,
        #[command(flatten)]
        opt: OptFlags,
    },
    Fmt {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        write: bool,
        /// Exit with error if any file would change (CI gate).
        #[arg(long)]
        check: bool,
    },
    /// Package manager (v0.4)
    #[command(subcommand)]
    Pkg(PkgCommands),
    /// Generate Rust crate bindings (C-ABI wrapper + `.ny` stubs).
    #[command(subcommand)]
    Bind(BindCommands),
    /// Native LLVM/clang toolchain (install under $NYRA_HOME — zig cc–style layout).
    #[command(subcommand)]
    Toolchain(ToolchainCommands),
    /// Language Server Protocol (stdio)
    Lsp,
    /// Debug Adapter Protocol (stdio) — use with VS Code `type: nyra`.
    Dap,
    /// IDE helpers (goto-definition, find-references) for scripts/CI.
    Ide {
        #[command(subcommand)]
        cmd: IdeCommands,
    },
    /// Rebuild on file changes (check, build, or run).
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Action on change: check (default), build, or run.
        #[arg(long, default_value = "check")]
        on: String,
    },
    /// Build with debug symbols and launch lldb/gdb.
    Debug {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Debugger executable (lldb or gdb); auto-detected if omitted.
        #[arg(long)]
        debugger: Option<String>,
        /// Write `.vscode/launch.json` + `tasks.json` for the project.
        #[arg(long)]
        init_vscode: bool,
        /// Arguments passed to the program under debug.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// C/C++ compiler driver (forwards to LLVM clang — zig cc–style foundation).
    ///
    /// Examples:
    ///   nyra cc -c vendor/shim.c -o vendor/shim.o
    ///   nyra cc --for wasm -c app.c -o app.o
    ///   nyra cc --print-toolchain
    ///   CC=nyra cc make
    Cc {
        #[command(flatten)]
        target_args: TargetArgs,
        /// Print discovered clang/opt/lld paths and exit.
        #[arg(long)]
        print_toolchain: bool,
        /// Echo the full clang command before running.
        #[arg(long, short = 'v')]
        verbose: bool,
        /// Arguments forwarded to clang (use `--` if an arg starts with `-`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        clang_args: Vec<String>,
    },
    /// Internal toolchain hooks (not stable).
    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        cmd: InternalCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum InternalCommands {
    /// Build O0 prebuilt runtime archive for fast debug links.
    BuildPrebuiltRt,
    /// Keep compiler warm on a Unix socket (dev speed).
    Daemon {
        /// Run in foreground (default). Use `--background` to detach.
        #[arg(long)]
        background: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum IdeCommands {
    /// Print definition location: file:line:col
    GotoDef {
        file: PathBuf,
        line: u32,
        #[arg(long, default_value = "0")]
        character: u32,
    },
    /// Print all reference locations (JSON)
    References {
        file: PathBuf,
        line: u32,
        #[arg(long, default_value = "0")]
        character: u32,
    },
}

#[derive(Subcommand)]
pub(crate) enum ToolchainCommands {
    /// Link or download LLVM/clang into $NYRA_HOME/lib/llvm (writes ~/env).
    Install {
        /// Download official LLVM release instead of symlinking system LLVM.
        #[arg(long)]
        download: bool,
        /// Also install WASI sysroot under $NYRA_HOME/lib/sysroot/wasi.
        #[arg(long)]
        wasi: bool,
    },
    /// Print clang/opt/lld paths and Nyra home.
    Info,
}

#[derive(Subcommand)]
pub(crate) enum BindCommands {
    /// Bind a crates.io crate: `nyra bind rust uuid` or `nyra bind rust serde_json@^1.0`
    Rust {
        /// Crate name (e.g. uuid, regex, serde_json).
        crate_name: String,
        /// Project root (default: current directory).
        #[arg(long)]
        project: Option<PathBuf>,
        /// Semver requirement (e.g. ^1.0.0).
        #[arg(long)]
        version: Option<String>,
        /// Bind only these symbols (repeatable), e.g. `--export Regex::new --export is_match`.
        #[arg(long = "export", value_name = "SYM")]
        export: Vec<String>,
        /// Skip syn bindgen; use hand-written template (uuid, serde_json only).
        #[arg(long)]
        template: bool,
    },
    /// Bind a C header via libclang: `nyra bind c sqlite3.h --lib sqlite3`
    C {
        /// C header path (e.g. /usr/include/zlib.h or vendor/foo.h).
        header: PathBuf,
        /// Project root (default: current directory).
        #[arg(long)]
        project: Option<PathBuf>,
        /// Native library for nyra.mod (`link sqlite3`, repeatable).
        #[arg(long = "lib", value_name = "NAME")]
        link_lib: Vec<String>,
        /// Include search path (`-I`, repeatable).
        #[arg(long = "include", short = 'I', value_name = "DIR")]
        include: Vec<PathBuf>,
        /// Preprocessor define (`-D`, repeatable).
        #[arg(long = "define", short = 'D', value_name = "MACRO")]
        define: Vec<String>,
        /// Output `.ny` file (default: vendor/bindings/{header-stem}.ny).
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Only functions whose name starts with this prefix.
        #[arg(long)]
        prefix: Option<String>,
        /// Optional: bind only these symbols (repeatable; `sqlite3_*` trailing `*`).
        /// Default: emit every bindable function from the header.
        #[arg(long = "export", value_name = "SYM")]
        export: Vec<String>,
        /// Append `link` lines to nyra.mod when missing.
        #[arg(long)]
        update_mod: bool,
        /// Print bindings to stdout instead of writing a file.
        #[arg(long)]
        stdout: bool,
        /// Generate C shims for complex signatures (experimental; off by default).
        #[arg(long)]
        shim: bool,
        /// Skip C shims even when `--shim` is set.
        #[arg(long)]
        no_shim: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum PkgCommands {
    /// Scaffold `nyra.mod` + `main.ny` (delegates to `nyrapkg`).
    Init {
        path: Option<PathBuf>,
    },
    /// Add a dependency (delegates to `nyrapkg`).
    Add {
        module: String,
    },
    /// Fetch package and update lock files (delegates to `nyrapkg`).
    /// Omit `module` to sync existing `require` lines only (`nyrapkg install` / `sync`).
    Install {
        module: Option<String>,
    },
    /// Fetch all `require` lines from `nyra.mod` and rewrite lock files (delegates to `nyrapkg`).
    Sync {
        path: Option<PathBuf>,
    },
    /// Verify lock files and checksums (delegates to `nyrapkg`).
    Verify {
        path: Option<PathBuf>,
    },
    /// Print nyrapkg and nyra versions (delegates to `nyrapkg`).
    Version,
    /// Show install paths (delegates to `nyrapkg`).
    Which,
    /// Install this nyrapkg binary to `~/.nyra/bin` (delegates to `nyrapkg`).
    Bootstrap,
    /// Update nyrapkg from GitHub releases (delegates to `nyrapkg`).
    #[command(name = "self-update")]
    SelfUpdate {
        version: Option<String>,
    },
    /// nyrapkg self-management (delegates to `nyrapkg`).
    #[command(name = "self")]
    SelfCmd {
        #[command(subcommand)]
        cmd: PkgSelfCommands,
    },
    /// Toolchain helpers (delegates to `nyrapkg`).
    Toolchain {
        #[command(subcommand)]
        cmd: PkgToolchainCommands,
    },
    /// Update nyra or nyrapkg (delegates to `nyrapkg`).
    Update {
        target: String,
        version: Option<String>,
    },
    Build {
        path: Option<PathBuf>,
        #[command(flatten)]
        opt: OptFlags,
        #[command(flatten)]
        target_args: TargetArgs,
    },
    /// Generate bindings into the current package (`vendor/bindings/`).
    Bind {
        #[command(subcommand)]
        cmd: PkgBindCommands,
    },
    /// System C libraries — one command to add/remove (raylib, zlib, sqlite3, …).
    #[command(subcommand, visible_alias = "clib")]
    C(PkgCCommands),
    /// Remove unused imports and prefix unused locals (W002/W003 lints).
    Prune {
        /// Project root (default: current directory).
        path: Option<PathBuf>,
        /// Report what would change without editing files.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum PkgSelfCommands {
    /// Update nyrapkg from GitHub releases.
    Update {
        version: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum PkgToolchainCommands {
    /// Update the Nyra compiler under `~/.nyra`.
    Update {
        version: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum PkgCCommands {
    /// Add a system C library: install (Homebrew/apt), bind header, update nyra.mod.
    Add {
        /// Library name (raylib, zlib, sqlite3, sdl2, …).
        name: String,
        /// Project root (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Do not run brew/apt install if the library is missing.
        #[arg(long)]
        no_install: bool,
    },
    /// Remove a C library from this project (bindings + nyra.mod lines).
    Remove {
        name: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// List C libraries installed in this project.
    List {
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub(crate) enum PkgBindCommands {
    /// Bind a C header via libclang (same as `nyra bind c`).
    C {
        /// C header path (e.g. vendor/api.h or /usr/include/zlib.h).
        header: PathBuf,
        /// Native library for nyra.mod (`link sqlite3`, repeatable).
        #[arg(long = "lib", value_name = "NAME")]
        link_lib: Vec<String>,
        /// Include search path (`-I`, repeatable).
        #[arg(long = "include", short = 'I', value_name = "DIR")]
        include: Vec<PathBuf>,
        /// Preprocessor define (`-D`, repeatable).
        #[arg(long = "define", short = 'D', value_name = "MACRO")]
        define: Vec<String>,
        /// Output `.ny` file (default: vendor/bindings/{header-stem}.ny).
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Only functions whose name starts with this prefix.
        #[arg(long)]
        prefix: Option<String>,
        /// Optional: bind only these symbols (repeatable; `sqlite3_*` trailing `*`).
        /// Default: emit every bindable function from the header.
        #[arg(long = "export", value_name = "SYM")]
        export: Vec<String>,
        /// Append `link` / `link-source` lines to nyra.mod when missing.
        #[arg(long)]
        update_mod: bool,
        /// Print bindings to stdout instead of writing a file.
        #[arg(long)]
        stdout: bool,
        /// Generate C shims for complex signatures (experimental; off by default).
        #[arg(long)]
        shim: bool,
        /// Skip C shims even when `--shim` is set.
        #[arg(long)]
        no_shim: bool,
        /// Project root (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
    },
}
