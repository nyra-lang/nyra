"""Terminal monitor reports for contributor recipes."""
from __future__ import annotations

from pathlib import Path

from .paths import ROOT
from .spec import RecipeResult
from .terminal_style import (
    ACCENT,
    BORDER,
    MUTED,
    NUM,
    RESET,
    TEXT,
    TITLE,
    box_bottom,
    box_top,
    hint_line,
    menu_item,
    use_color,
)
from .wizard_guide import GUIDES, monitor_sections


def _rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def _col(text: str, code: str, color: bool) -> str:
    return f"{code}{text}{RESET}" if color else text


def print_recipe_monitor(result: RecipeResult) -> None:
    changed = [p for p in result.patches if getattr(p, "changed", False)]
    guide = GUIDES.get(result.recipe)
    color = use_color()
    rule = _col("═" * 62, BORDER, color)
    thin = _col("─" * 62, BORDER, color)

    print("\n" + rule)
    print("  " + _col(f"CONTRIBUTE MONITOR — {result.title.upper()}", TITLE, color))
    print(rule)
    print("  " + _col("Recipe :", MUTED, color) + f" {_col(result.recipe, TEXT, color)}")
    print("  " + _col("Marker :", MUTED, color) + f" {_col(f'[contrib-dev:{result.marker}]', TEXT, color)}")
    if guide:
        print("  " + _col("Purpose:", MUTED, color) + f" {guide.when}")
    print(thin)

    print("\n" + _col("✅ TOOL DID", ACCENT, color) + _col(" (automatic — you do NOT edit these stubs to wire files):", MUTED, color))
    if changed:
        for p in changed:
            print("   • " + _col(_rel(p.path), TEXT, color))
            print("     └─ " + _col(p.message, MUTED, color))
    else:
        print("   • Nothing changed — scaffold may already exist (safe to skip).")

    if guide:
        print("\n" + _col("📁 TOOL touched these areas (summary):", ACCENT, color))
        for line in guide.tool_files.strip().splitlines():
            print(f"   {line}")

    print("\n" + _col("📋 YOU DO (your implementation work):", ACCENT, color))
    if result.user_tasks:
        for i, task in enumerate(result.user_tasks, 1):
            print("   " + _col(f"{i}.", NUM, color) + f" {task}")
    else:
        print("   " + _col("1.", NUM, color) + " Open each file above — search for TODO or [contrib-dev:…]")
        print("   " + _col("2.", NUM, color) + " Replace stubs with real logic and assertions")

    if guide:
        print("\n" + _col("📂 WHERE you edit (open these paths):", ACCENT, color))
        for line in guide.you_files.strip().splitlines():
            print(f"   {line}")

    print("\n" + _col("▶ VERIFY (run in order):", ACCENT, color))
    if guide:
        print("   " + _col("1.", NUM, color) + f" {_col(guide.verify, TEXT, color)}")
    print("   " + _col("2.", NUM, color) + " make contribute → 6 Verify → 2 Post-scaffold CI gates")
    print("   " + _col("3.", NUM, color) + " make install-dev     # if compiler/ or runtime_map changed")
    print("   " + _col("4.", NUM, color) + " make test-preflight  # fast gate before PR")
    print("   " + _col("5.", NUM, color) + " make test-all        # full CI before merge")

    if result.usage_lines:
        print("\n" + _col("💡 USAGE (after implementation):", ACCENT, color))
        for line in result.usage_lines:
            print("   " + _col(line, TEXT, color))

    if guide:
        why, _tool, _you = monitor_sections(guide)
        print("\n" + _col("❓ WHY this split?", ACCENT, color))
        for line in why:
            print(f"   • {line}")
        print("   • TOOL = wiring + stubs so you never miss a file")
        print("   • YOU  = semantics, tests, and compiler logic the tool cannot guess")

    if result.warnings:
        print("\n" + _col("⚠ NOTES:", NUM, color))
        for w in result.warnings:
            print(f"   • {w}")

    print("\n" + _col("🔄 UNDO:", MUTED, color) + " make contribute  →  2 Remove  →  paste marker " + result.marker)
    print()


def print_recipe_menu() -> None:
    """Recipe picker (1–8) — no tiger intro; used from hub Add or `contribute add -i`."""
    color = use_color()

    if color:
        print(f"  {TITLE}Add scaffold{RESET} {MUTED}— pick a recipe{RESET}")
        print(f"  {MUTED}TOOL wires stubs · YOU implement logic · WHY shown each step{RESET}")
    else:
        print("  Add scaffold — pick a recipe")
        print("  TOOL wires stubs · YOU implement logic · WHY shown each step")
    print()

    w = 45
    items = (
        ("1", "Stdlib Pure Function", "(Pattern A)", "Nyra fn in stdlib — no new C"),
        ("2", "Stdlib Extern + C", "(Pattern B)", "extern fn + rt/*.c + runtime_map"),
        ("3", "Built-in Method", "(.method)", "compiler + C wiring (hub runs internally)"),
        ("4", "Test + Example Pair", "", "tests/nyra/* + examples/* (typed pair)"),
        ("5", "NyraPkg Package", "", "examples/packages/<name>/"),
        ("6", "CLI Command / Flag", "", "scaffold → manual wire in cli/"),
        ("7", "Conformance Test", "", "pass/ or fail/ language contract"),
        ("8", "Syntax / Keyword Scaffold", "", "checklist — no auto lexer/parser"),
        ("0", "Back", "", "return to main hub menu"),
    )

    print(box_top(width=w, color=color))
    for num, title, tag, hint in items:
        t_row, h_row = menu_item(num, title, tag, hint, width=w, color=color)
        print(t_row)
        print(h_row)
    print(box_bottom(width=w, color=color))
    print()
    print(hint_line("Type 1–8, then answer each question (WHY / TOOL / YOU shown).", color=color))
    print(hint_line("Naming: Recipe 2 = str_* (C/extern). Recipe 3 = .method name (Nyra code).", color=color))
    print(hint_line("Preview + confirm before any file is written.", color=color))
    print()


def print_hub_banner() -> None:
    """Legacy alias — recipe menu only (main hub uses contrib_dev.hub.print_main_hub_banner)."""
    print_recipe_menu()


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
    print("\n  Remove: make contribute  →  2 Remove")
    print("  Patch:  make contribute  →  4 Patch")
    print("  List:   make contribute  →  3 List")
    print()
