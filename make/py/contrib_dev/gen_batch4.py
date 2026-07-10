#!/usr/bin/env python3
"""Generate batch4 JSON configs from batch4_catalog.py.

Usage:
  python3 make/py/contrib_dev/gen_batch4.py
  make gen-batch4
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MAKE_PY = ROOT / "make" / "py"
if str(MAKE_PY) not in sys.path:
    sys.path.insert(0, str(MAKE_PY))

BUILTIN_BATCH4 = MAKE_PY / "builtin_dev" / "examples" / "batch4"
CONTRIB_BATCH4 = MAKE_PY / "contrib_dev" / "examples" / "batch4"

from contrib_dev.batch4_catalog import (  # noqa: E402
    ENCODING_EXTERN,
    FORMAT_EXTERN,
    MAP_EXTERN,
    MATH_EXTERN,
    PURE_MODULES,
    STRING_BUILTINS,
    STRING_TEST_CASES,
    STRCONV_EXTERN,
    SYNC_EXTERN,
    VEC_EXTERN,
)
from contrib_dev.gen_batch3 import (  # noqa: E402
    STRVEC_EXTRA_METHODS,
    _consolidate_strvec_impl,
    _consolidate_strvec_insert,
    _fix_builtin_collisions,
    _fix_map_or_insert_block,
    _restore_vec_str_if_corrupted,
    _strip_recursive_alias_blocks,
    _write_json,
)

STRVEC_SET_METHOD = """
    fn set(self, index: i32, value: string) -> StrVec {
        vec_str_set(self.handle, index, value)
        return self
    }
"""


def _consolidate_strvec_set(vec_str_path: Path) -> None:
    if not vec_str_path.exists():
        return
    text = vec_str_path.read_text(encoding="utf-8")
    original = text
    text = re.sub(
        r"// \[contrib-dev:strvec_set_method:vec_str\].*?// \[/contrib-dev:strvec_set_method:vec_str\]\n*",
        "",
        text,
        flags=re.DOTALL,
    )
    if "fn set(self, index: i32" not in text and "impl StrVec {" in text:
        text = text.replace(
            "    fn index_of(self, needle: string) -> i32 {",
            STRVEC_SET_METHOD + "\n    fn index_of(self, needle: string) -> i32 {",
            1,
        )
    if text != original:
        vec_str_path.write_text(text, encoding="utf-8")
        print(f"  consolidated StrVec.set in {vec_str_path.relative_to(ROOT)}")


def _consolidate_vec_i32_swap_extend(vec_path: Path) -> None:
    if not vec_path.exists():
        return
    text = vec_path.read_text(encoding="utf-8")
    original = text
    text = re.sub(
        r"// \[contrib-dev:vec_i32_swap_extend:vec\].*?// \[/contrib-dev:vec_i32_swap_extend:vec\]\n*",
        "",
        text,
        flags=re.DOTALL,
    )
    orphan = re.compile(
        r"\n\}\n\n+    fn swap\(self, i: i32, j: i32\) -> VecI32 \{.*?"
        r"    fn append\(self, x: i32\) -> VecI32 \{[^}]+\}\n+",
        re.DOTALL,
    )
    if text.count("fn swap(self, i: i32, j: i32)") > 1:
        text = orphan.sub("\n}\n\n", text, count=1)
    block = """
    fn swap(self, i: i32, j: i32) -> VecI32 {
        vec_i32_swap(self.handle, i, j)
        return self
    }

    fn extend(self, other: VecI32) -> VecI32 {
        vec_i32_extend(self.handle, other.handle)
        return self
    }

    fn append(self, x: i32) -> VecI32 {
        return self.push(x)
    }
