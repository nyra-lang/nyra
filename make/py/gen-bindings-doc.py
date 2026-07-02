#!/usr/bin/env python3
"""Generate docs/bindings.md and webDocs/bindings.html from abi-manifest.toml + stdlib scan."""
from __future__ import annotations

import html
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
OUT_HTML = ROOT / "webDocs" / "bindings.html"
NAV_SOURCE = ROOT / "webDocs" / "scripts" / "generate-pages.py"

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


def map_naming_section_html() -> str:
    return """
      <h2 id="hashmap-naming">HashMap runtime naming</h2>
      <p>Hash-map symbols follow <code>map_&lt;key_type&gt;_&lt;value_type&gt;_&lt;operation&gt;</code>. The first type is the <strong>key</strong>, the second is the <strong>value</strong>, then the operation (<code>new</code>, <code>insert</code>, <code>get</code>, <code>contains</code>, <code>remove</code>, <code>keys</code>, <code>free</code>, <code>retain</code>).</p>
      <table>
        <thead><tr><th>Family</th><th>Example symbols</th><th>Use case</th></tr></thead>
        <tbody>
          <tr><td><code>map_str_i32_*</code></td><td><code>map_str_i32_insert</code>, <code>map_str_i32_get</code></td><td>String keys, integer values</td></tr>
          <tr><td><code>map_str_str_*</code></td><td><code>map_str_str_insert</code>, <code>map_str_str_get</code></td><td>String keys, string values</td></tr>
          <tr><td><code>map_i32_i32_*</code></td><td><code>map_i32_i32_insert</code>, <code>map_i32_i32_get</code></td><td>Integer keys and values (<code>map[int]int</code> parity)</td></tr>
        </tbody>
      </table>
      <p>When key and value types match, both appear in the name (e.g. <code>map_i32_i32_get</code>) so each C entry point has an unambiguous signature. Tutorial and examples: <a href="learn-hashmap.html">Learn → HashMap</a>. Stdlib wrappers: <code>stdlib/map.ny</code> (<code>HashMap_str_i32</code>, <code>HashMap_str_str</code>).</p>
"""


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


def load_nav(active: str = "bindings.html") -> str:
    text = NAV_SOURCE.read_text(encoding="utf-8")
    start = text.index("NAV = '''") + len("NAV = '''")
    end = text.index("'''", start)
    nav = text[start:end]
    return nav.replace(
        f'<a href="{active}"',
        f'<a class="active" href="{active}"',
        1,
    )


def table_rows(symbols: list[dict], stdlib_map: dict[str, list[str]]) -> str:
    rows = []
    for sym in sorted(symbols, key=lambda s: s["name"]):
        nyra = nyra_stdlib_cell(sym["name"], stdlib_map).replace("`", "")
        rows.append(
            "<tr>"
            f"<td><code>{html.escape(sym['name'])}</code></td>"
            f"<td><code>{html.escape(sym['c_sig'])}</code></td>"
            f"<td><code>{html.escape(sym.get('module', '?'))}</code></td>"
            f"<td>{html.escape(str(sym.get('since', '?')))}</td>"
            f"<td>{html.escape(nyra)}</td>"
            "</tr>"
        )
    return "\n".join(rows)


