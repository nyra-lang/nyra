"""Recipe: stdlib extern fn + C runtime (Pattern B)."""
from __future__ import annotations

from .. import patch, templates
from ..paths import ABI_MANIFEST, RUNTIME_MAP, STDLIB, TESTS_NYRA, EXAMPLES
from ..spec import RecipeResult, StdlibFnSpec

import sys
from pathlib import Path

_MAKE_PY = Path(__file__).resolve().parents[2]
if str(_MAKE_PY) not in sys.path:
    sys.path.insert(0, str(_MAKE_PY))
from naming_guide import format_extern_name_summary


def apply(spec: StdlibFnSpec, *, force: bool = False) -> RecipeResult:
    if not spec.rt_module:
        raise ValueError("rt_module is required for stdlib-extern")

    marker = spec.marker
    res = RecipeResult(
        title="Stdlib Extern + C",
        recipe="stdlib-extern",
        marker=marker,
        patches=[],
    )

    ny_path = STDLIB / spec.ny_module
    res.patches.append(patch.append_extern_line(ny_path, templates.extern_line(spec), marker))

    rt_path = STDLIB / "rt" / spec.rt_module
    rt_content = patch.read_text(rt_path) if rt_path.exists() else ""
    if (
        not patch.has_marker(rt_content, marker)
        and patch.c_function_defined(rt_content, spec.fn_name)
    ):
        res.patches.append(
            patch.PatchResult(
                rt_path,
                False,
                f"C symbol `{spec.fn_name}` already defined — skipped duplicate stub",
            )
        )
        res.warnings.append(
            f"C function `{spec.fn_name}` already exists in {rt_path.name} "
            "(likely from another recipe, e.g. Built-in Method). Skipped the "
            "duplicate stub to avoid a C redefinition error. Reuse that "
            "implementation, or remove it first before re-adding."
        )
    else:
        res.patches.append(patch.upsert_marked_block(rt_path, templates.c_stub(spec, marker), marker))

    sym_line = templates.runtime_map_line(spec)

    def add_runtime_map(content: str) -> tuple[str, bool]:
        if f'("{spec.fn_name}"' in content:
            return content, False
        return patch.add_line_before_anchor(content, "    ])", f"        {sym_line}", last=True)

    res.patches.append(patch.patch_file(RUNTIME_MAP, add_runtime_map))

    if spec.stable_abi:
        res.patches.append(
            patch.upsert_marked_block(
                ABI_MANIFEST, templates.abi_manifest_block(spec, marker), marker
            )
        )
        res.warnings.append("Run: make gen-abi-header && make gen-bindings-doc")

    test_base = f"{spec.fn_name}_test"
    from ..spec import TestExampleSpec

    test_spec = TestExampleSpec(name=test_base.replace("_test", ""), import_path=spec.stdlib_path)
    test_path = TESTS_NYRA / f"{test_base}.ny"
    res.patches.append(
        patch.write_new_file(test_path, templates.test_ny(test_spec, marker), marker, force=force)
    )
    typed_test = TESTS_NYRA / f"{test_base}.typed.ny"
    res.patches.append(
        patch.write_new_file(typed_test, templates.test_typed_ny(test_spec, marker), marker, force=force)
    )

    topic = spec.ny_module.split("/")[0]
    ex_dir = EXAMPLES / "builtins" / topic
    ex_path = ex_dir / f"{spec.fn_name}.ny"
    ex_typed = ex_dir / f"{spec.fn_name}.typed.ny"
    ex_spec = TestExampleSpec(name=spec.fn_name, import_path=spec.stdlib_path, use_testing=False)
    res.patches.append(
        patch.write_new_file(ex_path, templates.example_ny(ex_spec, marker), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(ex_typed, templates.example_typed_ny(ex_spec, marker), marker, force=force)
    )

    res.user_tasks = [
        f"Implement C in stdlib/rt/{spec.rt_module} ([contrib-dev:{marker}])",
        f"Fix test expectations in tests/nyra/{test_base}.ny",
        "make install-dev",
        f"nyra test tests/nyra/{test_base}.ny",
    ]
    if spec.stable_abi:
        res.user_tasks.append("make gen-abi-header && make gen-bindings-doc")
        res.user_tasks.append("Verify: cargo test -p driver abi_manifest -- manifest sync")
    res.usage_lines = [
        templates.extern_line(spec),
        *format_extern_name_summary(spec.fn_name),
    ]
    return res
