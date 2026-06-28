#!/usr/bin/env python3
"""Generate HTML benchmark report from data.tsv."""
from __future__ import annotations

import html
import os
import sys
from collections import defaultdict
from pathlib import Path


LANG_COLORS = {
    "Nyra (typed)": "#2dd4bf",
    "Nyra (untyped)": "#3d9a8b",
    "Rust": "#dea584",
    "Go": "#00a8cc",
    "C": "#7eb6ff",
    "C++": "#00599c",
    "Node": "#6bbd5b",
    "Python": "#5b9bd5",
    "Java": "#e76f00",
}
LANG_ORDER = [
    "Nyra (typed)",
    "Nyra (untyped)",
    "Rust",
    "Go",
    "C",
    "C++",
    "Node",
    "Python",
    "Java",
]
SUITE_ORDER = [
    "cpu_bound",
    "micro_short",
    "no_io",
    "no_alloc",
    "single_thread",
    "no_async",
    "no_net",
    "hello",
    "arithmetic",
    "loop",
    "loop_nofold",
    "comptime_table",
    "fib",
    "nested",
    "struct_sum",
    "dungeon",
]
SUITE_INFO = {
    "cpu_bound": "8M mod-mix hot loop — CPU-bound, no I/O",
    "micro_short": "125k sum — targets ~2–3 ms wall (minimal stdout)",
    "no_io": "5M xor loop — zero stdout (blackbox / KeepAlive only)",
    "no_alloc": "5M stack i32 only — no heap / strings / vec",
    "single_thread": "200×200 nested — one OS thread, sync only",
    "no_async": "Iterative fib(40) — no spawn / async / tasks",
    "no_net": "4M local mod hash — no sockets or HTTP",
    "hello": "Minimal stdout I/O",
    "arithmetic": "Two integer adds",
    "loop": "Linear sum 0..N-1 (N=10M)",
    "loop_nofold": "Loop with anti-constant-fold blackbox",
    "comptime_table": "Lookup table build + sum (Nyra-comptime = compile-time fold)",
    "fib": "Iterative Fibonacci (35 steps)",
    "nested": "2D nested loop (200×200)",
    "struct_sum": "Field accumulation (500k) — struct vs locals",
    "dungeon": "Multi-file Dungeon Steps app (typed Nyra only)",
}
SUITE_TAGS = {
    "cpu_bound": ["CPU-bound", "Single-threaded", "No I/O", "No net", "No alloc", "No async"],
    "micro_short": ["CPU-bound", "Single-threaded", "~2–3 ms", "No net", "No alloc", "No async"],
    "no_io": ["CPU-bound", "Single-threaded", "No I/O", "No net", "No alloc", "No async"],
    "no_alloc": ["CPU-bound", "Single-threaded", "No I/O", "No net", "No alloc", "No async"],
    "single_thread": ["CPU-bound", "Single-threaded", "No net", "No alloc", "No async"],
    "no_async": ["CPU-bound", "Single-threaded", "No net", "No alloc", "No async"],
    "no_net": ["CPU-bound", "Single-threaded", "No I/O", "No net", "No alloc", "No async"],
}


def fmt_ms(v: float) -> str:
    return f"{v:.2f}"


def fmt_mem(kb: int) -> str:
    if kb >= 1024:
        return f"{kb / 1024:.2f} MB"
    return f"{kb} KB"


def bar_width(value: float, max_val: float) -> int:
    if max_val <= 0:
        return 0
    return min(100, max(4, round(100 * value / max_val)))


def load_rows(tsv: Path) -> list[dict]:
    rows = []
    with tsv.open() as f:
        header = f.readline().rstrip("\n").split("\t")
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split("\t")
            row = dict(zip(header, parts))
            row["wall_ms"] = float(row["wall_ms"])
            row["peak_rss_kb"] = int(row["peak_rss_kb"])
            row["cpu_user_ms"] = float(row["cpu_user_ms"])
            row["cpu_sys_ms"] = float(row["cpu_sys_ms"])
            row["cpu_pct"] = float(row["cpu_pct"])
            row["minflt"] = int(row["minflt"])
            row["majflt"] = int(row["majflt"])
            row["vol_ctx"] = int(row["vol_ctx"])
            row["invol_ctx"] = int(row["invol_ctx"])
            row["gpu_util_pct"] = float(row["gpu_util_pct"])
            row["gpu_mem_mb"] = float(row["gpu_mem_mb"])
            rows.append(row)
    return rows


