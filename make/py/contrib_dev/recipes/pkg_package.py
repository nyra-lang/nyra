"""Recipe: NyraPkg package scaffold."""
from __future__ import annotations

from .. import patch, templates
from ..paths import PKG_EXAMPLES
from ..spec import PkgSpec, RecipeResult


def apply(spec: PkgSpec, *, force: bool = False) -> RecipeResult:
    marker = spec.marker
    root = PKG_EXAMPLES / spec.name
    res = RecipeResult(
        title="NyraPkg Package",
        recipe="pkg",
        marker=marker,
        patches=[],
    )

    res.patches.append(
        patch.write_new_file(root / "nyra.mod", templates.pkg_nyra_mod(spec, marker), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(
            root / f"{spec.module_name}.ny", templates.pkg_api_ny(spec, marker), marker, force=force
        )
    )
    res.patches.append(
        patch.write_new_file(root / "main.ny", templates.pkg_main_ny(spec, marker), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(root / "README.md", templates.pkg_readme(spec, marker), marker, force=force)
    )

    if spec.link_lib:
        rt_path = root / spec.rt_file
        res.patches.append(
            patch.write_new_file(rt_path, templates.pkg_rt_c(spec, marker), marker, force=force)
        )

    res.user_tasks = [
        f"Implement API in examples/packages/{spec.name}/{spec.module_name}.ny",
        f"Implement C shims in examples/packages/{spec.name}/{spec.rt_file}" if spec.link_lib else "Add extern/C as needed",
        f"Smoke: cd examples/packages/{spec.name} && nyra run main.ny",
        "See docs/nyrapkg-v1.md for publish workflow",
    ]
    res.usage_lines = [f'import "pkg/{spec.name}/{spec.module_name}.ny"']
    return res
