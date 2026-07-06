#!/usr/bin/env python3
"""Add a Nyra builtin — monitor report shows what was done and your next tasks.

Usage:
  make add-builtin ARGS='-i'
  make add-builtin ARGS='--config make/py/builtin_dev/examples/strip_suffix.json'
See: make/py/builtin_dev/README.md
"""
from __future__ import annotations

import runpy
import sys
from pathlib import Path

sys.argv = [str(Path(__file__).resolve()), "add", *sys.argv[1:]]
runpy.run_path(str(Path(__file__).resolve().parent / "builtin-dev.py"), run_name="__main__")
