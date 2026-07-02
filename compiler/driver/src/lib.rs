pub use codegen::RuntimeProfile;
pub use codegen::runtime_map;
pub use ownership::{EscapePlan, EscapeState};
pub use resolve::{load_program, load_program_with_options, LoadOptions, LoadOutput, paths, prelude};

pub const NYRA_VERSION: &str = env!("CARGO_PKG_VERSION");

mod stability;
mod features;
mod cache;
mod crate_incremental;

pub use features::FeatureSet;
pub use cache::{
    can_skip_codegen, compute_fingerprint, compute_source_fingerprint, is_incremental_hit,
    link_cache_key, mix_crate_manifest, options_cache_key, read_runtime_cache, write_cached_fingerprint,
    write_runtime_cache, BuildFingerprint,
};
pub use crate_incremental::{
    load_manifest, save_manifest, CrateManifest, CrateUnit,
};

use std::collections::HashSet;
use std::path::Path;

use ast::Program;
use borrowck::check_program as borrow_check;
use codegen::Codegen;
use const_eval::fold_program_consts;
pub use errors::{
    set_color_choice, set_diagnostic_root, clear_diagnostic_root, ColorChoice, ErrorReporter,
    NyraError,
};
use expand::expand_program;
use expand::{coerce_auto_borrow, desugar_try, finish_async_desugar, synthesize_clone_impls, synthesize_struct_json_helpers, synthesize_vec_nested_helpers, synthesize_vec_pod_helpers, synthesize_vec_reloc_helpers};
use lexer::Lexer;
use monomorph::monomorphize_program;
use ownership::{analyze_escapes, analyze_program, check_lifetimes};
use parser::Parser;
use lint::{check_unused_imports, check_unused_variables};
use typecheck::TypeChecker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStage {
    Lex,
    Parse,
    TypeCheck,
    Borrow,
    Codegen,
}

#[derive(Default)]
pub struct CompileOptions {
    pub stop_after: Option<CompileStage>,
    /// LLVM target triple (e.g. `wasm32-wasi`). Empty = host default.
    pub target: String,
    /// Force no_std semantics (also set by `no_std` directive in source).
    pub no_std: bool,
    /// Freestanding link: skip `nyra_rt` and use `-ffreestanding -nostdlib`.
    pub freestanding: bool,
    /// Treat Extended-tier warnings as errors (`nyra check --deny-extended`).
    pub deny_extended: bool,
    /// Treat all warnings as errors (`nyra check --deny-warnings`).
    pub deny_warnings: bool,
    /// Per-feature toggles for gradual rollout and RFC-gated changes.
    pub features: FeatureSet,
    /// Print escape-analysis diagnostics during compile (`nyra build --verbose`).
    pub verbose_escape: bool,
    /// Skip merging the full stdlib prelude (smaller IR; use explicit `import "stdlib/…"`).
    pub no_prelude: bool,
}


pub struct CompileOutput {
    pub llvm_ir: Option<String>,
    pub runtime_profile: RuntimeProfile,
    pub escape_plan: ownership::EscapePlan,
    pub lexer_errors: Vec<NyraError>,
    pub parser_errors: Vec<NyraError>,
    pub load_errors: Vec<NyraError>,
    pub type_errors: Vec<NyraError>,
    pub borrow_errors: Vec<NyraError>,
    pub warnings: Vec<NyraError>,
}

pub struct Compiler;

/// Parse a source file into an AST (no typecheck/codegen).
pub fn parse_file(path: &Path) -> Result<Program, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let file = path.to_string_lossy().into_owned();
    let (tokens, lexer_errors) = Lexer::new(&source, &file).tokenize();
    if !lexer_errors.is_empty() {
        let (shown, suppressed) = errors::finalize_lexer_diagnostics(lexer_errors);
        errors::eprint_diagnostics_suppressed(&shown, suppressed);
        return Err(errors::COMPILE_FAILED.into());
    }
    let (program, parser_errors) = Parser::new(tokens).parse();
    if !parser_errors.is_empty() {
        let (shown, suppressed) = errors::finalize_parser_diagnostics(parser_errors);
        errors::eprint_diagnostics_suppressed(&shown, suppressed);
        return Err(errors::COMPILE_FAILED.into());
    }
    Ok(program)
}

