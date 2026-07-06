#!/usr/bin/env python3
"""Patch an existing Nyra builtin — update wiring, preserve C code when possible.

Usage:
  make patch-builtin ARGS='-i'
  make patch-builtin ARGS='--method strip_suffix --config make/py/builtin_dev/examples/strip_suffix.json'
See: make/py/builtin_dev/README.md
"""
from __future__ import annotations

import runpy
import sys
from pathlib import Path

sys.argv = [str(Path(__file__).resolve()), "patch", *sys.argv[1:]]
runpy.run_path(str(Path(__file__).resolve().parent / "builtin-dev.py"), run_name="__main__")
