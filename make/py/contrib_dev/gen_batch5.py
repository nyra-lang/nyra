#!/usr/bin/env python3
"""Generate batch5 JSON configs from batch5_catalog.py.

Usage:
  python3 make/py/contrib_dev/gen_batch5.py
  make gen-batch5
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

BUILTIN_BATCH5 = MAKE_PY / "builtin_dev" / "examples" / "batch5"
CONTRIB_BATCH5 = MAKE_PY / "contrib_dev" / "examples" / "batch5"

from contrib_dev.batch5_catalog import (  # noqa: E402
    FORMAT_EXTERN,
    MATH_EXTERN,
    PURE_MODULES,
    STRING_BUILTINS,
    STRING_TEST_CASES,
    SYNC_EXTERN,
    VEC_EXTERN,
)
from contrib_dev.gen_batch3 import (  # noqa: E402
    _consolidate_strvec_impl,
    _consolidate_strvec_insert,
    _fix_builtin_collisions,
    _fix_map_or_insert_block,
    _restore_vec_str_if_corrupted,
    _strip_recursive_alias_blocks,
    _write_json,
)
from contrib_dev.gen_batch4 import (  # noqa: E402
    _consolidate_strvec_set,
    _consolidate_vec_i32_swap_extend,
)

VECI32_EXTRA_BLOCK = """
    fn capacity(self) -> i32 {
        return vec_i32_capacity(self.handle)
    }

    fn reserve(self, min_cap: i32) -> VecI32 {
        vec_i32_reserve(self.handle, min_cap)
        return self
    }

    fn fill(self, value: i32) -> VecI32 {
        vec_i32_fill(self.handle, value)
        return self
    }

    fn swap_remove(self, index: i32) -> i32 {
        return vec_i32_swap_remove(self.handle, index)
    }

    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }
"""


def _consolidate_vec_i32_extra(vec_path: Path) -> None:
    if not vec_path.exists():
        return
    text = vec_path.read_text(encoding="utf-8")
    original = text
    text = re.sub(
        r"// \[contrib-dev:vec_i32_extra_methods:vec\].*?"
        r"// \[/contrib-dev:vec_i32_extra_methods:vec\]\n*",
        "",
        text,
        flags=re.DOTALL,
    )
    if "fn capacity(self)" not in text and "impl VecI32 {" in text:
        text = text.replace(
            "    fn append(self, x: i32) -> VecI32 {",
            VECI32_EXTRA_BLOCK + "\n    fn append(self, x: i32) -> VecI32 {",
            1,
        )
    if text != original:
        vec_path.write_text(text, encoding="utf-8")
        print(f"  consolidated VecI32 extra methods in {vec_path.relative_to(ROOT)}")


def _consolidate_hashmap_i32_get_or(map_path: Path) -> None:
    if not map_path.exists():
        return
    text = map_path.read_text(encoding="utf-8")
    original = text
    text = re.sub(
        r"// \[contrib-dev:hashmap_i32_get_or:map\].*?"
        r"// \[/contrib-dev:hashmap_i32_get_or:map\]\n*",
        "",
        text,
        flags=re.DOTALL,
    )
    # Remove mistaken insertion into HashMap_str_i32 (i32-key methods in string-key impl).
    text = re.sub(
        r"\n    fn get_or\(self, key: i32, default: i32\) -> i32 \{.*?"
        r"    fn get_or_insert\(self, key: i32, value: i32\) -> i32 \{.*?"
        r"        return value\n    \}\n(?=    fn is_empty\(self\) -> i32 \{)",
        "\n",
        text,
        count=1,
        flags=re.DOTALL,
    )
    get_or_block = """
    fn get_or(self, key: i32, default: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        return default
    }

    fn get_or_insert(self, key: i32, value: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }
"""
    marker = "impl HashMap_i32_i32 {"
    if marker in text and "impl HashMap_i32_i32 {" in text:
        start = text.find(marker)
        end = text.find("impl Drop for HashMap_i32_i32", start)
        block = text[start:end] if end > start else ""
        if block and "fn get_or(self, key: i32" not in block:
            text = text.replace(
                "impl HashMap_i32_i32 {\n    fn insert(self, key: i32, value: i32)",
                "impl HashMap_i32_i32 {\n    fn insert(self, key: i32, value: i32)",
                1,
            )
            text = text.replace(
                "    fn is_empty(self) -> i32 {\n        if self.len() == 0 {\n            return 1\n        }\n        return 0\n    }\n}\n\nimpl Drop for HashMap_i32_i32",
                get_or_block
                + "\n    fn is_empty(self) -> i32 {\n        if self.len() == 0 {\n            return 1\n        }\n        return 0\n    }\n}\n\nimpl Drop for HashMap_i32_i32",
                1,
            )
    if text != original:
        map_path.write_text(text, encoding="utf-8")
        print(f"  consolidated HashMap_i32_i32 get_or in {map_path.relative_to(ROOT)}")


def _refresh_batch5_tests() -> None:
    from contrib_dev.example_codegen import extern_test_body
    from contrib_dev.templates import test_typed_ny
    from contrib_dev.wizard import spec_from_config

    tests_dir = ROOT / "tests" / "nyra"
    for cfg_path in sorted(CONTRIB_BATCH5.glob("*.json")):
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
            "vec_i32_extra_methods",
            "hashmap_i32_get_or",
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

    for cfg_path in sorted(BUILTIN_BATCH5.glob("*.json")):
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
    print("==> gen-batch5: emitting JSON configs")
    for entry in STRING_BUILTINS:
        _write_json(BUILTIN_BATCH5 / f"{entry['method']}.json", entry)

    for entry in MATH_EXTERN:
        _write_json(CONTRIB_BATCH5 / f"math_{entry['fn_name']}.json", entry)
    for entry in FORMAT_EXTERN:
        _write_json(CONTRIB_BATCH5 / f"format_{entry['fn_name']}.json", entry)
    for entry in VEC_EXTERN:
        _write_json(CONTRIB_BATCH5 / f"vec_{entry['fn_name']}.json", entry)
    for entry in SYNC_EXTERN:
        _write_json(CONTRIB_BATCH5 / f"sync_{entry['fn_name']}.json", entry)
    for entry in PURE_MODULES:
        _write_json(CONTRIB_BATCH5 / f"pure_{entry['fn_name']}.json", entry)

    expected_pure = {f"pure_{e['fn_name']}.json" for e in PURE_MODULES}
    for stale in CONTRIB_BATCH5.glob("pure_*.json"):
        if stale.name not in expected_pure:
            stale.unlink()
            print(f"  removed stale {stale.relative_to(ROOT)}")

    manifest = {
        "string_count": len(STRING_BUILTINS),
        "extern_count": len(MATH_EXTERN) + len(FORMAT_EXTERN) + len(VEC_EXTERN) + len(SYNC_EXTERN),
        "pure_count": len(PURE_MODULES),
    }
    _write_json(CONTRIB_BATCH5 / "manifest.json", manifest)

    _strip_recursive_alias_blocks(ROOT / "stdlib" / "math.ny")
    _fix_builtin_collisions()
    _restore_vec_str_if_corrupted(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_impl(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_insert(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_strvec_set(ROOT / "stdlib" / "vec_str.ny")
    _consolidate_vec_i32_swap_extend(ROOT / "stdlib" / "vec.ny")
    _consolidate_vec_i32_extra(ROOT / "stdlib" / "vec.ny")
    _fix_map_or_insert_block(ROOT / "stdlib" / "map.ny")
    _consolidate_hashmap_i32_get_or(ROOT / "stdlib" / "map.ny")
    _refresh_batch5_tests()

    print(
        f"==> done: {manifest['string_count']} builtins, "
        f"{manifest['extern_count']} extern, {manifest['pure_count']} pure"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