def sort_rows(rows, key):
    return sorted(
        rows,
        key=lambda r: (
            r[key],
            LANG_ORDER.index(r["lang"]) if r["lang"] in LANG_ORDER else 99,
        ),
    )


def build_report(rows: list[dict], meta: dict) -> str:
    by_suite = defaultdict(list)
    for r in rows:
        by_suite[r["suite"]].append(r)

    suites = [s for s in SUITE_ORDER if s in by_suite] + sorted(
        set(by_suite.keys()) - set(SUITE_ORDER)
    )

    langs_present = []
    for lang in LANG_ORDER:
        if any(r["lang"] == lang for r in rows):
            langs_present.append(lang)

    legend_html = "".join(
        f'<span class="legend-item">'
        f'<span class="lang-dot" style="background:{LANG_COLORS.get(lang, "#888")}"></span> '
        f"{html.escape(lang)}</span>"
        for lang in langs_present
    )

    suite_sections = []
    for suite in suites:
        suite_rows = sort_rows(by_suite[suite], "wall_ms")
        max_ms = suite_rows[-1]["wall_ms"]
        max_mem = max(r["peak_rss_kb"] for r in suite_rows)
        max_cpu = max(r["cpu_pct"] for r in suite_rows)

        bars_ms = []
        bars_mem = []
        bars_cpu = []
        for r in suite_rows:
            color = LANG_COLORS.get(r["lang"], "#888")
            badge = (
                f'<span class="lang-badge" style="--lang-color:{color}">'
                f"{html.escape(r['lang'])}</span>"
            )
            bars_ms.append(
                f'<div class="bar-row">'
                f"{badge}"
                f'<div class="bar-track"><div class="bar-fill" style="width:{bar_width(r["wall_ms"], max_ms)}%;background:{color}"></div></div>'
                f'<span class="bar-value">{fmt_ms(r["wall_ms"])} ms</span></div>'
            )
            bars_mem.append(
                f'<div class="bar-row">'
                f"{badge}"
                f'<div class="bar-track"><div class="bar-fill" style="width:{bar_width(r["peak_rss_kb"], max_mem)}%;background:{color}"></div></div>'
                f'<span class="bar-value">{fmt_mem(r["peak_rss_kb"])}</span></div>'
            )
            bars_cpu.append(
                f'<div class="bar-row">'
                f"{badge}"
                f'<div class="bar-track"><div class="bar-fill" style="width:{bar_width(r["cpu_pct"], max_cpu)}%;background:{color}"></div></div>'
                f'<span class="bar-value">{r["cpu_pct"]:.1f}%</span></div>'
            )

        table_rows = []
        for r in suite_rows:
            color = LANG_COLORS.get(r["lang"], "#888")
            gpu = (
                f'{r["gpu_util_pct"]:.0f}% / {r["gpu_mem_mb"]:.0f} MB'
                if r["gpu_util_pct"] > 0 or r["gpu_mem_mb"] > 0
                else "—"
            )
            table_rows.append(
                f"<tr>"
                f'<td><span class="lang-dot" style="background:{color}"></span> <strong>{html.escape(r["lang"])}</strong></td>'
                f'<td class="num">{fmt_ms(r["wall_ms"])} ms</td>'
                f'<td class="num">{fmt_mem(r["peak_rss_kb"])}</td>'
                f'<td class="num">{r["cpu_user_ms"]:.2f} ms</td>'
                f'<td class="num">{r["cpu_sys_ms"]:.2f} ms</td>'
                f'<td class="num">{r["cpu_pct"]:.1f}%</td>'
                f'<td class="num">{r["minflt"]}</td>'
                f'<td class="num">{r["majflt"]}</td>'
                f'<td class="num">{r["vol_ctx"]}</td>'
                f'<td class="num">{r["invol_ctx"]}</td>'
                f'<td class="num">{gpu}</td>'
                f"</tr>"
            )

        tags = SUITE_TAGS.get(suite, [])
        tags_html = ""
        if tags:
            tags_html = (
                '<div class="suite-tags">'
                + "".join(f'<span class="tag">{html.escape(t)}</span>' for t in tags)
                + "</div>"
            )

        suite_sections.append(
            f"""
        <article class="panel suite-panel" id="suite-{html.escape(suite)}">
          <header class="suite-head">
            <h2 class="suite-title">{html.escape(suite)}</h2>
            <p class="suite-desc">{html.escape(SUITE_INFO.get(suite, ""))}</p>
            {tags_html}
          </header>
          <div class="table-scroll">
            <table class="suite-results-table">
              <thead><tr>
                <th>Language</th><th>Wall time</th><th>Peak RAM</th>
                <th>CPU user</th><th>CPU sys</th><th>CPU %</th>
                <th>Min faults</th><th>Maj faults</th>
                <th>Vol ctx</th><th>Invol ctx</th><th>GPU util / VRAM</th>
              </tr></thead>
              <tbody>{''.join(table_rows)}</tbody>
            </table>
          </div>
          <div class="chart-grid">
            <div class="chart-box"><h3>Runtime</h3>{''.join(bars_ms)}</div>
            <div class="chart-box"><h3>Peak memory</h3>{''.join(bars_mem)}</div>
            <div class="chart-box"><h3>CPU utilization</h3>{''.join(bars_cpu)}</div>
          </div>
        </article>
        """
        )

    suite_toc = "".join(
        f'<a href="#suite-{html.escape(s)}">{html.escape(s)}</a>' for s in suites
    )

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Nyra benchmark — typed vs untyped vs Rust vs Go</title>
  <style>
    :root {{
      --brand: #3d9a8b; --bg: #06090d; --surface: #111820; --surface-2: #161f2a;
      --border: rgba(255,255,255,0.08); --text: #f0f4f8; --muted: #8b9cb0;
      --radius: 14px; --font: system-ui, sans-serif; --mono: ui-monospace, monospace;
    }}
    body {{ margin: 0; font-family: var(--font); background: var(--bg); color: var(--text); line-height: 1.5; }}
    .page {{ max-width: 1100px; margin: 0 auto; padding: 1.5rem; }}
    h1 {{ margin: 0 0 0.5rem; font-size: 1.75rem; }}
    .lead {{ color: var(--muted); max-width: 60ch; }}
    .meta-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 0.75rem; margin: 1.25rem 0; }}
    .meta-card {{ background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 0.75rem 1rem; }}
    .meta-card dt {{ font-size: 0.7rem; text-transform: uppercase; color: var(--muted); }}
    .meta-card dd {{ margin: 0.25rem 0 0; font-weight: 600; }}
    .legend {{ display: flex; flex-wrap: wrap; gap: 0.75rem; margin-bottom: 1rem; }}
    .lang-dot {{ display: inline-block; width: 9px; height: 9px; border-radius: 50%; margin-right: 0.35rem; }}
    .toc-nav {{ display: flex; flex-wrap: wrap; gap: 0.4rem; margin-bottom: 1rem; }}
    .toc-nav a {{ font-size: 0.8rem; padding: 0.3rem 0.6rem; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); text-decoration: none; }}
    .panel {{ background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 1.25rem; margin-bottom: 1.25rem; }}
    .suite-title {{ margin: 0; color: var(--brand); text-transform: capitalize; }}
    .suite-desc {{ margin: 0.25rem 0 0.5rem; color: var(--muted); font-size: 0.85rem; }}
    .suite-tags {{ display: flex; flex-wrap: wrap; gap: 0.35rem; margin-bottom: 0.75rem; }}
    .tag {{
      font-size: 0.65rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;
      color: var(--brand); background: rgba(61,154,139,0.12); border: 1px solid rgba(61,154,139,0.25);
      padding: 0.15rem 0.45rem; border-radius: 4px;
    }}
    .table-scroll {{ overflow-x: auto; margin-bottom: 1rem; }}
    .suite-results-table {{ width: 100%; border-collapse: collapse; font-size: 0.85rem; }}
    .suite-results-table th, .suite-results-table td {{ padding: 0.55rem 0.65rem; border-bottom: 1px solid var(--border); text-align: left; }}
    .suite-results-table thead th {{ background: #1a2430; font-size: 0.68rem; text-transform: uppercase; color: var(--muted); }}
    .num {{ font-family: var(--mono); font-variant-numeric: tabular-nums; }}
    .chart-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 0.75rem; }}
    .chart-box {{ background: var(--surface-2); border: 1px solid var(--border); border-radius: 8px; padding: 0.75rem; }}
    .chart-box h3 {{ margin: 0 0 0.65rem; font-size: 0.8rem; }}
    .bar-row {{ display: grid; grid-template-columns: auto 1fr auto; gap: 0.5rem; align-items: center; margin-bottom: 0.45rem; font-size: 0.8rem; }}
    .lang-badge {{ font-weight: 600; color: var(--lang-color); white-space: nowrap; }}
    .bar-track {{ height: 10px; background: rgba(255,255,255,0.06); border-radius: 5px; overflow: hidden; }}
    .bar-fill {{ height: 100%; border-radius: 5px; min-width: 3px; }}
    .bar-value {{ color: var(--muted); font-family: var(--mono); font-size: 0.75rem; }}
    .note {{ font-size: 0.82rem; color: var(--muted); margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--border); }}
    code {{ font-family: var(--mono); font-size: 0.8rem; background: var(--surface-2); padding: 0.1rem 0.35rem; border-radius: 4px; }}
  </style>
