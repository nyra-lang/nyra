"""Live, repo-aware suggestions for each `make contribute` question.

We know the whole language + repo layout, so instead of leaving a
contributor guessing, every wizard step can offer real, existing choices
(stdlib modules that actually exist, rt/*.c files present today, valid Nyra
types, example topics, conformance areas, …). Each provider scans the repo
on demand so suggestions never go stale.
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from builtin_dev.spec import NyraType

from .paths import CONFORMANCE, EXAMPLES, PKG_EXAMPLES, STDLIB

_SKIP_DIRS = {"rt", "rt_wasi", "prebuilt", "__pycache__", "target"}


@dataclass(frozen=True)
class Suggestion:
    value: str
    note: str = ""


def _safe_iterdir(path: Path):
    try:
        return sorted(p for p in path.iterdir())
    except OSError:
        return []


def stdlib_modules() -> list[Suggestion]:
    """Existing stdlib module paths a contributor can extend.

    Public API surface first: top-level ``X.ny`` files and ``X/mod.ny``
    package entries rank above deeper internal files so the short preview
    shows the modules people usually extend.
    """
    ranked: list[tuple[int, str, Suggestion]] = []
    seen: set[str] = set()
    if not STDLIB.exists():
        return []
    for path in STDLIB.rglob("*.ny"):
        rel = path.relative_to(STDLIB)
        if any(part in _SKIP_DIRS for part in rel.parts):
            continue
        rel_str = rel.as_posix()
        if rel_str in seen:
            continue
        seen.add(rel_str)
        is_top = len(rel.parts) == 1
        is_entry = path.name == "mod.ny"
        note = "package entry" if is_entry else ("core module" if is_top else "")
        rank = 0 if (is_top or is_entry) else 1
        ranked.append((rank, rel_str, Suggestion(rel_str, note)))
    ranked.sort(key=lambda t: (t[0], t[1]))
    return [s for _rank, _rel, s in ranked]


def rt_files() -> list[Suggestion]:
    """Existing stdlib/rt/*.c runtime files."""
    rt_dir = STDLIB / "rt"
    if not rt_dir.exists():
        return []
    return [Suggestion(p.name) for p in sorted(rt_dir.glob("rt_*.c"))]


def nyra_types() -> list[Suggestion]:
    notes = {
        "string": "text (char* in C)",
        "i32": "32-bit int",
        "i64": "64-bit int",
        "f64": "float",
        "bool": "true/false",
        "void": "no return value",
        "vec_str": "list of strings",
        "bytes": "raw byte buffer",
        "array": "generic array",
    }
    return [Suggestion(t.value, notes.get(t.value, "")) for t in NyraType]


def example_topics() -> list[Suggestion]:
    if not EXAMPLES.exists():
        return []
    return [Suggestion(p.name) for p in _safe_iterdir(EXAMPLES) if p.is_dir()]


def _conformance_areas(mode: str) -> list[Suggestion]:
    base = CONFORMANCE / mode
    if not base.exists():
        return []
    return [Suggestion(p.name) for p in _safe_iterdir(base) if p.is_dir()]


def conformance_areas_pass() -> list[Suggestion]:
    return _conformance_areas("pass")


def conformance_areas_fail() -> list[Suggestion]:
    return _conformance_areas("fail")


def conformance_areas() -> list[Suggestion]:
    """Union of pass + fail areas (mode is asked in a separate step)."""
    seen: dict[str, Suggestion] = {}
    for s in conformance_areas_pass() + conformance_areas_fail():
        seen.setdefault(s.value, s)
    return sorted(seen.values(), key=lambda s: s.value)


_EXTERN_RE = re.compile(r"extern\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)")


def extern_fns() -> list[Suggestion]:
    """Names of extern fns already declared in stdlib (for the wrap step)."""
    out: dict[str, Suggestion] = {}
    if not STDLIB.exists():
        return []
    for path in sorted(STDLIB.rglob("*.ny")):
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        for name in _EXTERN_RE.findall(text):
            out.setdefault(name, Suggestion(name, path.relative_to(STDLIB).as_posix()))
    return sorted(out.values(), key=lambda s: s.value)


def pkg_names() -> list[Suggestion]:
    if not PKG_EXAMPLES.exists():
        return []
    return [Suggestion(p.name) for p in _safe_iterdir(PKG_EXAMPLES) if p.is_dir()]


# Suggestion providers keyed by the `suggest` field on a WizardStep.
PROVIDERS = {
    "stdlib_module": stdlib_modules,
    "rt_file": rt_files,
    "nyra_type": nyra_types,
    "example_topic": example_topics,
    "conformance_area": conformance_areas,
    "extern_fn": extern_fns,
    "pkg_name": pkg_names,
}


def suggestions_for(key: str) -> list[Suggestion]:
    provider = PROVIDERS.get(key)
    if not provider:
        return []
    try:
        return provider()
    except Exception:
        return []
