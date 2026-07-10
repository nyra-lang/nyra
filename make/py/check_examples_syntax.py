#!/usr/bin/env python3
"""Fast syntax guards for Nyra examples before nyra check (optional-types / CI)."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = (
    ROOT / "examples" / "builtins",
    ROOT / "examples" / "syntax",
    ROOT / "examples" / "option",
    ROOT / "examples" / "result",
)


def _contrib_slug(text: str) -> str | None:
    m = re.search(r"contrib-dev:([^:]+):", text)
    return m.group(1) if m else None


def scan_file(path: Path) -> list[str]:
    issues: list[str] = []
    text = path.read_text(encoding="utf-8")
    slug = _contrib_slug(text)
    rel = path.relative_to(ROOT)
    for lineno, line in enumerate(text.splitlines(), 1):
        s = line.strip()
        if s.startswith("print(") and s.count("(") != s.count(")"):
            if not s.startswith("print(match"):
                issues.append(f"{rel}:{lineno}: unbalanced print(): {s}")
        if slug and re.search(rf"\b{re.escape(slug)}\s*\(\s*\)", s):
            issues.append(f"{rel}:{lineno}: scaffold slug called as function: {slug}()")
    return issues


def main() -> int:
    issues: list[str] = []
    for base in SCAN_ROOTS:
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.ny")):
            if not path.is_file():
                continue
            issues.extend(scan_file(path))
    if issues:
        print("check_examples_syntax FAILED:", file=sys.stderr)
        for item in issues:
            print(f"  {item}", file=sys.stderr)
        return 1
    print("check_examples_syntax: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
