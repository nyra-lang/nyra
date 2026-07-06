"""Scan the repo for scaffolds wired by contrib-dev markers."""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from .paths import ROOT, ABI_MANIFEST, CONFORMANCE, EXAMPLES, PKG_EXAMPLES, RUNTIME_MAP, SCAFFOLD_DIR, STDLIB, TESTS_NYRA

MARKER_RE = re.compile(r"\[contrib-dev:([^\]]+)\]")

SCAN_DIRS = (
    STDLIB,
    TESTS_NYRA,
    EXAMPLES,
    CONFORMANCE,
    PKG_EXAMPLES,
    SCAFFOLD_DIR,
    ROOT / "docs",
    ROOT / "compiler" / "codegen" / "src",
    ROOT / "grammar",
)

SKIP_DIR_NAMES = {"target", "node_modules", ".git", ".nyra-cache", "__pycache__"}


@dataclass(frozen=True)
class WiredContrib:
    marker: str
    recipe: str
    paths: tuple[Path, ...]

    @property
    def label(self) -> str:
        return f"{self.recipe} [{self.marker}]"


def infer_recipe(marker: str) -> str:
    if marker.startswith("test_example:"):
        return "test-example"
    if marker.startswith("pkg:"):
        return "pkg"
    if marker.startswith("cli:"):
        return "cli"
    if marker.startswith("conformance:"):
        return "conformance"
    if marker.startswith("syntax:"):
        return "syntax-scaffold"
    if ":" in marker:
        return "stdlib"
    return "unknown"


def _iter_files(root: Path):
    if not root.exists():
        return
    if root.is_file():
        yield root
        return
    for path in root.rglob("*"):
        if any(part in SKIP_DIR_NAMES for part in path.parts):
            continue
        if path.is_file() and path.suffix in {
            ".ny",
            ".rs",
            ".c",
            ".h",
            ".toml",
            ".md",
            ".json",
        }:
            yield path


def scan_markers() -> dict[str, list[Path]]:
    found: dict[str, list[Path]] = {}
    extra_files = [RUNTIME_MAP, ABI_MANIFEST]
    seen_paths: set[Path] = set()

    for root in SCAN_DIRS:
        for path in _iter_files(root):
            if path in seen_paths:
                continue
            seen_paths.add(path)
            try:
                text = path.read_text(encoding="utf-8")
            except OSError:
                continue
            for match in MARKER_RE.finditer(text):
                marker = match.group(1)
                found.setdefault(marker, [])
                if path not in found[marker]:
                    found[marker].append(path)

    for path in extra_files:
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8")
        for match in MARKER_RE.finditer(text):
            marker = match.group(1)
            found.setdefault(marker, [])
            if path not in found[marker]:
                found[marker].append(path)

    return found


def list_wired_contribs(*, recipe: str | None = None) -> list[WiredContrib]:
    merged = scan_markers()
    out: list[WiredContrib] = []
    for marker, paths in sorted(merged.items()):
        kind = infer_recipe(marker)
        if recipe is not None and kind != recipe:
            continue
        out.append(WiredContrib(marker=marker, recipe=kind, paths=tuple(sorted(paths))))
    return out
