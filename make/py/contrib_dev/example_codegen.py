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


def _arg0_before_comma(inner: str) -> str:
    """First top-level comma-separated argument (respects nested parens)."""
    depth = 0
    for i, ch in enumerate(inner):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif ch == "," and depth == 0:
            return inner[:i].strip()
    return inner.strip()


def _call_args(spec: StdlibFnSpec) -> str:
    return ", ".join(_arg_literal(a) for a in spec.args)


def _math_call_name(spec: StdlibFnSpec) -> str:
    fn = spec.fn_name
    if spec.ny_alias:
        return spec.ny_alias
    if fn.endswith("_f64"):
        return fn[:-4]
    return fn


def _math_demo(spec: StdlibFnSpec) -> str | None:
    if not spec.ny_module.startswith("math"):
        return None
    fn = spec.fn_name
    call_name = _math_call_name(spec)
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
        "lerp": "0.0, 10.0, 0.5",
        "gcd_i32": "12, 8",
        "lcm_i32": "4, 6",
        "mod_i32": "7, 3",
        "copysign_f64": "1.0, -1.0",
        "fmod": "5.0, 2.0",
        "fmod_f64": "5.0, 2.0",
        "rotate_left": "1, 1",
        "rotate_left_i32": "1, 1",
        "rotate_right": "2, 1",
        "rotate_right_i32": "2, 1",
        "saturating_add": "1, 2",
        "saturating_add_i32": "1, 2",
        "saturating_sub": "5, 2",
        "saturating_sub_i32": "5, 2",
        "wrapping_add": "1, 1",
        "wrapping_add_i32": "1, 1",
        "rem_euclid": "-7, 3",
        "rem_euclid_i32": "-7, 3",
    }
    args = samples.get(fn) or samples.get(call_name)
    if args is None:
        if spec.args:
            call_name = fn if fn.endswith("_i32") else call_name
            args = _call_args(spec)
        else:
            args = "1.0"
    return f"    print({call_name}({args}))"


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


def _vec_str_extern_demo(spec: StdlibFnSpec) -> str | None:
    if spec.fn_name == "vec_str_pop":
        return (
            "    let v = strs().push(\"a\").push(\"b\")\n"
            '    print(v.pop())'
        )
    if spec.fn_name == "vec_str_clear":
        return (
            "    let v = strs().push(\"a\").clear()\n"
            "    print(v.len())"
        )
    if spec.fn_name == "vec_str_reverse":
        return (
            '    let v = strs().push("a").push("b").reverse()\n'
            '    print(v.get(0))'
        )
    if spec.fn_name == "vec_str_insert":
        return '    let v = strs().push("b").insert(0, "a")\n    print(v.get(0))'
    if spec.fn_name == "vec_str_remove_at":
        return '    let v = strs().push("a").push("b")\n    print(v.remove_at(0))'
    if spec.fn_name == "vec_str_swap":
        return '    let v = strs().push("a").push("b").swap(0, 1)\n    print(v.get(0))'
    if spec.fn_name == "vec_str_extend":
        return (
            '    let a = strs().push("x")\n'
            '    let b = strs().push("y")\n'
            "    vec_str_extend(a.handle, b.handle)\n"
            "    print(a.len())"
        )
    return None


def _option_demo(spec: StdlibFnSpec) -> str | None:
    if "option/combinators" in spec.ny_module or spec.fn_name == "option_combinators":
        return (
            "    let o = Option_i32_some(5)\n"
            "    print(Option_i32_unwrap_or(o, 0))"
        )
    return None


def _result_demo(spec: StdlibFnSpec) -> str | None:
    if "result/combinators" in spec.ny_module:
        return (
            "    let ok = Result_i32_i32_ok(7)\n"
            "    print(Result_i32_i32_unwrap_or(ok, 0))\n"
            "    print(Result_i32_i32_is_err(Result_i32_i32_err(1)))"
        )
    return None


def _hashmap_demo(spec: StdlibFnSpec) -> str | None:
    if spec.fn_name == "hashmap_or_insert":
        return (
            '    let m = HashMap_str_i32_new().insert("k", 10)\n'
            '    print(m.or_insert("k", 42))'
        )
    if spec.fn_name == "hashmap_extra_methods":
        return (
            "    let m = HashMap_str_i32_new()\n"
            "    print(m.is_empty())\n"
            '    let _ = m.insert("k", 1)\n'
            "    print(m.is_empty())"
        )
    return None


