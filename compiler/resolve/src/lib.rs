pub mod paths;
pub mod prelude;
pub mod merge;
pub mod parse_cache;
pub mod sources;
pub mod stdlib;
pub mod symbols;
pub mod unit_map;
pub use paths::*;
pub use sources::collect_source_files;
pub use stdlib::{
    collect_stdlib_sources, collect_stdlib_sources_near, resolve_pkg_import,
    resolve_stdlib_import, resolve_stdlib_import_near, stdlib_roots, stdlib_roots_near,
};
pub use symbols::{collect_program_uses, top_level_export_names};
pub use unit_map::{assign_functions_to_units, collect_source_units, SourceUnit};

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ast::{ImportDecl, Program};
use errors::{
    eprint_diagnostics_suppressed, finalize_lexer_diagnostics, finalize_parser_diagnostics,
    set_diagnostic_root, COMPILE_FAILED, ErrorKind, NyraError,
};
use lexer::Lexer;
use parser::Parser;

pub(crate) fn project_root_for(path: &Path) -> PathBuf {
    let mut dir = if path.is_file() {
        path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        path.to_path_buf()
    };
    loop {
        if dir.join("nyra.mod").exists() || crate::paths::has_main_entry(&dir) {
            return dir;
        }
        if !dir.pop() {
            return path.parent().unwrap_or(Path::new(".")).to_path_buf();
        }
    }
}

pub struct LoadOutput {
    pub program: Program,
    pub errors: Vec<NyraError>,
}

/// Controls whether the compiler merges stdlib on demand before typecheck.
#[derive(Debug, Clone, Copy)]
pub struct LoadOptions {
    /// When true (default), referenced stdlib symbols are merged unless the entry sets `no_std`.
    pub auto_prelude: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            auto_prelude: true,
        }
    }
}

impl LoadOptions {
    pub fn no_prelude() -> Self {
        Self {
            auto_prelude: false,
        }
    }
}

pub fn load_program(entry: &Path) -> Result<LoadOutput, String> {
    load_program_with_options(entry, LoadOptions::default())
}

pub fn load_program_with_options(
    entry: &Path,
    options: LoadOptions,
) -> Result<LoadOutput, String> {
    let entry = entry.canonicalize().map_err(|e| e.to_string())?;
    let mut visited = HashSet::new();
    let mut errors = Vec::new();
    let mut program = load_file_recursive(&entry, &mut visited, &mut errors)?;
    if options.auto_prelude && !program.no_std && !program.comptime {
        prelude::inject_lazy_stdlib_prelude(&entry, &mut program, &mut visited, &mut errors)?;
    }
    Ok(LoadOutput { program, errors })
}

/// Parse a single file without resolving imports (for linting and analysis).
pub fn parse_file_only(path: &Path) -> Result<Program, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let hash = parse_cache::content_hash(&source);
    if let Some(cached) = parse_cache::get(&path, hash) {
        return Ok(cached);
    }
    let file = path.to_string_lossy().into_owned();
    let (tokens, lexer_errors) = Lexer::new(&source, &file).tokenize();
    if !lexer_errors.is_empty() {
        return Err(format!("Lexer errors in {}", path.display()));
    }
    let (program, parser_errors) = Parser::new(tokens).parse();
    if !parser_errors.is_empty() {
        return Err(format!("Parser errors in {}", path.display()));
    }
    parse_cache::insert(&path, hash, program.clone());
    Ok(program)
}

