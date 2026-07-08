#!/usr/bin/env python3
"""Nyra contributor hub — `make contribute`.

Subcommands: add (default), remove, list, patch.
Built-in methods (menu 3) delegate to `builtin-dev.py`.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

_MAKE_PY = Path(__file__).resolve().parent
_REPO_ROOT = _MAKE_PY.parent.parent
if str(_MAKE_PY) not in sys.path:
    sys.path.insert(0, str(_MAKE_PY))

from contrib_dev.discover import list_wired_contribs
from contrib_dev.monitor import print_hub_banner, print_list_monitor, print_recipe_monitor
from contrib_dev.patch_recipe import patch_apply
from contrib_dev.remove import remove_by_marker, remove_to_recipe_result
from contrib_dev.recipes import (
    cli_command,
    conformance,
    pkg_package,
    stdlib_extern,
    stdlib_pure,
    syntax_scaffold,
    test_example,
)
from contrib_dev.wizard import (
    load_json_config,
    run_cli_wizard,
    run_conformance_wizard,
    run_pkg_wizard,
    run_remove_wizard,
    run_stdlib_extern_wizard,
    run_stdlib_pure_wizard,
    run_syntax_wizard,
    run_test_example_wizard,
    spec_from_config,
)

SUBCOMMANDS = {"add", "remove", "list", "patch"}

RECIPES = {
    "1": ("stdlib-pure", "Stdlib Pure Function (Pattern A)", stdlib_pure.apply),
    "2": ("stdlib-extern", "Stdlib Extern + C (Pattern B)", stdlib_extern.apply),
    "3": ("builtin", "Built-in Method (.method)", None),
    "4": ("test-example", "Test + Example Pair", test_example.apply),
    "5": ("pkg", "NyraPkg Package", pkg_package.apply),
    "6": ("cli", "CLI Command / Flag", cli_command.apply),
    "7": ("conformance", "Conformance Test", conformance.apply),
    "8": ("syntax-scaffold", "Syntax / Keyword Scaffold", syntax_scaffold.apply),
}

WIZARDS = {
    "1": run_stdlib_pure_wizard,
    "2": run_stdlib_extern_wizard,
    "4": run_test_example_wizard,
    "5": run_pkg_wizard,
    "6": run_cli_wizard,
    "7": run_conformance_wizard,
    "8": run_syntax_wizard,
}

APPLY_BY_SLUG = {
    "stdlib-pure": stdlib_pure.apply,
    "stdlib-extern": stdlib_extern.apply,
    "test-example": test_example.apply,
    "pkg": pkg_package.apply,
    "cli": cli_command.apply,
    "conformance": conformance.apply,
    "syntax-scaffold": syntax_scaffold.apply,
}


def _ensure_add_subcommand(argv: list[str]) -> list[str]:
    if not argv:
        return ["add", "-i"]
    if argv[0] not in SUBCOMMANDS and not argv[0].startswith("-"):
        return ["add", *argv]
    if argv[0].startswith("-") and argv[0] not in ("-h", "--help"):
        return ["add", *argv]
    return argv


def regen_webdocs() -> None:
    """Regenerate webDocs so a freshly-added contribution shows up there.

    Runs make/lib/build-webdocs.sh, which embeds example sources, code tabs,
    the builtin gallery, the skill file, and the search index. None of these
    steps invoke `nyra`, so it is safe to run right after scaffolding (before
    the C logic is implemented) — captured example stdout is filled in later by
    the separate capture-builtin-outputs step. Failures are non-fatal: the
    scaffold already succeeded, so we only warn.
    """
    if os.environ.get("NYRA_CONTRIBUTE_SKIP_WEBDOCS"):
        print("\n(webDocs regen skipped: NYRA_CONTRIBUTE_SKIP_WEBDOCS set)")
        return
    script = _REPO_ROOT / "make" / "lib" / "build-webdocs.sh"
    if not script.exists():
        print(f"\n(webDocs regen skipped: {script} not found)", file=sys.stderr)
        return
    print("\n==> Updating webDocs so the new contribution is listed…")
    rc = subprocess.call(["bash", str(script)], cwd=str(_REPO_ROOT))
    if rc == 0:
        print("==> webDocs updated. (Run capture-builtin-outputs once the C logic")
        print("    builds, to fill in example stdout.)")
    else:
        print(
            "\nwarning: webDocs regen failed (exit "
            f"{rc}). Your scaffold is intact — re-run `make build-webdocs` "
            "manually after fixing the issue.",
            file=sys.stderr,
        )


def run_builtin_wizard() -> int:
    script = _MAKE_PY / "builtin-dev.py"
    print("\n── Built-in Method (.method) — option 3 ──")
    print("  WHY  → String/array methods need compiler + C wiring (10+ files).")
    print("  TOOL → Delegates to make add-builtin (same monitor style).")
    print("  YOU  → Implement C in stdlib/rt/; fix tests.")
    print("  NAME → Pick the Nyra method (e.g. to_snake_case); C gets str_to_snake_case.")
    print("         Do NOT also run Recipe 2 for the same feature.\n")
    return subprocess.call([sys.executable, str(script), "add", "-i"])


def pick_recipe(interactive: bool, recipe_arg: str | None) -> str:
    if recipe_arg:
        for key, (slug, _label, _fn) in RECIPES.items():
            if recipe_arg in (key, slug):
                return key
        raise SystemExit(f"Unknown recipe: {recipe_arg!r}. Use: make contribute -i")
    if not interactive:
        raise SystemExit("Pass -i for menu or --recipe <slug>")
    print_hub_banner()
    while True:
        choice = input("Select recipe [1-8]: ").strip()
        if choice in RECIPES:
            return choice
        print("  Enter a number from 1 to 8.")


def resolve_spec(choice: str, config: str | None, interactive: bool):
    if config:
        data = load_json_config(config)
        slug = RECIPES[choice][0]
        return spec_from_config(slug, data)
    if choice == "3":
        return None
    wizard = WIZARDS.get(choice)
    if not wizard:
        raise SystemExit(f"No wizard for choice {choice}")
    return wizard()


def cmd_add(args: argparse.Namespace) -> int:
    interactive = args.interactive or (not args.recipe and not args.config)
    choice = pick_recipe(interactive, args.recipe)
    want_webdocs = not getattr(args, "no_webdocs", False)
    if choice == "3":
        rc = run_builtin_wizard()
        if rc == 0 and want_webdocs:
            regen_webdocs()
        return rc
    _slug, _label, apply_fn = RECIPES[choice]
    spec = resolve_spec(choice, args.config, interactive)
    result = apply_fn(spec, force=args.force)
    print_recipe_monitor(result)
    changed = result.ok()
    already = any(getattr(p, "message", "") == "already present" for p in result.patches)
    if changed and want_webdocs:
        regen_webdocs()
    if changed or already:
        return 0
    return 1


def cmd_remove(args: argparse.Namespace) -> int:
    marker = args.marker
    if args.interactive or not marker:
        marker = run_remove_wizard()
    result = remove_to_recipe_result(remove_by_marker(marker))
    print_recipe_monitor(result)
    if result.ok() and not getattr(args, "no_webdocs", False):
        regen_webdocs()
    return 0 if result.ok() else 1


def cmd_list(args: argparse.Namespace) -> int:
    items = list_wired_contribs(recipe=args.recipe)
    print_list_monitor(items)
    return 0


def cmd_patch(args: argparse.Namespace) -> int:
    marker = args.marker
    want_webdocs = not getattr(args, "no_webdocs", False)
    if not marker and args.config:
        data = load_json_config(args.config)
        recipe = args.recipe or data.get("recipe", "stdlib-extern")
        if recipe == "stdlib-module":
            recipe = "stdlib-pure"
        spec = spec_from_config(recipe, data)
        marker = spec.marker
        apply_fn = APPLY_BY_SLUG.get(recipe)
        if not apply_fn:
            raise SystemExit(f"Cannot patch recipe {recipe!r}")
        result = patch_apply(marker=marker, apply_fn=apply_fn, spec=spec, force=True)
        print_recipe_monitor(result)
        if result.ok() and want_webdocs:
            regen_webdocs()
        return 0 if result.ok() else 1

    if not marker:
        marker = run_remove_wizard(title="Select scaffold to patch")
    wired = [w for w in list_wired_contribs() if w.marker == marker]
    if not wired:
        raise SystemExit(f"No wired scaffold for marker {marker!r}")
    recipe = wired[0].recipe
    if recipe == "stdlib":
        recipe = "stdlib-pure"
    apply_fn = APPLY_BY_SLUG.get(recipe)
    if not apply_fn:
        raise SystemExit(f"Patch not supported for recipe {recipe!r} — re-run add with --force")
    if args.config:
        spec = spec_from_config(recipe, load_json_config(args.config))
    else:
        print(f"\nRe-scaffold {marker} — run wizard for {recipe}")
        choice = next(k for k, (s, *_r) in RECIPES.items() if s == recipe)
        spec = resolve_spec(choice, None, True)
    result = patch_apply(marker=marker, apply_fn=apply_fn, spec=spec, force=True)
    print_recipe_monitor(result)
    if result.ok() and want_webdocs:
        regen_webdocs()
    return 0 if result.ok() else 1


def main() -> int:
    argv = _ensure_add_subcommand(sys.argv[1:])
    parser = argparse.ArgumentParser(description="Nyra contributor scaffolding hub")
    sub = parser.add_subparsers(dest="command")

    add_p = sub.add_parser("add", help="Add scaffold (default)")
    add_p.add_argument("-i", "--interactive", action="store_true")
    add_p.add_argument("--recipe", help="Recipe slug or number (1-8)")
    add_p.add_argument("--config", help="JSON spec")
    add_p.add_argument("--force", action="store_true")
    add_p.add_argument(
        "--no-webdocs",
        action="store_true",
        help="Skip regenerating webDocs after the scaffold (for CI/automation)",
    )
    add_p.set_defaults(func=cmd_add)

    rem_p = sub.add_parser("remove", help="Remove scaffold by marker")
    rem_p.add_argument("-i", "--interactive", action="store_true")
    rem_p.add_argument("--marker", help="contrib-dev marker (e.g. test_example:foo)")
    rem_p.add_argument(
        "--no-webdocs",
        action="store_true",
        help="Skip regenerating webDocs after removal (for CI/automation)",
    )
    rem_p.set_defaults(func=cmd_remove)

    list_p = sub.add_parser("list", help="List wired scaffolds")
    list_p.add_argument("--recipe", help="Filter by recipe slug")
    list_p.set_defaults(func=cmd_list)

    patch_p = sub.add_parser("patch", help="Remove + re-add scaffold")
    patch_p.add_argument("--marker", help="Existing marker to patch")
    patch_p.add_argument("--recipe", help="Recipe slug when using --config")
    patch_p.add_argument("--config", help="Updated JSON spec")
    patch_p.add_argument(
        "--no-webdocs",
        action="store_true",
        help="Skip regenerating webDocs after patch (recommended; webDocs is slow)",
    )
    patch_p.set_defaults(func=cmd_patch)

    args = parser.parse_args(argv)
    if not hasattr(args, "func"):
        args = parser.parse_args(["add", "-i"])
    return args.func(args)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print(
            "\n\nCancelled — no files were created or changed.\n"
            "  Files are written only after you answer all questions and confirm\n"
            "  “Apply scaffold now? (Y/n)” with Y.\n"
            "  Re-run: make contribute",
            file=sys.stderr,
        )
        raise SystemExit(130)
