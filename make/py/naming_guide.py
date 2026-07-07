"""Clarify Nyra programmer-facing names vs C runtime symbols in contributor wizards."""
from __future__ import annotations


def print_builtin_naming_legend() -> None:
    """Recipe 3 / make add-builtin — method syntax on string/array/etc."""
    print(
        """
  📛 NAMING — Nyra code vs C runtime (read before you pick names)
  ──────────────────────────────────────────────────────────────
  What PROGRAMMERS write in Nyra (this is what you choose as "method name"):
    • Method call     →  "hello".to_snake_case()
    • Free fn alias   →  to_snake_case("hello")   (wrapper in builtins_string.ny)

  What lives in C only (stdlib/rt/*.c — you implement logic here):
    • C symbol        →  str_to_snake_case

  Wizard rule:
    • Enter the METHOD name without str_  (e.g. to_snake_case).
    • The tool auto-derives the C name    (e.g. str_to_snake_case).

  ⚠ Do NOT run Recipe 2 and Recipe 3 for the same feature — that duplicates C code.
     For .method() syntax, use Recipe 3 only."""
    )


def print_extern_naming_legend() -> None:
    """Recipe 2 / stdlib-extern — direct extern fn, no compiler method wiring."""
    print(
        """
  📛 NAMING — Pattern B: extern fn + C (no .method syntax)
  ──────────────────────────────────────────────────────────
  What PROGRAMMERS write in Nyra:
    • Direct call     →  str_to_snake_case("hello")

  What lives in C (same identifier):
    • C symbol        →  str_to_snake_case   (in stdlib/rt/*.c)

  Wizard rule:
    • Enter the C / extern name here (usually str_*).

  ⚠ This recipe does NOT add "hello".to_snake_case().
     For .method() syntax, stop and use Recipe 3 (Built-in Method) instead."""
    )


def format_builtin_name_summary(method: str, c_name: str) -> list[str]:
    return [
        f'Nyra method (programmer code):  "x".{method}()',
        f"Nyra free fn (optional alias):   {method}(s)",
        f"C symbol (stdlib/rt/*.c only):  {c_name}",
    ]


def format_extern_name_summary(fn_name: str) -> list[str]:
    return [
        f"Nyra call (programmer code):     {fn_name}(…)",
        f"C symbol (stdlib/rt/*.c only):   {fn_name}",
        "No .method() — use Recipe 3 if you need that syntax.",
    ]
