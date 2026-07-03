//! Map merged function names back to source files (for incremental codegen).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ast::Program;

use crate::parse_file_only;

#[derive(Debug, Clone)]
pub struct SourceUnit {
    pub path: PathBuf,
    /// Import alias prefix when merged into parent (`h` → `h__foo`).
    pub merge_prefix: Option<String>,
}

/// Walk the import graph from `entry` and record merge prefixes per file.
pub fn collect_source_units(entry: &Path) -> Result<Vec<SourceUnit>, String> {
    let entry = entry.canonicalize().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut visited = HashSet::new();
    walk_units(&entry, None, &mut visited, &mut out)?;
    Ok(out)
}

fn walk_units(
    path: &Path,
    merge_prefix: Option<String>,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<SourceUnit>,
) -> Result<(), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if !visited.insert(path.clone()) {
        return Ok(());
    }
    out.push(SourceUnit {
        path: path.clone(),
        merge_prefix,
    });
    let program = parse_file_only(&path)?;
    let base = path.parent().unwrap_or(Path::new("."));
    for imp in &program.imports {
        let resolved = crate::resolve_import_path(base, &imp.path)?;
        walk_units(&resolved, imp.alias.clone(), visited, out)?;
    }
    Ok(())
}

/// Assign merged `program` function names to canonical source paths.
pub fn assign_functions_to_units(
    units: &[SourceUnit],
    program: &Program,
) -> HashMap<String, HashSet<String>> {
    let mut local_names: HashMap<String, HashSet<String>> = HashMap::new();
    for unit in units {
        let key = unit.path.to_string_lossy().into_owned();
        if let Ok(p) = parse_file_only(&unit.path) {
            let mut names = HashSet::new();
            for f in &p.functions {
                if f.public || unit.merge_prefix.is_none() {
                    names.insert(f.name.clone());
                }
            }
            for imp in &p.impls {
                for m in &imp.methods {
                    if m.public {
                        names.insert(m.name.clone());
                    }
                }
            }
            local_names.insert(key.clone(), names);
        }
    }

    let mut prefixes: Vec<(String, String)> = units
        .iter()
        .filter_map(|u| {
            u.merge_prefix
                .as_ref()
                .map(|p| (format!("{p}__"), u.path.to_string_lossy().into_owned()))
        })
        .collect();
    prefixes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let entry_key = units
        .first()
        .map(|u| u.path.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut by_path: HashMap<String, HashSet<String>> = HashMap::new();
    for unit in units {
        by_path.insert(unit.path.to_string_lossy().into_owned(), HashSet::new());
    }

    for f in &program.functions {
        let mut assigned = false;
        for (prefix, path) in &prefixes {
            if f.name.starts_with(prefix) {
                by_path.entry(path.clone()).or_default().insert(f.name.clone());
                assigned = true;
                break;
            }
        }
        if assigned {
            continue;
        }
        for (path, locals) in &local_names {
            if locals.contains(&f.name) {
                by_path.entry(path.clone()).or_default().insert(f.name.clone());
                assigned = true;
                break;
            }
        }
        if !assigned {
            by_path
                .entry(entry_key.clone())
                .or_default()
                .insert(f.name.clone());
        }
    }

    for imp in &program.impls {
        for m in &imp.methods {
            let mut assigned = false;
            for (prefix, path) in &prefixes {
                if m.name.starts_with(prefix) {
                    by_path.entry(path.clone()).or_default().insert(m.name.clone());
                    assigned = true;
                    break;
                }
            }
            if !assigned {
                by_path
                    .entry(entry_key.clone())
                    .or_default()
                    .insert(m.name.clone());
            }
        }
    }

    by_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn assigns_prefixed_functions_to_import_unit() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/nyra");
        let main = root.join("modules_test.ny");
        if !main.is_file() {
            return;
        }
        let entry = main.clone();
        let units = collect_source_units(&entry).expect("units");
        assert!(units.len() >= 2);
        let helpers = root.join("modules/helpers.ny");
        let program = parse_file_only(&main).expect("parse main");
        let merged = {
            let sub = parse_file_only(&helpers).expect("parse helpers");
            let mut p = program;
            crate::merge::merge_program(&mut p, sub, Some("h"));
            p
        };
        let map = assign_functions_to_units(&units, &merged);
        let helper_key = helpers.canonicalize().unwrap().to_string_lossy().into_owned();
        let main_key = main.canonicalize().unwrap().to_string_lossy().into_owned();
        assert!(map.get(&helper_key).is_some_and(|s| s.contains("h__greet")));
        assert!(map.get(&main_key).is_some_and(|s| s.contains("main")));
    }
}
