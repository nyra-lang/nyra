import "stdlib/testing.ny"
import "stdlib/json/mod.ny"
import "stdlib/strings/builder.ny"
import "stdlib/fs/mod.ny"
import "stdlib/path.ny"
import "stdlib/vec.ny"
import "stdlib/vec_str.ny"
import "stdlib/time/sugar.ny"
import "stdlib/env/mod.ny"
import "stdlib/error.ny"
import "stdlib/process.ny"
import "stdlib/uuid/mod.ny"
import "stdlib/encoding/mod.ny"

fn test_json_short() {
    let o = jparse("{\"name\":\"Nyra\",\"n\":7,\"ok\":true,\"user\":{\"id\":1}}")
    assert_eq(strcmp(jstr(o, "name"), "Nyra"), 0)
    assert_eq(jnum(o, "n"), 7)
    assert_eq(jbool(o, "ok"), 1)
    assert_eq(jnum(jobj(o, "user"), "id"), 1)
    let built = dict().insert("a", "1").insert("b", "two")
    assert_eq(strcmp(jraw(built, "a"), "1"), 0)
}

fn test_string_builder() {
    let out = sb().push("hi").push(" ").push_i32(42).build()
    assert_eq(strcmp(out, "hi 42"), 0)
    assert_eq(strcmp(cat3("a", "b", "c"), "abc"), 0)
}

fn test_fs_aliases() {
    let p = "nyra_sugar_demo.txt"
    assert_eq(spit(p, "hello"), 0)
    assert_eq(strcmp(slurp(p), "hello"), 0)
    assert_eq(path(p).exists(), 1)
    assert_eq(strcmp(path(p).read(), "hello"), 0)
    assert_eq(rm(p), 0)
}

fn test_vec_and_strs() {
    let v = vec().push(1).push(2).push(3)
    assert_eq(v.len(), 3)
    assert_eq(v.get(1), 2)
    let s = strs().push("a").push("b").joined(",")
    assert_eq(strcmp(s, "a,b"), 0)
}

fn test_time_env_misc() {
    let t0 = now()
    ms(1).sleep()
    assert_eq(1, if t0.elapsed_ms() >= 0 { 1 } else { 0 })
    assert_eq(strcmp(env_or("__NYRA_MISSING__", "fallback"), "fallback"), 0)
    assert_eq(strlen(uuid()), 36)
    assert_eq(strcmp(b64("ab"), base64_encode("ab")), 0)
    let c = cmd("true")
    assert_eq(strcmp(c.program, "true"), 0)
}

fn test_error_methods() {
    let e = err_io("boom").context("open")
    assert_eq(1, if strstr_pos(e.format(), "open") >= 0 { 1 } else { 0 })
}

fn main() {
    test_json_short()
    test_string_builder()
    test_fs_aliases()
    test_vec_and_strs()
    test_time_env_misc()
    test_error_methods()
    print("stdlib sugar ok")
}
