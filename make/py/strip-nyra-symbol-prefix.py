#!/usr/bin/env python3
"""Strip nyra_/ny_ prefixes from runtime ABI symbols across the repo."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "docs" / "abi-manifest.toml"

# Internal identifiers — never rename these substrings.
SKIP_SUBSTRINGS = frozenset(
    {
        "NYRA_HOME",
        "NYRA_RT_H",
        "nyra_rt.h",
        "nyra_rt.c",
        "nyra_rt_wasi",
        "nyra_home",
        "nyra_bin",
        "nyra_test_",
        "nyra_cli_",
        "nyra_lang",
        "nyra_link_test",
        "nyra_pgo_run",
        "nyra_enum_import",
        "nyra_main_nyra",
        "nyra_alt_ext",
        "nyra_unicode_",
        "nyra_color_",
        "nyra_input_cli_",
        "nyra_break_clone",
        "nyra_rt_modules",
        "nyra_rt_h_",
        "nyra_ann",
        "nyra-skill",
        "nyra-skill.md",
        "ny-toml",
        "ny-serde",
        "ny-postgres",
        "ny-redis",
        "ny-mysql",
        "ny-sqlite",
        "nyra_blackbox",  # handled via manifest
    }
)

GLOB_DIRS = [
    "docs",
    "stdlib",
    "compiler",
    "cli",
    "pkg",
    "rt",
    "examples",
    "tests",
    "webDocs",
    "scripts",
    "stubs",
    "Apps",
    "bindgen",
    "c-bindgen",
]

EXTENSIONS = {".c", ".h", ".ny", ".rs", ".toml", ".md", ".html", ".snap", ".sh", ".py", ".mjs"}


def load_old_symbols() -> list[str]:
    text = MANIFEST.read_text(encoding="utf-8")
    return re.findall(r'^name = "([^"]+)"', text, re.M)


def new_name(old: str) -> str | None:
    if old.startswith("nyra_"):
        return old[5:]
    if old.startswith("ny_"):
        return old[3:]
    return None


def build_rename_map(symbols: list[str]) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for sym in symbols:
        nn = new_name(sym)
        if nn:
            mapping[sym] = nn
    # Extra developer-facing symbols outside manifest
    extras = {
        "nyra_add": "add",
        "nyra_greet": "greet",
        "nyra_uuid_new_v4": "uuid_new_v4",
        "nyra_uuid_parse": "uuid_parse",
        "nyra_serde_json_parse": "serde_json_parse",
        "nyra_serde_json_stringify": "serde_json_stringify",
        "nyra_toml_parse": "toml_parse",
        "nyra_toml_stringify": "toml_stringify",
    }
    mapping.update(extras)
    return mapping


def should_skip_file(path: Path) -> bool:
    rel = str(path.relative_to(ROOT))
    if rel == "make/py/strip-nyra-symbol-prefix.py":
        return True
    if "strip-nyra-symbol-prefix" in rel:
        return True
    return False


def replace_in_text(text: str, mapping: dict[str, str]) -> tuple[str, int]:
    # Longest keys first to avoid partial overlaps.
    keys = sorted(mapping.keys(), key=len, reverse=True)
    total = 0
    for old in keys:
        new = mapping[old]
        if old not in text:
            continue
        # Word-boundary style: symbol names are identifiers.
        pattern = re.compile(rf"(?<![A-Za-z0-9_]){re.escape(old)}(?![A-Za-z0-9_])")
        text, n = pattern.subn(new, text)
        total += n
    return text, total


def update_manifest(mapping: dict[str, str]) -> int:
    text = MANIFEST.read_text(encoding="utf-8")
    for old, new in sorted(mapping.items(), key=lambda x: len(x[0]), reverse=True):
        text = text.replace(f'name = "{old}"', f'name = "{new}"')
        text = re.sub(
            rf"\b{re.escape(old)}\b",
            new,
            text,
        )
    MANIFEST.write_text(text, encoding="utf-8")
    return 1


def iter_files() -> list[Path]:
    out: list[Path] = []
    for d in GLOB_DIRS:
        base = ROOT / d
        if not base.is_dir():
            continue
        for p in base.rglob("*"):
            if not p.is_file():
                continue
            if p.suffix not in EXTENSIONS:
                continue
            if should_skip_file(p):
                continue
            out.append(p)
    return out


def main() -> int:
    symbols = load_old_symbols()
    mapping = build_rename_map(symbols)
    print(f"Renaming {len(mapping)} symbols")

    update_manifest(mapping)

    changed_files = 0
    total_repl = 0
    for path in iter_files():
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        if any(skip in text for skip in SKIP_SUBSTRINGS) and path.suffix not in {".c", ".h", ".ny", ".rs", ".toml"}:
            pass  # still process; skip list is for awareness only
        new_text, n = replace_in_text(text, mapping)
        if n:
            path.write_text(new_text, encoding="utf-8")
            changed_files += 1
            total_repl += n

  # Fix rust_bridge lib name pattern separately
    rb = ROOT / "pkg" / "src" / "rust_bridge.rs"
    if rb.is_file():
        t = rb.read_text(encoding="utf-8")
        t2 = t.replace('format!("nyra_bridge_{}"', 'format!("bridge_{}"')
        t2 = t2.replace('"nyra_bridge_serde_json"', '"bridge_serde_json"')
        if t2 != t:
            rb.write_text(t2, encoding="utf-8")
            changed_files += 1

    print(f"Updated {changed_files} files ({total_repl} replacements)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