pub(crate) fn load_file_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    errors: &mut Vec<NyraError>,
) -> Result<Program, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    set_diagnostic_root(project_root_for(&path));
    if !visited.insert(path.clone()) {
        return Ok(Program {
            module: None,
            no_std: false,
            comptime: false,
            allow_extended: false,
            imports: vec![],
            consts: vec![],
            structs: vec![],
            unions: vec![],
            enums: vec![],
            traits: vec![],
            trait_impls: vec![],
            macros: vec![],
            impls: vec![],
            externs: vec![],
            functions: vec![],
            export_instances: vec![],
        });
    }

    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let hash = parse_cache::content_hash(&source);
    if let Some(cached) = parse_cache::get(&path, hash) {
        let mut merged = cached;
        let base_dir = path.parent().unwrap_or(Path::new("."));
        let imports: Vec<ImportDecl> = std::mem::take(&mut merged.imports);
        for imp in imports {
            match resolve_import_path(base_dir, &imp.path) {
                Ok(resolved) => {
                    let sub = load_file_recursive(&resolved, visited, errors)?;
                    merge::merge_program(&mut merged, sub, imp.alias.as_deref());
                }
                Err(_msg) => {
                    errors.push(
                        NyraError::coded(
                            "E001",
                            ErrorKind::NameResolution,
                            imp.span.clone(),
                            format!("import not found: `{}`", imp.path),
                        )
                        .label("could not resolve this import path")
                        .note(format!(
                            "resolved relative to `{}`",
                            base_dir.display()
                        ))
                        .help(format!(
                            "check the path is correct relative to `{}`",
                            base_dir.display()
                        ))
                        .help("stdlib is auto-loaded (auto-prelude); explicit `import \"stdlib/…\"` is optional"),
                    );
                }
            }
        }
        if merged.comptime {
            errors.extend(const_eval::finalize_comptime_module(&mut merged));
        }
        return Ok(merged);
    }
    let file = path.to_string_lossy().into_owned();
    let (tokens, lexer_errors) = Lexer::new(&source, &file).tokenize();
    if !lexer_errors.is_empty() {
        let (shown, suppressed) = finalize_lexer_diagnostics(lexer_errors);
        eprint_diagnostics_suppressed(&shown, suppressed);
        return Err(COMPILE_FAILED.into());
    }

    let (mut program, parser_errors) = Parser::new(tokens).parse();
    if !parser_errors.is_empty() {
        let (shown, suppressed) = finalize_parser_diagnostics(parser_errors);
        eprint_diagnostics_suppressed(&shown, suppressed);
        return Err(COMPILE_FAILED.into());
    }
    parse_cache::insert(&path, hash, program.clone());

    let base_dir = path.parent().unwrap_or(Path::new("."));
    let imports: Vec<ImportDecl> = std::mem::take(&mut program.imports);

    let mut merged = program;
    for imp in imports {
        match resolve_import_path(base_dir, &imp.path) {
            Ok(resolved) => {
                let sub = load_file_recursive(&resolved, visited, errors)?;
                merge::merge_program(&mut merged, sub, imp.alias.as_deref());
            }
            Err(_msg) => {
                errors.push(
                    NyraError::coded(
                        "E001",
                        ErrorKind::NameResolution,
                        imp.span.clone(),
                        format!("import not found: `{}`", imp.path),
                    )
                    .label("could not resolve this import path")
                    .note(format!(
                        "resolved relative to `{}`",
                        base_dir.display()
                    ))
                    .help(format!(
                        "check the path is correct relative to `{}`",
                        base_dir.display()
                    ))
                    .help("stdlib is auto-loaded (auto-prelude); explicit `import \"stdlib/…\"` is optional"),
                );
            }
        }
    }

    if merged.comptime {
        errors.extend(const_eval::finalize_comptime_module(&mut merged));
    }

    Ok(merged)
}

fn try_resolve_path(base_dir: &Path, import_path: &str) -> Option<PathBuf> {
    let p = PathBuf::from(import_path);
    let resolved = if p.is_absolute() {
        p
    } else {
        base_dir.join(&p)
    };
    resolved.exists().then_some(resolved)
}

fn resolve_rust_import(base_dir: &Path, import_path: &str) -> Option<PathBuf> {
    let rest = import_path.strip_prefix("rust/")?;
    let root = project_root_for(base_dir);
    let cache = root.join(".nyra/cache/rust").join(rest);
    for name in ["bindings.ny", "mod.ny", "main.ny"] {
        let p = cache.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    // Compile-only stubs shipped with rust-bridge examples (CI / `nyra check` without bind).
    let stub = root.join("stubs").join(rest).join("bindings.ny");
    stub.is_file().then_some(stub)
}

fn resolve_pkg_cache_import(base_dir: &Path, import_path: &str) -> Option<PathBuf> {
    let root = project_root_for(base_dir);
    if !root.join("nyra.mod").exists() {
        return None;
    }
    let cache = root.join(".nyra/cache").join(import_path.replace('.', "/"));
    for ext in crate::paths::NYRA_EXTENSIONS {
        let as_file = cache.with_extension(ext);
        if as_file.exists() {
            return Some(as_file);
        }
    }
    for name in crate::paths::MAIN_ENTRY_NAMES {
        let main = cache.join(name);
        if main.exists() {
            return Some(main);
        }
    }
    None
}

pub fn resolve_import_path(base_dir: &Path, import_path: &str) -> Result<PathBuf, String> {
    if let Some(rest) = import_path.strip_prefix("@root/") {
        let root = project_root_for(base_dir);
        for candidate in crate::paths::import_candidates(rest) {
            if let Some(resolved) = try_resolve_path(&root, &candidate) {
                return Ok(resolved);
            }
        }
    }

    for candidate in crate::paths::import_candidates(import_path) {
        if let Some(resolved) = try_resolve_path(base_dir, &candidate) {
            return Ok(resolved);
        }
    }

    if let Some(resolved) = resolve_stdlib_import_near(import_path, Some(base_dir)) {
        return Ok(resolved);
    }

    if let Some(resolved) = resolve_pkg_import(base_dir, import_path) {
        return Ok(resolved);
    }

    if let Some(resolved) = resolve_rust_import(base_dir, import_path) {
        return Ok(resolved);
    }

    if !crate::paths::is_nyra_import_path(import_path) {
        if let Some(resolved) = resolve_pkg_cache_import(base_dir, import_path) {
            return Ok(resolved);
        }
    }

    Err(format!(
        "Import not found: {} (hint: use import \"stdlib/vec.ny\", import \"rust/uuid\", import \"pkg/name\")",
        base_dir.join(import_path).display()
    ))
}
