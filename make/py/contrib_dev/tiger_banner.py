"""Static Nyra tiger ASCII logo for `make contribute`."""
from __future__ import annotations

import shutil

from .terminal_style import RESET, use_color

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

# Softer tiger tones (match menu palette).
_C_AT = "\033[38;5;216m"    # soft coral
_C_HASH = "\033[38;5;179m"  # soft tan
_C_GOLD = "\033[38;5;186m"  # soft gold
_C_DIM = "\033[38;5;245m"   # muted outline


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
    parts: list[str] = []
    for ch in line:
        if ch == "@":
            parts.append(f"{_C_AT}{ch}{RESET}")
        elif ch in "#%":
            parts.append(f"{_C_HASH}{ch}{RESET}")
        elif ch in "+*=":
            parts.append(f"{_C_GOLD}{ch}{RESET}")
        elif ch in ".:-":
            parts.append(f"{_C_DIM}{ch}{RESET}")
        else:
            parts.append(ch)
    return "".join(parts)


def _terminal_width() -> int:
    try:
        return shutil.get_terminal_size(fallback=(80, 24)).columns
    except OSError:
        return 80


def play_tiger_intro() -> None:
    """Print centered tiger logo once."""
    color = use_color()
    lines = _center(_normalize(_TIGER), _terminal_width())
    block = "\n".join(_colorize_line(ln) if color else ln for ln in lines)
    print(block)
    print()
