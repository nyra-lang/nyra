use std::path::PathBuf;

use compiler::Compiler;
use pkg::verify_project;

use crate::app::args::{
    PkgBindCommands, PkgCCommands, PkgCommands, PkgSelfCommands, PkgToolchainCommands,
    StabilityFlags,
};
use crate::app::session::build;
use crate::bind::{bind_c, CBindOptions};
use crate::c_lib;
use crate::nyrapkg;
use crate::ui::Ui;

fn delegate_nyrapkg(args: Vec<String>) -> Result<(), String> {
    nyrapkg::run_nyrapkg(&args)
}

pub(crate) fn pkg_command(cmd: PkgCommands) -> Result<(), String> {
    match cmd {
        PkgCommands::Init { path } => {
            let mut args = vec!["init".to_string()];
            if let Some(p) = path {
                args.push(p.display().to_string());
            }
            delegate_nyrapkg(args)
        }
        PkgCommands::Add { module } => delegate_nyrapkg(vec!["add".into(), module]),
        PkgCommands::Install { module } => delegate_nyrapkg(vec!["install".into(), module]),
        PkgCommands::Verify { path } => {
            let mut args = vec!["verify".to_string()];
            if let Some(p) = path {
                args.push(p.display().to_string());
            }
            delegate_nyrapkg(args)
        }
        PkgCommands::Version => delegate_nyrapkg(vec!["version".into()]),
        PkgCommands::Which => delegate_nyrapkg(vec!["which".into()]),
        PkgCommands::Bootstrap => delegate_nyrapkg(vec!["bootstrap".into()]),
        PkgCommands::SelfUpdate { version } => {
            let mut args = vec!["self-update".to_string()];
            if let Some(v) = version {
                args.push(v);
            }
            delegate_nyrapkg(args)
        }
        PkgCommands::SelfCmd { cmd } => match cmd {
            PkgSelfCommands::Update { version } => {
                let mut args = vec!["self".into(), "update".into()];
                if let Some(v) = version {
                    args.push(v);
                }
                delegate_nyrapkg(args)
            }
        },
        PkgCommands::Toolchain { cmd } => match cmd {
            PkgToolchainCommands::Update { version } => {
                let mut args = vec!["toolchain".into(), "update".into()];
                if let Some(v) = version {
                    args.push(v);
                }
                delegate_nyrapkg(args)
            }
        },
        PkgCommands::Update { target, version } => {
            let mut args = vec!["update".into(), target];
            if let Some(v) = version {
                args.push(v);
            }
            delegate_nyrapkg(args)
        }
        PkgCommands::Build { path, opt, target_args } => {
            let dir = path.unwrap_or_else(|| PathBuf::from("."));
            verify_project(&dir)?;
            build(
                &dir,
                None,
                &opt,
                false,
                false,
                false,
                &target_args,
                &StabilityFlags::default(),
                false,
                false,
                false,
            )
        }
        PkgCommands::Bind { cmd } => match cmd {
            PkgBindCommands::C {
                header,
                link_lib,
                include,
                define,
                output,
                prefix,
                export,
                update_mod,
                stdout,
                shim,
                no_shim,
                path,
            } => bind_c(CBindOptions {
                header,
                project: path,
                link_lib,
                include,
                define,
                output,
                prefix,
                export,
                update_mod,
                stdout,
                generate_shims: shim && !no_shim,
            }),
        },
        PkgCommands::C(cmd) => match cmd {
            PkgCCommands::Add {
                name,
                path,
                no_install,
            } => c_lib::c_add(&name, path, no_install),
            PkgCCommands::Remove { name, path } => c_lib::c_remove(&name, path),
            PkgCCommands::List { path } => c_lib::c_list(path),
        },
        PkgCommands::Prune { path, check } => {
            let dir = path.unwrap_or_else(|| PathBuf::from("."));
            verify_project(&dir)?;
            let result = Compiler::prune_project(&dir, check)?;
            let ui = Ui::new();
            if result.files_changed == 0 {
                println!("{}", ui.success("nothing to prune"));
                return Ok(());
            }
            if check {
                println!(
                    "{}  {} file(s), {} import(s), {} variable(s)",
                    ui.success("prune check"),
                    result.files_changed,
                    result.imports_removed,
                    result.vars_prefixed
                );
                return Err("unused code found (run `nyra pkg prune` to apply)".into());
            }
            println!(
                "{}  {} file(s), {} import(s), {} variable(s)",
                ui.success("pruned unused code"),
                result.files_changed,
                result.imports_removed,
                result.vars_prefixed
            );
            Ok(())
        }
    }
}
