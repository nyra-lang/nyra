use crate::app::args::BindCommands;
use crate::bind::{bind_c, CBindOptions};
use crate::c_lib::{self, AddOptions};
use std::path::PathBuf;

pub(crate) fn bind_command(cmd: BindCommands) -> Result<(), String> {
    match cmd {
        BindCommands::Rust {
            crate_name,
            project,
            version,
            export,
            template,
        } => crate::bind::bind_rust(
            &crate_name,
            project,
            version,
            if export.is_empty() {
                None
            } else {
                Some(export)
            },
            template,
        ),
        BindCommands::C {
            header,
            project,
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
        } => bind_c(CBindOptions {
            header,
            project,
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
        BindCommands::Lib(args) => {
            // `nyra bind gsl` / `nyra bind gsl -y --header /path/h.h`
            let name = args
                .first()
                .ok_or_else(|| "usage: nyra bind <library>  (e.g. nyra bind gsl)".to_string())?
                .clone();
            let mut opts = AddOptions::default();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "-y" | "--yes" => opts.yes = true,
                    "--no-install" => opts.no_install = true,
                    "--header" => {
                        i += 1;
                        let p = args
                            .get(i)
                            .ok_or_else(|| "--header needs a path".to_string())?;
                        opts.header = Some(PathBuf::from(p));
                    }
                    "-I" | "--include" => {
                        i += 1;
                        let p = args
                            .get(i)
                            .ok_or_else(|| "--include needs a directory".to_string())?;
                        opts.include.push(PathBuf::from(p));
                    }
                    "--lib" => {
                        i += 1;
                        let p = args
                            .get(i)
                            .ok_or_else(|| "--lib needs a name".to_string())?;
                        opts.libs.push(p.clone());
                    }
                    "--project" | "--path" => {
                        i += 1;
                        let p = args
                            .get(i)
                            .ok_or_else(|| "--project needs a path".to_string())?;
                        opts.project = Some(PathBuf::from(p));
                    }
                    other => {
                        return Err(format!(
                            "unknown flag '{other}' — try: nyra bind {name} [-y] [--header PATH]"
                        ));
                    }
                }
                i += 1;
            }
            c_lib::bind_lib(&name, opts)
        }
    }
}
