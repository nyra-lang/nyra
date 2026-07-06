"""Terminal monitor reports for contributor recipes."""
from __future__ import annotations

from pathlib import Path

from .paths import ROOT
from .spec import RecipeResult
from .terminal_style import ACCENT, MUTED, RESET, TITLE, box_bottom, box_top, hint_line, menu_item, use_color
from .tiger_banner import play_tiger_intro
from .wizard_guide import GUIDES, monitor_sections


def _rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def print_recipe_monitor(result: RecipeResult) -> None:
    changed = [p for p in result.patches if getattr(p, "changed", False)]
    guide = GUIDES.get(result.recipe)

    print("\n" + "═" * 62)
    print(f"  CONTRIBUTE MONITOR — {result.title.upper()}")
    print("═" * 62)
    print(f"  Recipe : {result.recipe}")
    print(f"  Marker : [contrib-dev:{result.marker}]")
    if guide:
        print(f"  Purpose: {guide.when}")
    print("─" * 62)

    print("\n✅ TOOL DID (automatic — you do NOT edit these stubs to wire files):")
    if changed:
        for p in changed:
            print(f"   • {_rel(p.path)}")
            print(f"     └─ {p.message}")
    else:
        print("   • Nothing changed — scaffold may already exist (safe to skip).")

    if guide:
        print("\n📁 TOOL touched these areas (summary):")
        for line in guide.tool_files.strip().splitlines():
            print(f"   {line}")

    print("\n📋 YOU DO (your implementation work):")
    if result.user_tasks:
        for i, task in enumerate(result.user_tasks, 1):
            print(f"   {i}. {task}")
    else:
        print("   1. Open each file above — search for TODO or [contrib-dev:…]")
        print("   2. Replace stubs with real logic and assertions")

    if guide:
        print("\n📂 WHERE you edit (open these paths):")
        for line in guide.you_files.strip().splitlines():
            print(f"   {line}")

    print("\n▶ VERIFY (run in order):")
    if guide:
        print(f"   1. {guide.verify}")
    print("   2. make install-dev     # if compiler/ or runtime_map changed")
    print("   3. make test-preflight  # fast gate before PR")
    print("   4. make test-all        # full CI before merge")

    if result.usage_lines:
        print("\n💡 USAGE (after implementation):")
        for line in result.usage_lines:
            print(f"   {line}")

    if guide:
        why, _tool, _you = monitor_sections(guide)
        print("\n❓ WHY this split?")
        for line in why:
            print(f"   • {line}")
        print("   • TOOL = wiring + stubs so you never miss a file")
        print("   • YOU  = semantics, tests, and compiler logic the tool cannot guess")

    if result.warnings:
        print("\n⚠ NOTES:")
        for w in result.warnings:
            print(f"   • {w}")

    print("\n🔄 UNDO: make contribute-remove ARGS='--marker " + result.marker + "'")
    print()


def print_hub_banner() -> None:
    play_tiger_intro()
    color = use_color()

    if color:
        print(f"  {TITLE}make contribute{RESET} {MUTED}— Nyra contributor hub{RESET}")
        print(f"  {MUTED}Step-by-step monitor — {ACCENT}TOOL{RESET}{MUTED} wires, {ACCENT}YOU{RESET}{MUTED} code{RESET}")
    else:
        print("  make contribute — Nyra contributor hub")
        print("  Step-by-step monitor — TOOL wires, YOU code")
    print()

    w = 45
    items = (
        ("1", "Stdlib Pure Function", "(Pattern A)", "Nyra fn in stdlib — no new C"),
        ("2", "Stdlib Extern + C", "(Pattern B)", "extern fn + rt/*.c + runtime_map"),
        ("3", "Built-in Method", "(.method)", "→ make add-builtin wizard"),
        ("4", "Test + Example Pair", "", "tests/nyra/* + examples/* (typed pair)"),
        ("5", "NyraPkg Package", "", "examples/packages/<name>/"),
        ("6", "CLI Command / Flag", "", "scaffold → manual wire in cli/"),
        ("7", "Conformance Test", "", "pass/ or fail/ language contract"),
        ("8", "Syntax / Keyword Scaffold", "", "checklist — no auto lexer/parser"),
    )

    print(box_top(width=w, color=color))
    for num, title, tag, hint in items:
        t_row, h_row = menu_item(num, title, tag, hint, width=w, color=color)
        print(t_row)
        print(h_row)
    print(box_bottom(width=w, color=color))
    print()
    print(hint_line("Type 1–8, then answer each question (WHY / TOOL / YOU shown).", color=color))
    print(hint_line("Preview + confirm before any file is written.", color=color))
    print(hint_line("Docs: CONTRIBUTING.md § Contributor hub guide", color=color))
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
    print("\n  Remove: make contribute-remove ARGS='-i'")
    print("  Patch:  make contribute-patch ARGS='--marker <m> --config …'")
    print()