"""
    if "fn swap(self, i: i32, j: i32)" not in text and "impl VecI32 {" in text:
        text = text.replace(
            "    fn binary_search(self, x: i32) -> i32 {",
            block + "\n    fn binary_search(self, x: i32) -> i32 {",
            1,
        )
    if text != original:
        vec_path.write_text(text, encoding="utf-8")
        print(f"  consolidated VecI32 swap/extend in {vec_path.relative_to(ROOT)}")



def _refresh_batch4_tests() -> None:
    from contrib_dev.example_codegen import extern_test_body
    from contrib_dev.templates import test_ny_from_stdlib, test_typed_ny
    from contrib_dev.wizard import spec_from_config

    tests_dir = ROOT / "tests" / "nyra"
    for cfg_path in sorted(CONTRIB_BATCH4.glob("*.json")):
        if cfg_path.name == "manifest.json":
            continue
        data = json.loads(cfg_path.read_text(encoding="utf-8"))
        recipe = data.get("recipe", "stdlib-extern")
        if recipe not in ("stdlib-pure", "stdlib-extern"):
            recipe = "stdlib-extern" if data.get("rt_module") else "stdlib-pure"
        spec = spec_from_config(recipe, data)
        marker = spec.marker
        test_base = f"{spec.fn_name}_test"
        plain = tests_dir / f"{test_base}.ny"
        if not plain.exists():
            continue
        if recipe == "stdlib-extern" or spec.fn_name in (
            "strvec_set_method",
            "vec_i32_swap_extend",
            "hashmap_i32_i32",
            "hashmap_update",
        ):
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
        typed = tests_dir / f"{test_base}.typed.ny"
        if typed.exists() and recipe == "stdlib-pure":
            from contrib_dev.spec import TestExampleSpec

            tspec = TestExampleSpec(name=test_base.replace("_test", ""), import_path=spec.stdlib_path)
            typed.write_text(test_typed_ny(tspec, marker), encoding="utf-8")

    for cfg_path in sorted(BUILTIN_BATCH4.glob("*.json")):
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
    print("==> gen-batch4: emitting JSON configs")
    for entry in STRING_BUILTINS:
        _write_json(BUILTIN_BATCH4 / f"{entry['method']}.json", entry)

    for entry in MATH_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"math_{entry['fn_name']}.json", entry)
    for entry in STRCONV_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"strconv_{entry['fn_name']}.json", entry)
    for entry in FORMAT_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"format_{entry['fn_name']}.json", entry)
    for entry in VEC_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"vec_{entry['fn_name']}.json", entry)
    for entry in MAP_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"map_{entry['fn_name']}.json", entry)
    for entry in ENCODING_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"encoding_{entry['fn_name']}.json", entry)
    for entry in SYNC_EXTERN:
        _write_json(CONTRIB_BATCH4 / f"sync_{entry['fn_name']}.json", entry)
    for entry in PURE_MODULES:
        _write_json(CONTRIB_BATCH4 / f"pure_{entry['fn_name']}.json", entry)

    expected_pure = {f"pure_{e['fn_name']}.json" for e in PURE_MODULES}
    for stale in CONTRIB_BATCH4.glob("pure_*.json"):
        if stale.name not in expected_pure:
            stale.unlink()
            print(f"  removed stale {stale.relative_to(ROOT)}")

    manifest = {
        "string_count": len(STRING_BUILTINS),
        "extern_count": len(MATH_EXTERN)
        + len(STRCONV_EXTERN)
        + len(FORMAT_EXTERN)
        + len(VEC_EXTERN)
        + len(MAP_EXTERN)
        + len(ENCODING_EXTERN)
        + len(SYNC_EXTERN),
        "pure_count": len(PURE_MODULES),
    }
    _write_json(CONTRIB_BATCH4 / "manifest.json", manifest)

    _strip_recursive_alias_blocks(ROOT / "stdlib" / "math.ny")
    _fix_builtin_collisions()
    _restore_vec_str_if_corrupted(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_impl(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_insert(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_set(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_vec_i32_swap_extend(ROOT / "stdlib" / "vec.ny")
    _fix_map_or_insert_block(ROOT / "stdlib" / "map.ny")
    _refresh_batch4_tests()

    print(
        f"==> done: {manifest['string_count']} builtins, "
        f"{manifest['extern_count']} extern, {manifest['pure_count']} pure"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
