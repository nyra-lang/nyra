//! Conformance tests: type inference (CONF-INF-*).

use crate::common::compile;
use compiler::CompileStage;

#[test]
fn conf_inf_001_let_infers_i32() {
    let out = compile(
        r#"fn main() {
    let x = 42
    print(x)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty());
}

#[test]
fn conf_inf_002_void_return_default() {
    let out = compile(
        r#"fn run() {
    print(1)
}
fn main() {
    run()
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_003_return_type_inferred() {
    let out = compile(
        r#"fn twice(x: i32) {
    return x + x
}
fn main() {
    print(twice(3))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_inf_004_generic_call_site_inference() {
    let out = compile(
        r#"fn id<T>(x: T) -> T {
    return x
}
fn main() {
    print(id(7))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("id__i32") || ir.contains("@id"));
}

#[test]
fn conf_inf_005_struct_ctor_sugar() {
    let out = compile(
        r#"struct User {
    name: string
    age: i32
}
fn main() {
    let u = User("Ada")
    print(u.age)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_006_fn_param_inferred_without_annotation() {
    let out = compile(
        r#"fn add(a, b) {
    return a + b
}
fn main() {
    print(add(10, 20))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_008_untyped_string_from_call() {
    let out = compile(
        r#"extern fn strcat(a: string, b: string) -> string

fn show(path) {
    let s = strcat(path, "")
    print(s)
}
fn main() {
    show("x.txt")
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_009_inference_failure_requires_explicit_type() {
    let out = compile(
        r#"fn unused_param(x) {
}
fn main() {
    print(1)
}"#,
    );
    assert!(!out.type_errors.is_empty(), "expected inference failure");
    let msg = format!("{:?}", out.type_errors);
    assert!(msg.contains("E004"), "{msg}");
    assert!(msg.contains("unused_param"), "{msg}");
    assert!(msg.contains("could not infer"), "{msg}");
}

#[test]
fn conf_inf_010_param_inferred_from_call_site() {
    let out = compile(
        r#"fn echo(x) {
    print(x)
}
fn main() {
    echo(42)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_011_strvec_cli_untyped() {
    let out = compile(
        r#"struct StrVec { handle: ptr }
extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_get(v: ptr, index: i32) -> string
extern fn vec_str_len(v: ptr) -> i32
extern fn strcmp(a: string, b: string) -> i32
extern fn strlen(s: string) -> i32

fn StrVec_new() -> StrVec {
    return StrVec { handle: vec_str_new() }
}

impl StrVec {
    fn push(self, value: string) -> StrVec {
        vec_str_push(self.handle, value)
        return StrVec { handle: self.handle }
    }
    fn get(self, index: i32) -> string {
        return vec_str_get(self.handle, index)
    }
    fn len(self) -> i32 {
        return vec_str_len(self.handle)
    }
}

fn Cli_strip_flags(args) {
    let n = args.len()
    let mut v = StrVec_new()
    let mut i = 0
    while i < n {
        let a = args.get(i)
        if strlen(a) == 0 {
            v = v.push(a)
        }
        i = i + 1
    }
    return v
}

fn Cat_run(args) {
    let files = Cli_strip_flags(args)
    return files.len()
}

fn main() {
    return Cat_run(StrVec_new())
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_012_strcat_param_inferred_in_print() {
    let out = compile(
        r#"extern fn strcat(a: string, b: string) -> string

fn Cli_usage(tool, text) {
    print(strcat(strcat("usage: ", tool), text))
}

fn main() {
    Cli_usage("cat", " [file...]")
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_012b_strcmp_body_only_ref() {
    let out = compile(
        r#"fn f(flag) {
    if strcmp("x", flag) == 0 {
        return 1
    }
    return 0
}
fn main() {
    print(0)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_014_cat_run_files_get() {
    let out = compile(
        r#"struct StrVec { handle: ptr }
extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_get(v: ptr, index: i32) -> string
extern fn vec_str_len(v: ptr) -> i32

fn StrVec_new() -> StrVec {
    return StrVec { handle: vec_str_new() }
}

impl StrVec {
    fn push(self, value: string) -> StrVec {
        vec_str_push(self.handle, value)
        return StrVec { handle: self.handle }
    }
    fn get(self, index: i32) -> string {
        return vec_str_get(self.handle, index)
    }
    fn len(self) -> i32 {
        return vec_str_len(self.handle)
    }
}

fn Cli_strip_flags(args) {
    let n = args.len()
    let mut v = StrVec_new()
    let mut i = 0
    while i < n {
        let a = args.get(i)
        if strlen(a) == 0 {
            v = v.push(a)
        }
        i = i + 1
    }
    return v
}

fn Cat_run(args) {
    let files = Cli_strip_flags(args)
    let n = files.len()
    let mut i = 0
    while i < n {
        let path = files.get(i)
        i = i + 1
    }
    return n
}

fn main() {
    return Cat_run(StrVec_new())
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_015_enum_param_and_field_return() {
    let out = compile(
        r#"enum SplitDirection {
    Horizontal
    Vertical
}

fn split_active(app, dir) {
    if dir == SplitDirection.Horizontal {
        return 1
    }
    return 0
}

fn main() {
    split_active(0, SplitDirection.Vertical)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_016_tab_manager_active_returns_i32() {
    let out = crate::common::compile_stage(
        r#"extern fn i32_to_string(n: i32) -> string
extern fn map_str_str_get(m: ptr, key: string) -> string

struct TabRecord {
    id: i32
    name: string
}

struct TabManager {
    names: ptr
    active_id: i32
}

fn Id_to_key(id) {
    return i32_to_string(id)
}

fn TabManager_load_record(mgr, id: i32) {
    let key = Id_to_key(id)
    return TabRecord {
        id: id
        name: map_str_str_get(mgr.names, key)
    }
}

fn TabManager_active(mgr) {
    return mgr.active_id
}

fn use_active(mgr) {
    let active = TabManager_active(mgr)
    let tab = TabManager_load_record(mgr, active)
    return tab.name
}

fn main() {
    print(1)
}"#,
        CompileStage::TypeCheck,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_013_filesystem_cat_untyped() {
    let out = crate::common::compile_example("zero_types_cli.ny");
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_017_array_return_inferred_for_codegen() {
    let out = compile(
        r#"fn copy(src: [i32; 4], len: i32) {
    let mut out = [0; 4]
    let mut i = 0
    while i < len {
        out[i] = src[i]
        i = i + 1
    }
    return out
}
fn main() {
    let a = [1, 2, 3, 4]
    let b = copy(a, 4)
    print(b[0])
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("define [4 x i32] @copy("),
        "expected array return type in LLVM signature, got:\n{ir}"
    );
}

#[test]
fn conf_inf_019_if_branch_env_isolated() {
    let out = compile(
        r#"struct TaskList {
    items: StrVec
    next_id: i32
}
fn TaskList_done(list, id: i32) {
    return TaskList_set_status(list, id, "done")
}
fn TaskList_set_status(list, id: i32, status) {
    return TaskList { items: StrVec_new(), next_id: list.next_id }
}
fn TaskList_del(list, id: i32) {
    return TaskList { items: StrVec_new(), next_id: list.next_id }
}
fn run() {
    let mut list = TaskList { items: StrVec_new(), next_id: 1 }
    let cmd = "del 1"
    if strstr_pos(cmd, "done ") == 0 {
        let id = str_to_i32(substring(cmd, 5, strlen(cmd) - 5))
        list = TaskList_done(list, id)
    } else {
        if strstr_pos(cmd, "del ") == 0 {
            let id = str_to_i32(substring(cmd, 4, strlen(cmd) - 4))
            list = TaskList_del(list, id)
        }
    }
}
fn main() {
    run()
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}

#[test]
fn conf_inf_018_struct_param_field_access_codegen() {
    let out = compile(
        r#"struct Tree {
    height: [i32; 4]
    size: i32
}
fn tree_height(t, id) {
    return t.height[id]
}
fn main() {
    let t = Tree { height: [1, 2, 3, 4], size: 4 }
    print(tree_height(t, 0))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("%Tree") && ir.contains("tree_height"),
        "expected struct param in LLVM IR"
    );
}

#[test]
fn conf_inf_anonymous_object_literal() {
    let out = compile(
        r#"fn main() {
    let family = {
        name: "hamdy",
        age: 20,
        city: "cairo"
    }
    print(family.name)
    print(family.age)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty());
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("%__Anon") || ir.contains("%Family"),
        "expected synthesized or matched struct in IR"
    );
}

#[test]
fn conf_inf_anonymous_matches_declared_struct() {
    let out = compile(
        r#"struct Point {
    x: i32
    y: i32
}
fn main() {
    let p = { x: 1, y: 2 }
    print(p.x)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("%Point"), "anonymous literal should match Point");
}

#[test]
fn conf_inf_array_of_struct_literals_for_in() {
    let out = compile(
        r#"struct NumberColor {
    number: i32
    color: string
}

fn main() {
    let collections = [
        NumberColor { number: 1, color: "red" },
        NumberColor { number: 2, color: "blue" },
        NumberColor { number: 3, color: "green" },
    ]
    for item in collections {
        print(item.number)
    }
}"#,
    );
    assert!(out.parser_errors.is_empty(), "{:?}", out.parser_errors);
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    assert!(out.llvm_ir.is_some());
}