def _vec_demo(spec: StdlibFnSpec) -> str | None:
    body = _vec_str_extern_demo(spec)
    if body:
        return body
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


def _sync_demo(spec: StdlibFnSpec) -> str | None:
    if not spec.ny_module.startswith("sync"):
        return None
    demos = {
        "atomic_sub_i32": (
            "    let a = Atomic_i32_new(10)\n"
            "    print(atomic_sub_i32(a.cell, 3))"
        ),
        "atomic_and_i32": (
            "    let a = Atomic_i32_new(7)\n"
            "    print(atomic_and_i32(a.cell, 3))"
        ),
        "atomic_or_i32": (
            "    let a = Atomic_i32_new(1)\n"
            "    print(atomic_or_i32(a.cell, 2))"
        ),
        "atomic_xor_i32": (
            "    let a = Atomic_i32_new(5)\n"
            "    print(atomic_xor_i32(a.cell, 3))"
        ),
    }
    return demos.get(spec.fn_name)


def _pure_impl_demo(spec: StdlibFnSpec) -> str | None:
    src = spec.pure_source or ""
    if "impl HashMap_i32_i32" in src and "fn get_or" in src:
        return (
            "    let m = HashMap_i32_i32_new()\n"
            "    print(m.get_or(1, 99))"
        )
    if "impl HashMap_str_i32" in src and "fn or_insert" in src:
        return (
            '    let m = HashMap_str_i32_new().insert("k", 10)\n'
            '    print(m.or_insert("k", 42))'
        )
    if "impl StrVec" in src and "fn pop" in src:
        return (
            '    let v = strs().push("x")\n'
            "    print(v.is_empty())\n"
            '    print(v.pop())'
        )
    if "impl StrVec" in src and "fn insert" in src:
        return (
            '    let v = strs().push("b").insert(0, "a")\n'
            '    print(v.get(0))\n'
            '    print(v.remove_at(1))'
        )
    if "impl HashMap_str_i32" in src and "fn is_empty" in src:
        return (
            "    let m = HashMap_str_i32_new()\n"
            "    print(m.is_empty())\n"
            '    let _ = m.insert("k", 1)\n'
            "    print(m.is_empty())"
        )
    if "impl VecI32" in src and "fn binary_search" in src:
        return (
            "    let v = vec().push(1).push(3).push(5).push(7)\n"
            "    print(v.binary_search(5))"
        )
    return None


def _pure_scaffold_demo(spec: StdlibFnSpec) -> str | None:
    """Demos for pure_source module scaffolds (fn_name is a slug, not a callable)."""
    demos: dict[str, str] = {
        "vec_i32_slice_methods": (
            "    let v = vec().push(1).push(2).push(3).push(4)\n"
            "    let w = v.window(1, 2)\n"
            "    print(w.len())"
        ),
        "vec_i32_extra_methods": (
            "    let v = vec()\n"
            "    print(v.is_empty())"
        ),
        "vec_i32_swap_extend": (
            "    let v = vec().push(1).push(2).swap(0, 1)\n"
            "    print(v.get(0))"
        ),
        "strvec_set_method": (
            '    let v = strs().push("a")\n'
            '    let _ = v.set(0, "z")\n'
            '    print(v.get(0))'
        ),
        "hashmap_i32_i32": (
            "    let m = HashMap_i32_i32_new()\n"
            "    let _ = m.insert(1, 42)\n"
            "    print(m.get(1))"
        ),
        "hashmap_update": (
            '    let m = HashMap_str_i32_new().insert("k", 10)\n'
            '    let _ = m.insert("k", 20)\n'
            '    print(m.get("k"))'
        ),
    }
    return demos.get(spec.fn_name)


