//! `nyra bind rust` / `nyra bind c` — generate FFI bindings.

use std::path::PathBuf;

use nyra_c_bindgen::BindConfig;
use pkg::{
    bind_rust_crate_with_options, parse_req, parse_rust_module, BindOptions,
};

use crate::ui::Ui;

pub struct CBindOptions {
    pub header: PathBuf,
    pub project: Option<PathBuf>,
    pub link_lib: Vec<String>,
    pub include: Vec<PathBuf>,
    pub define: Vec<String>,
    pub output: Option<PathBuf>,
    pub prefix: Option<String>,
    pub export: Vec<String>,
    pub update_mod: bool,
    pub stdout: bool,
    pub generate_shims: bool,
}

pub fn bind_rust(
    crate_spec: &str,
    project: Option<PathBuf>,
    version: Option<String>,
    exports: Option<Vec<String>>,
    force_template: bool,
) -> Result<(), String> {
    let project_root = project.unwrap_or_else(|| PathBuf::from("."));
    let (name, ver_inline) = crate_spec.split_once('@').unwrap_or((crate_spec, ""));
    let crate_name = name.trim();
    if crate_name.is_empty() {
        return Err("crate name required".into());
    }
    if parse_rust_module(&format!("rust::{crate_name}")).is_none() && crate_name.contains("::") {
        return Err(format!("expected crate name (e.g. uuid), got '{crate_name}'"));
    }
    let version_req = if let Some(v) = version {
        Some(parse_req(&v)?)
    } else if !ver_inline.trim().is_empty() {
        Some(parse_req(ver_inline.trim())?)
    } else {
        None
    };
    let options = BindOptions {
        export_filter: exports,
        force_template,
    };
    let meta = bind_rust_crate_with_options(
        &project_root,
        crate_name,
        version_req.as_ref(),
        &options,
    )?;
    let ui = Ui::new();
    println!("{}", ui.success(&format!("bound rust::{crate_name} {}", meta.version)));
    println!("{}", ui.field("mode", &meta.mode));
    println!("{}", ui.field("import", &format!("\"rust/{crate_name}\"")));
    println!("{}", ui.field("link", &format!("link-crate {crate_name}")));
    Ok(())
}

pub fn bind_c(opts: CBindOptions) -> Result<(), String> {
    let project_root = opts.project.unwrap_or_else(|| PathBuf::from("."));
    let header = if opts.header.is_absolute() {
        opts.header
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(opts.header)
    };
    let mut includes = opts.include;
    for p in crate::c_lib::system_includes().unwrap_or_default() {
        if !includes.iter().any(|x| x == &p) {
            includes.push(p);
        }
    }
    let config = BindConfig {
        header: header.clone(),
        includes,
        defines: opts.define,
        link_libs: opts.link_lib,
        function_prefix: opts.prefix,
        export_filter: opts.export,
        output: opts.output.clone(),
        update_mod: opts.update_mod,
        generate_shims: opts.generate_shims,
    };

    if opts.stdout {
        let gen = nyra_c_bindgen::bind_header(&config)?;
        print!("{}", gen.bindings_ny);
        if !gen.mod_lines.is_empty() {
            eprintln!("// nyra.mod hints:");
            for line in &gen.mod_lines {
                eprintln!("// {line}");
            }
        }
        eprintln!(
            "{}  {} function(s) from {} (shims {}, skipped {})",
            Ui::new().dim("bind c"),
            gen.functions,
            header.display(),
            gen.shims,
            gen.skipped
        );
        return Ok(());
    }

    let out = opts.output.unwrap_or_else(|| {
        let stem = header
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "bindings".into());
        project_root.join(format!("vendor/bindings/{stem}.ny"))
    });
    let shim_rel = config.shim_source_path();
    let gen = nyra_c_bindgen::bind_header_to_project(
        &BindConfig {
            output: Some(out.clone()),
            ..config
        },
        &project_root,
    )?;
    let ui = Ui::new();
    let import = out
        .strip_prefix(&project_root)
        .unwrap_or(&out)
        .to_string_lossy();
    println!(
        "{}  {} function(s)",
        ui.success("C bindings generated"),
        gen.functions
    );
    println!("{}", ui.field("output", &out.display().to_string()));
    println!("{}", ui.field("import", &format!("\"{import}\"")));
    if gen.shims > 0 {
        let shim = project_root.join(shim_rel);
        println!("{}", ui.field("shims", &shim.display().to_string()));
    }
    if !gen.mod_lines.is_empty() {
        println!("{}", ui.field("nyra.mod", &gen.mod_lines.join(", ")));
    }
    if gen.skipped > 0 {
        eprintln!(
            "{}  skipped {} symbol(s) (unsupported C types)",
            ui.dim("note"),
            gen.skipped
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_accepts_unknown_crate_with_bindgen() {
        // bindgen path — fails at fetch/network, not at template check
        let err = bind_rust("!!!invalid-crate-name!!!", None, None, None, false);
        assert!(err.is_err());
    }
}
