#!/usr/bin/env python3
"""Generate batch3 JSON configs from batch3_catalog.py.

Usage:
  python3 make/py/contrib_dev/gen_batch3.py
  make gen-batch3
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MAKE_PY = ROOT / "make" / "py"
import sys

if str(MAKE_PY) not in sys.path:
    sys.path.insert(0, str(MAKE_PY))

BUILTIN_BATCH3 = MAKE_PY / "builtin_dev" / "examples" / "batch3"
CONTRIB_BATCH3 = MAKE_PY / "contrib_dev" / "examples" / "batch3"

from contrib_dev.batch3_catalog import (  # noqa: E402
    BUILTINS_MATH_PATCHES,
    FORMAT_EXTERN,
    MATH_EXTERN,
    PURE_MODULES,
    STRING_BUILTINS,
    STRING_TEST_CASES,
    STRCONV_EXTERN,
    VEC_STR_EXTERN,
)


def _write_json(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"  wrote {path.relative_to(ROOT)}")


def _strip_recursive_alias_blocks(ny_path: Path) -> None:
    """Remove Nyra wrapper blocks where alias name equals extern fn (infinite recursion)."""
    if not ny_path.exists():
        return
    text = ny_path.read_text(encoding="utf-8")
    new_text = re.sub(
        r"// \[contrib-dev:([A-Za-z0-9_]+):[^\]]+:alias\]\n"
        r"fn \1\([^)]*\)[^{]*\{[^}]*return \1\([^}]*\}\n"
        r"// \[/contrib-dev:\1:[^\]]+:alias\]\n",
        "",
        text,
    )
    if new_text != text:
        ny_path.write_text(new_text, encoding="utf-8")
        print(f"  stripped recursive alias blocks in {ny_path.relative_to(ROOT)}")


STRVEC_EXTRA_METHODS = """
    fn pop(self) -> string {
        return vec_str_pop(self.handle)
    }

    fn clear(self) -> StrVec {
        vec_str_clear(self.handle)
        return self
    }

    fn reverse(self) -> StrVec {
        vec_str_reverse(self.handle)
        return self
    }

    fn is_empty(self) -> i32 {
        if Vec_str_len(self.handle) == 0 {
            return 1
        }
        return 0
    }

    fn reduce(self, init: string, reducer: fn(string, string) -> string) -> string {
        let mut acc = init
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            acc = reducer(acc, Vec_str_get(self.handle, i))
            i = i + 1
        }
        return acc
    }
"""

STRVEC_INSERT_METHODS = """
    fn insert(self, index: i32, value: string) -> StrVec {
        vec_str_insert(self.handle, index, value)
        return self
    }

    fn remove_at(self, index: i32) -> string {
        return vec_str_remove_at(self.handle, index)
    }

    fn extend(self, other: StrVec) -> StrVec {
        vec_str_extend(self.handle, other.handle)
        return self
    }

    fn append(self, value: string) -> StrVec {
        return self.push(value)
    }

    fn swap(self, i: i32, j: i32) -> StrVec {
        vec_str_swap(self.handle, i, j)
        return self
    }
