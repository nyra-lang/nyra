"""Generate runnable examples/tests for stdlib-extern and stdlib-pure scaffolds."""
from __future__ import annotations

from builtin_dev.spec import ArgSpec, NyraType

from .patch import wrap_scaffold
from .spec import StdlibFnSpec


def _main_open(typed: bool) -> str:
    return "fn main() -> void {" if typed else "fn main() {"


def _import_line(spec: StdlibFnSpec) -> str:
    return f'import "{spec.stdlib_path}"'


def _arg_literal(arg: ArgSpec) -> str:
    if arg.nyra_type == NyraType.F64:
        if arg.name in ("base", "x", "y", "lo", "hi", "exp"):
            defaults = {"base": "2.0", "x": "3.0", "y": "4.0", "lo": "0.0", "hi": "1.0", "exp": "3.0"}
            return defaults.get(arg.name, "1.0")
        return "1.0"
    if arg.nyra_type == NyraType.I32:
        return "0" if arg.name == "index" else "1"
    if arg.nyra_type == NyraType.STRING:
        if arg.name == "json":
            return '"4869"'
        return '"x"'
    if arg.nyra_type == NyraType.PTR:
        return "ptr(0)"
    return "0"


def _call_args(spec: StdlibFnSpec) -> str:
    return ", ".join(_arg_literal(a) for a in spec.args)


def _math_demo(spec: StdlibFnSpec) -> str | None:
    if not spec.ny_module.startswith("math"):
        return None
    fn = spec.fn_name
    alias = spec.ny_alias or (fn.replace("_f64", "") if fn.endswith("_f64") else fn)
    samples: dict[str, str] = {
        "floor": "3.7",
        "ceil": "3.2",
        "round": "3.5",
        "sqrt": "16.0",
        "pow": "2.0, 3.0",
        "log": "2.718281828",
        "exp": "1.0",
        "clamp": "1.5, 0.0, 1.0",
        "trunc": "1.9",
        "hypot": "3.0, 4.0",
        "asin": "0.0",
        "acos": "1.0",
        "atan": "1.0",
        "log10": "100.0",
        "log2": "8.0",
    }
    args = samples.get(alias, "1.0")
    return f"    print({alias}({args}))"


def _map_demo(spec: StdlibFnSpec) -> str | None:
    fn = spec.fn_name
    if fn.startswith("map_str_i32_"):
        method = fn.replace("map_str_i32_", "")
        if method == "len":
            return (
                '    let m = HashMap_str_i32_new().insert("a", 1).insert("b", 2)\n'
                "    print(m.len())"
            )
        if method == "values":
            return (
                '    let m = HashMap_str_i32_new().insert("a", 1).insert("b", 2)\n'
                "    print(m.values().len())"
            )
        if method == "clear":
            return (
                '    let m = HashMap_str_i32_new().insert("a", 1).clear()\n'
                "    print(m.len())"
            )
    if fn.startswith("map_str_str_"):
        method = fn.replace("map_str_str_", "")
        if method == "len":
            return (
                '    let m = HashMap_str_str_new().insert("a", "1").insert("b", "2")\n'
                "    print(m.len())"
            )
        if method == "values":
            return (
                '    let m = HashMap_str_str_new().insert("a", "1").insert("b", "2")\n'
                "    print(m.values().len())"
            )
        if method == "clear":
            return (
                '    let m = HashMap_str_str_new().insert("a", "1").clear()\n'
                "    print(m.len())"
            )
    return None


def _vec_demo(spec: StdlibFnSpec) -> str | None:
    demos = {
        "vec_i32_insert": (
            "    let v = vec().push(1).push(3).insert(1, 2)\n"
            "    print(v.get(1))"
        ),
        "vec_i32_remove_at": (
            "    let v = vec().push(10).push(20).remove(0)\n"
            "    print(v.len())"
        ),
        "vec_i32_clear": (
            "    let v = vec().push(1).clear()\n"
            "    print(v.len())"
        ),
        "vec_i32_reverse": (
            "    let v = vec().push(1).push(2).push(3).reverse()\n"
            "    print(v.get(0))"
        ),
        "vec_i32_sort": (
            "    let v = vec().push(3).push(1).push(2).sort()\n"
            "    print(v.get(0))"
        ),
    }
    return demos.get(spec.fn_name)


