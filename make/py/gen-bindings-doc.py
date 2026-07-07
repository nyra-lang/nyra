#!/usr/bin/env python3
"""Generate docs/bindings.md from abi-manifest.toml + stdlib scan.

This is an INTERNAL developer reference (C symbols, runtime modules, ABI
versions). It is intentionally NOT published to webDocs, which is the
user-facing site and must not expose build/runtime internals.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "docs" / "abi-manifest.toml"
STDLIB = ROOT / "stdlib"
OUT_MD = ROOT / "docs" / "bindings.md"

EXTERN_RE = re.compile(r"^\s*extern\s+fn\s+(\w+)\s*\(")


def load_manifest() -> list[dict]:
    data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    return data.get("symbol", [])


def scan_stdlib_bindings() -> dict[str, list[str]]:
    found: dict[str, list[str]] = {}
    for path in sorted(STDLIB.rglob("*.ny")):
        rel = path.relative_to(STDLIB).as_posix()
        for line in path.read_text(encoding="utf-8").splitlines():
            m = EXTERN_RE.match(line)
            if m:
                found.setdefault(m.group(1), []).append(rel)
    return found


def nyra_stdlib_cell(name: str, stdlib_map: dict[str, list[str]]) -> str:
    paths = stdlib_map.get(name, [])
    if not paths:
        return "—"
    return ", ".join(f"`stdlib/{p}`" for p in paths)


def map_naming_section_md() -> str:
    return """
## HashMap runtime naming

Hash-map symbols follow `map_<key_type>_<value_type>_<operation>`. The first type is the **key**, the second is the **value**, then the operation (`new`, `insert`, `get`, `contains`, `remove`, `keys`, `free`, `retain`).

| Family | Example symbols | Use case |
|--------|-----------------|----------|
| `map_str_i32_*` | `map_str_i32_insert`, `map_str_i32_get` | String keys, integer values |
| `map_str_str_*` | `map_str_str_insert`, `map_str_str_get` | String keys, string values |
| `map_i32_i32_*` | `map_i32_i32_insert`, `map_i32_i32_get` | Integer keys and values (`map[int]int` parity) |

When key and value types match, both appear in the name (e.g. `map_i32_i32_get`) so each C entry point has an unambiguous signature. Tutorial: [Learn → HashMap](../webDocs/learn-hashmap.html). Stdlib: `stdlib/map.ny` (`HashMap_str_i32`, `HashMap_str_str`).
"""


def generate_md(symbols: list[dict], stdlib_map: dict[str, list[str]]) -> str:
    stable = [s for s in symbols if s.get("tier") == "stable"]
    experimental = [s for s in symbols if s.get("tier") != "stable"]
    lines = [
        "# Nyra runtime bindings reference",
        "",
        "**Generated** by `make gen-bindings-doc` — do not edit by hand.",
        "",
        "Stable C symbols live in [`abi-manifest.toml`](abi-manifest.toml) and [`stdlib/nyra_rt.h`](../stdlib/nyra_rt.h).",
        "Nyra stdlib modules declare `extern fn` wrappers that call into the C runtime.",
        "",
        "Regenerate:",
        "",
        "```bash",
        "make gen-bindings-doc",
        "```",
        "",
    ]
    lines.extend(map_naming_section_md().strip().splitlines())
    lines.extend(
        [
            "",
            "## Stable bindings",
            "",
            "| Symbol | C signature | RT module | Since | Nyra stdlib |",
            "|--------|-------------|-----------|-------|-------------|",
        ]
    )
    for sym in sorted(stable, key=lambda s: s["name"]):
        lines.append(
            f"| `{sym['name']}` | `{sym['c_sig']}` | `{sym.get('module', '?')}` | {sym.get('since', '?')} | {nyra_stdlib_cell(sym['name'], stdlib_map)} |"
        )
    lines.extend(
        [
            "",
            "## Experimental bindings",
            "",
            "| Symbol | C signature | RT module | Since | Nyra stdlib |",
            "|--------|-------------|-----------|-------|-------------|",
        ]
    )
    for sym in sorted(experimental, key=lambda s: s["name"]):
        lines.append(
            f"| `{sym['name']}` | `{sym['c_sig']}` | `{sym.get('module', '?')}` | {sym.get('since', '?')} | {nyra_stdlib_cell(sym['name'], stdlib_map)} |"
        )
    lines.extend(
        [
            "",
            "## Package bindings (NyraPkg)",
            "",
            "Third-party packages ship their own `extern fn` + `link-source` C shims. Example:",
            "",
            "| Package | Nyra module | C shim | Native lib |",
            "|---------|-------------|--------|------------|",
            "| `ny-sqlite` | `examples/packages/ny-sqlite/sqlite.ny` | `rt/sqlite.c` | `-lsqlite3` |",
            "",
            "Install with `nyra pkg install ny-sqlite@^0.1.0` then `import \"pkg/ny-sqlite\"`.",
            "",
            "See [`docs/nyrapkg-v1.md`](nyrapkg-v1.md) and [`docs/integration-ideas/native-bindings/README.md`](integration-ideas/native-bindings/README.md).",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    if not MANIFEST.is_file():
        print(f"error: manifest not found: {MANIFEST}", file=sys.stderr)
        return 1
    symbols = load_manifest()
    stdlib_map = scan_stdlib_bindings()
    OUT_MD.write_text(generate_md(symbols, stdlib_map), encoding="utf-8")
    print(
        f"wrote {OUT_MD} "
        f"({len(symbols)} symbols, {len(stdlib_map)} stdlib extern fns)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
