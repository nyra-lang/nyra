"""Re-apply a contrib scaffold: remove by marker, then add with updated spec."""
from __future__ import annotations

from .remove import remove_by_marker, remove_to_recipe_result
from .spec import RecipeResult


def patch_apply(*, marker: str, apply_fn, spec, force: bool = True) -> RecipeResult:
    removed = remove_by_marker(marker)
    added: RecipeResult = apply_fn(spec, force=force)
    added.title = "Patch Scaffold"
    added.recipe = "patch"
    added.marker = marker
    added.warnings = list(removed.warnings) + list(added.warnings)
    added.user_tasks = [
        "Review re-wired files after patch",
        *added.user_tasks,
    ]
    if removed.ok():
        added.patches = removed.patches + added.patches
    return added
