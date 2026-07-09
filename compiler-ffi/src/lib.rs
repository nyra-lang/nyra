//! Stable C ABI for in-process compile/check without spawning the `nyra` CLI.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use compiler::{CompileOptions, CompileOutput, CompileStage, Compiler, NyraError};
use errors::{ErrorKind, Span};

fn diag_json_from_output(output: &CompileOutput) -> String {
    let mut all = Vec::new();
    for e in output
        .warnings
        .iter()
        .chain(&output.load_errors)
        .chain(&output.lexer_errors)
        .chain(&output.parser_errors)
        .chain(&output.type_errors)
        .chain(&output.borrow_errors)
    {
        all.push(serde_json::json!({
            "file": e.span.file,
            "line": e.span.start.line,
            "column": e.span.start.column,
            "message": e.message,
            "kind": format!("{:?}", e.kind),
            "severity": format!("{:?}", e.severity),
        }));
    }
    serde_json::to_string(&all).unwrap_or_else(|_| "[]".into())
}

fn compile_path(path: &Path) -> Result<CompileOutput, String> {
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        ..CompileOptions::default()
    };
    if path.is_dir() {
        Compiler::compile_project(path, &options)
    } else {
        Compiler::compile_file(path, &options)
    }
}

fn has_errors(output: &CompileOutput) -> bool {
    !output.load_errors.is_empty()
        || !output.lexer_errors.is_empty()
        || !output.parser_errors.is_empty()
        || !output.type_errors.is_empty()
        || !output.borrow_errors.is_empty()
}

fn malloc_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn ffi_error_json(message: &str, file: &str) -> String {
    let err = NyraError::new(
        ErrorKind::Type,
        Span::new(file, Default::default(), Default::default()),
        message,
    );
    diag_json_from_output(&CompileOutput {
        parser_errors: vec![err],
        llvm_ir: None,
        runtime_profile: Default::default(),
        escape_plan: Default::default(),
        lexer_errors: vec![],
        load_errors: vec![],
        type_errors: vec![],
        borrow_errors: vec![],
        warnings: vec![],
        inspect_report: None,
    })
}

/// Typecheck a file or project directory. Returns 0 on success, 1 on diagnostics, negative on internal error.
#[no_mangle]
pub extern "C" fn nyra_check_file(path: *const c_char) -> i32 {
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    match compile_path(Path::new(path_str)) {
        Ok(out) => {
            if has_errors(&out) {
                1
            } else {
                0
            }
        }
        Err(_) => -2,
    }
}

/// Return JSON diagnostics for a path (malloc'd UTF-8). Caller frees with `nyra_compiler_free`.
#[no_mangle]
pub extern "C" fn nyra_diag_json_file(path: *const c_char) -> *mut c_char {
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let json = match compile_path(Path::new(path_str)) {
        Ok(out) => diag_json_from_output(&out),
        Err(e) => ffi_error_json(&e, path_str),
    };
    malloc_c_string(&json)
}

/// Typecheck source text. `file` is used only for diagnostic paths.
#[no_mangle]
pub extern "C" fn nyra_check_source(source: *const c_char, file: *const c_char) -> i32 {
    let source_str = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let file_str = match unsafe { CStr::from_ptr(file) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        ..CompileOptions::default()
    };
    match Compiler::compile_source(source_str, file_str, &options) {
        Ok(out) => {
            if has_errors(&out) {
                1
            } else {
                0
            }
        }
        Err(_) => -2,
    }
}

/// JSON diagnostics for source text. Caller frees with `nyra_compiler_free`.
#[no_mangle]
pub extern "C" fn nyra_diag_json_source(source: *const c_char, file: *const c_char) -> *mut c_char {
    let source_str = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let file_str = match unsafe { CStr::from_ptr(file) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        ..CompileOptions::default()
    };
    let json = match Compiler::compile_source(source_str, file_str, &options) {
        Ok(out) => diag_json_from_output(&out),
        Err(e) => ffi_error_json(&e, file_str),
    };
    malloc_c_string(&json)
}

#[no_mangle]
pub extern "C" fn nyra_compiler_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: pointer must come from this crate's malloc_c_string.
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn nyra_compiler_version() -> *const c_char {
    static VER: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VER.as_ptr() as *const c_char
}
