"""Post-scaffold CI safety gates for make contribute.

Catches the failures that most often break tier-1 CI after batch/stdlib wiring:
  - abi-manifest duplicates / missing C symbols (cargo test abi_manifest)
  - optional-types check on generated examples (nyra check)
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MAKE_PY = ROOT / "make" / "py"


def _nyra_bin() -> list[str]:
    debug = ROOT / "target" / "debug" / "nyra"
    if debug.is_file():
        return [str(debug)]
    return ["nyra"]


def _run(cmd: list[str], *, cwd: Path | None = None) -> int:
    print("\n>>>", " ".join(cmd), flush=True)
    return subprocess.call(cmd, cwd=str(cwd or ROOT))


def check_example_paths(paths: list[Path]) -> int:
    """Run `nyra check` on example files (optional-types gate subset)."""
    nyra = _nyra_bin()
    rc = 0
    for path in paths:
        if not path.is_file():
            print(f"  skip (missing): {path.relative_to(ROOT)}")
            continue
        print(f"\n── optional-types: check {path.relative_to(ROOT)} ──")
        step = _run([*nyra, "check", str(path)])
        if step != 0:
            print(f"  FAILED: {path.relative_to(ROOT)}")
            rc = step
        else:
            print(f"  ok: {path.relative_to(ROOT)}")
    return rc


def check_abi_manifest() -> int:
    """Run compiler abi_manifest integration tests."""
    from contrib_dev.manifest_dedupe import dedupe_abi_manifest, strip_pure_nyra_symbols

    dedupe_abi_manifest()
    strip_pure_nyra_symbols()
    print("\n── cargo-workspace: abi_manifest tests ──")
    return _run(
        ["cargo", "test", "-p", "compiler", "--test", "abi_manifest", "--", "--nocapture"]
    )


def collect_example_paths_from_result(result) -> list[Path]:
    """Gather example paths touched by a RecipeResult."""
    paths: list[Path] = []
    examples_root = ROOT / "examples"
    for patch in getattr(result, "patches", []):
        path = getattr(patch, "path", None)
        if path is None:
            continue
        p = Path(path)
        if not p.is_absolute():
            p = ROOT / p
        if p.is_file() and examples_root in p.parents and p.suffix == ".ny":
            paths.append(p)
            typed = p.with_name(f"{p.stem}.typed.ny")
            if typed.is_file():
                paths.append(typed)
    return paths


def run_post_scaffold_gates(
    *,
    result=None,
    example_paths: list[Path] | None = None,
    skip_abi: bool = False,
    skip_examples: bool = False,
) -> int:
    """Run CI gates likely to fail after scaffolding. Returns first non-zero exit."""
    rc = 0
    if not skip_abi:
        step = check_abi_manifest()
        if step != 0:
            return step

    paths = list(example_paths or [])
    if result is not None:
        paths.extend(collect_example_paths_from_result(result))
    # dedupe preserving order
    seen: set[str] = set()
    unique: list[Path] = []
    for p in paths:
        key = str(p.resolve())
        if key not in seen:
            seen.add(key)
            unique.append(p)

    if not skip_examples and unique:
        step = check_example_paths(unique)
        if step != 0:
            rc = step

    if rc == 0:
        print("\n✅ Post-scaffold CI gates passed (abi manifest + example checks).")
    else:
        print(
            "\n❌ Post-scaffold CI gates FAILED — fix before pushing.\n"
            "   Common fixes:\n"
            "   • pure Nyra fn wired as extern → remove abi-manifest + runtime_map entry\n"
            "   • duplicate abi symbol → remove older manifest block\n"
            "   • bad example demo → update make/py/contrib_dev/example_codegen.py\n"
            "   Re-run: make contribute → 6 Verify → Post-scaffold CI gates",
            file=sys.stderr,
        )
    return rc


def hub_run_gates_menu() -> int:
    """Interactive verify submenu for full tier-1 subset."""
    from contrib_dev.wizard import prompt_choice

    choice = prompt_choice(
        "CI safety gates",
        {
            "1": "Post-scaffold gates (abi_manifest + last scaffold examples)",
            "2": "abi_manifest only (cargo test -p compiler --test abi_manifest)",
            "3": "optional-types full suite (make test-optional-types)",
            "4": "cargo workspace tests (make test-cargo-workspace)",
            "0": "Back",
        },
    )
    if choice == "0":
        return 0
    if choice == "1":
        return run_post_scaffold_gates()
    if choice == "2":
        return check_abi_manifest()
    if choice == "3":
        return _run(["make", "test-optional-types"])
    if choice == "4":
        return _run(["make", "test-cargo-workspace"])
    return 1


if __name__ == "__main__":
    raise SystemExit(run_post_scaffold_gates())