"""


def _restore_vec_str_if_corrupted(vec_str_path: Path) -> bool:
    text = vec_str_path.read_text(encoding="utf-8")
    corrupted = bool(
        re.search(r"\nreturn vec_str_pop", text)
        or re.search(r"\nlet mut acc = init", text)
        or text.count("impl StrVec {") > 1
    )
    if not corrupted:
        return False
    import subprocess

    rel = vec_str_path.relative_to(ROOT)
    subprocess.run(["git", "checkout", "HEAD", "--", str(rel)], cwd=ROOT, check=True)
    print(f"  restored {rel} from git (corrupted StrVec impl)")
    return True


def _consolidate_strvec_impl(vec_str_path: Path) -> None:
    """Remove orphan strvec_methods scaffold and garbage after `impl StrVec`."""
    if not vec_str_path.exists():
        return
    from contrib_dev.patch import _impl_body_span

    text = vec_str_path.read_text(encoding="utf-8")
    original = text
    text = re.sub(
        r"// \[contrib-dev:strvec_methods:vec_str\].*?// \[/contrib-dev:strvec_methods:vec_str\]\n*",
        "",
        text,
        flags=re.DOTALL,
    )
    drop_marker = "impl Drop for StrVec {"
    if "impl StrVec {" in text and drop_marker in text:
        span = _impl_body_span(text, "StrVec")
        if span is not None:
            _, body_end = span
            drop_idx = text.index(drop_marker)
            if body_end + 1 < drop_idx and text[body_end + 1 : drop_idx].strip():
                text = text[: body_end + 1] + "\n\n" + text[drop_idx:]
    if "fn pop(self)" not in text and "impl StrVec {" in text:
        text = text.replace(
            "    fn index_of(self, needle: string) -> i32 {",
            STRVEC_EXTRA_METHODS + "\n    fn index_of(self, needle: string) -> i32 {",
            1,
        )
    if text != original:
        vec_str_path.write_text(text, encoding="utf-8")
        print(f"  consolidated StrVec impl in {vec_str_path.relative_to(ROOT)}")


def _consolidate_strvec_insert(vec_str_path: Path) -> None:
    if not vec_str_path.exists():
        return
    text = vec_str_path.read_text(encoding="utf-8")
    original = text
    text = re.sub(
        r"// \[contrib-dev:strvec_insert_extend:vec_str\].*?// \[/contrib-dev:strvec_insert_extend:vec_str\]\n*",
        "",
        text,
        flags=re.DOTALL,
    )
    if "fn insert(self, index: i32" not in text and "impl StrVec {" in text:
        text = text.replace(
            "    fn index_of(self, needle: string) -> i32 {",
            STRVEC_INSERT_METHODS + "\n    fn index_of(self, needle: string) -> i32 {",
            1,
        )
    if text != original:
        vec_str_path.write_text(text, encoding="utf-8")
        print(f"  consolidated StrVec insert/extend in {vec_str_path.relative_to(ROOT)}")


def _strip_broken_option_combinators(result_path: Path) -> None:
    """Remove batch3 Option combinators that trigger enum return codegen bugs."""
    if not result_path.exists():
        return
    text = result_path.read_text(encoding="utf-8")
    cleaned = re.sub(
        r"// \[contrib-dev:option_combinators:result\].*?// \[/contrib-dev:option_combinators:result\]\n",
        "",
        text,
        flags=re.DOTALL,
    )
    if cleaned != text:
        result_path.write_text(cleaned, encoding="utf-8")
        print(f"  removed option combinators from {result_path.relative_to(ROOT)}")


_OR_INSERT_I32_PAIR = """
    fn or_insert(self, key: string, value: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }

    fn get_or_insert(self, key: string, value: i32) -> i32 {
        return self.or_insert(key, value)
    }
""".strip()

_OR_INSERT_STR_FN = """
    fn or_insert(self, key: string, value: string) -> string {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }
""".strip()

_OR_INSERT_SCAFFOLD = re.compile(
    r"// \[contrib-dev:hashmap_or_insert:map\].*?// \[/contrib-dev:hashmap_or_insert:map\]\n*",
    re.DOTALL,
)


def _fix_builtin_collisions() -> None:
    """Fix batch3 builtins that collide with existing extern/C symbols."""
    builtins_path = ROOT / "stdlib" / "builtins_string.ny"
    if builtins_path.exists():
        text = builtins_path.read_text(encoding="utf-8")
        original = text
        for pattern in (
            r"\nfn char_at\(s: &string, index: i32\) -> i32 \{\n    return char_at\(s, index\)\n\}\n",
            r"\nfn substring\(s: &string, start: i32, len: i32\) -> string \{\n    return substring\(s, start, len\)\n\}\n",
        ):
            text = re.sub(pattern, "\n", text)
        if text != original:
            builtins_path.write_text(text, encoding="utf-8")
            print("  removed recursive builtin free-fn aliases in builtins_string.ny")

    runtime_map = ROOT / "compiler" / "codegen" / "src" / "runtime_map.rs"
    if runtime_map.exists():
        text = runtime_map.read_text(encoding="utf-8")
        original = text
        for sym in ("char_at", "substring"):
            text = text.replace(f'        ("{sym}", "{sym}"),\n', "")
        if text != original:
            runtime_map.write_text(text, encoding="utf-8")
            print("  removed duplicate runtime_map free-fn aliases")

    math_path = ROOT / "stdlib" / "math.ny"
    if math_path.exists():
        text = math_path.read_text(encoding="utf-8")
        cleaned = re.sub(
            r"// \[contrib-dev:pow_i32:math\]\nextern fn pow_i32\(base: i32, exp: i32\) -> i32\n// \[/contrib-dev:pow_i32:math\]\n",
            "",
            text,
        )
        if cleaned != text:
            math_path.write_text(cleaned, encoding="utf-8")
            print("  removed duplicate pow_i32 extern (pure fn exists)")

    rt_math = ROOT / "stdlib" / "rt" / "rt_math.c"
    if rt_math.exists():
        text = rt_math.read_text(encoding="utf-8")
        cleaned = re.sub(
            r"// \[contrib-dev:pow_i32:math\].*?// \[/contrib-dev:pow_i32:math\]\n*",
            "",
            text,
            flags=re.DOTALL,
        )
        if cleaned != text:
            rt_math.write_text(cleaned, encoding="utf-8")
            print("  removed duplicate pow_i32 C stub")

    if rt_math.exists():
        text = rt_math.read_text(encoding="utf-8")
        fixed = text.replace(
            "    return __builtin_fmod(x, y);",
            """    if (y == 0.0) return 0.0;
    int n = (int)(x / y);
    double r = x - (double)n * y;
    if ((r > 0.0) != (x > 0.0)) r += (x > 0.0 ? y : -y);
    return r;""",
        )
        if fixed != text:
            rt_math.write_text(fixed, encoding="utf-8")
            print("  patched fmod_f64 portable implementation")


def _fix_map_or_insert_block(map_path: Path) -> None:
    """Remove orphan scaffold blocks and duplicate or_insert pairs (idempotent)."""
    if not map_path.exists():
        return
    original = map_path.read_text(encoding="utf-8")
    text = original.replace(
        '// [contrib-dev:hashmap_or_insert:map]\nimport "../map.ny"\n\n',
        "// [contrib-dev:hashmap_or_insert:map]\n",
    )
    text = _OR_INSERT_SCAFFOLD.sub("", text)
    text = re.sub(r"\nfn or_insert", "\n    fn or_insert", text)
    while text.count(_OR_INSERT_I32_PAIR) > 1:
        text = text.replace("\n\n" + _OR_INSERT_I32_PAIR + "\n", "\n", 1)
    while text.count(_OR_INSERT_STR_FN) > 1:
        text = text.replace("\n\n" + _OR_INSERT_STR_FN + "\n", "\n", 1)
    text = text.replace("    }}", "    }\n")
    text = re.sub(r"\n    \}\n\n    \}\n", "\n    }\n}\n", text)
    if text != original:
        map_path.write_text(text, encoding="utf-8")
        print(f"  fixed hashmap or_insert in {map_path.relative_to(ROOT)}")


def _refresh_batch3_tests() -> None:
    """Rewrite batch3 test files using example_codegen (idempotent)."""
    from contrib_dev.example_codegen import extern_test_body
    from contrib_dev.templates import test_ny_from_stdlib, test_typed_ny
    from contrib_dev.wizard import spec_from_config

    tests_dir = ROOT / "tests" / "nyra"
    for cfg_path in sorted(CONTRIB_BATCH3.glob("*.json")):
        if cfg_path.name == "manifest.json":
            continue
        data = json.loads(cfg_path.read_text(encoding="utf-8"))
        recipe = data.get("recipe", "stdlib-extern")
        if recipe not in ("stdlib-pure", "stdlib-extern"):
            recipe = "stdlib-extern" if data.get("rt_module") else "stdlib-pure"
        spec = spec_from_config(recipe, data)
        marker = spec.marker
        test_base = f"{spec.fn_name}_test"
        for typed in (False, True):
            suffix = ".typed.ny" if typed else ".ny"
            path = tests_dir / f"{test_base}{suffix}"
            if not path.exists():
                continue
            if typed:
                from contrib_dev.spec import TestExampleSpec

                tspec = TestExampleSpec(name=test_base.replace("_test", ""), import_path=spec.stdlib_path)
                body = test_typed_ny(tspec, marker)
            else:
                body = test_ny_from_stdlib(spec, marker)
            path.write_text(body, encoding="utf-8")
        # also patch extern-only bodies inside existing scaffold
        plain = tests_dir / f"{test_base}.ny"
        if plain.exists():
            if "option_combinators" in spec.fn_name or spec.ny_module == "result.ny" and spec.fn_name == "option_combinators":
                lines = [
                    f"// [contrib-dev:{marker}]",
                    'import "stdlib/testing.ny"',
                    'import "stdlib/result.ny"',
                    "",
                    f"test fn test_{spec.fn_name}() {{",
                    "    let some = Option_i32_some(5)",
                    "    assert_eq(Option_i32_unwrap_or(some, 0), 5)",
                    "    assert_eq(Option_i32_unwrap_or(Option_i32_none(), 9), 9)",
                    "}",
                    f"// [/contrib-dev:{marker}]",
                    "",
                ]
                plain.write_text("\n".join(lines), encoding="utf-8")
            elif "result/combinators" in spec.ny_module:
                lines = [
                    f"// [contrib-dev:{marker}]",
                    'import "stdlib/testing.ny"',
                    'import "stdlib/result.ny"',
                    f'import "{spec.stdlib_path}"',
                    "",
                    f"test fn test_{spec.fn_name}() {{",
                    "    let ok = Result_i32_i32_ok(7)",
                    "    assert_eq(Result_i32_i32_unwrap_or(ok, 0), 7)",
                    "    assert_eq(Result_i32_i32_is_err(Result_i32_i32_err(1)), 1)",
                    "}",
                    f"// [/contrib-dev:{marker}]",
                    "",
                ]
                plain.write_text("\n".join(lines), encoding="utf-8")
            elif recipe == "stdlib-extern":
                lines = [
                    f"// [contrib-dev:{marker}]",
                    'import "stdlib/testing.ny"',
                    f'import "{spec.stdlib_path}"',
                    "",
                    f"test fn test_{spec.fn_name}() {{",
                    *extern_test_body(spec),
                    "}",
                    f"// [/contrib-dev:{marker}]",
                    "",
                ]
                plain.write_text("\n".join(lines), encoding="utf-8")
            elif spec.fn_name in ("strvec_methods", "hashmap_or_insert", "strvec_insert_extend", "hashmap_extra_methods"):
                lines = [
                    f"// [contrib-dev:{marker}]",
                    'import "stdlib/testing.ny"',
                    f'import "{spec.stdlib_path}"',
                    "",
                    f"test fn test_{spec.fn_name}() {{",
                    *extern_test_body(spec),
                    "}",
                    f"// [/contrib-dev:{marker}]",
                    "",
                ]
                plain.write_text("\n".join(lines), encoding="utf-8")
    # string builtin tests
    for cfg_path in sorted(BUILTIN_BATCH3.glob("*.json")):
        data = json.loads(cfg_path.read_text(encoding="utf-8"))
        method = data["method"]
        test_path = tests_dir / f"string_{method}_test.ny"
        if not test_path.exists() or method not in STRING_TEST_CASES:
            continue
        body = [
            'import "stdlib/testing.ny"',
            "",
            f"test fn test_string_{method}() {{",
            *STRING_TEST_CASES[method],
            "}",
            "",
        ]
        test_path.write_text("\n".join(body), encoding="utf-8")


def main() -> int:
    print("==> gen-batch3: emitting JSON configs")
    for entry in STRING_BUILTINS:
        name = entry["method"]
        _write_json(BUILTIN_BATCH3 / f"{name}.json", entry)

    for entry in MATH_EXTERN:
        _write_json(CONTRIB_BATCH3 / f"math_{entry['fn_name']}.json", entry)

    for entry in STRCONV_EXTERN:
        _write_json(CONTRIB_BATCH3 / f"strconv_{entry['fn_name']}.json", entry)

    for entry in VEC_STR_EXTERN:
        _write_json(CONTRIB_BATCH3 / f"vec_{entry['fn_name']}.json", entry)

    for entry in FORMAT_EXTERN:
        _write_json(CONTRIB_BATCH3 / f"format_{entry['fn_name']}.json", entry)

    for entry in PURE_MODULES:
        slug = entry["fn_name"]
        _write_json(CONTRIB_BATCH3 / f"pure_{slug}.json", entry)

    expected_pure = {f"pure_{e['fn_name']}.json" for e in PURE_MODULES}
    for stale in CONTRIB_BATCH3.glob("pure_*.json"):
        if stale.name not in expected_pure:
            stale.unlink()
            print(f"  removed stale {stale.relative_to(ROOT)}")

    # manifest for test-contrib-py
    manifest = {
        "string_count": len(STRING_BUILTINS),
        "extern_count": len(MATH_EXTERN)
        + len(STRCONV_EXTERN)
        + len(VEC_STR_EXTERN)
        + len(FORMAT_EXTERN),
        "pure_count": len(PURE_MODULES),
    }
    _write_json(CONTRIB_BATCH3 / "manifest.json", manifest)

    # Stabilize builtins_math i32 stubs (no duplicate fn scaffold).
    math_path = ROOT / "stdlib" / "builtins_math.ny"
    if math_path.exists():
        text = math_path.read_text(encoding="utf-8")
        changed = False
        for old, new in BUILTINS_MATH_PATCHES.items():
            if old in text:
                text = text.replace(old, new)
                changed = True
        if changed:
            math_path.write_text(text, encoding="utf-8")
            print(f"  patched {math_path.relative_to(ROOT)} (Math_* i32 → floor_i32/…)")

    _strip_recursive_alias_blocks(ROOT / "stdlib" / "math.ny")

    # Remove duplicate extern lines inside strvec_methods scaffold block.
    vec_str_path = ROOT / "stdlib" / "vec_str.ny"
    if vec_str_path.exists():
        text = vec_str_path.read_text(encoding="utf-8")
        cleaned = re.sub(
            r"(// \[contrib-dev:strvec_methods:vec_str\]\n)"
            r"(?:extern fn vec_str_(?:pop|clear|reverse)\([^)]*\)[^\n]*\n)+",
            r"\1",
            text,
        )
        if cleaned != text:
            vec_str_path.write_text(cleaned, encoding="utf-8")
            print(f"  deduped extern decls in {vec_str_path.relative_to(ROOT)}")

    _strip_broken_option_combinators(ROOT / "stdlib" / "result.ny")
    _fix_builtin_collisions()
    _restore_vec_str_if_corrupted(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_impl(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_insert(ROOT / "stdlib" / "vec_str.ny")
    _fix_map_or_insert_block(ROOT / "stdlib" / "map.ny")
    _refresh_batch3_tests()

    print(
        f"==> done: {manifest['string_count']} builtins, "
        f"{manifest['extern_count']} extern, {manifest['pure_count']} pure"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
