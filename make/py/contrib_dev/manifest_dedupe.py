"""Deduplicate docs/abi-manifest.toml — keep first symbol block per name."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ABI_PATH = ROOT / "docs" / "abi-manifest.toml"


def _iter_symbol_blocks(lines: list[str]):
    """Yield (name, start, end_exclusive) for each [[symbol]] block."""
    i = 0
    n = len(lines)
    while i < n:
        start = i
        while i < n and lines[i].startswith("# ["):
            i += 1
        if i >= n or not lines[i].startswith("[[symbol]]"):
            i = start + 1
            continue
        i += 1
        name: str | None = None
        while i < n:
            if lines[i].startswith('name = "'):
                m = re.match(r'name = "([^"]+)"', lines[i])
                if m:
                    name = m.group(1)
            if lines[i].startswith("since = "):
                i += 1
                if i < n and lines[i].startswith("# [/"):
                    i += 1
                break
            i += 1
        yield name, start, i


def dedupe_abi_manifest(path: Path = ABI_PATH) -> list[str]:
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    seen: set[str] = set()
    removed: list[str] = []
    drop_ranges: list[tuple[int, int]] = []

    for name, start, end in _iter_symbol_blocks(lines):
        if not name:
            continue
        if name in seen:
            removed.append(name)
            drop_ranges.append((start, end))
        else:
            seen.add(name)

    if not drop_ranges:
        return removed

    drop = set()
    for start, end in drop_ranges:
        drop.update(range(start, end))

    out = [line for idx, line in enumerate(lines) if idx not in drop]
    path.write_text("".join(out), encoding="utf-8")
    return removed


def strip_pure_nyra_symbols(path: Path = ABI_PATH) -> list[str]:
    """Remove manifest blocks for pure Nyra fns that must not be in C ABI."""
    pure_only = {"pow_i32"}
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    drop: set[int] = set()
    removed: list[str] = []
    for name, start, end in _iter_symbol_blocks(lines):
        if name in pure_only:
            removed.append(name)
            drop.update(range(start, end))
    if drop:
        out = [line for idx, line in enumerate(lines) if idx not in drop]
        path.write_text("".join(out), encoding="utf-8")
    return removed


if __name__ == "__main__":
    dupes = dedupe_abi_manifest()
    pure = strip_pure_nyra_symbols()
    if dupes:
        print("removed duplicate abi symbols:", ", ".join(sorted(set(dupes))))
    if pure:
        print("removed pure-only abi symbols:", ", ".join(pure))
    if not dupes and not pure:
        print("abi-manifest: ok")
