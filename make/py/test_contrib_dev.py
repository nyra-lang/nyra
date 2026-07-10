#!/usr/bin/env python3
"""Backward-compatible alias — run CONF-CONTRIB-PY conformance suite."""
from __future__ import annotations

from test_contrib_conformance import main

if __name__ == "__main__":
    raise SystemExit(main())
