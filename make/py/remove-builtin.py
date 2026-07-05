#!/usr/bin/env python3
"""Remove a Nyra builtin — monitor report shows what was removed and next steps.

Usage:
  make remove-builtin ARGS='--method strip_suffix'
  make remove-builtin ARGS='-i'
See: make/py/builtin_dev/README.md
"""
from __future__ import annotations

import runpy
import sys
from pathlib import Path

sys.argv = [str(Path(__file__).resolve()), "remove", *sys.argv[1:]]
runpy.run_path(str(Path(__file__).resolve().parent / "builtin-dev.py"), run_name="__main__")