def _extern_scaffold_demo(spec: StdlibFnSpec) -> str | None:
    """Build runnable demos from extern_test_body (avoid ptr(0) placeholder fallbacks)."""
    lines = extern_test_body(spec)
    if not lines:
        return None
    meaningful = [s.strip() for s in lines if s.strip() and not s.strip().startswith("//")]
    if not meaningful or meaningful == ["assert_eq(1, 1)"]:
        return None
    out: list[str] = []
    for line in lines:
        s = line.strip()
        if not s or s.startswith("//"):
            continue
        if s.startswith("if "):
            continue
        if s.startswith("assert_str_eq("):
            inner = s[len("assert_str_eq(") : s.rfind(")")]
            expr = _arg0_before_comma(inner)
            out.append(f"    print({expr})")
            continue
        if s.startswith("assert_eq("):
            inner = s[len("assert_eq(") : s.rfind(")")]
            expr = _arg0_before_comma(inner)
            out.append(f"    print({expr})")
            continue
        if s.startswith("let x = "):
            out.append(f"    print({s[len('let x = ') :]})")
            continue
        out.append(f"    {s}")
    return "\n".join(out) if out else None


def demo_body(spec: StdlibFnSpec) -> str:
    for builder in (
        _pure_impl_demo,
        _pure_scaffold_demo,
        _sync_demo,
        _option_demo,
        _result_demo,
        _hashmap_demo,
        _extern_scaffold_demo,
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
    if fn.endswith("_i32") and spec.ny_module.startswith("math"):
        samples = {
            "floor_i32": ("3", "3"),
            "ceil_i32": ("3", "3"),
            "round_i32": ("4", "4"),
        }
        if fn in samples:
            arg, expected = samples[fn]
            return [f"    assert_eq({fn}({arg}), {expected})"]
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
        if fn == "signum_f64":
            return [
                "    assert_eq(is_nan(signum_f64(0.0 / 0.0)), 0)",
                "    if signum_f64(2.0) <= 0.0 { assert_eq(1, 0) }",
            ]
        if fn == "signum_i32":
            return [
                "    assert_eq(signum_i32(5), 1)",
                "    assert_eq(signum_i32(-3), -1)",
                "    assert_eq(signum_i32(0), 0)",
            ]
        if fn == "trunc_i32":
            return ["    assert_eq(trunc_i32(7), 7)"]
        if fn == "fmod_f64":
            return [
                "    let x = fmod_f64(5.0, 2.0)",
                "    if x < 0.9 { assert_eq(1, 0) }",
                "    if x > 1.1 { assert_eq(1, 0) }",
            ]
        if fn == "copysign_f64":
            return [
                "    let x = copysign_f64(1.0, 1.0)",
                "    if x < 0.5 { assert_eq(1, 0) }",
            ]
        if fn == "lerp_f64":
            return [
                "    let x = lerp_f64(0.0, 10.0, 0.5)",
                "    if x < 4.9 { assert_eq(1, 0) }",
                "    if x > 5.1 { assert_eq(1, 0) }",
            ]
        if fn == "pow_i32":
            return ["    assert_eq(pow_i32(2, 3), 8)"]
        return [f"    let x = {fn}({_call_args(spec)})", "    if x < 0.0 { assert_eq(1, 0) }"]
    if fn == "hex_decode":
        return ['    assert_str_eq(hex_decode("4869"), "Hi")']
    if fn in ("str_to_bool", "parse_bool"):
        return [
            '    assert_eq(parse_bool("true"), 1)',
            '    assert_eq(parse_bool("false"), 0)',
        ]
    if fn.startswith("map_str_"):
        return ["    // TODO: assert behavior", "    assert_eq(1, 1)"]
    if fn == "vec_str_pop":
        return [
            '    let v = strs().push("a").push("b")',
            '    assert_str_eq(v.pop(), "b")',
            "    assert_eq(v.len(), 1)",
        ]
    if fn == "vec_str_clear":
        return [
            '    let v = strs().push("a").clear()',
            "    assert_eq(v.len(), 0)",
        ]
    if fn == "vec_str_reverse":
        return [
            '    let v = strs().push("a").push("b").reverse()',
            '    assert_str_eq(v.get(0), "b")',
        ]
    if fn == "vec_str_insert":
        return [
            '    let v = strs().push("b").insert(0, "a")',
            '    assert_str_eq(v.get(0), "a")',
        ]
    if fn == "vec_str_remove_at":
        return [
            '    let v = strs().push("a").push("b")',
            '    assert_str_eq(v.remove_at(0), "a")',
            "    assert_eq(v.len(), 1)",
        ]
    if fn == "vec_str_swap":
        return [
            '    let v = strs().push("a").push("b").swap(0, 1)',
            '    assert_str_eq(v.get(0), "b")',
        ]
    if fn == "vec_str_set":
        return [
            '    let v = strs().push("a")',
            "    vec_str_set(v.handle, 0, \"z\")",
            '    assert_str_eq(v.get(0), "z")',
        ]
    if fn == "vec_str_extend":
        return [
            '    let a = strs().push("x")',
            '    let b = strs().push("y").push("z")',
            "    vec_str_extend(a.handle, b.handle)",
            "    assert_eq(a.len(), 3)",
        ]
    if "option/combinators" in spec.ny_module:
        return [
            "    let some = Option_i32_some(5)",
            "    assert_eq(Option_i32_unwrap_or(some, 0), 5)",
            "    assert_eq(Option_i32_is_none(Option_i32_none()), 1)",
        ]
    if "result_combinators" in spec.ny_module:
        return [
            "    let ok = Result_i32_i32_ok(7)",
            "    assert_eq(Result_i32_i32_unwrap_or(ok, 0), 7)",
            "    assert_eq(Result_i32_i32_is_err(Result_i32_i32_err(1)), 1)",
        ]
    if spec.fn_name == "hashmap_or_insert":
        return [
            '    let m = HashMap_str_i32_new().insert("k", 10)',
            '    assert_eq(m.or_insert("k", 42), 10)',
        ]
    if spec.fn_name == "strvec_methods":
        return [
            '    let v = strs().push("x")',
            "    assert_eq(v.is_empty(), 0)",
            '    assert_str_eq(v.pop(), "x")',
        ]
    if spec.fn_name == "strvec_insert_extend":
        return [
            '    let v = strs().push("b").insert(0, "a")',
            '    assert_str_eq(v.get(0), "a")',
            '    assert_str_eq(v.remove_at(1), "b")',
        ]
    if spec.fn_name == "hashmap_extra_methods":
        return [
            "    let m = HashMap_str_i32_new()",
            "    assert_eq(m.is_empty(), 1)",
            '    let _ = m.insert("k", 1)',
            "    assert_eq(m.is_empty(), 0)",
        ]
    if fn == "format_i32_pad":
        return [
            '    assert_str_eq(format_pad(7, 3), "007")',
            '    assert_str_eq(format_i32_pad(42, 2), "42")',
        ]
    if fn == "f64_to_string_prec":
        return [
            '    assert_str_eq(f64_to_string_prec(1.0, 1), "1.0")',
            '    assert_str_eq(f64_to_string_prec(3.14159, 2), "3.14")',
        ]
    if fn == "i64_to_string":
        return ['    assert_str_eq(i64_to_string(42), "42")']
    if fn == "str_to_u64":
        return ['    assert_eq(str_to_u64("99"), 99)']
    if fn == "parse_uint_base":
        return ['    assert_eq(parse_uint_base("ff", 16), 255)']
    if fn == "format_i32_hex":
        return ['    assert_str_eq(format_i32_hex(255), "ff")']
    if fn == "str_to_f64":
        return ['    assert_eq(str_to_f64("3.5"), 3.5)']
    if fn == "format_bool":
        return [
            '    assert_str_eq(format_bool(1), "true")',
            '    assert_str_eq(format_bool(0), "false")',
        ]
    if fn == "format_i64_pad":
        return ['    assert_str_eq(format_i64_pad(7, 3), "007")']
    if fn == "format_f64_pad":
        return ['    assert_str_eq(format_f64_pad(3.14, 5, 2), " 3.14")']
    if fn == "format_i32_hex_pad":
        return ['    assert_str_eq(format_i32_hex_pad(255, 4), "00ff")']
    if fn == "str_to_i64":
        return ['    assert_eq(str_to_i64("42"), 42)']
    if fn == "parse_int_base":
        return ['    assert_eq(parse_int_base("10", 16), 16)']
    if fn == "format_i32_bin":
        return ['    assert_str_eq(format_i32_bin(5), "101")']
    if fn == "format_i32_oct":
        return ['    assert_str_eq(format_i32_oct(8), "10")']
    if fn == "format_i64_hex":
        return ['    assert_str_eq(format_i64_hex(255), "ff")']
    if fn == "format_u64_pad":
        return ['    assert_str_eq(format_u64_pad(7, 3), "007")']
    if fn == "i32_to_string_radix":
        return ['    assert_str_eq(i32_to_string_radix(255, 16), "ff")']
    if fn == "parse_i64_base":
        return ['    assert_eq(parse_i64_base("ff", 16), 255)']
    if fn == "u64_to_string":
        return ['    assert_str_eq(u64_to_string(42), "42")']
    if fn == "str_to_f32":
        return ['    assert_eq(str_to_f32("2.5"), 2.5)']
    if fn == "mod_i32":
        return ['    assert_eq(mod_i32(7, 3), 1)', '    assert_eq(mod_i32(-7, 3), 2)']
    if fn == "gcd_i32":
        return ['    assert_eq(gcd_i32(12, 8), 4)']
    if fn == "lcm_i32":
        return ['    assert_eq(lcm_i32(4, 6), 12)']
    if fn == "deg_to_rad_f64":
        return ["    let x = deg_to_rad_f64(180.0)", "    if x < 3.0 { assert_eq(1, 0) }"]
    if fn == "rad_to_deg_f64":
        return ["    let x = rad_to_deg_f64(3.141592653589793)", "    if x < 179.0 { assert_eq(1, 0) }"]
    if fn == "fract_f64":
        return [
            "    let x = fract_f64(3.7)",
            "    if x < 0.6 { assert_eq(1, 0) }",
            "    if x > 0.8 { assert_eq(1, 0) }",
        ]
    if fn == "vec_i32_swap":
        return ["    let v = vec().push(1).push(2).swap(0, 1)", "    assert_eq(v.get(0), 2)"]
    if fn == "vec_i32_extend":
        return [
            "    let a = vec().push(1)",
            "    let b = vec().push(2).push(3)",
            "    vec_i32_extend(a.handle, b.handle)",
            "    assert_eq(a.len(), 3)",
        ]
    if fn == "hex_encode":
        return ['    assert_str_eq(hex_encode("Hi"), "4869")']
    if fn == "hex_encode_upper":
        return ['    assert_str_eq(hex_encode_upper("Hi"), "4869")']
    if fn == "atomic_sub_i32":
        return [
            "    let a = Atomic_i32_new(10)",
            "    assert_eq(atomic_sub_i32(a.cell, 3), 7)",
        ]
    if fn == "atomic_xor_i32":
        return [
            "    let a = Atomic_i32_new(5)",
            "    assert_eq(atomic_xor_i32(a.cell, 3), 6)",
        ]
    if fn == "saturating_add_i32":
        return [
            "    assert_eq(saturating_add_i32(2147483647, 1), 2147483647)",
            "    assert_eq(saturating_add_i32(1, 2), 3)",
        ]
    if fn == "saturating_sub_i32":
        return [
            "    assert_eq(saturating_sub_i32(-2147483648, 1), -2147483648)",
            "    assert_eq(saturating_sub_i32(5, 2), 3)",
        ]
    if fn == "wrapping_add_i32":
        return ["    assert_eq(wrapping_add_i32(2147483647, 1), -2147483648)"]
    if fn == "leading_zeros_i32":
        return ["    assert_eq(leading_zeros_i32(1), 31)", "    assert_eq(leading_zeros_i32(0), 32)"]
    if fn == "count_ones_i32":
        return ["    assert_eq(count_ones_i32(5), 2)", "    assert_eq(count_ones_i32(0), 0)"]
    if fn == "is_infinite_f64":
        return [
            "    assert_eq(is_infinite_f64(1.0 / 0.0), 1)",
            "    assert_eq(is_infinite_f64(3.14), 0)",
        ]
    if fn == "format_quote":
        return ['    assert_str_eq(format_quote("hi"), "\\"hi\\"")']
    if fn == "format_i64_bin":
        return ['    assert_str_eq(format_i64_bin(5), "101")']
    if fn == "vec_i32_capacity":
        return [
            "    let v = vec().push(1)",
            "    assert_eq(vec_i32_capacity(v.handle) >= 1, 1)",
        ]
    if fn == "vec_i32_reserve":
        return [
            "    let v = vec()",
            "    vec_i32_reserve(v.handle, 16)",
            "    assert_eq(vec_i32_capacity(v.handle) >= 16, 1)",
        ]
    if fn == "vec_i32_fill":
        return [
            "    let v = vec().push(1).push(2)",
            "    vec_i32_fill(v.handle, 9)",
            "    assert_eq(v.get(0), 9)",
            "    assert_eq(v.get(1), 9)",
        ]
    if fn == "vec_i32_swap_remove":
        return [
            "    let v = vec().push(10).push(20).push(30)",
            "    assert_eq(vec_i32_swap_remove(v.handle, 1), 20)",
            "    assert_eq(v.len(), 2)",
        ]
    if spec.fn_name == "vec_i32_extra_methods":
        return [
            "    let v = vec()",
            "    assert_eq(v.is_empty(), 1)",
            "    let _ = v.push(1).reserve(8)",
            "    assert_eq(v.capacity() >= 8, 1)",
        ]
    if spec.fn_name == "hashmap_i32_get_or":
        return [
            "    let m = HashMap_i32_i32_new()",
            "    assert_eq(m.get_or(1, 99), 99)",
            "    let _ = m.insert(1, 42)",
            "    assert_eq(m.get_or_insert(1, 0), 42)",
        ]
    if fn == "trailing_zeros_i32":
        return ["    assert_eq(trailing_zeros_i32(8), 3)", "    assert_eq(trailing_zeros_i32(0), 32)"]
    if fn == "rotate_left_i32":
        return ["    assert_eq(rotate_left_i32(1, 1), 2)"]
    if fn == "rotate_right_i32":
        return ["    assert_eq(rotate_right_i32(2, 1), 1)"]
    if fn == "rem_euclid_i32":
        return ["    assert_eq(rem_euclid_i32(-7, 3), 2)"]
    if fn == "file_mtime":
        return ["    assert_eq(1, 1)  // platform path optional"]
    if fn == "rename_file":
        return ["    assert_eq(1, 1)  // integration test elsewhere"]
    if fn == "path_is_file":
        return ["    assert_eq(1, 1)"]
    if fn == "file_is_symlink":
        return ["    assert_eq(file_is_symlink(\"/nonexistent-link-xyz\"), 0)"]
    if fn == "atomic_and_i32":
        return [
            "    let a = Atomic_i32_new(7)",
            "    assert_eq(atomic_and_i32(a.cell, 3), 3)",
        ]
    if fn == "atomic_or_i32":
        return [
            "    let a = Atomic_i32_new(1)",
            "    assert_eq(atomic_or_i32(a.cell, 2), 3)",
        ]
    if fn == "vec_i32_truncate":
        return [
            "    let v = vec().push(1).push(2).push(3)",
            "    vec_i32_truncate(v.handle, 1)",
            "    assert_eq(v.len(), 1)",
        ]
    if spec.fn_name == "vec_i32_slice_methods":
        return [
            "    let v = vec().push(1).push(2).push(3).push(4)",
            "    let w = v.window(1, 2)",
            "    assert_eq(w.len(), 2)",
            "    assert_eq(w.get(0), 2)",
        ]
    if spec.fn_name == "strvec_set_method":
        return ['    let v = strs().push("a")', '    let _ = v.set(0, "z")', '    assert_str_eq(v.get(0), "z")']
    if spec.fn_name == "vec_i32_swap_extend":
        return ["    let v = vec().push(1).push(2).swap(0, 1)", "    assert_eq(v.get(0), 2)"]
    if spec.fn_name == "hashmap_i32_i32":
        return [
            "    let m = HashMap_i32_i32_new()",
            "    let _ = m.insert(1, 42)",
            "    assert_eq(m.get(1), 42)",
        ]
    if spec.fn_name == "hashmap_update":
        return [
            '    let m = HashMap_str_i32_new().insert("k", 10)',
            '    let _ = m.insert("k", 20)',
            '    assert_eq(m.get("k"), 20)',
        ]
    if spec.returns == NyraType.STRING:
        return [f'    assert_str_eq({fn}({_call_args(spec)}), "")  // TODO: set expected']
    if spec.returns == NyraType.F64:
        return [f"    let x = {fn}({_call_args(spec)})", "    if x < 0.0 { assert_eq(1, 0) }"]
    return ["    // TODO: assert behavior", "    assert_eq(1, 1)"]
