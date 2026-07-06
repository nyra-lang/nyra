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


def main() -> int:
    for path in MODULES:
        subprocess.check_call([sys.executable, "-m", "py_compile", str(path)])

    sys.path.insert(0, str(MAKE_PY))
    from contrib_dev.discover import list_wired_contribs  # noqa: WPS433
    from contrib_dev.wizard import spec_from_config  # noqa: WPS433

    list_wired_contribs()

    for name in (
        "stdlib_pure.json",
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
            "stdlib_extern.json": "stdlib-extern",
            "test_example.json": "test-example",
            "pkg.json": "pkg",
            "cli.json": "cli",
            "conformance.json": "conformance",
            "syntax_scaffold.json": "syntax-scaffold",
        }[name]
        spec_from_config(recipe, data)

    print("test-contrib-py: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
