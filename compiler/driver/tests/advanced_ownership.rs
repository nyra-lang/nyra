use compiler::{CompileOptions, Compiler};

fn compile(src: &str) -> compiler::CompileOutput {
    Compiler::compile_source(src, "test.ny", &CompileOptions::default()).unwrap()
}

#[test]
fn spawn_emits_capture_and_spawn_capture_call() {
    let out = compile(
        r#"fn main() {
    let mut n = 99
    n = 100
    spawn:thread {
        print(n)
    }
}"#,
    );
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(ir.contains("spawn_capture"));
    assert!(ir.contains("SpawnCap"));
}

#[test]
fn custom_drop_trait_compiles() {
    let out = compile(
        r#"struct Box Send {
    data: string
}

impl Drop for Box {
    fn drop(self) -> void {
        free(self.data)
    }
}

extern fn free(p: string) -> void

fn main() {
    let b = Box { data: "x" }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("Drop_Box_drop"),
        "expected Drop_Box_drop in IR, got:\n{ir}"
    );
}

#[test]
fn hrtb_fn_ptr_param_accepts_compatible_fn() {
    let out = compile(
        r#"fn echo<'a>(x: &'a string) -> &'a string {
    return x
}

fn with_cb(f: for<'a> fn(&'a string) -> &'a string) {
    let s = "hi"
    let r = f(&s)
    print(r)
}

fn main() {
    with_cb(echo)
}"#,
    );
    assert!(
        !out
            .borrow_errors
            .iter()
            .any(|e| e.message.contains("higher-ranked")),
        "{:?}",
        out.borrow_errors
    );
}

#[test]
fn fn_ptr_call_through_parameter() {
    let out = compile(
        r#"struct Context {
    value: i32
}

fn handler(ctx: Context) -> Context {
    return ctx
}

fn dispatch(f: fn(Context) -> Context, ctx: Context) -> Context {
    return f(ctx)
}

fn main() {
    let c = Context { value: 1 }
    let out = dispatch(handler, c)
    print(out.value)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.parser_errors.is_empty(), "{:?}", out.parser_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("call %Context"),
        "expected indirect fn ptr call in IR, got:\n{ir}"
    );
}

#[test]
fn struct_literal_with_call_field_compiles() {
    let out = compile(
        r#"struct Inner {
    n: i32
}

struct Outer {
    inner: Inner
}

fn make_inner() -> Inner {
    return Inner { n: 42 }
}

fn main() {
    let x = Outer { inner: make_inner() }
    print(x.inner.n)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        !ir.contains("store %Inner alloca"),
        "struct literal must load call result before store, got:\n{ir}"
    );
}

#[test]
fn struct_spread_updates_selected_fields() {
    let out = compile(
        r#"struct Pair {
    a: i32
    b: i32
}

fn main() {
    let p = Pair { a: 1, b: 2 }
    let q = Pair { ..p, b: 9 }
    print(q.a)
    print(q.b)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("%Pair = type"),
        "expected struct spread IR, got:\n{ir}"
    );
    assert!(
        ir.contains("load i32, i32* %gep.4"),
        "expected spread field load from base, got:\n{ir}"
    );
}

#[test]
fn fn_ptr_handler_route_style() {
    let out = compile(
        r#"struct Context {
    status: i32
}

fn handler(ctx: Context) -> Context {
    return Context { ..ctx, status: 200 }
}

fn invoke(f: fn(Context) -> Context, ctx: Context) -> Context {
    return f(ctx)
}

fn main() {
    let c = Context { status: 0 }
    let out = invoke(handler, c)
    print(out.status)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.parser_errors.is_empty(), "{:?}", out.parser_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("call %Context"),
        "expected fn ptr handler call, got:\n{ir}"
    );
}

#[test]
fn fn_ptr_passed_to_call_arg() {
    let out = compile(
        r#"struct Context {
    status: i32
}

fn handler(ctx: Context) -> Context {
    return Context { ..ctx, status: 200 }
}

fn register(_path: string, h: fn(Context) -> Context) -> void {
    return
}

fn main() {
    register("/hello", handler)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("ptr @handler"),
        "expected fn ptr arg without string GEP, got:\n{ir}"
    );
    assert!(
        !ir.contains("getelementptr inbounds i8, ptr @handler"),
        "handler must not be treated as string, got:\n{ir}"
    );
}

#[test]
fn mut_struct_reassign_from_call() {
    let out = compile(
        r#"struct Context {
    status: i32
}

fn bump(ctx: Context) -> Context {
    return Context { ..ctx, status: ctx.status + 1 }
}

fn main() {
    let mut ctx = Context { status: 0 }
    ctx = bump(ctx)
    print(ctx.status)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        !ir.contains("alloca %Context*"),
        "mut struct slot should be %Context not %Context*, got:\n{ir}"
    );
}

#[test]
fn fn_ptr_stored_and_called_from_local() {
    let out = compile(
        r#"struct Context {
    n: i32
}

fn handler(ctx: Context) -> Context {
    return ctx
}

fn main() {
    let f = handler
    let c = Context { n: 1 }
    let out = f(c)
    print(out.n)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("call %Context"),
        "expected indirect fn ptr call from local, got:\n{ir}"
    );
}

#[test]
fn return_struct_param_by_value() {
    let out = compile(
        r#"struct Pair {
    a: i32
    b: i32
}

fn identity(p: Pair) -> Pair {
    return p
}

fn main() {
    let p = Pair { a: 1, b: 2 }
    let q = identity(p)
    print(q.a)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("ret %Pair %"),
        "expected ret %Pair by value, got:\n{ir}"
    );
    assert!(
        !ir.contains("ret %Pair*"),
        "must not return struct pointer, got:\n{ir}"
    );
}

#[test]
fn mut_struct_param_updates_caller_slot() {
    let out = compile(
        r#"struct Counter {
    n: i32
}

fn bump(mut c: Counter) -> void {
    c.n = c.n + 1
}

fn main() {
    let mut c = Counter { n: 1 }
    bump(c)
    print(c.n)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("llvm");
    assert!(
        ir.contains("define void @bump(%Counter*"),
        "mut struct param should be pointer, got:\n{ir}"
    );
    assert!(
        ir.contains("call void @bump(%Counter*"),
        "call should pass struct pointer, got:\n{ir}"
    );
}
