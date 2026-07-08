#!/usr/bin/env python3
"""Regenerate comparison *_typed.ny from zero-types sources (same algorithm, explicit types)."""
from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMP = ROOT / "examples" / "comparison"

_spec = importlib.util.spec_from_file_location(
    "snippet_types", ROOT / "make" / "py" / "snippet-types.py"
)
_st = importlib.util.module_from_spec(_spec)
assert _spec and _spec.loader
_spec.loader.exec_module(_st)

SINGLE_FILE = [
    "hello/hello.ny",
    "arithmetic/sum.ny",
    "loop/sum_loop.ny",
    "loop_nofold/sum_loop_nofold.ny",
    "fib/fib.ny",
    "nested/nested.ny",
    "struct_sum/struct_sum.ny",
    "cpu_bound/bench.ny",
    "mix/mix.ny",
]

EXTENDED_GLOB = [
    "memory/*/bench.ny",
    "strings/*/bench.ny",
    "collections/*/bench.ny",
    "algorithms/*/bench.ny",
    "concurrency/*/bench.ny",
]


def polish_typed(src: str, typed: str) -> str:
    """Fix patterns add_explicit_types misses for benchmark snippets."""
    typed = re.sub(r"(?<!\blet )\bmut\s+(\w+)", r"let mut \1", typed)
    typed = re.sub(
        r"extern fn blackbox_i32\(x\)",
        "extern fn blackbox_i32(x: i32)",
        typed,
    )
    typed = re.sub(
        r"extern fn blackbox_i32\(x: i32\) -> i32",
        "extern fn blackbox_i32(x: i32) -> i32",
        typed,
    )
    # Preserve fully-typed extern signatures from zero-types source
    for m in re.finditer(r"^(extern fn .+)$", src, re.M):
        line = m.group(1)
        if ":" not in line and "->" not in line:
            continue
        name = re.search(r"extern fn (\w+)", line).group(1)
        typed = re.sub(
            rf"^extern fn {re.escape(name)}\(.+$",
            line,
            typed,
            count=1,
            flags=re.M,
        )
    # Preserve struct blocks from source when generator drops commas
    if "struct Point" in src:
        typed = re.sub(
            r"struct Point \{[^}]+\}",
            re.search(r"struct Point \{[^}]+\}", src, re.S).group(0),  # type: ignore
            typed,
            count=1,
        )
    for m in re.finditer(r"^(struct \w+ \{[^}]+\})", src, re.S | re.M):
        typed = re.sub(
            re.escape(m.group(1)),
            m.group(1),
            typed,
            count=1,
        )
    # Preserve non-main helper fn signatures/bodies from source (e.g. pass-by-value use_pair)
    for m in re.finditer(r"^fn (?!main)(\w+)\([^)]*\)[^{]*\{[^}]*\}", src, re.S | re.M):
        typed = re.sub(
            rf"^fn {re.escape(m.group(1))}\([^)]*\)[^{{]*\{{[^}}]*\}}",
            m.group(0),
            typed,
            count=1,
            flags=re.S | re.M,
        )
    return typed


def typed_from_easy(easy: str) -> str:
    easy = _st.fix_struct_literal_commas(easy)
    typed = _st.add_explicit_types(easy)
    return polish_typed(easy, typed)


def write_single(rel: str) -> None:
    src = COMP / rel
    out = src.with_name(src.stem + "_typed.ny")
    easy = src.read_text(encoding="utf-8")
    out.write_text(typed_from_easy(easy), encoding="utf-8")
    print(f"  {out.relative_to(ROOT)}")


def sync_dungeon_typed() -> None:
    """dungeon_typed/ is hand-maintained (fully typed multi-file app)."""
    dst = COMP / "dungeon_typed"
    if not dst.exists():
        print("  warn: dungeon_typed/ missing — copy from typed dungeon sources")


def main() -> int:
    print("sync-comparison-typed:")
    for rel in SINGLE_FILE:
        write_single(rel)
    for pattern in EXTENDED_GLOB:
        for src in sorted(COMP.glob(pattern)):
            rel = str(src.relative_to(COMP))
            write_single(rel)
    sync_dungeon_typed()
    return 0


if __name__ == "__main__":
    sys.exit(main())