def generate_html(symbols: list[dict], stdlib_map: dict[str, list[str]]) -> str:
    stable = [s for s in symbols if s.get("tier") == "stable"]
    experimental = [s for s in symbols if s.get("tier") != "stable"]
    nav = load_nav()
    return f"""<!DOCTYPE html>
<html lang="en" dir="ltr" data-theme="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="theme-color" content="#06090d">
  <meta name="color-scheme" content="dark light">
  <title>Runtime bindings — Nyra Docs</title>
  <link rel="stylesheet" href="css/style.css">
  <link rel="stylesheet" href="css/search.css">
</head>
<body data-page="bindings">
  <input type="checkbox" id="nav-check" class="nav-check" hidden aria-hidden="true">
  <header class="site-header">
    <a class="logo" href="index.html">
      <img src="../assets/Nyrabgremoved.png" alt="Nyra">
      <span>Nyra</span>
    </a>
    <div class="site-toolbar">
      <div class="toolbar-group" role="group" aria-label="Search">
        <button type="button" class="toolbar-btn search-btn" id="search-open" title="Search (Ctrl+K)">
          <span aria-hidden="true">⌕</span>
          <kbd>Ctrl+K</kbd>
        </button>
      </div>
      <div class="toolbar-group" role="group" aria-label="Theme">
        <button type="button" class="toolbar-btn" id="theme-toggle" title="Toggle theme">
          <span class="theme-icon theme-icon-sun" aria-hidden="true">☀</span>
          <span class="theme-icon theme-icon-moon" aria-hidden="true">☽</span>
        </button>
      </div>
    </div>
    <label for="nav-check" class="nav-toggle" aria-label="Open navigation menu">
      <span></span><span></span><span></span>
    </label>
    <span class="tagline">Fast · Safe · Minimal</span>
  </header>
  <label for="nav-check" class="sidebar-backdrop" aria-hidden="true"></label>
  <div class="layout">
    <aside class="sidebar">
{nav}
    </aside>
    <main class="content">
      <h1>Runtime bindings</h1>
      <p class="lead">C runtime symbols mapped to Nyra <code>extern fn</code> in stdlib and NyraPkg packages.</p>
      <p>Generated from <code>docs/abi-manifest.toml</code> + stdlib scan. Regenerate: <code>make gen-bindings-doc</code>. C header: <code>stdlib/nyra_rt.h</code>.</p>
{map_naming_section_html()}
      <h2>Stable bindings ({len(stable)})</h2>
      <table>
        <thead><tr><th>Symbol</th><th>C signature</th><th>RT module</th><th>Since</th><th>Nyra stdlib</th></tr></thead>
        <tbody>
{table_rows(stable, stdlib_map)}
        </tbody>
      </table>

      <h2>Experimental bindings ({len(experimental)})</h2>
      <table>
        <thead><tr><th>Symbol</th><th>C signature</th><th>RT module</th><th>Since</th><th>Nyra stdlib</th></tr></thead>
        <tbody>
{table_rows(experimental, stdlib_map)}
        </tbody>
      </table>

      <h2>Package bindings (NyraPkg)</h2>
      <table>
        <thead><tr><th>Package</th><th>Nyra module</th><th>C shim</th><th>Native lib</th></tr></thead>
        <tbody>
          <tr><td><code>ny-sqlite</code></td><td><code>sqlite.ny</code></td><td><code>rt/sqlite.c</code></td><td><code>-lsqlite3</code></td></tr>
        </tbody>
      </table>
      <p>Install: <code>nyra pkg install ny-sqlite@^0.1.0</code> then <code>import "pkg/ny-sqlite"</code>. See <a href="packages.html">NyraPkg</a>.</p>

      <footer class="site-footer"><a href="packages.html">NyraPkg →</a></footer>
    </main>
  </div>
  <script src="vendor/lunr.min.js"></script>
  <script src="js/search.js"></script>
  <script src="js/site.js"></script>
</body>
</html>
"""


def main() -> int:
    if not MANIFEST.is_file():
        print(f"error: manifest not found: {MANIFEST}", file=sys.stderr)
        return 1
    symbols = load_manifest()
    stdlib_map = scan_stdlib_bindings()
    OUT_MD.write_text(generate_md(symbols, stdlib_map), encoding="utf-8")
    OUT_HTML.write_text(generate_html(symbols, stdlib_map), encoding="utf-8")
    print(
        f"wrote {OUT_MD} and {OUT_HTML} "
        f"({len(symbols)} symbols, {len(stdlib_map)} stdlib extern fns)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
