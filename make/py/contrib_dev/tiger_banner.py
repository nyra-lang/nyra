"""Static Nyra tiger ASCII logo for `make contribute`."""
from __future__ import annotations

import os
import shutil
import sys

_TIGER = """
                                          @@#
                                          @@@##+**+=-
                                      .++@%@@@%%%%*#+--:
                                   .*++@@@@@%%%%%#**. %*+
                                 %%##*@@@%@@%###@%**==++%=.
                              .*=#.%%@@@@@+@.@%%%@@@@%*+==--
                            :%#:###%%@@*%##@@*@@@@@@@@@@%#*#
                          :%%%@=%%%@@ %%%#@@@@@..@@@@%+%%*.
                         .:@@@@-@@@-@%.%%@%%@@@.     @@@%:
                            %.%@@.@@@%%.:::%%%
                              %%%.@@@@@ ### +. .##
                                -@@@@@@ ####+  .## *#*    ** *#-.#. .*##=
                                  @@+@@ ###### .##  ##=  ##= ####:. #. .###
                                   -@.@ ### .#####   ## -##  ###    #######
                                     @@ ###   ####    ####   ###   ##.  ###
                                      : ###     ##    :##    ###    ####.##
                                                   .####
""".strip("\n").splitlines()


def _normalize(lines: list[str]) -> list[str]:
    trimmed = [ln.rstrip() for ln in lines]
    nonempty = [ln for ln in trimmed if ln.strip()]
    if not nonempty:
        return trimmed
    min_indent = min(len(ln) - len(ln.lstrip()) for ln in nonempty)
    return [(ln[min_indent:] if ln.strip() else "") for ln in trimmed]


def _center(lines: list[str], width: int) -> list[str]:
    max_len = max((len(ln) for ln in lines if ln), default=0)
    left = max(0, (width - max_len) // 2)
    pad = " " * left
    return [f"{pad}{ln}" if ln else "" for ln in lines]


def _colorize_line(line: str) -> str:
    if not line:
        return line
    reset = "\033[0m"
    parts: list[str] = []
    for ch in line:
        if ch == "@":
            parts.append(f"\033[38;5;208m{ch}{reset}")
        elif ch in "#%":
            parts.append(f"\033[38;5;214m{ch}{reset}")
        elif ch in "+*=":
            parts.append(f"\033[38;5;220m{ch}{reset}")
        elif ch in ".:-":
            parts.append(f"\033[38;5;240m{ch}{reset}")
        else:
            parts.append(ch)
    return "".join(parts)


def _terminal_width() -> int:
    try:
        return shutil.get_terminal_size(fallback=(80, 24)).columns
    except OSError:
        return 80


def _use_color() -> bool:
    if os.environ.get("NO_COLOR"):
        return False
    if os.environ.get("FORCE_COLOR"):
        return True
    return sys.stdout.isatty()


def play_tiger_intro() -> None:
    """Print centered tiger logo once."""
    use_color = _use_color()
    lines = _center(_normalize(_TIGER), _terminal_width())
    block = "\n".join(_colorize_line(ln) if use_color else ln for ln in lines)
    print(block)
    print()