/// Parse source text into an AST (no typecheck/codegen).
pub fn parse_source(source: &str, file: &str) -> Result<Program, String> {
    let (tokens, lexer_errors) = Lexer::new(source, file).tokenize();
    if !lexer_errors.is_empty() {
        let (shown, suppressed) = errors::finalize_lexer_diagnostics(lexer_errors);
        errors::eprint_diagnostics_suppressed(&shown, suppressed);
        return Err(errors::COMPILE_FAILED.into());
    }
    let (program, parser_errors) = Parser::new(tokens).parse();
    if !parser_errors.is_empty() {
        let (shown, suppressed) = errors::finalize_parser_diagnostics(parser_errors);
        errors::eprint_diagnostics_suppressed(&shown, suppressed);
        return Err(errors::COMPILE_FAILED.into());
    }
    Ok(program)
}

impl Compiler {
    pub fn compile_source(
        source: &str,
        file: &str,
        options: &CompileOptions,
    ) -> Result<CompileOutput, String> {
        errors::register_source(file, source);
        let (tokens, lexer_errors) = Lexer::new(source, file).tokenize();
        if options.stop_after == Some(CompileStage::Lex) {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan: ownership::EscapePlan::default(),
                lexer_errors,
                parser_errors: vec![],
                load_errors: vec![],
                type_errors: vec![],
                borrow_errors: vec![],
                warnings: vec![],
            });
        }

        let (program, parser_errors) = Parser::new(tokens).parse();
        if options.stop_after == Some(CompileStage::Parse) {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan: ownership::EscapePlan::default(),
                lexer_errors,
                parser_errors,
                load_errors: vec![],
                type_errors: vec![],
                borrow_errors: vec![],
                warnings: vec![],
            });
        }

        let lint_entry = std::path::Path::new(file);
        let lint_entry = lint_entry.exists().then_some(lint_entry);
        Self::compile_program(&program, file, options, lint_entry, vec![]).map(|mut out| {
            out.lexer_errors = lexer_errors;
            out.parser_errors = parser_errors;
            out
        })
    }

    pub fn compile_file(path: &Path, options: &CompileOptions) -> Result<CompileOutput, String> {
        if path.is_dir() {
            return Self::compile_project(path, options);
        }
        let loaded = load_program_with_options(path, Self::load_options(options))?;
        let file = path.to_string_lossy().into_owned();
        set_diagnostic_root(Self::project_root_for_path(path));
        Self::compile_program(
            &loaded.program,
            &file,
            options,
            Some(path),
            loaded.errors,
        )
    }

    pub fn compile_project(dir: &Path, options: &CompileOptions) -> Result<CompileOutput, String> {
        let main = paths::find_main_entry(dir).ok_or_else(|| {
            format!(
                "Project directory must contain main.ny or main.nyra: {}",
                dir.display()
            )
        })?;
        let loaded = load_program_with_options(&main, Self::load_options(options))?;
        set_diagnostic_root(dir);
        Self::compile_program(
            &loaded.program,
            &main.to_string_lossy(),
            options,
            Some(&main),
            loaded.errors,
        )
    }

    fn project_root_for_path(path: &Path) -> &Path {
        if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        }
    }

    fn load_options(options: &CompileOptions) -> LoadOptions {
        LoadOptions {
            auto_prelude: !options.no_std && !options.freestanding && !options.no_prelude,
        }
    }

    pub fn compile_program(
        program: &Program,
        file: &str,
        options: &CompileOptions,
        lint_entry: Option<&Path>,
        load_errors: Vec<NyraError>,
    ) -> Result<CompileOutput, String> {
        if options.stop_after == Some(CompileStage::Lex)
            || options.stop_after == Some(CompileStage::Parse)
        {
            return Err("compile_program requires full pipeline".into());
        }

        let mut program = program.clone();
        let mut load_errors = load_errors;
        if program.comptime {
            load_errors.extend(const_eval::finalize_comptime_module(&mut program));
        }
        expand_program(&mut program);
        let mut mono_errors = monomorphize_program(&mut program);
        synthesize_vec_pod_helpers(&mut program);
        synthesize_vec_reloc_helpers(&mut program);
        synthesize_vec_nested_helpers(&mut program);
        synthesize_struct_json_helpers(&mut program);
        if !options.no_std
            && !program.no_std
            && !program.comptime
            && !options.no_prelude
            && !options.freestanding
        {
            let entry = lint_entry
                .filter(|p| p.is_file())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| Path::new(file).to_path_buf());
            let mut visited = HashSet::new();
            let mut prelude_errors = Vec::new();
            let _ = prelude::inject_lazy_stdlib_prelude(
                &entry,
                &mut program,
                &mut visited,
                &mut prelude_errors,
            );
            mono_errors.extend(monomorphize_program(&mut program));
            synthesize_vec_pod_helpers(&mut program);
            synthesize_vec_reloc_helpers(&mut program);
            synthesize_vec_nested_helpers(&mut program);
            synthesize_struct_json_helpers(&mut program);
            let _ = prelude::inject_lazy_stdlib_prelude(
                &entry,
                &mut program,
                &mut visited,
                &mut prelude_errors,
            );
            mono_errors.extend(monomorphize_program(&mut program));
            synthesize_vec_pod_helpers(&mut program);
            synthesize_vec_reloc_helpers(&mut program);
            synthesize_vec_nested_helpers(&mut program);
            synthesize_struct_json_helpers(&mut program);
        }
        desugar_try(&mut program);
        coerce_auto_borrow(&mut program);
        fold_program_consts(&mut program);
        let comptime_fn_errors = const_eval::fold_attributed_comptime_functions(&mut program);

        let mut warnings = if program.allow_extended {
            vec![]
        } else {
            stability::extended_tier_warnings(&program, &options.features, Some(file))
        };
        if options.deny_extended {
            for w in &mut warnings {
                w.severity = errors::Severity::Error;
            }
        }

        let mut type_checker = TypeChecker::new();
        type_checker.set_target(&options.target);
        if options.no_std || program.no_std {
            type_checker.no_std = true;
        }
        type_checker.check_program(&program);
        if !type_checker.has_errors() {
            type_checker.apply_inferred_signatures(&mut program);
            type_checker.apply_anonymous_struct_literals(&mut program);
            if type_checker.synthesized_anon_structs() {
                synthesize_clone_impls(&mut program);
            }
            finish_async_desugar(&mut program, &type_checker);
            synthesize_vec_pod_helpers(&mut program);
            synthesize_vec_reloc_helpers(&mut program);
            synthesize_vec_nested_helpers(&mut program);
            synthesize_struct_json_helpers(&mut program);
        }
        let mut type_errors = type_checker.errors.clone();
        type_errors.extend(mono_errors);
        type_errors.extend(comptime_fn_errors);

        if let Some(entry) = lint_entry {
            warnings.extend(check_unused_imports(entry, Some(&program)));
        }
        warnings.extend(check_unused_variables(&program));

        if options.deny_warnings {
            for w in &mut warnings {
                w.severity = errors::Severity::Error;
            }
        }

        let (own_ctx, drop_plan) = analyze_program(&program);

        let mut borrow_errors = vec![];
        borrow_check(&program, &own_ctx, &mut borrow_errors);
        check_lifetimes(&program, &own_ctx, &mut borrow_errors);

        let escape_plan = if type_checker.has_errors() {
            ownership::EscapePlan::default()
        } else {
            analyze_escapes(&program)
        };
        if options.verbose_escape && !type_checker.has_errors() {
            eprintln!("   Checking  escape analysis");
            for line in escape_plan.report_lines() {
                eprintln!("   {line}");
            }
        }

        let mut no_escape_errors = vec![];
        if !type_checker.has_errors() {
            ownership::check_no_escape(&program, &escape_plan, &mut no_escape_errors);
        }
        borrow_errors.extend(no_escape_errors);

        if !load_errors.is_empty() || options.stop_after == Some(CompileStage::TypeCheck) {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan,
                lexer_errors: vec![],
                parser_errors: vec![],
                load_errors,
                type_errors,
                borrow_errors,
                warnings,
            });
        }

        if type_checker.has_errors() {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan,
                lexer_errors: vec![],
                parser_errors: vec![],
                load_errors: vec![],
                type_errors,
                borrow_errors,
                warnings,
            });
        }

        if options.stop_after == Some(CompileStage::Borrow) {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan,
                lexer_errors: vec![],
                parser_errors: vec![],
                load_errors: vec![],
                type_errors: vec![],
                borrow_errors,
                warnings,
            });
        }

        if !borrow_errors.is_empty() {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan,
                lexer_errors: vec![],
                parser_errors: vec![],
                load_errors: vec![],
                type_errors: vec![],
                borrow_errors,
                warnings,
            });
        }

        if program.comptime {
            return Ok(CompileOutput {
                llvm_ir: None,
                runtime_profile: RuntimeProfile::default(),
                escape_plan,
                lexer_errors: vec![],
                parser_errors: vec![],
                load_errors: vec![],
                type_errors: vec![],
                borrow_errors: vec![],
                warnings,
            });
        }

        let mut codegen = Codegen::new(file);
        codegen.set_target(&options.target);
        codegen.set_drop_plan(drop_plan);
        codegen.set_escape_plan(escape_plan.clone());
        let ir = codegen.compile_program(&program);
        let mut runtime_profile = codegen.take_runtime_profile();
        if options.freestanding || program.no_std || options.no_std {
            runtime_profile = RuntimeProfile::default();
        }

        Ok(CompileOutput {
            llvm_ir: Some(ir),
            runtime_profile,
            escape_plan,
            lexer_errors: vec![],
            parser_errors: vec![],
            load_errors: vec![],
            type_errors: vec![],
            borrow_errors: vec![],
            warnings,
        })
    }

    pub fn report_errors(output: &CompileOutput) -> bool {
        let (lexer_errors, lex_sup) =
            errors::finalize_lexer_diagnostics(output.lexer_errors.clone());
        let (parser_errors, par_sup) =
            errors::finalize_parser_diagnostics(output.parser_errors.clone());
        let suppressed = lex_sup + par_sup;

        let mut reporter = ErrorReporter::new();
        for e in &output.load_errors {
            reporter.report(e.clone());
        }
        for e in &lexer_errors {
            reporter.report(e.clone());
        }
        for e in &parser_errors {
            reporter.report(e.clone());
        }
        for e in &output.type_errors {
            reporter.report(e.clone());
        }
        for e in &output.borrow_errors {
            reporter.report(e.clone());
        }
        for w in &output.warnings {
            reporter.report(w.clone());
        }
        if reporter.errors.is_empty() {
            return false;
        }
        errors::print_diagnostics(&reporter.errors, suppressed);
        reporter.has_errors()
    }

    /// Remove unused imports and prefix unused locals with `_` (like `cargo fix` for lints).
    pub fn prune_project(dir: &Path, dry_run: bool) -> Result<lint::PruneResult, String> {
        let main = paths::find_main_entry(dir).ok_or_else(|| {
            format!(
                "Project directory must contain main.ny or main.nyra: {}",
                dir.display()
            )
        })?;
        let loaded = load_program(&main)?;
        if !loaded.errors.is_empty() {
            return Err(format!(
                "cannot prune project with load errors ({} issue(s))",
                loaded.errors.len()
            ));
        }
        let mut program = loaded.program;
        expand_program(&mut program);
        let plan = lint::plan_prune(&main, &program);
        lint::apply_prune(&plan, dry_run)
    }
}
