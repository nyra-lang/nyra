"""Soft terminal colors for `make contribute` (easy on the eyes)."""
from __future__ import annotations

import os
import re
import sys

RESET = "\033[0m"

# Muted palette — works well on dark terminals.
BORDER = "\033[38;5;108m"   # soft sea green
TITLE = "\033[38;5;117m"    # soft sky blue
ACCENT = "\033[38;5;150m"   # soft mint
NUM = "\033[38;5;214m"      # soft amber
TEXT = "\033[38;5;252m"     # soft white
MUTED = "\033[2;38;5;245m"  # dim gray
TAG = "\033[38;5;139m"      # soft purple (Pattern tags)


def use_color() -> bool:
    if os.environ.get("NO_COLOR"):
        return False
    if os.environ.get("FORCE_COLOR"):
        return True
    return sys.stdout.isatty()


def visible_len(text: str) -> int:
    return len(re.sub(r"\033\[[0-9;]*m", "", text))


def pad(text: str, width: int) -> str:
    return text + " " * max(0, width - visible_len(text))


def box_row(inner: str, *, width: int = 45, color: bool = True) -> str:
    b = BORDER if color else ""
    r = RESET if color else ""
    return f"{b}│{r}{pad(inner, width)}{b}│{r}"


def box_top(*, width: int = 45, color: bool = True) -> str:
    b = BORDER if color else ""
    r = RESET if color else ""
    return f"{b}┌{'─' * width}┐{r}"


def box_bottom(*, width: int = 45, color: bool = True) -> str:
    b = BORDER if color else ""
    r = RESET if color else ""
    return f"{b}└{'─' * width}┘{r}"


def menu_item(
    num: str,
    title: str,
    tag: str,
    hint: str,
    *,
    width: int = 45,
    color: bool = True,
) -> tuple[str, str]:
    """Return (title row, hint row) for one menu entry."""
    if not color:
        title_row = f" {num}. {title} {tag}".rstrip()
        hint_row = f"    {hint}"
        return box_row(title_row, width=width, color=False), box_row(hint_row, width=width, color=False)

    n = f"{NUM}{num}.{RESET}"
    t = f"{TEXT}{title}{RESET}"
    g = f" {TAG}{tag}{RESET}" if tag else ""
    title_inner = f" {n} {t}{g}"
    if hint.startswith("→"):
        hint_inner = f"{MUTED}   {RESET}{ACCENT}→{RESET}{MUTED}{hint[1:]}{RESET}"
    else:
        hint_inner = f"{MUTED}   {hint}{RESET}"
    return box_row(title_inner, width=width, color=True), box_row(hint_inner, width=width, color=True)


def hint_line(text: str, *, color: bool = True) -> str:
    if not color:
        return f"  {text}"
    return f"  {MUTED}{text}{RESET}"
