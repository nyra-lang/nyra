//! Comptime module directive and compile-time evaluation.

use crate::common::{assert_clean, compile, compile_file_rel};

#[test]
fn comptime_module_folds_imported_const() {
    let out = compile_file_rel("tests/nyra/comptime/import_main.ny");
    assert_clean(&out);
}

#[test]
fn comptime_for_in_generic_import() {
    let out = compile_file_rel("tests/nyra/comptime/for_in_generic_test.ny");
    assert_clean(&out);
}

#[test]
fn comptime_directive_must_be_at_top() {
    let out = compile(
        r#"const X = 1
comptime
const Y = 2
"#,
    );
    assert!(
        out.parser_errors
            .iter()
            .any(|e| e.message.contains("comptime") && e.message.contains("top")),
        "parser: {:?}",
        out.parser_errors
    );
}

#[test]
fn comptime_module_rejects_main() {
    let out = compile(
        r#"comptime
fn main() {
    return 0
}
"#,
    );
    assert!(
        out.load_errors
            .iter()
            .chain(out.type_errors.iter())
            .any(|e| e.message.contains("main")),
        "errors: load={:?} type={:?}",
        out.load_errors,
        out.type_errors
    );
}

#[test]
fn comptime_entry_check_only_no_codegen() {
    let out = compile_file_rel("tests/nyra/comptime/tables.ny");
    assert_clean(&out);
    assert!(out.llvm_ir.is_none(), "comptime entry should skip codegen");
}