</head>
<body>
  <main class="page">
    <h1>Nyra typed vs untyped vs Rust vs Go</h1>
    <p class="lead">Wall-clock runtime, peak RAM (max RSS), CPU time (user + system), page faults, context switches, and optional NVIDIA GPU sampling. Compile time excluded.</p>
    <dl class="meta-grid">
      <div class="meta-card"><dt>Generated (UTC)</dt><dd>{html.escape(meta["generated"])}</dd></div>
      <div class="meta-card"><dt>Platform</dt><dd>{html.escape(meta["platform"])}</dd></div>
      <div class="meta-card"><dt>Timed runs</dt><dd>{html.escape(meta["runs"])} (warmup {html.escape(meta["warmup"])} discarded)</dd></div>
      <div class="meta-card"><dt>Nyra build</dt><dd>{html.escape(meta["nyra_mode"])}</dd></div>
      <div class="meta-card"><dt>Languages</dt><dd>{html.escape(meta["langs"])}</dd></div>
    </dl>
    <nav class="legend">{legend_html}</nav>
    <nav class="toc-nav">{suite_toc}</nav>
    {''.join(suite_sections)}
    <p class="note">GPU columns show peak samples from <code>nvidia-smi</code> during each run (— on CPU-only / Apple Silicon). CPU% ≈ (user+sys)/wall; can exceed 100% on multi-core. Set <code>BENCH_LANGS=all</code> to include C, C++, Node, Python, Java. Re-run: <code>./scripts/bench.sh</code></p>
  </main>
</body>
</html>
"""


def main() -> None:
    tsv = Path(os.environ["BENCH_TSV"])
    out = Path(os.environ["BENCH_HTML"])
    rows = load_rows(tsv)
    meta = {
        "generated": os.environ["BENCH_GENERATED"],
        "platform": os.environ["BENCH_PLATFORM"],
        "runs": os.environ["BENCH_RUNS"],
        "warmup": os.environ["BENCH_WARMUP"],
        "nyra_mode": os.environ["BENCH_NYRA_MODE"],
        "langs": os.environ.get("BENCH_LANGS_LABEL", ""),
    }
    out.write_text(build_report(rows, meta), encoding="utf-8")


if __name__ == "__main__":
    main()
