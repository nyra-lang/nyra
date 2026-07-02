//! Conformance tests: language gaps fixed in v1.17.0 (CONF-LANG-*).

use crate::common::compile;
use compiler::parse_source;

#[test]
fn conf_lang_001_match_on_strings() {
    let out = compile(
        r#"fn dispatch(cmd) {
    return match cmd {
        "GET" => 1,
        "POST" => 2,
        _ => 0,
    }
}
fn main() {
    print(dispatch("GET"))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_lang_002_i64_to_string() {
    let out = compile(
        r#"extern fn instant_now() -> i64
fn main() {
    print(i64_to_string(instant_now()))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("i64_to_string"), "expected i64_to_string in IR:\n{ir}");
}

#[test]
fn conf_lang_003_nested_struct_return() {
    let out = compile(
        r#"struct SourceLoc {
    file: string
    line: i32
    col: i32
}
struct ParseCursor {
    text: string
    pos: i32
    loc: SourceLoc
}
fn advance(cur) {
    return ParseCursor {
        text: cur.text,
        pos: cur.pos + 1,
        loc: cur.loc
    }
}
fn main() {
    let loc = SourceLoc { file: "a.ny", line: 1, col: 1 }
    let c = ParseCursor { text: "hi", pos: 0, loc: loc }
    let c2 = advance(c)
    print(c2.pos)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("str_clone"),
        "nested struct return should deep-clone heap fields:\n{ir}"
    );
}

#[test]
fn conf_lang_004_continue_multi_mut() {
    let out = compile(
        r#"fn main() {
    let mut i = 0
    let mut sum = 0
    let mut prod = 1
    while i < 5 {
        i = i + 1
        if i == 3 {
            continue
        }
        sum = sum + i
        prod = prod * i
    }
    print(sum)
    print(prod)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_lang_006_match_or_patterns() {
    let out = compile(
        r#"enum Color { Red Green Blue }
fn bucket(c) {
    return match c {
        Color.Red | Color.Blue => 1
        Color.Green => 2
    }
}
fn main() {
    print(bucket(Color.Red))
    print(bucket(Color.Green))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_lang_007_match_nested_enum_patterns() {
    let out = compile(
        r#"enum Option_i32 { None, Some(i32) }
enum Result_Opt { Ok(Option_i32), Fail(Option_i32) }
fn peel(r) {
    return match r {
        Result_Opt.Ok(Some(x)) => x
        Result_Opt.Ok(Option_i32.None) => 0
        Result_Opt.Fail(_) => -1
    }
}
fn main() {
    print(peel(Result_Opt.Ok(Option_i32.Some(5))))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_lang_005_struct_param_call_site_inference() {
    let out = compile(
        r#"struct Point {
    x: i32
    y: i32
}
fn use_point(p) {
    return p.x + p.y
}
fn main() {
    print(use_point(Point { x: 2, y: 5 }))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_lang_008_match_struct_and_tuple_patterns() {
    let out = compile(
        r#"struct Point {
    x: i32
    y: i32
}
fn sum_point(p) {
    return match p {
        Point { x, y } => x + y
    }
}
fn main() {
    let p = Point { x: 3, y: 4 }
    print(sum_point(p))
    let pair = (10, 20)
    let total = match pair {
        (a, b) => a + b
    }
    print(total)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_lang_009_import_alias_and_visibility() {
    let program = parse_source(
        r#"pub fn greet(name) {
    return "hi " + name
}
priv fn secret() {
    return 99
}"#,
        "test.ny",
    )
    .expect("parse");
    let greet = program
        .functions
        .iter()
        .find(|f| f.name == "greet")
        .expect("greet");
    assert!(greet.public);
    let secret = program
        .functions
        .iter()
        .find(|f| f.name == "secret")
        .expect("secret");
    assert!(!secret.public, "priv fn must parse as non-public");
}

#[test]
fn conf_lang_010_c_union_repr_c() {
    let out = compile(
        r#"union IpAddr repr(C) {
    v4: i32
    v6: [u8; 16]
}
fn main() {
    unsafe {
        let u = IpAddr { v4: 0x7F000001 }
        print(u.v4)
    }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("%IpAddr = type"), "expected union type in IR:\n{ir}");
}

#[test]
fn conf_lang_011_size_of_intrinsic() {
    let out = compile(
        r#"fn main() {
    print(size_of<i32>())
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("i32 4"), "expected size_of<i32>() = 4 in IR:\n{ir}");
}

#[test]
fn conf_lang_012_heterogeneous_enum_payload() {
    let out = compile(
        r#"enum Result_str_i32 {
    Ok(string)
    Err(i32)
}
fn main() {
    let err = Result_str_i32.Err(42)
    let v = match err {
        Result_str_i32.Ok(s) => s.len(),
        Result_str_i32.Err(e) => e,
    }
    print(v)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_lang_013_stack_buffer_return_rejected() {
    let out = compile(
        r#"struct StackBuffer_i32_64 {
    data: [i32; 64]
}
fn leak() -> &StackBuffer_i32_64 {
    let buf = StackBuffer_i32_64 { data: [0; 64] }
    return &buf
}
fn main() {
    print(0)
}"#,
    );
    assert!(
        out.type_errors
            .iter()
            .any(|e| e.message.contains("StackBuffer")),
        "expected StackBuffer escape error: {:?}",
        out.type_errors
    );
}
