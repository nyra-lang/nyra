"""Recipe: conformance pass/fail contract test."""
from __future__ import annotations

from .. import patch, templates
from ..paths import CONFORMANCE
from ..spec import ConformanceMode, ConformanceSpec, RecipeResult


def apply(spec: ConformanceSpec, *, force: bool = False) -> RecipeResult:
    marker = spec.marker
    subdir = "pass" if spec.mode == ConformanceMode.PASS else "fail"
    path = CONFORMANCE / subdir / spec.area / f"{spec.name}.ny"
    res = RecipeResult(
        title="Conformance Test",
        recipe="conformance",
        marker=marker,
        patches=[],
    )

    if spec.mode == ConformanceMode.PASS:
        content = templates.conformance_pass(spec, marker)
    else:
        content = templates.conformance_fail(spec, marker)

    res.patches.append(patch.write_new_file(path, content, marker, force=force))

    rel = f"tests/conformance/{subdir}/{spec.area}/{spec.name}.ny"
    if spec.mode == ConformanceMode.PASS:
        res.user_tasks = [
            f"Implement contract assertions in {rel}",
            f"Run: nyra test {rel}",
            "Run: make test-conformance",
        ]
    else:
        res.user_tasks = [
            f"Replace stub with code that must NOT compile in {rel}",
            f"Run: nyra check {rel}  # must exit non-zero",
            "Run: make test-conformance",
        ]
    return res
