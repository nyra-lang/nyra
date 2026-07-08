"""Recipe: stdlib pure Nyra function (Pattern A)."""
from __future__ import annotations

from .. import patch, templates
from ..paths import STDLIB, TESTS_NYRA, EXAMPLES
from ..spec import RecipeResult, StdlibFnSpec


def apply(spec: StdlibFnSpec, *, force: bool = False) -> RecipeResult:
    marker = spec.marker
    res = RecipeResult(
        title="Stdlib Pure Function",
        recipe="stdlib-pure",
        marker=marker,
        patches=[],
    )

    ny_path = STDLIB / spec.ny_module
    res.patches.append(
        patch.upsert_marked_block(ny_path, templates.pure_fn_block(spec, marker), marker)
    )

    test_base = f"{spec.fn_name}_test"
    test_path = TESTS_NYRA / f"{test_base}.ny"
    test_spec = _test_spec(spec, test_base)
    res.patches.append(
        patch.write_new_file(test_path, templates.test_ny(test_spec, marker), marker, force=force)
    )
    typed_test = TESTS_NYRA / f"{test_base}.typed.ny"
    res.patches.append(
        patch.write_new_file(typed_test, templates.test_typed_ny(test_spec, marker), marker, force=force)
    )

    topic = (spec.example_topic or spec.ny_module.split("/")[0]).strip() or "stdlib"
    ex_dir = EXAMPLES / topic
    ex_path = ex_dir / f"{spec.fn_name}.ny"
    ex_typed = ex_dir / f"{spec.fn_name}.typed.ny"
    ex_spec = _example_spec(spec)
    res.patches.append(
        patch.write_new_file(ex_path, templates.example_ny(ex_spec, marker), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(ex_typed, templates.example_typed_ny(ex_spec, marker), marker, force=force)
    )

    if spec.pure_source:
        res.user_tasks = [
            f"Flesh out structs/fns in {spec.stdlib_path} (search [contrib-dev:{marker}])",
            f"Wire imports from stdlib/net/http/mod.ny (or the owning module) if needed",
            f"Update tests in tests/nyra/{test_base}.ny (+ .typed.ny)",
            f"Demo in examples/{topic}/{spec.fn_name}.ny",
            f"Run: nyra test tests/nyra/{test_base}.ny",
        ]
        res.usage_lines = [
            f"import \"{spec.stdlib_path}\"  # multi-fn / struct module scaffold",
        ]
        res.warnings.append(
            "Module scaffold: returns type in JSON is ignored when pure_source is set — "
            "put full Nyra source (structs + fns) in pure_source."
        )
    else:
        res.user_tasks = [
            f"Implement fn body in {spec.stdlib_path} (search [contrib-dev:{marker}])",
            f"Update tests in tests/nyra/{test_base}.ny (+ .typed.ny)",
            f"Demo in examples/{topic}/{spec.fn_name}.ny",
            f"Run: nyra test tests/nyra/{test_base}.ny",
        ]
        res.usage_lines = [
            f'{spec.fn_name}(…)  # auto-prelude if public in {spec.stdlib_path}',
        ]
    return res


def _test_spec(spec: StdlibFnSpec, name: str):
    from ..spec import TestExampleSpec

    return TestExampleSpec(
        name=name.replace("_test", ""),
        import_path=spec.stdlib_path,
        use_testing=True,
        example_topic=(spec.example_topic or "syntax"),
    )


def _example_spec(spec: StdlibFnSpec):
    from ..spec import TestExampleSpec

    return TestExampleSpec(
        name=spec.fn_name,
        import_path=spec.stdlib_path,
        use_testing=False,
        example_topic=(spec.example_topic or spec.ny_module.split("/")[0] or "stdlib"),
    )
