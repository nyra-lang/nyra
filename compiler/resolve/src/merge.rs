//! Merge imported compilation units into the entry program.

use ast::*;

fn mangle_name(prefix: Option<&str>, name: &str) -> String {
    prefix
        .map(|p| format!("{p}__{name}"))
        .unwrap_or_else(|| name.to_string())
}

pub(crate) fn merge_program(target: &mut Program, other: Program, import_alias: Option<&str>) {
    for c in other.consts {
        if !c.public {
            continue;
        }
        let name = mangle_name(import_alias, &c.name);
        if !target.consts.iter().any(|x| x.name == name) {
            let mut item = c;
            item.name = name;
            target.consts.push(item);
        }
    }
    for s in other.structs {
        if !s.public {
            continue;
        }
        let name = mangle_name(import_alias, &s.name);
        if !target.structs.iter().any(|x| x.name == name) {
            let mut item = s;
            item.name = name;
            target.structs.push(item);
        }
    }
    for u in other.unions {
        if !u.public {
            continue;
        }
        let name = mangle_name(import_alias, &u.name);
        if !target.unions.iter().any(|x| x.name == name) {
            let mut item = u;
            item.name = name;
            target.unions.push(item);
        }
    }
    for e in other.enums {
        if !e.public {
            continue;
        }
        let name = mangle_name(import_alias, &e.name);
        if !target.enums.iter().any(|x| x.name == name) {
            let mut item = e;
            item.name = name;
            target.enums.push(item);
        }
    }
    for t in other.traits {
        if !target.traits.iter().any(|x| x.name == t.name) {
            target.traits.push(t);
        }
    }
    for ti in other.trait_impls {
        if !target
            .trait_impls
            .iter()
            .any(|x| x.type_name == ti.type_name && x.trait_name == ti.trait_name)
        {
            target.trait_impls.push(ti);
        }
    }
    for m in other.macros {
        if !target.macros.iter().any(|x| x.name == m.name) {
            target.macros.push(m);
        }
    }
    for i in other.impls {
        if !i.methods.iter().all(|m| m.public) {
            continue;
        }
        let type_name = mangle_name(import_alias, &i.type_name);
        if !target.impls.iter().any(|x| x.type_name == type_name) {
            let mut item = i;
            item.type_name = type_name;
            target.impls.push(item);
        }
    }
    for e in other.externs {
        if !target.externs.iter().any(|x| x.name == e.name) {
            target.externs.push(e);
        }
    }
    for f in other.functions {
        if other.comptime {
            continue;
        }
        if !f.public {
            continue;
        }
        let name = mangle_name(import_alias, &f.name);
        if !target.functions.iter().any(|x| x.name == name) {
            let mut item = f;
            item.name = name;
            target.functions.push(item);
        }
    }
    for inst in other.export_instances {
        let dup = target.export_instances.iter().any(|x| {
            x.fn_name == inst.fn_name && x.type_args == inst.type_args
        });
        if !dup {
            target.export_instances.push(inst);
        }
    }
    if target.module.is_none() {
        target.module = other.module;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn merge_skips_priv_functions_and_mangles_alias() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let helpers = root.join("tests/nyra/modules/helpers.ny");
        let other = crate::parse_file_only(&helpers).expect("parse helpers");
        let secret = other
            .functions
            .iter()
            .find(|f| f.name == "secret")
            .expect("secret fn");
        assert!(!secret.public, "priv fn should parse as public=false");
        let greet = other
            .functions
            .iter()
            .find(|f| f.name == "greet")
            .expect("greet fn");
        assert!(greet.public, "pub fn should parse as public=true");

        let mut target = Program {
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
        };
        merge_program(&mut target, other, Some("h"));
        let names: Vec<_> = target.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"h__greet"), "expected h__greet, got {names:?}");
        assert!(!names.iter().any(|n| *n == "h__secret"), "priv fn must not merge: {names:?}");
    }
}
