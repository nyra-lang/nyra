//! Virtual stdlib symbol table + lazy on-demand prelude injection.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ast::Program;
use errors::NyraError;

use crate::symbols::{collect_program_uses, top_level_export_names};
use crate::{collect_stdlib_sources_near, parse_file_only};
use crate::merge;

/// Maps exported stdlib symbol names → defining source files (virtual symbol table).
#[derive(Debug, Clone, Default)]
pub struct StdlibVirtualIndex {
    symbol_to_files: HashMap<String, Vec<PathBuf>>,
}

impl StdlibVirtualIndex {
    pub fn build(near: Option<&Path>) -> Self {
        let mut symbol_to_files: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for path in collect_stdlib_sources_near(near) {
            let Ok(program) = parse_file_only(&path) else {
                continue;
            };
            for name in top_level_export_names(&program) {
                symbol_to_files
                    .entry(name)
                    .or_default()
                    .push(path.clone());
            }
        }
        Self { symbol_to_files }
    }

    pub fn files_for_symbols<'a>(
        &self,
        names: impl IntoIterator<Item = &'a str>,
    ) -> HashSet<PathBuf> {
        let mut out = HashSet::new();
        for name in names {
            if let Some(paths) = self.symbol_to_files.get(name) {
                out.extend(paths.iter().cloned());
            }
        }
        out
    }

    pub fn defines(&self, name: &str) -> bool {
        self.symbol_to_files.contains_key(name)
    }

    pub fn symbol_count(&self) -> usize {
        self.symbol_to_files.len()
    }
}

/// Merge only stdlib modules needed by symbols referenced in `program` (fixed-point).
pub fn inject_lazy_stdlib_prelude(
    entry: &Path,
    program: &mut Program,
    visited: &mut HashSet<PathBuf>,
    _errors: &mut Vec<NyraError>,
) -> Result<(), String> {
    let index = StdlibVirtualIndex::build(Some(entry));
    if index.symbol_count() == 0 {
        return Ok(());
    }

    let mut loaded_stdlib: HashSet<PathBuf> = HashSet::new();
    const MAX_ROUNDS: usize = 32;

    for _ in 0..MAX_ROUNDS {
        let defined = top_level_export_names(program);
        let used = collect_program_uses(program);
        let mut missing: Vec<String> = used
            .iter()
            .filter(|name| !defined.contains(*name) && index.defines(name))
            .cloned()
            .collect();
        missing.sort();
        if missing.is_empty() {
            break;
        }

        let to_load = index.files_for_symbols(missing.iter().map(String::as_str));
        let mut new_files: Vec<PathBuf> = to_load
            .into_iter()
            .filter(|p| !loaded_stdlib.contains(p))
            .collect();
        new_files.sort();
        if new_files.is_empty() {
            break;
        }

        for path in new_files {
            loaded_stdlib.insert(path.clone());
            let sub = parse_file_only(&path).map_err(|e| e.to_string())?;
            merge::merge_program(program, sub, None);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn virtual_index_includes_vec_symbols() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
        let index = StdlibVirtualIndex::build(Some(&repo));
        assert!(index.defines("Vec_i32_new"));
        assert!(index.defines("vec_push"));
        let files = index.files_for_symbols(["Vec_i32_new"]);
        assert!(files.iter().any(|p| p.ends_with("vec.ny")));
    }

    #[test]
    fn lazy_prelude_loads_only_used_modules() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entry = repo.join("tests/suite/run/stdlib/auto_prelude.ny");
        assert!(entry.is_file());

        let mut visited = HashSet::new();
        let mut errors = Vec::new();
        let mut program = crate::parse_file_only(&entry).unwrap();
        let before_fns = program.functions.len();

        inject_lazy_stdlib_prelude(&entry, &mut program, &mut visited, &mut errors).unwrap();

        assert!(program.functions.iter().any(|f| f.name == "Vec_i32_new"));
        assert!(program.functions.iter().any(|f| f.name == "vec_push"));
        assert!(
            program.functions.len() < before_fns + 200,
            "lazy prelude should not merge entire stdlib"
        );
        assert!(
            !program.functions.iter().any(|f| f.name == "Router_new"),
            "HTTP router should not load for vec-only program"
        );
    }

    #[test]
    fn load_program_with_options_loads_strvec_for_cat() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entry = repo.join("examples/zero_types_cli.ny");
        let loaded = crate::load_program_with_options(
            &entry,
            crate::LoadOptions {
                auto_prelude: true,
            },
        )
        .unwrap();
        assert!(
            loaded.program.functions.iter().any(|f| f.name == "StrVec_new"),
            "missing StrVec_new with auto_prelude"
        );
        assert!(
            loaded.program.functions.iter().any(|f| f.name == "strip_flags"),
            "missing strip_flags in zero_types_cli example"
        );
    }

    #[test]
    fn lazy_prelude_loads_strvec_for_untyped_cat() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entry = repo.join("examples/zero_types_cli.ny");
        assert!(entry.is_file());

        let mut visited = HashSet::new();
        let mut errors = Vec::new();
        let mut program = crate::load_file_recursive(&entry, &mut visited, &mut errors).unwrap();
        let used = collect_program_uses(&program);
        assert!(
            used.contains("StrVec_new"),
            "StrVec_new not in uses: {:?}",
            used.iter().filter(|s| s.contains("Str") || s.contains("Vec")).collect::<Vec<_>>()
        );

        inject_lazy_stdlib_prelude(&entry, &mut program, &mut visited, &mut errors).unwrap();
        assert!(
            program.functions.iter().any(|f| f.name == "StrVec_new"),
            "StrVec_new not loaded; defined StrVec fns: {:?}",
            program
                .functions
                .iter()
                .filter(|f| f.name.contains("StrVec"))
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
        );
    }
}
