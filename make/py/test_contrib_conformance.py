#!/usr/bin/env python3
"""Conformance tests for contributor automation (CONF-CONTRIB-PY).

Run: make test-contrib-conformance
Contract: tests/conformance/pass/contrib_automation/README.md
"""
from __future__ import annotations

import json
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAKE_PY = ROOT / "make" / "py"
EXAMPLES = MAKE_PY / "contrib_dev" / "examples"
ABI_PATH = ROOT / "docs" / "abi-manifest.toml"
RUNTIME_MAP = ROOT / "compiler" / "codegen" / "src" / "runtime_map.rs"

# Pure Nyra stdlib fns that must never appear in abi-manifest / runtime_map as C symbols.
PURE_NYRA_ABI_DENYLIST = frozenset({"pow_i32", "sqrt_i32"})

AUTOMATION_GLOBS = (
    MAKE_PY / "contribute.py",
    MAKE_PY / "builtin-dev.py",
    MAKE_PY / "test_contrib_conformance.py",
    MAKE_PY / "test_contrib_dev.py",
    MAKE_PY / "builtin_dev" / "*.py",
    MAKE_PY / "contrib_dev" / "*.py",
    MAKE_PY / "contrib_dev" / "recipes" / "*.py",
)


def _ok(label: str) -> None:
    print(f"  ok — {label}")


def _fail(msg: str) -> None:
    print(f"CONF-CONTRIB-PY FAILED: {msg}", file=sys.stderr)
    raise SystemExit(1)


def _collect_py_files() -> list[Path]:
    out: list[Path] = []
    for pattern in AUTOMATION_GLOBS:
        if pattern.suffix == ".py" and pattern.is_file():
            out.append(pattern)
        else:
            out.extend(sorted(pattern.parent.glob(pattern.name)))
    # dedupe
    seen: set[str] = set()
    unique: list[Path] = []
    for p in out:
        key = str(p.resolve())
        if key not in seen:
            seen.add(key)
            unique.append(p)
    return unique


def check_py_compile() -> None:
    for path in _collect_py_files():
        subprocess.check_call([sys.executable, "-m", "py_compile", str(path)])
    _ok(f"py_compile ({len(_collect_py_files())} automation modules)")


