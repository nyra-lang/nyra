//! Conformance tests: trait dynamic dispatch (CONF-TRAIT-*).

use crate::common::compile;

#[test]
fn conf_trait_001_dyn_dispatch() {
    let out = compile(
        r#"trait Add {
    fn add(self, other: i32) -> i32
}

struct Counter {
    value: i32
}

impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}

fn call_add(g: dyn Add) -> i32 {
    return g.add(1)
}

fn main() {
    let c = Counter { value: 10 }
    print(call_add(c as dyn Add))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.as_ref().expect("llvm ir");
    assert!(ir.contains("__dyn_Add_add"), "missing dyn dispatch fn");
    assert!(ir.contains("Add_dyn_Counter"), "missing box fn");
    assert!(ir.contains("vtable_Add_Counter"), "missing vtable");
}

#[test]
fn conf_trait_002_cast_without_impl_errors() {
    let out = compile(
        r#"trait Greet {
    fn hello(self) -> i32
}

struct Box {
    n: i32
}

fn main() {
    let b = Box { n: 1 }
    let _g = b as dyn Greet
}"#,
    );
    assert!(
        !out.type_errors.is_empty(),
        "expected error for missing impl"
    );
}

#[test]
fn conf_trait_003_dyn_send_bounds() {
    let out = compile(
        r#"trait Add {
    fn add(self, other: i32) -> i32
}

struct Counter {
    value: i32
}

impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}

fn call_add(g: dyn Add + Send) -> i32 {
    return g.add(1)
}

fn main() {
    let c = Counter { value: 10 }
    print(call_add(c as dyn Add + Send))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_trait_004_dyn_send_rejects_non_send() {
    let out = compile(
        r#"trait Add {
    fn add(self, other: i32) -> i32
}

struct Holder {
    p: *i32
}

impl Add for Holder {
    fn add(self, other: i32) -> i32 {
        return other
    }
}

fn main() {
    let h = Holder { p: 0 as *i32 }
    let g = h as dyn Add + Send
    print(g.add(1))
}"#,
    );
    assert!(
        !out.type_errors.is_empty(),
        "expected Send error for raw pointer field"
    );
    assert!(out.type_errors.iter().any(|e| e.message.contains("is not `Send`")));
}

#[test]
fn conf_trait_005_multi_method_vtable_index() {
    let out = compile(
        r#"trait Calc {
    fn add(self, other: i32) -> i32
    fn mul(self, other: i32) -> i32
}

struct Counter {
    value: i32
}

impl Calc for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
    fn mul(self, other: i32) -> i32 {
        return self.value * other
    }
}

fn call_calc(g: dyn Calc) -> i32 {
    return g.add(1) + g.mul(2)
}

fn main() {
    let c = Counter { value: 10 }
    print(call_calc(c as dyn Calc))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.as_ref().expect("llvm ir");
    assert!(ir.contains("__dyn_Calc_add"), "missing add dispatch");
    assert!(ir.contains("__dyn_Calc_mul"), "missing mul dispatch");
    assert!(
        ir.contains("getelementptr ptr, ptr %vt, i32 1"),
        "mul should use vtable index 1, got:\n{ir}"
    );
    assert!(ir.contains("__dyn_Calc_drop"), "missing dyn drop");
    assert!(ir.contains("dynthunk_drop_Counter"), "missing drop thunk");
}
