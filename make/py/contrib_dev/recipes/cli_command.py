"""Recipe: CLI command or flag scaffold (manual wiring)."""
from __future__ import annotations

from .. import patch, templates
from ..paths import SCAFFOLD_DIR
from ..spec import CliSpec, RecipeResult


def apply(spec: CliSpec, *, force: bool = False) -> RecipeResult:
    marker = spec.marker
    root = SCAFFOLD_DIR / f"cli_{spec.name}"
    res = RecipeResult(
        title="CLI Command / Flag",
        recipe="cli",
        marker=marker,
        patches=[],
    )

    res.patches.append(
        patch.write_new_file(
            root / "command.rs", templates.cli_command_stub(spec, marker), marker, force=force
        )
    )
    res.patches.append(
        patch.write_new_file(
            root / "args_snippet.rs", templates.cli_args_snippet(spec, marker), marker, force=force
        )
    )
    res.patches.append(
        patch.write_new_file(root / "README.md", templates.cli_readme(spec, marker), marker, force=force)
    )

    res.user_tasks = [
        f"Read docs/contrib_scaffold/cli_{spec.name}/README.md",
        "Copy args_snippet.rs into cli/src/app/args.rs",
        f"Move command.rs → cli/src/commands/{spec.name}.rs and implement",
        "Wire mod + dispatch in cli/src/commands/mod.rs and cli/src/app/session.rs",
        "Run: cargo test -p cli && make smoke-cli",
    ]
    res.warnings.append("CLI wiring is manual — scaffold avoids breaking the build.")
    return res