def check_cli_help() -> None:
    for script, args in (
        (MAKE_PY / "contribute.py", ["--help"]),
        (MAKE_PY / "contribute.py", ["hub", "--help"]),
        (MAKE_PY / "contribute.py", ["add", "--help"]),
        (MAKE_PY / "builtin-dev.py", ["--help"]),
        (MAKE_PY / "builtin-dev.py", ["add", "--help"]),
    ):
        rc = subprocess.call(
            [sys.executable, str(script), *args],
            cwd=str(ROOT),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if rc != 0:
            _fail(f"{script.name} {' '.join(args)} exited {rc}")
    _ok("CLI --help (contribute + builtin-dev)")


def check_manifest_invariant() -> None:
    import inspect

    from builtin_dev import add as builtin_add
    from builtin_dev import templates as btpl
    from builtin_dev.spec import BuiltinSpec, NyraType, ReceiverKind

    from contrib_dev import templates as ctpl
    from contrib_dev.spec import StdlibFnSpec

    for stable, expected in ((True, 'tier = "stable"'), (False, 'tier = "experimental"')):
        b = BuiltinSpec(
            receiver=ReceiverKind.FREE,
            method="probe_fn",
            returns=NyraType.I32,
            stable_abi=stable,
        )
        block = btpl.abi_manifest_block(b)
        if expected not in block:
            _fail(f"builtin manifest tier wrong (stable={stable})")
        if 'name = "probe_fn"' not in block:
            _fail("builtin manifest missing name")

        s = StdlibFnSpec(
            fn_name="probe_fn",
            args=[],
            returns=NyraType.I32,
            ny_module="probe.ny",
            rt_module="rt_probe.c",
            stable_abi=stable,
        )
        cblock = ctpl.abi_manifest_block(s, s.marker)
        if expected not in cblock:
            _fail(f"stdlib-extern manifest tier wrong (stable={stable})")

    free_src = inspect.getsource(builtin_add._add_free)
    string_src = inspect.getsource(builtin_add._add_string)
    for name, src in (("_add_free", free_src), ("_add_string", string_src)):
        if 'paths["runtime_map"]' not in src:
            _fail(f"{name} no longer wires runtime_map")
        if 'paths["abi_manifest"]' not in src:
            _fail(f"{name} wires runtime_map but not abi_manifest")
    _ok("manifest template + wiring invariant")


def check_example_codegen() -> None:
    from builtin_dev.spec import ArgSpec, NyraType
    from contrib_dev.example_codegen import demo_body, extern_test_body
    from contrib_dev.spec import StdlibFnSpec, builtin_example_topic, guess_rt_module

    assert builtin_example_topic("math.ny") == "math"
    assert builtin_example_topic("encoding/mod.ny") == "encoding"
    assert guess_rt_module("map.ny", "map_str_str_clear") == "rt_map_str_str.c"

    floor = StdlibFnSpec(
        fn_name="floor_f64",
        args=[ArgSpec("x", NyraType.F64)],
        returns=NyraType.F64,
        ny_module="math.ny",
        rt_module="rt_math.c",
    )
    if "print(floor(3.7))" not in demo_body(floor):
        _fail("math demo_body floor_f64")

    vec = StdlibFnSpec(
        fn_name="vec_i32_remove_at",
        args=[ArgSpec("handle", NyraType.PTR), ArgSpec("index", NyraType.I32)],
        returns=NyraType.I32,
        ny_module="vec.ny",
        rt_module="rt_vec.c",
    )
    body = demo_body(vec)
    if ".remove(0)" not in body or "remove_at" in body:
        _fail("vec_i32_remove_at demo should use .remove() sugar")

    atomic = StdlibFnSpec(
        fn_name="atomic_sub_i32",
        args=[ArgSpec("p", NyraType.PTR), ArgSpec("delta", NyraType.I32)],
        returns=NyraType.I32,
        ny_module="sync/atomic.ny",
        rt_module="rt_atomic.c",
    )
    ademo = demo_body(atomic)
    if "ptr(0)" in ademo or "Atomic_i32_new" not in ademo:
        _fail("atomic_sub_i32 demo must use Atomic_i32_new, not ptr(0)")

    pure_map = StdlibFnSpec(
        fn_name="hashmap_i32_get_or",
        args=[],
        returns=NyraType.VOID,
        ny_module="map.ny",
        pure_source="impl HashMap_i32_i32 {\n    fn get_or(self, key: i32, default: i32) -> i32 { return default }\n}",
    )
    mdemo = demo_body(pure_map)
    if "hashmap_i32_get_or()" in mdemo or "get_or(" not in mdemo:
        _fail("hashmap_i32_get_or demo must call m.get_or(...)")

    hex_spec = StdlibFnSpec(
        fn_name="hex_decode",
        args=[ArgSpec("hex", NyraType.STRING)],
        returns=NyraType.STRING,
        ny_module="encoding/mod.ny",
        rt_module="rt_strings.c",
    )
    tests = "\n".join(extern_test_body(hex_spec))
    if 'assert_str_eq(hex_decode("4869"), "Hi")' not in tests:
        _fail("hex_decode extern_test_body")
    _ok("example_codegen demos + tests")


def check_recipe_json_examples() -> None:
    from contrib_dev.wizard import spec_from_config

    mapping = {
        "stdlib_pure.json": "stdlib-pure",
        "stdlib_module.json": "stdlib-pure",
        "stdlib_extern.json": "stdlib-extern",
        "test_example.json": "test-example",
        "pkg.json": "pkg",
        "cli.json": "cli",
        "conformance.json": "conformance",
        "syntax_scaffold.json": "syntax-scaffold",
    }
    for name, recipe in mapping.items():
        data = json.loads((EXAMPLES / name).read_text(encoding="utf-8"))
        spec_from_config(recipe, data)
        if name == "stdlib_module.json" and not (data.get("pure_source") or data.get("source_file")):
            _fail("stdlib_module.json needs pure_source")
    _ok("recipe JSON examples → spec_from_config")


def check_batch_json_catalogs() -> None:
    from contrib_dev.wizard import spec_from_config

    count = 0
    for batch_dir in sorted(EXAMPLES.glob("batch*")):
        if not batch_dir.is_dir():
            continue
        for cfg in sorted(batch_dir.glob("*.json")):
            if cfg.name == "manifest.json":
                continue
            data = json.loads(cfg.read_text(encoding="utf-8"))
            recipe = data.get("recipe")
            if not recipe:
                recipe = "stdlib-extern" if data.get("rt_module") else "stdlib-pure"
            if recipe == "builtin":
                count += 1
                continue
            spec_from_config(recipe, data)
            count += 1
    if count < 10:
        _fail(f"expected batch JSON configs, got {count}")
    _ok(f"batch JSON catalogs ({count} configs)")


def check_abi_manifest_tree() -> None:
    if not ABI_PATH.is_file():
        _fail("docs/abi-manifest.toml missing")
    try:
        manifest = tomllib.loads(ABI_PATH.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        _fail(f"abi-manifest.toml parse: {exc}")
    symbols = manifest.get("symbol", [])
    names = [s["name"] for s in symbols]
    dupes = [n for n, c in Counter(names).items() if c > 1]
    if dupes:
        _fail(f"duplicate abi symbol names: {', '.join(sorted(dupes))}")
    for name in PURE_NYRA_ABI_DENYLIST:
        if name in names:
            _fail(f"pure Nyra fn {name!r} must not be in abi-manifest.toml")
    _ok("abi-manifest.toml (parse, no dupes, no pure-only symbols)")


def check_runtime_map_pure_denylist() -> None:
    text = RUNTIME_MAP.read_text(encoding="utf-8")
    for name in PURE_NYRA_ABI_DENYLIST:
        if f'("{name}",' in text:
            _fail(f"pure Nyra fn {name!r} must not be in runtime_map.rs")
    _ok("runtime_map.rs pure-fn denylist")


def check_manifest_dedupe_idempotent() -> None:
    import shutil
    import tempfile

    from contrib_dev.manifest_dedupe import dedupe_abi_manifest, strip_pure_nyra_symbols

    with tempfile.TemporaryDirectory() as td:
        copy = Path(td) / "abi-manifest.toml"
        shutil.copy(ABI_PATH, copy)
        dedupe_abi_manifest(copy)
        strip_pure_nyra_symbols(copy)
        once = copy.read_text(encoding="utf-8")
        dedupe_abi_manifest(copy)
        strip_pure_nyra_symbols(copy)
        twice = copy.read_text(encoding="utf-8")
        if once != twice:
            _fail("manifest_dedupe not idempotent")
    _ok("manifest_dedupe idempotent")


def check_discover_smoke() -> None:
    from builtin_dev.discover import list_wired_builtins
    from contrib_dev.discover import list_wired_contribs

    list_wired_contribs()
    list_wired_builtins()
    _ok("discover list_wired_* smoke")


def check_batch_add_dry_run() -> None:
    script = MAKE_PY / "builtin_dev" / "batch_add.py"
    rc = subprocess.call(
        [sys.executable, str(script), "--batch", "batch6", "--only", "pure", "--dry-run"],
        cwd=str(ROOT),
    )
    if rc != 0:
        _fail("batch_add.py --dry-run batch6 failed")
    _ok("batch_add.py --dry-run batch6")


def check_hub_imports() -> None:
    from contrib_dev import hub, validate  # noqa: F401

    _ok("contrib_dev.hub + validate import")


def main() -> int:
    if str(MAKE_PY) not in sys.path:
        sys.path.insert(0, str(MAKE_PY))

    print("CONF-CONTRIB-PY: contributor automation conformance")
    check_py_compile()
    check_cli_help()
    check_hub_imports()
    check_discover_smoke()
    check_manifest_invariant()
    check_example_codegen()
    check_recipe_json_examples()
    check_batch_json_catalogs()
    check_abi_manifest_tree()
    check_runtime_map_pure_denylist()
    check_manifest_dedupe_idempotent()
    check_batch_add_dry_run()
    print("CONF-CONTRIB-PY: all checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
