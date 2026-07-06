"""Recipe: test + example pair."""
from __future__ import annotations

from .. import patch, templates
from ..paths import EXAMPLES, TESTS_NYRA
from ..spec import RecipeResult, TestExampleSpec


def apply(spec: TestExampleSpec, *, force: bool = False) -> RecipeResult:
    marker = spec.marker
    res = RecipeResult(
        title="Test + Example Pair",
        recipe="test-example",
        marker=marker,
        patches=[],
    )

    test_path = TESTS_NYRA / f"{spec.test_base}.ny"
    res.patches.append(
        patch.write_new_file(test_path, templates.test_ny(spec, marker), marker, force=force)
    )
    if spec.with_typed:
        typed_path = TESTS_NYRA / f"{spec.test_base}.typed.ny"
        res.patches.append(
            patch.write_new_file(typed_path, templates.test_typed_ny(spec, marker), marker, force=force)
        )

    ex_dir = EXAMPLES / spec.example_topic
    ex_path = ex_dir / f"{spec.name}.ny"
    ex_typed = ex_dir / f"{spec.name}.typed.ny"
    res.patches.append(
        patch.write_new_file(ex_path, templates.example_ny(spec, marker), marker, force=force)
    )
    if spec.with_typed:
        res.patches.append(
            patch.write_new_file(ex_typed, templates.example_typed_ny(spec, marker), marker, force=force)
        )

    res.user_tasks = [
        f"Implement tests in tests/nyra/{spec.test_base}.ny",
        f"Implement demo in examples/{spec.example_topic}/{spec.name}.ny",
        f"Run: nyra test tests/nyra/{spec.test_base}.ny",
        f"Run: nyra run examples/{spec.example_topic}/{spec.name}.ny",
    ]
    return res