def _encoding_demo(spec: StdlibFnSpec) -> str | None:
    if spec.fn_name == "hex_decode":
        return '    print(hex_decode("4869"))'
    return None


def _strconv_demo(spec: StdlibFnSpec) -> str | None:
    if spec.fn_name in ("str_to_bool", "parse_bool"):
        return (
            '    print(parse_bool("true"))\n'
            '    print(parse_bool("false"))'
        )
    return None


def _pure_impl_demo(spec: StdlibFnSpec) -> str | None:
    src = spec.pure_source or ""
    if "impl HashMap_str_i32" in src and "fn len" in src:
        return (
            '    let mut m = HashMap_str_i32_new().insert("a", 1).insert("b", 2)\n'
            "    print(m.len())\n"
            "    m = m.clear()\n"
            "    print(m.len())"
        )
    if "impl VecI32" in src and "fn binary_search" in src:
        return (
            "    let v = vec().push(1).push(3).push(5).push(7)\n"
            "    print(v.binary_search(5))"
        )
    return None


def demo_body(spec: StdlibFnSpec) -> str:
    for builder in (
        _pure_impl_demo,
        _math_demo,
        _map_demo,
        _vec_demo,
        _encoding_demo,
        _strconv_demo,
    ):
        body = builder(spec)
        if body:
            return body
    if spec.args:
        return f"    print({_call_args(spec)})"
    if spec.returns == NyraType.VOID:
        return f"    {spec.fn_name}()"
    return f"    print({spec.fn_name}())"


def example_ny_from_stdlib(spec: StdlibFnSpec, marker: str, *, typed: bool = False) -> str:
    lines = [_import_line(spec), "", _main_open(typed), demo_body(spec), "}"]
    return wrap_scaffold("\n".join(lines), marker)


def extern_test_body(spec: StdlibFnSpec) -> list[str]:
    fn = spec.fn_name
    if fn.endswith("_f64") or spec.ny_module.startswith("math"):
        args = _call_args(spec)
        if fn == "hypot_f64" or (spec.ny_alias == "hypot"):
            return [
                f"    let x = {fn}({_call_args(spec)})" if args else f"    let x = {fn}(3.0, 4.0)",
                "    if x < 2.0 {",
                "        assert_eq(1, 0)",
                "    }",
            ]
        if fn == "trunc_f64":
            return [
                "    let x = trunc_f64(1.9)",
                "    if x < 1.0 { assert_eq(1, 0) }",
                "    if x > 1.0 { assert_eq(1, 0) }",
            ]
        return [f"    let x = {fn}({_call_args(spec)})", "    if x < 0.0 { assert_eq(1, 0) }"]
    if fn == "hex_decode":
        return ['    assert_str_eq(hex_decode("4869"), "Hi")']
    if fn in ("str_to_bool", "parse_bool"):
        return [
            '    assert_eq(parse_bool("true"), 1)',
            '    assert_eq(parse_bool("false"), 0)',
        ]
    if fn.startswith("map_str_") or fn.startswith("vec_i32_"):
        return ["    // TODO: assert behavior", "    assert_eq(1, 1)"]
    if spec.returns == NyraType.I32:
        return [f"    assert_eq({fn}({_call_args(spec)}), 0)  // TODO: set expected"]
    if spec.returns == NyraType.STRING:
        return [f'    assert_str_eq({fn}({_call_args(spec)}), "")  // TODO: set expected']
    if spec.returns == NyraType.F64:
        return [f"    let x = {fn}({_call_args(spec)})", "    if x < 0.0 { assert_eq(1, 0) }"]
    return ["    // TODO: assert behavior", "    assert_eq(1, 1)"]
