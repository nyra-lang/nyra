"""Monitor-style terminal reports — what the tool did and what you should do next."""
from __future__ import annotations

from pathlib import Path

from .add import ActionResult
from .method_catalog import usage_snippets
from .remove import RemoveResult
from .spec import BuiltinSpec
from .wire_patch import PatchResult

ROOT = Path(__file__).resolve().parents[3]


def _rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def print_add_monitor(result: ActionResult) -> None:
    spec = result.spec
    changed = [p for p in result.patches if p.changed]

    print("\n" + "═" * 62)
    print("  BUILTIN MONITOR — ADD")
    print("═" * 62)
    print(f"  Nyra method (programmer code): .{spec.method}()")
    print(f"  C symbol (stdlib/rt only)    : {spec.c_name}")
    print(f"  Runtime: stdlib/rt/{spec.rt_module}")
    print("─" * 62)

    print("\n✅ DONE (tool applied automatically):")
    if changed:
        for p in changed:
            print(f"   • {_rel(p.path)} — {p.message}")
    else:
        print("   • Nothing changed — builtin may already be wired.")

    print("\n📋 YOUR TASKS (you must do these):")
    if result.user_tasks:
        for i, task in enumerate(result.user_tasks, 1):
            print(f"   {i}. {task}")
    else:
        print("   (none — review generated stubs if any)")

    print("\n▶ NEXT STEPS:")
    print("   1. Open the C stub tagged [builtin-dev:…] and implement logic")
    print("   2. Fix test expected values (search for TODO in tests/nyra/)")
    print("   3. Install fresh CLI (cargo build alone is NOT enough for `nyra` in PATH):")
    print("        cargo install --path cli --force")
    print("      — or use the repo binary directly:")
    print("        ./target/debug/nyra test tests/nyra/string_<method>_test.ny")
    print(f"   4. Run:  nyra run examples/builtins/strings/{spec.method}.ny")
    print("   5. If run works but test fails: clear stale cache, then reinstall CLI:")
    print("        rm -rf examples/builtins/strings/target tests/nyra/target")
    print("   6. Run:  make test-nyra-lang   (or make test-all before PR)")

    _print_usage_examples(spec)

    if result.warnings:
        print("\n⚠ NOTES:")
        for w in result.warnings:
            print(f"   • {w}")
    print()


def print_remove_monitor(result: RemoveResult) -> None:
    spec = result.spec
    changed = [p for p in result.patches if p.changed]

    print("\n" + "═" * 62)
    print("  BUILTIN MONITOR — REMOVE")
    print("═" * 62)
    print(f"  Method : {spec.receiver.value}.{spec.method}")
    print(f"  C sym  : {spec.c_name}")
    print("─" * 62)

    print("\n✅ DONE (tool removed automatically):")
    if changed:
        for p in changed:
            print(f"   • {_rel(p.path)} — {p.message}")
    else:
        print("   • Nothing found — builtin may not exist or was hand-wired only.")

    print("\n📋 YOUR TASKS:")
    print("   1. Search repo for leftover references to the method / C symbol")
    print("   2. Run:  cargo test --workspace")
    print("   3. Run:  make test-nyra-lang")

    if result.warnings:
        print("\n⚠ NOTES:")
        for w in result.warnings:
            print(f"   • {w}")

    print("\n▶ REBUILD (when ready):")
    print(f"   make add-builtin ARGS='--config make/py/builtin_dev/examples/{spec.method}.json'")
    print("   — or—")
    print(f"   make add-builtin ARGS='-i'   # interactive wizard with choices")
    print()


def print_patch_monitor(result: PatchResult) -> None:
    spec = result.new_spec
    changed = [p for p in result.add.patches if p.changed]

    print("\n" + "═" * 62)
    print("  BUILTIN MONITOR — PATCH")
    print("═" * 62)
    print(f"  Method : {result.old_spec.receiver.value}.{result.old_spec.method}", end="")
    if result.old_spec.method != spec.method:
        print(f"  →  {spec.receiver.value}.{spec.method}")
    else:
        print(f"  (updated wiring)")
    print(f"  C sym  : {spec.c_name}")
    if result.preserved_c:
        print("  C code : preserved from previous implementation ✓")
    print("─" * 62)

    print("\n✅ DONE:")
    for p in changed:
        print(f"   • {_rel(p.path)} — {p.message}")

    print("\n📋 YOUR TASKS:")
    for i, task in enumerate(result.add.user_tasks, 1):
        print(f"   {i}. {task}")
    if not result.add.user_tasks:
        print("   1. Run tests and verify behavior unchanged")

    _print_usage_examples(spec)

    if result.warnings:
        print("\n⚠ NOTES:")
        for w in result.warnings:
            print(f"   • {w}")
    print()


def _print_usage_examples(spec: BuiltinSpec) -> None:
    snippets = usage_snippets(spec)
    if not snippets:
        return
    print("\n💡 USAGE (after C implementation is done):")
    print(f"   examples/builtins/strings/{spec.method}.ny:")
    for line in snippets:
        print(f"   {line}")


def print_spec_preview(spec: BuiltinSpec, *, action: str) -> None:
    import sys
    from pathlib import Path

    _make_py = Path(__file__).resolve().parents[1]
    if str(_make_py) not in sys.path:
        sys.path.insert(0, str(_make_py))
    from naming_guide import format_builtin_name_summary

    print("\n" + "─" * 62)
    print(f"  PREVIEW — will {action}:")
    print(f"    receiver : {spec.receiver.value}")
    print(f"    method   : {spec.method}  (Nyra — what programmers type)")
    print(f"    Nyra API : .{spec.method}({', '.join(a.name + ': ' + a.nyra_type.value for a in spec.args) or '—'})")
    print(f"    args     : {[f'{a.name}:{a.nyra_type.value}' for a in spec.args] or '(none)'}")
    print(f"    returns  : {spec.returns.value}")
    print(f"    c_name   : {spec.c_name}  (C only — stdlib/rt/{spec.rt_module})")
    print(f"    borrows  : {spec.borrows_receiver}")
    if spec.free_fn_alias and spec.receiver.value == "string":
        print(f"    free fn  : {spec.method}(s, …) in builtins_string.ny")
    print("  Naming:")
    for line in format_builtin_name_summary(spec.method, spec.c_name):
        print(f"    • {line}")
    print("─" * 62)
