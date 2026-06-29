//! LLVM IR golden snapshots (insta).

mod common;

use common::{compile, normalize_ir};

macro_rules! snap_ir {
    ($name:ident, $src:expr) => {
        #[test]
        fn $name() {
            let out = compile($src);
            assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
            assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
            let ir = out.llvm_ir.expect("llvm ir");
            insta::with_settings!({
                filters => vec![
                    (r"\.\d+", ".N"),
                ],
            }, {
                insta::assert_snapshot!(stringify!($name), normalize_ir(&ir));
            });
        }
    };
}

snap_ir!(snap_hello, r#"fn main() { print("Hello") }"#);
snap_ir!(snap_math_add, r#"fn main() { print(10 + 20) }"#);
snap_ir!(snap_if_else, r#"fn main() {
    let x = 1
    if x == 1 { print(1) } else { print(0) }
}"#);
snap_ir!(snap_while_loop, r#"fn main() {
    let mut i = 0
    while i < 3 { i = i + 1 }
    print(i)
}"#);
snap_ir!(snap_for_range, r#"fn main() {
    let mut sum = 0
    for i in 0..5 { sum = sum + i }
    print(sum)
}"#);
snap_ir!(snap_for_in_array, r#"fn main() {
    let arr = [1, 2, 3]
    for x in arr { print(x) }
}"#);
snap_ir!(snap_array_length, r#"fn main() {
    let arr = [1, 2, 3, 4]
    print(arr.length())
}"#);
snap_ir!(snap_array_sort, r#"fn main() {
    let nums = [10, 1, 2, 8, 5]
    let sorted = nums.sort()
    for n in sorted { print(n) }
}"#);
snap_ir!(snap_array_sort_by, r#"fn main() {
    let nums = [3, 1, 2]
    let sorted = nums.sort_by((a, b) => a - b)
    for n in sorted { print(n) }
}"#);
snap_ir!(snap_string_length, r#"fn main() {
    let s = "hi"
    print(s.length())
}"#);
snap_ir!(snap_string_split, r#"fn main() {
    let parts = "a,b,c".split(",")
    print(parts.length())
}"#);
snap_ir!(snap_trim, r#"fn main() {
    let s = "  hi  "
    print(s.trim())
}"#);
snap_ir!(snap_date_builtin, r#"fn main() {
    let d = date()
    print(d.year)
    print(d.month)
    print(d.day)
}"#);
snap_ir!(snap_copy_i32, r#"fn main() {
    let a = 1
    let b = a
    print(a)
    print(b)
}"#);
snap_ir!(snap_string_move, r#"fn main() {
    let a = "hello"
    let b = a
    print(b)
}"#);
snap_ir!(snap_enum_match, r#"enum Color { Red Green }
fn main() {
    let c = Color.Red
    let n = match c { Color.Red => 1 Color.Green => 2 }
    print(n)
}"#);
snap_ir!(snap_struct_literal, r#"struct Point { x: i32 y: i32 }
fn main() {
    let p = Point { x: 1, y: 2 }
    print(p.x)
}"#);
snap_ir!(snap_modulo, r#"fn main() { print(7 % 4) }"#);
snap_ir!(snap_logical_and, r#"fn main() {
    let ok = true && false
    if ok { print(1) } else { print(0) }
}"#);
snap_ir!(snap_const_fold, r#"const N = 2 + 3
fn main() { print(N) }"#);
snap_ir!(snap_comparison_chain, r#"fn main() {
    let v = 5
    if v >= 3 && v <= 10 { print(1) } else { print(0) }
}"#);
snap_ir!(snap_negation, r#"fn main() { print(-42) }"#);
snap_ir!(snap_abs_intrinsic, r#"fn main() {
    let x = abs_i32(-42)
    print(x)
}"#);
snap_ir!(snap_bool_not, r#"fn main() {
    let ok = !false
    if ok { print(1) } else { print(0) }
}"#);
snap_ir!(snap_void_fn, r#"fn noop() -> void { return }
fn main() { noop() print(0) }"#);
snap_ir!(snap_return_value, r#"fn add(a: i32, b: i32) -> i32 { return a + b }
fn main() { print(add(3, 4)) }"#);
snap_ir!(snap_array_literal, r#"fn main() {
    let arr = [1, 2, 3]
    print(arr[0])
}"#);
snap_ir!(snap_tuple_literal, r#"fn main() {
    let t = (1, 2)
    print(t.0)
}"#);
snap_ir!(snap_spawn, r#"fn main() {
    let mut n = 99
    n = 100
    spawn { print(n) }
}"#);
snap_ir!(snap_async_fn, r#"async fn work() -> i32 { return 42 }
fn main() {
    let h = work()
    print(await h)
}"#);
snap_ir!(snap_defer, r#"fn main() {
    defer print(2)
    print(1)
}"#);
snap_ir!(snap_defer_return, r#"fn cleanup() -> void { print(2) }
fn main() {
    defer cleanup()
    return
}"#);
snap_ir!(snap_unsafe_block, r#"fn main() {
    unsafe {
        print(0)
    }
}"#);
snap_ir!(snap_generic_call, r#"fn id<T>(x: T) -> T { return x }
fn main() { print(id<i32>(7)) }"#);
snap_ir!(snap_template_string, r#"fn main() {
    let name = "Nyra"
    print(`Hello ${name}`)
}"#);
snap_ir!(snap_impl_method, r#"struct Counter { n: i32 }
impl Counter {
    fn value(self) -> i32 {
        return self.n
    }
}
fn main() {
    let c = Counter { n: 5 }
    print(c.value())
}"#);
snap_ir!(snap_break_while, r#"fn main() {
    let mut i = 0
    while i < 10 {
        i = i + 1
        if i == 3 { break }
    }
    print(i)
}"#);
snap_ir!(snap_continue_while, r#"fn main() {
    let mut sum = 0
    let mut i = 0
    while i < 5 {
        i = i + 1
        if i == 3 { continue }
        sum = sum + i
    }
    print(sum)
}"#);
snap_ir!(snap_clone_method, r#"fn main() {
    let a = "hi"
    let b = a.clone()
    print(b)
}"#);

#[test]
fn normalize_ir_canonicalizes_target_triple() {
    let ir = r#"source_filename = "test.ny"
target triple = "x86_64-unknown-linux-gnu"

define i32 @main() {
  ret i32 0
}
"#;
    let norm = normalize_ir(ir);
    assert!(norm.contains("target triple = \"nyra-snapshot-host\""));
    assert!(!norm.contains("x86_64-unknown-linux-gnu"));
}
