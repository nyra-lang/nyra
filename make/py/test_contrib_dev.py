#!/usr/bin/env python3
"""Smoke tests for contrib-dev Python tooling (run via `make test-contrib-py`)."""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAKE_PY = ROOT / "make" / "py"
EXAMPLES = MAKE_PY / "contrib_dev" / "examples"

MODULES = [
    MAKE_PY / "contribute.py",
    *(MAKE_PY / "contrib_dev").glob("*.py"),
    *(MAKE_PY / "contrib_dev" / "recipes").glob("*.py"),
]


def _check_manifest_invariant() -> None:
    """Every runtime_map symbol a recipe wires must also get a manifest entry.

    A runtime_map symbol missing from docs/abi-manifest.toml fails the
    `runtime_map_matches_manifest` ABI test (test-cargo-workspace). The scaffolding
    must therefore always emit a manifest block — `stable` when opted in, else
    `experimental` (which stays out of the generated C header). This locks in the
    behavior for both the builtin-method and stdlib-extern recipes so contributors
    running `make contribute` never land the repo in a drifted state.
    """
    import inspect

    from builtin_dev import add as builtin_add
    from builtin_dev import templates as btpl
    from builtin_dev.spec import BuiltinSpec, NyraType, ReceiverKind

    from contrib_dev import templates as ctpl
    from contrib_dev.spec import StdlibFnSpec

    # tier reflects stable_abi and is never omitted (both recipes' templates).
    for stable, expected in ((True, 'tier = "stable"'), (False, 'tier = "experimental"')):
        b = BuiltinSpec(
            receiver=ReceiverKind.FREE, method="probe_fn",
            returns=NyraType.I32, stable_abi=stable,
        )
        block = btpl.abi_manifest_block(b)
        assert expected in block, f"builtin manifest tier wrong (stable={stable}): {block}"
        assert 'name = "probe_fn"' in block

        s = StdlibFnSpec(
            fn_name="probe_fn", args=[], returns=NyraType.I32,
            ny_module="probe.ny", rt_module="rt_probe.c", stable_abi=stable,
        )
        cblock = ctpl.abi_manifest_block(s, s.marker)
        assert expected in cblock, f"stdlib-extern manifest tier wrong (stable={stable}): {cblock}"

    # The recipes that patch runtime_map must also patch the manifest,
    # unconditionally (not gated behind `if spec.stable_abi`).
    free_src = inspect.getsource(builtin_add._add_free)
    string_src = inspect.getsource(builtin_add._add_string)
    for name, src in (("_add_free", free_src), ("_add_string", string_src)):
        assert 'paths["runtime_map"]' in src, f"{name} no longer wires runtime_map?"
        assert 'paths["abi_manifest"]' in src, (
            f"{name} wires runtime_map but not abi_manifest — this reintroduces "
            "runtime_map_matches_manifest drift"
        )

    print("manifest-invariant: ok")


def main() -> int:
    for path in MODULES:
        subprocess.check_call([sys.executable, "-m", "py_compile", str(path)])

    sys.path.insert(0, str(MAKE_PY))
    from contrib_dev.discover import list_wired_contribs  # noqa: WPS433
    from contrib_dev.wizard import spec_from_config  # noqa: WPS433

    list_wired_contribs()

    _check_manifest_invariant()

    for name in (
        "stdlib_pure.json",
        "stdlib_module.json",
        "stdlib_extern.json",
        "test_example.json",
        "pkg.json",
        "cli.json",
        "conformance.json",
        "syntax_scaffold.json",
    ):
        data = json.loads((EXAMPLES / name).read_text(encoding="utf-8"))
        recipe = {
            "stdlib_pure.json": "stdlib-pure",
            "stdlib_module.json": "stdlib-pure",
            "stdlib_extern.json": "stdlib-extern",
            "test_example.json": "test-example",
            "pkg.json": "pkg",
            "cli.json": "cli",
            "conformance.json": "conformance",
            "syntax_scaffold.json": "syntax-scaffold",
        }[name]
        spec_from_config(recipe, data)
        if name == "stdlib_module.json":
            assert data.get("pure_source") or data.get("source_file"), "module example needs pure_source"

    print("test-contrib-py: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
