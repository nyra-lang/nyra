"""Scan the repo for builtins wired by builtin-dev (via markers or known patterns)."""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from .paths import ARRAY_PATHS, BYTES_PATHS, FREE_PATHS, STRING_PATHS, repo_path

MARKER_RE = re.compile(r"\[builtin-dev:([^:\]]+):([^\]]+)\]")


@dataclass(frozen=True)
class WiredBuiltin:
    receiver: str
    method: str
    marker: str

    @property
    def label(self) -> str:
        return f"{self.receiver}.{self.method}"


def _scan_file(path: Path) -> list[WiredBuiltin]:
    if not path.exists():
        return []
    found: dict[str, WiredBuiltin] = {}
    for match in MARKER_RE.finditer(path.read_text(encoding="utf-8")):
        method, receiver = match.group(1), match.group(2)
        key = f"{receiver}:{method}"
        found[key] = WiredBuiltin(receiver=receiver, method=method, marker=key)
    return list(found.values())


def list_wired_builtins(*, receiver: str | None = None) -> list[WiredBuiltin]:
    """Return builtins found via `[builtin-dev:method:receiver]` markers."""
    paths = [
        STRING_PATHS["rt_c"],
        STRING_PATHS["builtins_ny"],
        STRING_PATHS["typecheck"],
        STRING_PATHS["codegen_strings"],
        BYTES_PATHS["rt_c"],
        ARRAY_PATHS["typecheck"],
        repo_path("docs/abi-manifest.toml"),
    ]
    merged: dict[str, WiredBuiltin] = {}
    for path in paths:
        for item in _scan_file(path):
            if receiver is None or item.receiver == receiver:
                merged[item.marker] = item
    return sorted(merged.values(), key=lambda b: (b.receiver, b.method))


def suggest_string_args(method: str) -> list[str]:
    from .method_catalog import method_profile

    profile = method_profile(method)
    return list(profile.default_args) if profile else []
