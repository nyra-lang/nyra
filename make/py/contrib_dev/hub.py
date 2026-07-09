"""Unified contributor hub — single entry point for all contribution automation.

`make contribute` (no args) launches this menu. Sub-commands delegate internally
to builtin-dev, batch_add, gen_batch*, and recipe wizards — contributors never
need to run make add-builtin or make batch-add-builtin directly.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

_MAKE_PY = Path(__file__).resolve().parents[1]
_REPO_ROOT = _MAKE_PY.parent.parent
if str(_MAKE_PY) not in sys.path:
    sys.path.insert(0, str(_MAKE_PY))

from contrib_dev.monitor import print_list_monitor
from contrib_dev.terminal_style import ACCENT, BORDER, MUTED, RESET, TITLE, box_bottom, box_top, hint_line, menu_item, use_color
from contrib_dev.tiger_banner import play_tiger_intro
from contrib_dev.wizard import prompt_choice, prompt_yes_no

BUILTIN_PY = _MAKE_PY / "builtin-dev.py"
BATCH_ADD_PY = _MAKE_PY / "builtin_dev" / "batch_add.py"
BUILTIN_EXAMPLES = _MAKE_PY / "builtin_dev" / "examples"


def _col(text: str, code: str, color: bool) -> str:
    return f"{code}{text}{RESET}" if color else text


def _print_section(title: str, *, why: str, tool: str, you: str) -> None:
    color = use_color()
    thin = _c("─" * 62, BORDER, color)
    print("\n" + thin)
    print("  " + _c(title.upper(), TITLE, color))
    print(thin)
    print("  " + _c("WHY ", MUTED, color) + _c(" → ", ACCENT, color) + why)
    print("  " + _c("TOOL", MUTED, color) + _c(" → ", ACCENT, color) + tool)
    print("  " + _c("YOU ", MUTED, color) + _c(" → ", ACCENT, color) + you)
    print(thin)


def _c(text: str, code: str, color: bool) -> str:
    return _col(text, code, color)


def print_main_hub_banner() -> None:
    play_tiger_intro()
    color = use_color()
    if color:
        print(f"  {TITLE}make contribute{RESET} {MUTED}— Nyra contributor hub (single entry point){RESET}")
        print(
            f"  {MUTED}Every automation runs from here — {ACCENT}TOOL{RESET}{MUTED} wires, "
            f"{ACCENT}YOU{RESET}{MUTED} implement{RESET}"
        )
    else:
        print("  make contribute — Nyra contributor hub (single entry point)")
        print("  Every automation runs from here — TOOL wires, YOU implement")
    print()

    w = 48
    items = (
        ("1", "Add", "scaffold", "stdlib, builtin, test, pkg, CLI, syntax…"),
        ("2", "Remove", "scaffold", "undo contrib-dev or builtin-dev wiring"),
        ("3", "List", "wired", "show all [contrib-dev:…] and [builtin-dev:…]"),
        ("4", "Patch", "update", "re-scaffold contrib or re-wire builtin"),
        ("5", "Batch", "catalog", "gen-batchN + batch-add (many APIs at once)"),
        ("6", "Verify", "next steps", "install-dev, tests, nyra test"),
        ("0", "Exit", "", "leave without changes"),
    )
    print(box_top(width=w, color=color))
    for num, title, tag, hint in items:
        t_row, h_row = menu_item(num, title, tag, hint, width=w, color=color)
        print(t_row)
        print(h_row)
    print(box_bottom(width=w, color=color))
    print()
    print(hint_line("Pick 1–6. Each submenu shows WHY / TOOL / YOU at every step.", color=color))
    print(hint_line("Scripts/CI: make contribute ARGS='add --recipe … --config …'", color=color))
    print()


def _run_python(script: Path, *args: str) -> int:
    cmd = [sys.executable, str(script), *args]
    print("\n>>>", " ".join(cmd), flush=True)
    return subprocess.call(cmd, cwd=str(_REPO_ROOT))


def _run_make(target: str, *, env: dict | None = None) -> int:
    import os

    cmd = ["make", target]
    print("\n>>>", " ".join(cmd), flush=True)
    merged = {**os.environ, **(env or {})}
    return subprocess.call(cmd, cwd=str(_REPO_ROOT), env=merged)


def _list_batch_folders() -> list[str]:
    if not BUILTIN_EXAMPLES.is_dir():
        return []
    return sorted(
        p.name for p in BUILTIN_EXAMPLES.iterdir() if p.is_dir() and p.name.startswith("batch")
    )


def _gen_batch_script(batch: str) -> Path | None:
    """Return gen_batchN.py for batchN, else None."""
    if batch == "batch":
        return None
    suffix = batch.replace("batch", "", 1)
    if not suffix.isdigit():
        return None
    path = _MAKE_PY / "contrib_dev" / f"gen_{batch}.py"
    return path if path.is_file() else None


def hub_add(regen_webdocs: bool) -> int:
    import argparse
    import os

    _print_section(
        "Add scaffold",
        why="Start a new contribution with marked stubs and wiring.",
        tool="Creates files tagged [contrib-dev:…] or [builtin-dev:…]; updates runtime_map when needed.",
        you="Implement Nyra/C logic, fix test assertions, run Verify (menu 6).",
    )
    from contribute import cmd_add

    os.environ["NYRA_CONTRIBUTE_FROM_HUB"] = "1"
    try:
        ns = argparse.Namespace(
            interactive=True,
            recipe=None,
            config=None,
            force=False,
            no_webdocs=not regen_webdocs,
        )
        return cmd_add(ns)
    finally:
        os.environ.pop("NYRA_CONTRIBUTE_FROM_HUB", None)


def hub_remove(regen_webdocs: bool) -> int:
    _print_section(
        "Remove scaffold",
        why="Undo wiring you no longer want — safe when marked by the hub.",
        tool="Deletes marked blocks/files; cleans runtime_map / ABI entries when applicable.",
        you="Search for leftover references; run Verify (menu 6).",
    )
    kind = prompt_choice(
        "What do you want to remove?",
        {
            "1": "Contrib scaffold ([contrib-dev:…] — stdlib pure/extern, test, pkg, …)",
            "2": "Built-in method ([builtin-dev:…] — .method on string/array/…)",
            "0": "Back to main menu",
        },
    )
    if kind == "0":
        return 0
    if kind == "1":
        import argparse

        from contribute import cmd_remove

        ns = argparse.Namespace(interactive=True, marker=None, no_webdocs=not regen_webdocs)
        return cmd_remove(ns)
    return _run_python(BUILTIN_PY, "remove", "-i")


def hub_list() -> int:
    from builtin_dev.discover import list_wired_builtins
    from contrib_dev.discover import list_wired_contribs

    _print_section(
        "List wired scaffolds",
        why="See what the hub already wired before adding duplicates.",
        tool="Scans repo for [contrib-dev:…] and [builtin-dev:…] markers.",
        you="Pick Remove (2) or Patch (4) if you need to change an existing scaffold.",
    )
    items = list_wired_contribs()
    print_list_monitor(items)
    builtins = list_wired_builtins()
    print("\n" + "═" * 62)
    print("  BUILT-IN METHODS ([builtin-dev:…])")
    print("═" * 62)
    if not builtins:
        print("  (none — use Add → Built-in Method to wire one)")
    else:
        for b in builtins:
            print(f"  • {b.label}  [{b.marker}]")
    print("\n  Patch builtin: main menu → 4 → Built-in method")
    print("  Remove builtin: main menu → 2 → Built-in method")
    print()
    return 0


def hub_patch(regen_webdocs: bool) -> int:
    _print_section(
        "Patch / update scaffold",
        why="Fix wiring (args, return type, paths) without hand-editing 10+ files.",
        tool="Removes old marker block and re-applies an updated spec (C body preserved for builtins).",
        you="Re-run tests; implement any new TODO stubs.",
    )
    kind = prompt_choice(
        "What do you want to patch?",
        {
            "1": "Contrib scaffold ([contrib-dev:…])",
            "2": "Built-in method ([builtin-dev:…])",
            "0": "Back to main menu",
        },
    )
    if kind == "0":
        return 0
    if kind == "1":
        import argparse

        from contribute import cmd_patch

        ns = argparse.Namespace(
            marker=None,
            recipe=None,
            config=None,
            no_webdocs=not regen_webdocs,
        )
        return cmd_patch(ns)
    return _run_python(BUILTIN_PY, "patch", "-i")


def hub_batch() -> int:
    batches = _list_batch_folders()
    if not batches:
        print("\nNo batch* folders under make/py/builtin_dev/examples/")
        return 1

    _print_section(
        "Batch add APIs",
        why="Land many stdlib/builtin APIs from a Python catalog (gap-fill batches).",
        tool="gen-batchN writes JSON configs; batch-add runs builtin-dev + contribute recipes.",
        you="Implement C bodies in stdlib/rt/; fix tests; run make install-dev.",
    )

    print("\n→ Available batch folders:")
    for i, name in enumerate(batches, 1):
        gen = _gen_batch_script(name)
        gen_note = f" (catalog: gen_{name}.py)" if gen else " (no catalog — JSON only)"
        print(f"    {i}. {name}{gen_note}")

    while True:
        raw = input("\n→ Batch number or name [batch6]: ").strip() or "batch6"
        if raw.isdigit():
            idx = int(raw) - 1
            if 0 <= idx < len(batches):
                batch = batches[idx]
                break
        elif raw in batches:
            batch = raw
            break
        print(f"  Pick 1–{len(batches)} or type folder name.")

    action = prompt_choice(
        "Batch action",
        {
            "1": "Generate JSON configs from catalog (gen-batchN only)",
            "2": "Apply batch (batch-add-builtin — scaffold all JSON in folder)",
            "3": "Full pipeline: generate → apply → generate (consolidate)",
            "0": "Back to main menu",
        },
    )
    if action == "0":
        return 0

    only_default = "all"
    if action in ("2", "3"):
        only_raw = input(
            "\n→ Filter categories (comma-separated, or Enter for all):\n"
            "   string,math,vec,map,encoding,strconv,format,sync,fs,pure\n"
            "   [all]: "
        ).strip()
        only_default = only_raw or "all"

    rc = 0
    gen_script = _gen_batch_script(batch)

    if action in ("1", "3"):
        if gen_script is None:
            print(f"\n  No gen_{batch}.py catalog — skip generate step.")
        else:
            print("\n── TOOL: generate JSON configs from catalog ──")
            rc = _run_python(gen_script)
            if rc != 0:
                return rc

    if action in ("2", "3"):
        print("\n── TOOL: batch-add-builtin (delegates to builtin-dev + contribute) ──")
        args = ["--batch", batch, "--only", only_default, "--no-webdocs"]
        if prompt_yes_no("Pass --force (overwrite existing stubs)?", default=False):
            args.append("--force")
        rc = _run_python(BATCH_ADD_PY, *args)
        if rc != 0:
            return rc

        if action == "3" and gen_script is not None:
            print("\n── TOOL: consolidate (second gen-batch pass) ──")
            rc = _run_python(gen_script)

    if rc == 0:
        print("\n── YOU DO ──")
        print("  1. Implement C in stdlib/rt/ (search [contrib-dev:…] / [builtin-dev:…])")
        print("  2. make install-dev")
        print("  3. nyra test tests/nyra/<feature>_test.ny")
        print("  4. Main menu → 6 Verify → 2 Post-scaffold CI gates")
        if prompt_yes_no("Run post-batch CI gates now (abi_manifest)?", default=True):
            from contrib_dev.validate import check_abi_manifest

            rc = check_abi_manifest()
    return rc


def hub_verify() -> int:
    _print_section(
        "Verify & next steps",
        why="Confirm your implementation builds and passes tests before opening a PR.",
        tool="Runs standard make / nyra commands from the repo root.",
        you="Fix failures in the files the monitor listed (TOOL vs YOU).",
    )
    choice = prompt_choice(
        "What should the hub run?",
        {
            "1": "make install-dev        (reinstall CLI + sync stdlib)",
            "2": "Post-scaffold CI gates (abi_manifest + examples)",
            "3": "make test-contrib-py    (Python hub smoke tests)",
            "4": "make test-preflight     (fast CI smoke ~1–3 min)",
            "5": "make test-nyra-lang     (all tests/nyra/)",
            "6": "make test-optional-types (all examples/builtins check)",
            "7": "nyra test <path>        (single test file)",
            "8": "make build-webdocs      (refresh webDocs after user-facing change)",
            "0": "Back to main menu",
        },
    )
    if choice == "0":
        return 0
    if choice == "1":
        return _run_make("install-dev")
    if choice == "2":
        from contrib_dev.validate import hub_run_gates_menu

        return hub_run_gates_menu()
    if choice == "3":
        return _run_make("test-contrib-py")
    if choice == "4":
        return _run_make("test-preflight")
    if choice == "5":
        return _run_make("test-nyra-lang")
    if choice == "6":
        return _run_make("test-optional-types")
    if choice == "8":
        return _run_make("build-webdocs")
    if choice == "7":
        path = input("\n→ Test file path (e.g. tests/nyra/foo_test.ny): ").strip()
        if not path:
            print("  (cancelled — no path)")
            return 1
        nyra = _REPO_ROOT / "target" / "debug" / "nyra"
        if nyra.is_file():
            return subprocess.call([str(nyra), "test", path], cwd=str(_REPO_ROOT))
        return subprocess.call(["nyra", "test", path], cwd=str(_REPO_ROOT))
    return 1


def run_main_hub(*, regen_webdocs: bool = True) -> int:
    handlers = {
        "1": lambda: hub_add(regen_webdocs),
        "2": lambda: hub_remove(regen_webdocs),
        "3": hub_list,
        "4": lambda: hub_patch(regen_webdocs),
        "5": hub_batch,
        "6": hub_verify,
    }
    while True:
        print_main_hub_banner()
        choice = input("→ Main menu [1-6, 0=exit]: ").strip()
        if choice in ("0", "q", "quit", "exit"):
            print("\n  Hub closed — no changes from this menu selection.\n")
            return 0
        handler = handlers.get(choice)
        if handler is None:
            print("  Enter 1–6 or 0 to exit.")
            continue
        rc = handler()
        if rc != 0:
            return rc
        if not prompt_yes_no("\nReturn to main hub menu?", default=True):
            return 0
