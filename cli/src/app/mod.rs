pub mod args;
pub(crate) mod session;

use std::path::Path;

use compiler::{set_color_choice, ColorChoice};

use crate::app::args::{Cli, ColorArgs, Commands, InternalCommands, OptFlags, StabilityFlags};
use crate::app::session::{build, compile_and_link, project_root, run_file};
use crate::commands::{bind, check, explain, fmt, ide, pkg, test, toolchain};
use crate::debug;
use crate::target::TargetSpec;
use crate::watch::{self, WatchMode};

pub(crate) fn apply_color_choice(color: &ColorArgs) {
    let choice = match color.color.as_str() {
        "always" => ColorChoice::Always,
        "never" => ColorChoice::Never,
        _ => ColorChoice::Auto,
    };
    set_color_choice(choice);
    crate::ui::set_cli_color(choice);
}

pub(crate) fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Build {
            file,
            output,
            opt,
            debug_symbols,
            cdylib,
            lto_full,
            target_args,
            stability,
            no_std,
            freestanding,
            no_prelude,
        } => build(
            &file,
            output.as_deref(),
            &opt,
            debug_symbols,
            cdylib,
            lto_full,
            &target_args,
            &stability,
            no_std,
            freestanding,
            no_prelude,
        ),
        Commands::Run {
            file,
            opt,
            target_args,
            stability,
            no_std,
            freestanding,
            no_prelude,
        } => run_file(
            &file,
            &opt,
            &target_args,
            &stability,
            no_std,
            freestanding,
            no_prelude,
        ),
        Commands::Lsp => {
            let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            rt.block_on(lsp::run_stdio());
            Ok(())
        }
        Commands::Dap => nyra_dap::run_stdio(),
        Commands::Ide { cmd } => ide::ide_command(cmd),
        Commands::Watch { path, on } => {
            let mode = match on.as_str() {
                "check" => WatchMode::Check,
                "build" => WatchMode::Build,
                "run" => WatchMode::Run,
                other => return Err(format!("unknown watch mode '{other}' (use check, build, run)")),
            };
            watch::watch(&path, mode)
        }
        Commands::Debug {
            path,
            debugger,
            init_vscode,
            args,
        } => debug_cmd(&path, debugger.as_deref(), init_vscode, &args),
        Commands::Check { file, stability } => {
            check::check(&check::path_or_file(&file), &stability)
        }
        Commands::Diag { file, json, stability } => {
            check::diag(&check::path_or_file(&file), json, &stability)
        }
        Commands::Explain { code, list } => {
            explain::explain_cmd(code.as_deref(), list)
        }
        Commands::Test {
            path,
            list_json,
            filter,
            target_args,
            opt,
        } => test::test_dir(&path, &target_args, &opt, list_json, filter.as_deref()),
        Commands::Fmt { path, write, check } => fmt::fmt_path(&path, write, check),
        Commands::Pkg(cmd) => pkg::pkg_command(cmd),
        Commands::Bind(cmd) => bind::bind_command(cmd),
        Commands::Toolchain(cmd) => toolchain::toolchain_command(cmd),
        Commands::Cc {
            target_args,
            print_toolchain,
            verbose,
            clang_args,
        } => crate::cc::run_cc(&target_args, print_toolchain, verbose, &clang_args),
        Commands::Internal { cmd } => match cmd {
            InternalCommands::BuildPrebuiltRt => {
                let spec = TargetSpec::host();
                let path = crate::prebuilt_rt::ensure_prebuilt_runtime(&spec)?;
                println!("{}", path.display());
                let tls = crate::prebuilt_tls::ensure_prebuilt_tls(&spec)?;
                println!("{}", tls.display());
                let tls_native = crate::prebuilt_tls_native::ensure_prebuilt_native_tls(&spec)?;
                println!("{}", tls_native.display());
                Ok(())
            }
            InternalCommands::Daemon { background } => crate::daemon::serve(background),
        },
    }
}

fn debug_cmd(
    path: &Path,
    debugger: Option<&str>,
    init_vscode: bool,
    args: &[String],
) -> Result<(), String> {
    let spec = TargetSpec::host();
    let bin_path = compile_and_link(
        path,
        &OptFlags::default(),
        true,
        false,
        false,
        &spec,
        None,
        &StabilityFlags::default(),
        false,
        false,
        false,
        None,
    )?;
    if init_vscode {
        let root = project_root(path);
        let rel = bin_path
            .strip_prefix(&root)
            .unwrap_or(&bin_path)
            .to_string_lossy();
        let launch = debug::write_vscode_launch(&root, &rel)?;
        println!("wrote {}", launch.display());
    }
    debug::debug_program(&bin_path, args, debugger)
}

