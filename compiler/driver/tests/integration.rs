mod common;

use std::thread;
use std::time::Duration;

use common::{examples_dir, ir_defines_main, nyra_bin, run_nyra, run_nyra_file, tests_dir};
use compiler::{CompileOptions, Compiler};

#[test]
fn compiles_try_in_return_match_arm() {
    let src = r#"fn step(n: i32) -> Result<i32, i32> { return Result.Ok(n) }
fn main() -> Result<i32, i32> {
    let res = step(1)
    return match res {
        Result.Ok(x) => step(x)?
        Result.Err(e) => Result.Err(e)
    }
}"#;
    let output = Compiler::compile_source(src, "try_match.ny", &CompileOptions::default()).unwrap();
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_math_without_errors() {
    let path = examples_dir().join("syntax/math.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty());
    assert!(output.parser_errors.is_empty());
    assert!(output.type_errors.is_empty());
    assert!(output.llvm_ir.is_some());
}

#[test]
fn math_llvm_ir_contains_main_and_add() {
    let path = examples_dir().join("syntax/math.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    let ir = output.llvm_ir.unwrap();
    assert!(ir_defines_main(&ir));
    assert!(ir.contains("add i32"));
}

#[test]
fn rejects_undefined_variable() {
    let src = r#"fn main() {
    let y = x
}"#;
    let output = Compiler::compile_source(src, "bad.ny", &CompileOptions::default()).unwrap();
    assert!(!output.type_errors.is_empty());
    assert!(output.llvm_ir.is_none());
}

#[test]
fn compiles_sum_loop_without_errors() {
    let path = examples_dir().join("comparison/loop/sum_loop_small.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
    let ir = output.llvm_ir.unwrap();
    assert!(ir.contains("while.cond"));
}

#[test]
fn rejects_assign_to_immutable() {
    let src = r#"fn main() {
    let x = 1
    x = 2
}"#;
    let output = Compiler::compile_source(src, "bad.ny", &CompileOptions::default()).unwrap();
    assert!(!output.type_errors.is_empty());
    assert!(output.llvm_ir.is_none());
}

#[test]
fn end_to_end_sum_loop_output() {
    let path = examples_dir().join("comparison/loop/sum_loop_small.ny");
    assert_eq!(run_nyra_file(&path), "499500");
}

#[test]
fn end_to_end_math_program_output() {
    let math = examples_dir().join("syntax/math.ny");
    assert_eq!(run_nyra_file(&math), "30");
}

#[test]
fn end_to_end_f32_print_output() {
    let path = examples_dir().join("syntax/f32_basics.ny");
    let stdout = run_nyra_file(&path);
    assert!(stdout.contains("1.5"), "expected f32 sum, got: {stdout}");
    assert!(stdout.contains("4"), "expected point sum, got: {stdout}");
}

#[test]
fn end_to_end_f64_print_output() {
    let path = examples_dir().join("syntax/f64_basics.ny");
    let stdout = run_nyra_file(&path);
    assert!(stdout.contains("61."), "expected lat+lng sum, got: {stdout}");
    assert!(stdout.contains("5.13") || stdout.contains("5.1"), "expected total, got: {stdout}");
}

#[test]
fn end_to_end_systems_types_output() {
    let path = examples_dir().join("syntax/systems_types.ny");
    let stdout = run_nyra_file(&path);
    assert!(stdout.contains("42"), "expected closure sum, got: {stdout}");
    assert!(stdout.contains("9"), "expected raw ptr deref, got: {stdout}");
    assert!(stdout.contains("99"), "expected map value, got: {stdout}");
}

#[test]
fn end_to_end_primitive_types_output() {
    let path = examples_dir().join("syntax/primitive_types.typed.ny");
    let stdout = run_nyra_file(&path);
    assert!(stdout.contains("-1"), "expected small+negative sum, got: {stdout}");
    assert!(stdout.contains("255"), "expected u8, got: {stdout}");
}

#[test]
fn compiles_imported_module_consts() {
    let dir = common::tests_dir().join("fixtures/import_consts");
    let output = Compiler::compile_project(&dir, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty());
    assert!(output.parser_errors.is_empty());
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn end_to_end_imported_const_prints_value() {
    let dir = common::tests_dir().join("fixtures/import_consts");
    let stdout = run_nyra_file(&dir);
    assert_eq!(stdout, "Hello\n42");
}

#[test]
fn infers_void_for_function_without_return_type() {
    let src = r#"fn oops() {
    print(1)
}"#;
    let output = Compiler::compile_source(src, "oops.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_modulo_and_logical_and() {
    let src = r#"fn main() -> void {
    let a = 7 % 4
    let b = 1 == 1 && a < 5
    if b {
        print(a)
    }
}"#;
    let output = Compiler::compile_source(src, "ops.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_numeric_separators() {
    let src = r#"fn main() -> void {
    let n = 1_000_000
    let m = 10_0000
    print(n + m)
}"#;
    let output = Compiler::compile_source(src, "nums.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_mem_start_and_mem_end() {
    let src = r#"fn main() -> void {
    mem_start("alloc")
    let mut sum = 0
    for i in 0..100 {
        sum = sum + i
    }
    mem_end("alloc")
    print(sum)
}"#;
    let output = Compiler::compile_source(src, "mem.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(ir.contains("call void @mem_start"));
    assert!(ir.contains("call void @mem_end"));
}

#[test]
fn compiles_time_start_and_time_end() {
    let src = r#"fn main() -> void {
    time_start("loop")
    let mut sum = 0
    for i in 0..10 {
        sum = sum + i
    }
    time_end("loop")
    print(sum)
}"#;
    let output = Compiler::compile_source(src, "time.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(ir.contains("call void @time_start"));
    assert!(ir.contains("call void @time_end"));
}

#[test]
fn rejects_time_start_without_string_label() {
    let src = r#"fn main() -> void {
    time_start(1)
}"#;
    let output = Compiler::compile_source(src, "time.ny", &CompileOptions::default()).unwrap();
    assert!(!output.type_errors.is_empty());
    assert!(output.type_errors.iter().any(|e| {
        e.message.contains("string label")
            || e.notes.iter().any(|n| n.contains("string label"))
    }));
}

#[test]
fn compiles_imported_enum_return_type() {
    let dir = std::env::temp_dir().join("nyra_enum_import_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("types.ny"), "enum Color { Red Green }").unwrap();
    std::fs::write(
        dir.join("main.ny"),
        r#"import "types.ny"
fn pick() -> Color {
    return Color.Red
}
fn main() -> void {
    let c = pick()
    let n = match c {
        Color.Red => 1
        Color.Green => 2
    }
    print(n)
}"#,
    )
    .unwrap();
    let output = Compiler::compile_project(&dir, &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compiles_nyra_extension_single_file() {
    let src = r#"fn main() -> void {
    print(42)
}"#;
    let output = Compiler::compile_source(src, "hello.nyra", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_main_nyra_entry() {
    let dir = std::env::temp_dir().join("nyra_main_nyra_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.nyra"),
        r#"fn main() -> void {
    print(7)
}"#,
    )
    .unwrap();
    let output = Compiler::compile_project(&dir, &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn import_resolves_alternate_extension() {
    let dir = std::env::temp_dir().join("nyra_alt_ext_import_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("types.nyra"), "enum Color { Red Green }").unwrap();
    std::fs::write(
        dir.join("main.ny"),
        r#"import "types.ny"
fn pick() -> Color {
    return Color.Red
}
fn main() -> void {
    print(1)
}"#,
    )
    .unwrap();
    let output = Compiler::compile_project(&dir, &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compiles_if_with_const_comparison() {
    let src = r#"const LIMIT = 4
fn main() -> void {
    let v = 3
    if v >= LIMIT {
        print(1)
    } else {
        print(0)
    }
}"#;
    let output = Compiler::compile_source(src, "lim.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
}

#[test]
fn same_file_struct_type_annotation_in_parser() {
    let src = r#"struct Box {
    n: i32
}
fn id(b: Box) -> Box {
    return b
}
fn main() -> void {
    print(0)
}"#;
    let output = Compiler::compile_source(src, "box.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
}

#[test]
fn compiles_calculator_project() {
    let dir = examples_dir().join("projects/calculator");
    let output = Compiler::compile_project(&dir, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty());
    assert!(output.parser_errors.is_empty());
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let ir = output.llvm_ir.unwrap();
    assert!(ir.contains("%Calculator = type"));
    assert!(ir_defines_main(&ir));
}

#[test]
fn calculator_project_field_access() {
    let dir = examples_dir().join("projects/calculator");
    let output = Compiler::compile_project(&dir, &CompileOptions::default()).unwrap();
    let ir = output.llvm_ir.unwrap();
    assert!(ir.contains("getelementptr"));
}

#[test]
fn type_error_includes_line_number() {
    let src = r#"fn main() {
    let x = y
}"#;
    let output = Compiler::compile_source(src, "line.ny", &CompileOptions::default()).unwrap();
    assert!(!output.type_errors.is_empty());
    let err = &output.type_errors[0];
    assert_eq!(err.span.file, "line.ny");
    assert!(err.span.start.line >= 2);
    let msg = format!("{err}");
    assert!(msg.contains("line.ny:2:"), "expected line in output: {msg}");
}

#[test]
fn allows_copy_i32_after_assign_in_borrow_check() {
    let src = r#"fn main() {
    let b = 1
    let a = b
    print(a)
    print(b)
}"#;
    let output = Compiler::compile_source(src, "ok.ny", &CompileOptions::default()).unwrap();
    assert!(output.borrow_errors.is_empty());
    assert!(output.llvm_ir.is_some());
}

#[test]
fn rejects_use_after_move_string_in_borrow_check() {
    let src = r#"fn main() {
    let a = "hello"
    let b = a
    print(a)
}"#;
    let output = Compiler::compile_source(src, "bad.ny", &CompileOptions::default()).unwrap();
    assert!(!output.borrow_errors.is_empty());
    assert!(output.llvm_ir.is_none());
}

#[test]
fn language_features_demo_match_enum() {
    let path = examples_dir().join("language_features/demo.ny");
    assert_eq!(run_nyra_file(&path), "1");
}

#[test]
fn compiles_buffered_io_without_errors() {
    let path = examples_dir().join("syntax/buffered_io.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(ir.contains("stdout_write_str"));
    assert!(ir.contains("stdout_writeln_i32"));
    assert!(ir.contains("stdout_flush"));
}

#[test]
fn hello_ir_uses_printf_for_print() {
    let path = examples_dir().join("comparison/hello/hello.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(ir.contains("@printf") || ir.contains("@puts"));
    assert!(
        ir.contains("call i32 (ptr, ...) @printf") || ir.contains("call i32 @puts"),
        "expected printf or puts for print: {ir}"
    );
}

#[test]
fn immutable_let_avoids_stack_alloca() {
    let src = r#"fn main() -> void {
    let x = 10
    let y = 20
    print(x + y)
}"#;
    let output = Compiler::compile_source(src, "ssa.ny", &CompileOptions::default()).unwrap();
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(!ir.contains("alloca"), "expected SSA locals without alloca:\n{ir}");
    // Entry glue always records `rt_args_init`; print uses libc printf (not runtime modules).
    assert!(
        output
            .runtime_profile
            .symbols
            .iter()
            .all(|s| s == "rt_args_init"),
        "unexpected runtime symbols: {:?}",
        output.runtime_profile.symbols
    );
}

#[test]
fn immutable_bool_avoids_stack_alloca() {
    let src = r#"fn main() -> void {
    let ok = true
    if ok {
        print(1)
    }
}"#;
    let output = Compiler::compile_source(src, "ssa_bool.ny", &CompileOptions::default()).unwrap();
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(!ir.contains("alloca"), "expected SSA bool without alloca:\n{ir}");
}

#[test]
fn mutable_i32_uses_ssa_not_alloca_for_reassignment() {
    let src = r#"fn main() -> void {
    let mut n = 0
    n = 1
    print(n)
}"#;
    let output = Compiler::compile_source(src, "mut.ny", &CompileOptions::default()).unwrap();
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(
        !ir.contains("alloca i32"),
        "mut i32 should promote to SSA:\n{ir}"
    );
}

#[test]
fn end_to_end_spawn_channel_sync() {
    let path = examples_dir().join("syntax/spawn_channel.ny");
    let output = run_nyra(&["run", &path.to_string_lossy()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
}

#[test]
fn end_to_end_async_hello() {
    let path = examples_dir().join("syntax/async_hello.ny");
    let output = run_nyra(&["run", &path.to_string_lossy()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
}

#[test]
fn end_to_end_buffered_io_output() {
    let path = examples_dir().join("syntax/buffered_io.ny");
    let output = run_nyra(&["run", &path.to_string_lossy()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "lines:\n1\n2\n3\n"
    );
}

#[test]
fn rejects_flush_with_arguments() {
    let src = r#"fn main() {
    flush(1)
}"#;
    let output = Compiler::compile_source(src, "bad.ny", &CompileOptions::default()).unwrap();
    assert!(!output.type_errors.is_empty());
}

#[test]
fn compiles_template_strings_and_multi_arg_print() {
    let path = examples_dir().join("syntax/template_strings.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(ir.contains("@printf"));
    assert!(ir.contains("Hello"));
    assert!(
        ir.contains("i32_to_string") || ir.contains("%d"),
        "template i32 interpolation should use printf %d or i32_to_string"
    );
}

#[test]
fn end_to_end_template_strings_output() {
    let path = examples_dir().join("syntax/template_strings.ny");
    let output = run_nyra(&["run", &path.to_string_lossy()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, hamdy\nHello hamdy\nHello hamdy, age 25\n"
    );
}

#[test]
fn compiles_tcp_echo_examples() {
    for name in ["server.ny", "client.ny"] {
        let path = examples_dir().join("projects/tcp_echo").join(name);
        let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
        assert!(output.lexer_errors.is_empty(), "{name}: {:?}", output.lexer_errors);
        assert!(output.parser_errors.is_empty(), "{name}: {:?}", output.parser_errors);
        assert!(output.type_errors.is_empty(), "{name}: {:?}", output.type_errors);
        assert!(output.borrow_errors.is_empty(), "{name}: {:?}", output.borrow_errors);
    }
}

#[test]
fn end_to_end_tcp_echo() {
    use std::process::Command;

    let nyra = nyra_bin();
    let server_path = examples_dir().join("projects/tcp_echo/server.ny");
    let client_path = examples_dir().join("projects/tcp_echo/client.ny");
    let mut server = Command::new(&nyra)
        .env_remove("NYRA_HOME")
        .arg("run")
        .arg(&server_path)
        .spawn()
        .expect("spawn tcp server");
    thread::sleep(Duration::from_millis(1500));
    let client = Command::new(&nyra)
        .env_remove("NYRA_HOME")
        .arg("run")
        .arg(&client_path)
        .output()
        .expect("run tcp client");
    let _ = server.kill();
    let _ = server.wait();
    assert!(
        client.status.success(),
        "client stderr: {}",
        String::from_utf8_lossy(&client.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&client.stdout).trim(), "1");
}

#[test]
fn end_to_end_http_fetch() {
    let path = examples_dir().join("projects/http_hello/main.ny");
    assert_eq!(run_nyra_file(&path), "1");
}

#[test]
fn compiles_arrays_example() {
    let path = examples_dir().join("syntax/arrays.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_tuples_example() {
    let path = examples_dir().join("syntax/tuples.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn end_to_end_tuples_output() {
    let path = examples_dir().join("syntax/tuples.ny");
    assert_eq!(run_nyra_file(&path), "1\n1");
}

#[test]
fn end_to_end_vectors_output() {
    use std::process::Command;

    let path = examples_dir().join("syntax/vectors.ny");
    let output = Command::new(nyra_bin())
        .env("NYRA_HOME", "")
        .arg("run")
        .arg(&path)
        .output()
        .expect("run");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2\n10");
}

#[test]
fn end_to_end_unicode_print() {
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("nyra_unicode_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("unicode.ny");
    {
        let mut f = std::fs::File::create(&src).unwrap();
        write!(f, "fn main() {{ print(\"█\") }}\n").unwrap();
    }
    let output = run_nyra(&["run", src.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "█");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn end_to_end_colored_print() {
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("nyra_color_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("color.ny");
    {
        let mut f = std::fs::File::create(&src).unwrap();
        write!(
            f,
            "fn main() {{\n    print(\"Hello\", color: red)\n    print(\"Hex\", color: \"#00FF00\")\n}}\n"
        )
        .unwrap();
    }
    let output = run_nyra(&["run", src.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello"));
    assert!(stdout.contains("Hex"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn async_state_machine_string_links_async_future_done() {
    use compiler::{load_program_with_options, parse_source, set_diagnostic_root, LoadOptions};
    let path = tests_dir().join("nyra/async_state_machine_string_test.ny");
    let compile_opts = CompileOptions::default();
    let load_opts = LoadOptions {
        auto_prelude: true,
    };
    let loaded = load_program_with_options(&path, load_opts).unwrap();
    let mut program = loaded.program;
    program.functions.retain(|f| f.name != "main");
    let harness_main = parse_source(
        "fn main() {\n    test_state_machine_string_return()\n}",
        "harness.ny",
    )
    .unwrap();
    program.functions.extend(harness_main.functions);
    set_diagnostic_root(path.parent().unwrap());
    let output = Compiler::compile_program(
        &program,
        &path.to_string_lossy(),
        &compile_opts,
        Some(&path),
        loaded.errors,
    )
    .unwrap();
    assert!(
        output.type_errors.is_empty(),
        "type errors: {:?}",
        output.type_errors
    );
    let ir = output.llvm_ir.expect("llvm ir");
    assert!(
        ir.contains("async_future_done"),
        "expected async_future_done in IR"
    );
    assert!(
        output.runtime_profile.symbols.contains("async_future_done"),
        "runtime profile missing async_future_done: {:?}",
        output.runtime_profile.symbols
    );
    use compiler::runtime_map::resolve_runtime_modules_installed;
    let mods = resolve_runtime_modules_installed(&output.runtime_profile, "").unwrap();
    assert!(
        mods.iter().any(|p| p.ends_with("rt_async.c")),
        "link modules missing rt_async.c: {:?}",
        mods
    );
}

#[test]
fn compiles_input_builtin() {
    let src = r#"fn main() {
    let x = input()
    let y = input("Name: ")
    print(x)
    print(y)
}"#;
    let output = Compiler::compile_source(src, "input.ny", &CompileOptions::default()).unwrap();
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    let ir = output.llvm_ir.expect("llvm");
    assert!(ir.contains("@stdin_read_line"));
    assert!(
        output
            .runtime_profile
            .symbols
            .contains("stdin_read_line"),
        "runtime profile: {:?}",
        output.runtime_profile.symbols
    );
}

#[test]
fn nyra_cli_builds_input_example() {
    use std::io::Write;
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!("nyra_input_cli_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("prompt.ny");
    {
        let mut f = std::fs::File::create(&src).unwrap();
        write!(
            f,
            "fn main() {{\n    let n = input(\"Name: \")\n    print(n)\n}}\n"
        )
        .unwrap();
    }
    let output = Command::new(common::nyra_bin())
        .env_remove("NYRA_HOME")
        .arg("build")
        .arg(&src)
        .arg("-o")
        .arg("prompt")
        .output()
        .expect("nyra build");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bin = dir.join("target/debug/prompt");
    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        stdin.write_all(b"Nyra\n").unwrap();
    }
    let finished = child.wait_with_output().unwrap();
    assert!(finished.status.success());
    assert!(String::from_utf8_lossy(&finished.stdout).contains("Nyra"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn end_to_end_hashmap_output() {
    use std::process::Command;

    let path = examples_dir().join("syntax/hashmap.ny");
    let output = Command::new(nyra_bin())
        .env("NYRA_HOME", "")
        .arg("run")
        .arg(&path)
        .output()
        .expect("run");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "100\n1");
}

#[test]
fn end_to_end_hashmap_chain_output() {
    use std::process::Command;

    let path = examples_dir().join("syntax/hashmap_chain.ny");
    let output = Command::new(nyra_bin())
        .env("NYRA_HOME", "")
        .arg("run")
        .arg(&path)
        .output()
        .expect("run");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1\n2\n1");
}

#[test]
fn end_to_end_break_and_clone() {
    use std::process::Command;

    let src = r#"fn main() {
    let mut i = 0
    while i < 10 {
        i = i + 1
        if i == 3 { break }
    }
    let a = "ok"
    let b = a.clone()
    print(i)
    print(b)
}"#;
    let dir = std::env::temp_dir().join("nyra_break_clone_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.ny");
    std::fs::write(&path, src).unwrap();
    let output = Command::new(nyra_bin())
        .env("NYRA_HOME", "")
        .arg("run")
        .arg(&path)
        .output()
        .expect("run");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "3\nok");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compiles_arrow_fn_smoke_without_errors() {
    let path = examples_dir().join("arrow_fn_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_closure_smoke_without_errors() {
    let path = examples_dir().join("closure_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_inferred_arrow_smoke_without_errors() {
    let path = examples_dir().join("inferred_arrow_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_nullish_smoke_without_errors() {
    let path = examples_dir().join("nullish_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_closure_loop_smoke_without_errors() {
    let path = examples_dir().join("closure_loop_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_closure_escape_smoke_without_errors() {
    let path = examples_dir().join("closure_escape_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_optional_chain_smoke_without_errors() {
    let path = examples_dir().join("optional_chain_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_monolith_struct_smoke_without_errors() {
    let path = examples_dir().join("monolith_struct_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    assert!(output.llvm_ir.is_some());
}

#[test]
fn compiles_graph_arc_smoke_without_errors() {
    let path = examples_dir().join("graph_arc_smoke.ny");
    let output = Compiler::compile_file(&path, &CompileOptions::default()).unwrap();
    assert!(output.lexer_errors.is_empty(), "{:?}", output.lexer_errors);
    assert!(output.parser_errors.is_empty(), "{:?}", output.parser_errors);
    assert!(output.type_errors.is_empty(), "{:?}", output.type_errors);
    assert!(output.borrow_errors.is_empty(), "{:?}", output.borrow_errors);
    assert!(output.llvm_ir.is_some());
}
