"""Remove contrib-dev scaffolds by marker."""
from __future__ import annotations

import shutil
from dataclasses import dataclass, field
from pathlib import Path

from . import patch
from .discover import WiredContrib, list_wired_contribs, scan_markers
from .paths import PKG_EXAMPLES, RUNTIME_MAP
from .spec import RecipeResult


@dataclass
class RemoveResult:
    marker: str
    recipe: str
    patches: list[patch.PatchResult] = field(default_factory=list)
    user_tasks: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    def ok(self) -> bool:
        return any(p.changed for p in self.patches)


def remove_by_marker(marker: str) -> RemoveResult:
    wired = [w for w in list_wired_contribs() if w.marker == marker]
    recipe = wired[0].recipe if wired else "unknown"
    res = RemoveResult(marker=marker, recipe=recipe)

    paths = scan_markers().get(marker, [])
    for path in paths:
        if path.suffix == ".ny" and _is_standalone_scaffold(path, marker):
            if path.exists():
                path.unlink()
                res.patches.append(patch.PatchResult(path, True, "deleted"))
            continue
        res.patches.append(patch.delete_if_marker_only(path, marker))
        if path.exists() and patch.has_marker(patch.read_text(path), marker):
            res.patches.append(patch.strip_marker_from_file(path, marker))

    fn_name = marker.split(":")[0]
    if recipe == "stdlib" and fn_name not in ("test_example", "pkg", "cli", "conformance", "syntax"):

        def strip_runtime_map(content: str) -> tuple[str, bool]:
            return patch.remove_line_with(content, f'("{fn_name}"')

        res.patches.append(patch.patch_file(RUNTIME_MAP, strip_runtime_map))

    if marker.startswith("pkg:"):
        pkg_name = marker.split(":", 1)[1]
        pkg_root = PKG_EXAMPLES / pkg_name
        if pkg_root.is_dir() and _dir_only_scaffold(pkg_root, marker):
            shutil.rmtree(pkg_root)
            res.patches.append(patch.PatchResult(pkg_root, True, "deleted package dir"))

    if marker.startswith("cli:") or marker.startswith("syntax:"):
        slug = marker.split(":", 1)[1].split(":")[0]
        prefix = "cli_" if marker.startswith("cli:") else "syntax_"
        from .paths import SCAFFOLD_DIR

        scaffold = SCAFFOLD_DIR / f"{prefix}{slug}"
        if scaffold.is_dir() and _dir_only_scaffold(scaffold, marker):
            shutil.rmtree(scaffold)
            res.patches.append(patch.PatchResult(scaffold, True, "deleted scaffold dir"))

    res.user_tasks = [
        "Search repo for leftover references to removed symbols/files",
        "Run: cargo test --workspace",
        "Run: make test-preflight",
    ]
    if not res.ok():
        res.warnings.append("No contrib-dev markers found for this marker — already removed?")
    return res


def _is_standalone_scaffold(path: Path, marker: str) -> bool:
    if not path.exists():
        return False
    content = patch.read_text(path)
    if not patch.has_marker(content, marker):
        return False
    stripped, _ = patch.remove_marked_block(content, marker)
    return not stripped.strip() or content.strip().startswith("//")


def _dir_only_scaffold(root: Path, marker: str) -> bool:
    if not root.is_dir():
        return False
    for path in root.rglob("*"):
        if path.is_file():
            try:
                text = path.read_text(encoding="utf-8")
            except OSError:
                return False
            if patch.has_marker(text, marker):
                continue
            if text.strip():
                return False
    return True


def remove_to_recipe_result(result: RemoveResult) -> RecipeResult:
    return RecipeResult(
        title="Remove Scaffold",
        recipe="remove",
        marker=result.marker,
        patches=result.patches,
        user_tasks=result.user_tasks,
        warnings=result.warnings,
    )
