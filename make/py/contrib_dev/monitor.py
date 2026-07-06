"""Terminal monitor reports for contributor recipes."""
from __future__ import annotations

from pathlib import Path

from .paths import ROOT
from .spec import RecipeResult


def _rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def print_recipe_monitor(result: RecipeResult) -> None:
    changed = [p for p in result.patches if getattr(p, "changed", False)]

    print("\n" + "═" * 62)
    print(f"  CONTRIBUTE MONITOR — {result.title.upper()}")
    print("═" * 62)
    print(f"  Recipe : {result.recipe}")
    print(f"  Marker : [contrib-dev:{result.marker}]")
    print("─" * 62)

    print("\n✅ DONE (tool applied automatically):")
    if changed:
        for p in changed:
            print(f"   • {_rel(p.path)} — {p.message}")
    else:
        print("   • Nothing changed — scaffold may already exist.")

    print("\n📋 YOUR TASKS (you must do these):")
    if result.user_tasks:
        for i, task in enumerate(result.user_tasks, 1):
            print(f"   {i}. {task}")
    else:
        print("   1. Review generated files and implement TODO sections")

    print("\n▶ NEXT STEPS:")
    print("   1. Implement TODO logic in generated stubs")
    print("   2. make install-dev   # when compiler/stdlib wiring changed")
    print("   3. nyra test tests/nyra/…   or   make test-preflight")
    print("   4. make test-all   # before opening PR")

    if result.usage_lines:
        print("\n💡 USAGE:")
        for line in result.usage_lines:
            print(f"   {line}")

    if result.warnings:
        print("\n⚠ NOTES:")
        for w in result.warnings:
            print(f"   • {w}")
    print()


def print_hub_banner() -> None:
    print()
    print("┌─────────────────────────────────────────────┐")
    print("│             make contribute                 │")
    print("├─────────────────────────────────────────────┤")
    print("│ 1. Stdlib Pure Function (Pattern A)         │")
    print("│ 2. Stdlib Extern + C (Pattern B)            │")
    print("│ 3. Built-in Method (.method)                │")
    print("│ 4. Test + Example Pair                      │")
    print("│ 5. NyraPkg Package                          │")
    print("│ 6. CLI Command / Flag                       │")
    print("│ 7. Conformance Test                         │")
    print("│ 8. Syntax / Keyword Scaffold                │")
    print("└─────────────────────────────────────────────┘")
    print()


def print_list_monitor(items) -> None:
    print("\n" + "═" * 62)
    print("  CONTRIBUTE — WIRED SCAFFOLDS")
    print("═" * 62)
    if not items:
        print("  (none — run `make contribute` to add one)")
        print()
        return
    for item in items:
        print(f"  • {item.label}")
        for path in item.paths:
            print(f"      {_rel(path)}")
    print("\n  Remove: make contribute-remove ARGS='--marker <marker>'")
    print("  Patch:  make contribute-patch ARGS='--marker <marker> --config …'")
    print()
