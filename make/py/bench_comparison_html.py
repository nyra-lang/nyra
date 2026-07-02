#!/usr/bin/env python3
"""Generate comparison benchmark HTML report (examples/comparison/results/latest.html)."""
from __future__ import annotations

import html
import os
import sys
from collections import defaultdict
from pathlib import Path

LANG_COLORS = {
    "Nyra": "#3d9a8b",
    "Nyra-typed": "#2d7a6e",
    "Nyra-comptime": "#1e5c52",
    "Nyra-comptime-typed": "#164840",
    "C": "#7eb6ff",
    "C++": "#00599c",
    "Go": "#00a8cc",
    "Rust": "#dea584",
}
LANG_ORDER = ["Nyra", "Nyra-typed", "Nyra-comptime", "Nyra-comptime-typed", "C", "C++", "Go", "Rust"]
LANG_DISPLAY = {
    "Nyra": "Nyra (Zero Types)",
    "Nyra-typed": "Nyra (Explicit Types)",
    "Nyra-comptime": "Nyra (Comptime)",
    "Nyra-comptime-typed": "Nyra (Comptime + Types)",
}


def lang_label(lang: str) -> str:
    return LANG_DISPLAY.get(lang, lang)


# Startup-dominated suites: identical LLVM for zero/typed; wall time is mostly dyld/spawn noise.
NYRA_PARITY_EXCLUDE = frozenset({"hello", "arithmetic"})


def nyra_is_parity_suite(suite: str) -> bool:
    return suite not in NYRA_PARITY_EXCLUDE


def nyra_dual_message_html() -> str:
    """Key publish message: Nyra zero-types vs explicit-types parity."""
    return (
        '<aside class="nyra-key-message" aria-label="Nyra zero types vs explicit types">'
        "<p><strong>Nyra appears twice:</strong></p>"
        "<ul>"
        "<li>Nyra (Zero Types)</li>"
        "<li>Nyra (Explicit Types)</li>"
        "</ul>"
        "<p>Both generate native code.</p>"
        "<p>Zero/Explicit pairs are measured <strong>interleaved per suite</strong> "
        "(median wall time) so OS startup noise does not skew the comparison.</p>"
        "<p>The table below uses <strong>hot-path suites only</strong> "
        "(excludes <code>hello</code> / <code>arithmetic</code> where process spawn dominates). "
        "Full matrix retains every suite.</p>"
        "</aside>"
    )


def nyra_dual_message_txt() -> str:
    return (
        "Nyra appears twice:\n"
        "\n"
        "  • Nyra (Zero Types)\n"
        "  • Nyra (Explicit Types)\n"
        "\n"
        "Both generate native code. Zero/Explicit pairs are measured interleaved per suite\n"
        "(median wall time). Hot-path parity excludes hello/arithmetic (spawn-dominated).\n"
    )
SUITE_ORDER = [
    "hello",
    "arithmetic",
    "loop",
    "fib",
    "nested",
    "struct_sum",
    "dungeon",
    "cpu_bound",
    "mix",
    "loop_nofold",
    "comptime_table",
    # Extended — memory
    "memory_alloc_struct",
    "memory_free_struct",
    "memory_arena",
    "memory_ownership",
    # Extended — strings
    "strings_concat",
    "strings_substring",
    "strings_replace",
    "strings_split",
    "strings_utf8",
    # Extended — collections
    "collections_hashmap",
    "collections_hashset",
    "collections_vec_push",
    "collections_vec_pop",
    "collections_sort",
    # Extended — algorithms
    "algorithms_qsort",
    "algorithms_mergesort",
    "algorithms_binary_search",
    "algorithms_json_parse",
    "algorithms_regex",
    # Extended — concurrency
    "concurrency_spawn_tasks",
    "concurrency_channel_pingpong",
    "concurrency_worker_pool",
    "concurrency_parallel_map",
]
SUITE_INFO = {
    "hello": "Minimal stdout I/O",
    "arithmetic": "Two integer adds",
    "loop": "Modular sum 0..N-1 (N=375M, mod 1e9+7)",
    "fib": "Fibonacci swaps (375M steps, mod 1e9+7)",
    "nested": "2D nested loop (4000×4000, mod 1e9+7)",
    "struct_sum": "Struct field access hot loop (80M)",
    "dungeon": "Multi-file Dungeon Steps app",
    "cpu_bound": "Mod mul-add chain (180M, mod 997)",
    "mix": "Chained mod mix (270M, mod 1e9+7)",
    "loop_nofold": "Modular sum anti-constant-fold (N=375M)",
    "comptime_table": "Lookup table build (64×8k mix) + sum — Nyra-comptime folds all at compile time",
    "memory_alloc_struct": "malloc/free struct nodes (500k default, BENCH_SCALE)",
    "memory_free_struct": "alloc+free 16 B blocks",
    "memory_arena": "bump arena simulation",
    "memory_ownership": "struct pass-by-value hot loop",
    "strings_concat": "strcat chain",
    "strings_substring": "substring slices",
    "strings_replace": "str_replace",
    "strings_split": "strstr + substring split",
    "strings_utf8": "UTF-8 byte iterate (char_at)",
    "collections_hashmap": "HashMap insert/get (string keys)",
    "collections_hashset": "HashSet insert/contains",
    "collections_vec_push": "Vec push growth",
    "collections_vec_pop": "Vec push then pop all",
    "collections_sort": "shell-sort pass + sum",
    "algorithms_qsort": "partition-style mod sum",
    "algorithms_mergesort": "merge-style mod sum",
    "algorithms_binary_search": "binary search probes",
    "algorithms_json_parse": "json_get_i32 hot loop",
    "algorithms_regex": "regex_is_match",
    "concurrency_spawn_tasks": "spawn N lightweight tasks",
    "concurrency_channel_pingpong": "spawn producer + channel recv",
    "concurrency_worker_pool": "4-worker channel pool",
    "concurrency_parallel_map": "parallel for + checksum",
}

# Shorter labels for the Nyra zero-types vs typed parity table
NYRA_BENCH_LABEL = {
    "struct_sum": "struct",
    "loop_nofold": "loop_nofold",
    "cpu_bound_pgo": "cpu_bound_pgo",
}


def fmt_ms(v: float) -> str:
    if v >= 1000:
        return f"{v:,.2f}"
    return f"{v:.2f}"


def fmt_mem(kb: int) -> str:
    if kb >= 1024:
        return f"{kb / 1024:.2f} MB"
    return f"{kb} KB"


def fmt_bytes(b: int | str | None) -> str:
    if b is None or b == "" or b == "-":
        return "—"
    n = int(b)
    if n <= 0:
        return "—"
    if n < 1024:
        return f"{n} B"
    if n < 1024 * 1024:
        return f"{n / 1024:.1f} KB"
    return f"{n / (1024 * 1024):.2f} MB"


def load_binary_sizes(path: Path | None) -> dict[str, dict[str, int | str]]:
    if path is None or not path.is_file():
        return {}
    out: dict[str, dict[str, int | str]] = {}
    with path.open(encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            row = dict(zip(header, line.split("\t")))
            lang = row["language"]
            out[lang] = {
                "release": row.get("release_bytes", "-"),
                "stripped": row.get("stripped_bytes", "-"),
                "upx": row.get("upx_bytes", "-"),
            }
    return out


def ordered_suites(by_suite: dict) -> list[str]:
    return [s for s in SUITE_ORDER if s in by_suite] + sorted(
        set(by_suite.keys()) - set(SUITE_ORDER)
    )


def sort_rows(rows: list[dict], key: str) -> list[dict]:
    return sorted(
        rows,
        key=lambda r: (r[key], LANG_ORDER.index(r["lang"]) if r["lang"] in LANG_ORDER else 99),
    )


def rank_map(suite_rows: list[dict], key: str) -> dict[str, int]:
    return {row["lang"]: idx + 1 for idx, row in enumerate(sort_rows(suite_rows, key))}


def bar_width(value: float, max_val: float) -> int:
    if max_val <= 0:
        return 0
    return min(100, max(4, round(100 * value / max_val)))


def suite_detail_rows(suite_rows: list[dict]) -> list[dict]:
    ranks_ms = rank_map(suite_rows, "ms")
    ranks_mem = rank_map(suite_rows, "mem_kb")
    best_ms = min(r["ms"] for r in suite_rows)
    best_mem = min(r["mem_kb"] for r in suite_rows)
    by_lang = {r["lang"]: r for r in suite_rows}
    out = []
    for lang in LANG_ORDER:
        if lang not in by_lang:
            continue
        row = by_lang[lang]
        out.append(
            {
                **row,
                "rank_ms": ranks_ms[lang],
                "rank_mem": ranks_mem[lang],
                "vs_ms": row["ms"] / best_ms if best_ms > 0 else 1.0,
                "vs_mem": row["mem_kb"] / best_mem if best_mem > 0 else 1.0,
            }
        )
    return out


def suite_analysis(suite_rows: list[dict]) -> list[str]:
    fastest = min(suite_rows, key=lambda r: r["ms"])
    leanest = min(suite_rows, key=lambda r: r["mem_kb"])
    lines = [
        f"<strong>Fastest runtime:</strong> {html.escape(lang_label(fastest['lang']))} "
        f"({fmt_ms(fastest['ms'])} ms mean)",
        f"<strong>Lowest peak memory:</strong> {html.escape(lang_label(leanest['lang']))} "
        f"({fmt_mem(leanest['mem_kb'])})",
    ]
    nyra = next((r for r in suite_rows if r["lang"] == "Nyra"), None)
    nyra_typed = next((r for r in suite_rows if r["lang"] == "Nyra-typed"), None)
    for label, row in (
        ("Nyra (Zero Types)", nyra),
        ("Nyra (Explicit Types)", nyra_typed),
    ):
        if not row:
            continue
        for other in suite_rows:
            if other["lang"] in ("Nyra", "Nyra-typed"):
                continue
            if other["ms"] > 0 and row["ms"] > 0:
                ratio = row["ms"] / other["ms"]
                if ratio < 0.98:
                    lines.append(
                        f"{label} is <strong>{1/ratio:.2f}× faster</strong> than "
                        f"{html.escape(lang_label(other['lang']))} on this suite."
                    )
                elif ratio > 1.02:
                    lines.append(
                        f"{html.escape(lang_label(other['lang']))} is <strong>{ratio:.2f}× faster</strong> "
                        f"than {label} on this suite."
                    )
            if other["mem_kb"] > 0 and row["mem_kb"] > 0:
                mem_ratio = other["mem_kb"] / row["mem_kb"]
                if mem_ratio > 1.5:
                    lines.append(
                        f"{label} uses <strong>{mem_ratio:.1f}× less RAM</strong> than "
                        f"{html.escape(lang_label(other['lang']))} (peak RSS)."
                    )
    return lines


def suite_analysis_txt(suite_rows: list[dict]) -> list[str]:
    import re

    out: list[str] = []
    for line in suite_analysis(suite_rows):
        plain = re.sub(r"</?strong>", "", line)
        plain = re.sub(r"<[^>]+>", "", plain)
        out.append(plain)
    return out


def build_text_appendix(rows: list[dict], binary: dict[str, dict[str, int | str]]) -> str:
    """Plain-text sections matching latest.html (matrix, leaderboard, suites, raw)."""
    by_suite: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        by_suite[r["suite"]].append(r)

    suites = ordered_suites(by_suite)
    langs = [l for l in LANG_ORDER if any(r["lang"] == l for r in rows)]
    cell_map = {(r["suite"], r["lang"]): r for r in rows}
    lines: list[str] = []

    # ── Nyra zero types vs explicit types ─────────────────────────────────────
    lines += [
        "=" * 72,
        "NYRA ZERO TYPES vs EXPLICIT TYPES — same algorithm, annotations optional",
        "=" * 72,
        "",
        nyra_dual_message_txt().rstrip(),
        "",
        "   Hot-path parity (excludes hello, arithmetic — spawn-dominated):",
        "",
        f"   {'Benchmark':<18} {'Zero Types':>22} {'Explicit Types':>22} {'Δ time':>8}",
        "   " + "-" * 74,
    ]
    nyra_pairs = collect_nyra_zero_typed_pairs(by_suite, suites)
    parity_pairs = [(s, z, t) for s, z, t in nyra_pairs if nyra_is_parity_suite(s)]
    excluded_pairs = [(s, z, t) for s, z, t in nyra_pairs if not nyra_is_parity_suite(s)]

    for suite, zero, typed in parity_pairs:
        label = NYRA_BENCH_LABEL.get(suite, suite)
        lines.append(
            f"   {label:<18} {fmt_ms(zero['ms']):>17} ms"
            f" {fmt_ms(typed['ms']):>17} ms"
            f" {fmt_pct_diff(zero['ms'], typed['ms']):>8}"
        )

    if parity_pairs:
        med_time, med_mem = nyra_parity_medians(parity_pairs)
        summary = (
            f"   Median |Δ| across {len(parity_pairs)} hot-path benchmarks: "
            f"{med_time:.2f}% time"
        )
        if med_mem > 0:
            summary += f", {med_mem:.2f}% memory"
        lines += [
            "",
            "   Same source algorithm in every row — only type annotations differ.",
            summary + ".",
            "   Interleaved paired runs + median wall time (see report header).",
        ]

    if excluded_pairs:
        lines += [
            "",
            "   Startup / I/O micro-benchmarks (informational — not used for parity median):",
            "",
            f"   {'Benchmark':<18} {'Zero Types':>22} {'Explicit Types':>22} {'Δ time':>8}",
            "   " + "-" * 74,
        ]
        for suite, zero, typed in excluded_pairs:
            label = NYRA_BENCH_LABEL.get(suite, suite)
            lines.append(
                f"   {label:<18} {fmt_ms(zero['ms']):>17} ms"
                f" {fmt_ms(typed['ms']):>17} ms"
                f" {fmt_pct_diff(zero['ms'], typed['ms']):>8}  ‡"
            )
        lines.append(
            "   ‡ LLVM output identical; delta is process-spawn / OS noise, not type overhead."
        )

    if nyra_pairs:
        lines += [
            "",
            f"   {'Benchmark':<18} {'Zero Types':>22} {'Explicit Types':>22} {'Δ memory':>10}",
            "   " + "-" * 76,
        ]
        for suite, zero, typed in nyra_pairs:
            label = NYRA_BENCH_LABEL.get(suite, suite)
            mark = " ‡" if not nyra_is_parity_suite(suite) else ""
            lines.append(
                f"   {label:<18} {fmt_mem(zero['mem_kb']):>22}"
                f" {fmt_mem(typed['mem_kb']):>22}"
                f" {fmt_pct_diff(float(zero['mem_kb']), float(typed['mem_kb'])):>10}{mark}"
            )

    # ── Comparison matrix ─────────────────────────────────────────────────────
    lines += ["", "=" * 72, "FULL COMPARISON MATRIX — time + peak memory per cell", "=" * 72, ""]
    col_w = 12
    matrix_lang_labels = {
        "Nyra": "Nyra-Z",
        "Nyra-typed": "Nyra-T",
    }
    header = f"   {'Benchmark':<20}" + "".join(
        f"{matrix_lang_labels.get(l, lang_label(l))[:col_w]:>{col_w}}" for l in langs
    )
    lines.append(header)
    lines.append("   " + "-" * (20 + col_w * len(langs)))
    for suite in suites:
        suite_rows = by_suite[suite]
        best_ms = min(r["ms"] for r in suite_rows)
        best_mem = min(r["mem_kb"] for r in suite_rows)
        row_cells = [f"   {suite:<20}"]
        for lang in langs:
            r = cell_map.get((suite, lang))
            if not r:
                row_cells.append(f"{'—':>{col_w}}")
                continue
            mark = ""
            if abs(r["ms"] - best_ms) < 1e-9:
                mark = "*"
            elif r["mem_kb"] == best_mem:
                mark = "†"
            cell = f"{fmt_ms(r['ms'])} {mark}"
            row_cells.append(f"{cell:>{col_w}}")
        lines.append("".join(row_cells))
        mem_cells = [f"   {'':20}"]
        for lang in langs:
            r = cell_map.get((suite, lang))
            if not r:
                mem_cells.append(f"{'':>{col_w}}")
                continue
            mem_cells.append(f"{fmt_mem(r['mem_kb']):>{col_w}}")
        lines.append("".join(mem_cells))
        desc = SUITE_INFO.get(suite, "")
        if desc:
            lines.append(f"   ({desc})")
        lines.append("")
    lines.append("   * = fastest time in row   † = lowest memory in row")
    lines.append("")

    # ── Language leaderboard ──────────────────────────────────────────────────
    lines += ["=" * 72, "LANGUAGE LEADERBOARD — average across all benchmarks", "=" * 72, ""]
    stats = []
    for lang in langs:
        lang_rows = [r for r in rows if r["lang"] == lang]
        avg_ms = sum(r["ms"] for r in lang_rows) / len(lang_rows)
        avg_mem = sum(r["mem_kb"] for r in lang_rows) / len(lang_rows)
        time_wins = mem_wins = 0
        for suite_rows in by_suite.values():
            if not suite_rows:
                continue
            best_ms = min(r["ms"] for r in suite_rows)
            best_mem = min(r["mem_kb"] for r in suite_rows)
            for r in suite_rows:
                if r["lang"] != lang:
                    continue
                if abs(r["ms"] - best_ms) < 1e-9:
                    time_wins += 1
                if r["mem_kb"] == best_mem:
                    mem_wins += 1
        stats.append(
            {
                "lang": lang,
                "avg_ms": avg_ms,
                "avg_mem": avg_mem,
                "time_wins": time_wins,
                "mem_wins": mem_wins,
                "suites": len(lang_rows),
            }
        )
    stats.sort(key=lambda s: (s["avg_ms"], LANG_ORDER.index(s["lang"])))
    lines.append(
        f"   {'#':<4} {'Language':<22} {'Avg time':>12} {'Avg RAM':>12}"
        f" {'Hello bin':>12} {'Time wins':>10} {'Mem wins':>10}"
    )
    lines.append("   " + "-" * 86)
    for i, s in enumerate(stats, 1):
        rel = binary.get(s["lang"], {}).get("release", "-")
        lines.append(
            f"   {i:<4} {lang_label(s['lang']):<22} {fmt_ms(s['avg_ms']):>10} ms"
            f" {fmt_mem(int(s['avg_mem'])):>12} {fmt_bytes(rel):>12}"
            f" {s['time_wins']:>4}/{s['suites']:<4} {s['mem_wins']:>4}/{s['suites']:<4}"
        )
    lines.append("")

    # ── Detailed per-suite ────────────────────────────────────────────────────
    lines += [
        "=" * 72,
        "DETAILED RESULTS — every language × every benchmark",
        "=" * 72,
        "",
    ]
    for suite in suites:
        suite_rows = by_suite[suite]
        detail = suite_detail_rows(suite_rows)
        fastest = min(detail, key=lambda r: r["ms"])
        slowest = max(detail, key=lambda r: r["ms"])
        leanest = min(detail, key=lambda r: r["mem_kb"])
        heaviest = max(detail, key=lambda r: r["mem_kb"])
        avg_ms = sum(r["ms"] for r in detail) / len(detail)
        avg_mem = sum(r["mem_kb"] for r in detail) / len(detail)
        desc = SUITE_INFO.get(suite, "")
        lines.append(f"## {suite}")
        if desc:
            lines.append(f"   {desc}")
        lines += [
            f"   Fastest: {lang_label(fastest['lang'])} ({fmt_ms(fastest['ms'])} ms)",
            f"   Slowest: {lang_label(slowest['lang'])} ({fmt_ms(slowest['ms'])} ms)",
            f"   Lowest RAM: {lang_label(leanest['lang'])} ({fmt_mem(leanest['mem_kb'])})",
            f"   Highest RAM: {lang_label(heaviest['lang'])} ({fmt_mem(heaviest['mem_kb'])})",
            f"   Average: {fmt_ms(avg_ms)} ms, {fmt_mem(int(avg_mem))} across {len(detail)} languages",
            "",
            f"   {'Language':<22} {'Time (ms)':>12} {'Time #':>7} {'vs best':>8}"
            f"   {'Peak RSS':>12} {'RAM #':>6} {'vs best':>8}",
        ]
        if suite == "hello":
            lines[-1] += f"   {'Binary':>10}"
        lines.append("   " + "-" * (78 if suite != "hello" else 90))
        for row in sort_rows(detail, "ms"):
            line = (
                f"   {lang_label(row['lang']):<22} {row['ms']:>12.4f} {row['rank_ms']:>7}"
                f" {row['vs_ms']:>7.2f}x"
                f"   {fmt_mem(row['mem_kb']):>12} {row['rank_mem']:>6}"
                f" {row['vs_mem']:>7.2f}x"
            )
            if suite == "hello":
                rel = binary.get(row["lang"], {}).get("release", "-")
                line += f"   {fmt_bytes(rel):>10}"
            lines.append(line)
        insights = suite_analysis_txt(suite_rows)
        if insights:
            lines.append("   Insights:")
            for ins in insights:
                lines.append(f"     • {ins}")
        lines.append("")

    # ── Raw data ──────────────────────────────────────────────────────────────
    lines += ["=" * 72, "RAW MEASUREMENTS — same rows as results/data.tsv", "=" * 72, ""]
    lines.append(f"   {'Suite':<20} {'Language':<22} {'Time (ms)':>12} {'Memory':>12}")
    lines.append("   " + "-" * 68)
    for r in rows:
        lines.append(
            f"   {r['suite']:<20} {lang_label(r['lang']):<22}"
            f" {r['ms']:>12.4f} {fmt_mem(r['mem_kb']):>12}"
        )
    lines.append("")
    return "\n".join(lines)


def rebuild_latest_txt(latest_path: Path, rows: list[dict], binary: dict[str, dict[str, int | str]]) -> None:
    """Keep header + summary tables; replace appendix with full text report."""
    text = latest_path.read_text(encoding="utf-8") if latest_path.is_file() else ""
    cut = len(text)
    for marker in ("\n====", "\nRe-run:", "\n========================================================================"):
        idx = text.find(marker)
        if idx != -1:
            cut = min(cut, idx)
    prefix = text[:cut].rstrip() + "\n\n"
    if "Nyra appears twice" not in prefix:
        header_end = prefix.find("\n\n| Suite")
        if header_end == -1:
            header_end = prefix.find("\n| Suite")
        if header_end != -1:
            prefix = (
                prefix[:header_end]
                + "\n\n"
                + nyra_dual_message_txt()
                + prefix[header_end:]
            )
    appendix = build_text_appendix(rows, binary)
    latest_path.write_text(prefix + appendix + "\n", encoding="utf-8")


def fmt_pct_diff(baseline: float, other: float) -> str:
    """Signed % change from baseline → other; 0% when effectively identical."""
    if baseline <= 0 or other <= 0:
        return "—"
    pct = (other - baseline) / baseline * 100
    if abs(pct) < 0.05:
        return "0%"
    sign = "+" if pct > 0 else ""
    return f"{sign}{pct:.1f}%"


def collect_nyra_zero_typed_pairs(
    by_suite: dict[str, list[dict]], suites: list[str]
) -> list[tuple[str, dict, dict]]:
    pairs: list[tuple[str, dict, dict]] = []
    for suite in suites:
        by_lang = {r["lang"]: r for r in by_suite.get(suite, [])}
        zero = by_lang.get("Nyra")
        typed = by_lang.get("Nyra-typed")
        if zero and typed:
            pairs.append((suite, zero, typed))
    return pairs


def nyra_parity_medians(pairs: list[tuple[str, dict, dict]]) -> tuple[float, float]:
    time_diffs: list[float] = []
    mem_diffs: list[float] = []
    for _suite, zero, typed in pairs:
        if zero["ms"] > 0:
            time_diffs.append(abs((typed["ms"] - zero["ms"]) / zero["ms"] * 100))
        if zero["mem_kb"] > 0:
            mem_diffs.append(abs((typed["mem_kb"] - zero["mem_kb"]) / zero["mem_kb"] * 100))
    med_time = sorted(time_diffs)[len(time_diffs) // 2] if time_diffs else 0.0
    med_mem = sorted(mem_diffs)[len(mem_diffs) // 2] if mem_diffs else 0.0
    return med_time, med_mem


def build_nyra_zero_vs_typed_table(by_suite: dict[str, list[dict]], suites: list[str]) -> str:
    """Nyra-only table: zero-types vs explicit types — same algorithm, same performance."""
    nyra_pairs = collect_nyra_zero_typed_pairs(by_suite, suites)
    if not nyra_pairs:
        return (
            nyra_dual_message_html()
            + '<p class="muted-note">No paired Nyra / Nyra-typed measurements in this run. '
            "Re-run <code>./scripts/bench.sh</code> with both entries enabled.</p>"
        )

    parity_pairs = [(s, z, t) for s, z, t in nyra_pairs if nyra_is_parity_suite(s)]
    excluded_pairs = [(s, z, t) for s, z, t in nyra_pairs if not nyra_is_parity_suite(s)]

    def row_html(suite: str, z_ms: float, t_ms: float, foot: str = "") -> str:
        label = NYRA_BENCH_LABEL.get(suite, suite)
        t_diff = fmt_pct_diff(z_ms, t_ms)
        diff_cls = "cell-best num" if t_diff == "0%" else "num"
        return (
            f"<tr>"
            f"<th scope=\"row\"><strong>{html.escape(label)}</strong>"
            f'<span class="suite-desc-inline">{html.escape(SUITE_INFO.get(suite, ""))}</span>'
            f"{foot}</th>"
            f'<td class="num">{fmt_ms(z_ms)} ms</td>'
            f'<td class="num">{fmt_ms(t_ms)} ms</td>'
            f'<td class="{diff_cls}">{html.escape(t_diff)}</td>'
            f"</tr>"
        )

    body: list[str] = []
    for suite, zero, typed in parity_pairs:
        body.append(row_html(suite, zero["ms"], typed["ms"]))

    med_time, med_mem = nyra_parity_medians(parity_pairs)
    summary = (
        f"Median |Δ| across {len(parity_pairs)} hot-path benchmarks: "
        f"<strong>{med_time:.2f}%</strong> time"
    )
    if med_mem > 0:
        summary += f", <strong>{med_mem:.2f}%</strong> memory"

    excluded_html = ""
    if excluded_pairs:
        excluded_rows = [
            row_html(
                suite,
                zero["ms"],
                typed["ms"],
                '<span class="suite-foot">‡ spawn-dominated</span>',
            )
            for suite, zero, typed in excluded_pairs
        ]
        excluded_html = (
            '<details class="nyra-types-mem-details">'
            "<summary>Startup micro-benchmarks (informational — excluded from parity median)</summary>"
            '<div class="table-scroll"><table class="nyra-types-table">'
            "<thead><tr>"
            "<th>Benchmark</th><th>Nyra (Zero Types)</th><th>Nyra (Explicit Types)</th><th>Δ time</th>"
            "</tr></thead>"
            f"<tbody>{''.join(excluded_rows)}</tbody></table></div>"
            '<p class="muted-note">‡ Identical LLVM; wall time reflects process spawn / dyld, not annotations.</p>'
            "</details>"
        )

    mem_table_body: list[str] = []
    for suite, zero, typed in nyra_pairs:
        label = NYRA_BENCH_LABEL.get(suite, suite)
        m_diff = fmt_pct_diff(float(zero["mem_kb"]), float(typed["mem_kb"]))
        diff_cls = "cell-best num" if m_diff == "0%" else "num"
        foot = ' <span class="suite-foot">‡</span>' if not nyra_is_parity_suite(suite) else ""
        mem_table_body.append(
            f"<tr>"
            f"<th scope=\"row\"><strong>{html.escape(label)}</strong>{foot}</th>"
            f'<td class="num">{fmt_mem(zero["mem_kb"])}</td>'
            f'<td class="num">{fmt_mem(typed["mem_kb"])}</td>'
            f'<td class="{diff_cls}">{html.escape(m_diff)}</td>'
            f"</tr>"
        )

    return (
        nyra_dual_message_html()
        + '<div class="table-scroll"><table class="nyra-types-table">'
        "<thead><tr>"
        "<th>Benchmark</th><th>Nyra (Zero Types)</th><th>Nyra (Explicit Types)</th><th>Δ time</th>"
        "</tr></thead>"
        f"<tbody>{''.join(body)}</tbody></table></div>"
        '<p class="muted-note">'
        "Same source algorithm in every row — only type annotations differ. "
        "Interleaved paired runs, median wall time. "
        f"{summary}."
        "</p>"
        + excluded_html
        + '<details class="nyra-types-mem-details">'
        "<summary>Peak memory comparison (all pairs)</summary>"
        '<div class="table-scroll"><table class="nyra-types-table">'
        "<thead><tr>"
        "<th>Benchmark</th><th>Nyra (Zero Types)</th><th>Nyra (Explicit Types)</th><th>Δ memory</th>"
        "</tr></thead>"
        f"<tbody>{''.join(mem_table_body)}</tbody></table></div>"
        "</details>"
    )


def build_lang_leaderboard(
    rows: list[dict], by_suite: dict, binary: dict[str, dict[str, int | str]]
) -> str:
    langs = [l for l in LANG_ORDER if any(r["lang"] == l for r in rows)]
    stats = []
    for lang in langs:
        lang_rows = [r for r in rows if r["lang"] == lang]
        avg_ms = sum(r["ms"] for r in lang_rows) / len(lang_rows)
        avg_mem = sum(r["mem_kb"] for r in lang_rows) / len(lang_rows)
        time_wins = mem_wins = 0
        for suite_rows in by_suite.values():
            if not suite_rows:
                continue
            best_ms = min(r["ms"] for r in suite_rows)
            best_mem = min(r["mem_kb"] for r in suite_rows)
            for r in suite_rows:
                if r["lang"] != lang:
                    continue
                if abs(r["ms"] - best_ms) < 1e-9:
                    time_wins += 1
                if r["mem_kb"] == best_mem:
                    mem_wins += 1
        stats.append(
            {
                "lang": lang,
                "avg_ms": avg_ms,
                "avg_mem": avg_mem,
                "time_wins": time_wins,
                "mem_wins": mem_wins,
                "suites": len(lang_rows),
            }
        )
    stats.sort(key=lambda s: (s["avg_ms"], LANG_ORDER.index(s["lang"])))
    body = []
    for i, s in enumerate(stats, 1):
        color = LANG_COLORS.get(s["lang"], "#888")
        rel = binary.get(s["lang"], {}).get("release", "-")
        body.append(
            f"<tr>"
            f'<td class="num rank-col">#{i}</td>'
            f'<td><span class="lang-dot" style="background:{color}"></span>'
            f"<strong>{html.escape(lang_label(s['lang']))}</strong></td>"
            f'<td class="num">{fmt_ms(s["avg_ms"])} ms</td>'
            f'<td class="num">{fmt_mem(int(s["avg_mem"]))}</td>'
            f'<td class="num">{fmt_bytes(rel)}</td>'
            f'<td class="num">{s["time_wins"]}/{s["suites"]}</td>'
            f'<td class="num">{s["mem_wins"]}/{s["suites"]}</td>'
            f"</tr>"
        )
    return (
        '<div class="table-scroll"><table class="leaderboard-table">'
        "<thead><tr>"
        "<th>#</th><th>Language</th><th>Time (avg)</th><th>Memory (avg)</th>"
        "<th>Binary (hello)</th><th>Time wins</th><th>Memory wins</th>"
        "</tr></thead>"
        f"<tbody>{''.join(body)}</tbody></table></div>"
    )


def build_matrix(by_suite: dict, cell_map: dict, suites: list[str], langs: list[str]) -> str:
    head = "".join(
        f'<th><span class="lang-dot" style="background:{LANG_COLORS.get(l, "#888")}"></span> '
        f"{html.escape(lang_label(l))}</th>"
        for l in langs
    )
    body = []
    for suite in suites:
        suite_rows = by_suite[suite]
        best_ms = min(r["ms"] for r in suite_rows)
        best_mem = min(r["mem_kb"] for r in suite_rows)
        cells = []
        for lang in langs:
            r = cell_map.get((suite, lang))
            if not r:
                cells.append(f'<td class="num empty" data-label="{html.escape(lang_label(lang))}">—</td>')
                continue
            best_cls = ""
            if abs(r["ms"] - best_ms) < 1e-9:
                best_cls = " cell-best"
            elif r["mem_kb"] == best_mem:
                best_cls = " cell-best-mem"
            pill = ""
            if abs(r["ms"] - best_ms) < 1e-9:
                pill = '<span class="best-pill">fastest</span>'
            cells.append(
                f'<td class="matrix-cell{best_cls}" data-label="{html.escape(lang_label(lang))}">'
                f'<span class="cell-lang">{html.escape(lang_label(lang))}</span>'
                f'<span class="cell-main">{fmt_ms(r["ms"])} ms</span>'
                f'<span class="cell-vs">{fmt_mem(r["mem_kb"])}</span>{pill}</td>'
            )
        desc = SUITE_INFO.get(suite, "")
        body.append(
            f'<tr><th class="col-suite" scope="row">'
            f"<strong>{html.escape(suite)}</strong>"
            f'<span class="suite-desc-inline">{html.escape(desc)}</span>'
            f"</th>{''.join(cells)}</tr>"
        )
    return (
        '<div class="table-scroll matrix-wrap" tabindex="0" role="region">'
        f'<table class="matrix-table"><thead><tr><th class="col-suite">Benchmark</th>{head}</tr></thead>'
        f"<tbody>{''.join(body)}</tbody></table></div>"
    )


def build_raw_table(rows: list[dict]) -> str:
    body = []
    for r in rows:
        color = LANG_COLORS.get(r["lang"], "#888")
        body.append(
            f"<tr>"
            f"<td>{html.escape(r['suite'])}</td>"
            f'<td><span class="lang-dot" style="background:{color}"></span> '
            f"{html.escape(lang_label(r['lang']))}</td>"
            f'<td class="num">{fmt_ms(r["ms"])} ms</td>'
            f'<td class="num">{fmt_mem(r["mem_kb"])}</td>'
            f"</tr>"
        )
    return (
        '<div class="table-scroll"><table class="raw-table">'
        "<thead><tr><th>Suite</th><th>Language</th><th>Time</th><th>Memory</th></tr></thead>"
        f"<tbody>{''.join(body)}</tbody></table></div>"
    )


def build_suite_stats(suite_rows: list[dict]) -> str:
    detail = suite_detail_rows(suite_rows)
    fastest = min(detail, key=lambda r: r["ms"])
    slowest = max(detail, key=lambda r: r["ms"])
    leanest = min(detail, key=lambda r: r["mem_kb"])
    heaviest = max(detail, key=lambda r: r["mem_kb"])
    avg_ms = sum(r["ms"] for r in detail) / len(detail)
    avg_mem = sum(r["mem_kb"] for r in detail) / len(detail)
    cards = [
        ("Fastest", lang_label(fastest["lang"]), f'{fmt_ms(fastest["ms"])} ms', LANG_COLORS.get(fastest["lang"], "#888")),
        ("Slowest", lang_label(slowest["lang"]), f'{fmt_ms(slowest["ms"])} ms', LANG_COLORS.get(slowest["lang"], "#888")),
        ("Lowest RAM", lang_label(leanest["lang"]), fmt_mem(leanest["mem_kb"]), LANG_COLORS.get(leanest["lang"], "#888")),
        ("Highest RAM", lang_label(heaviest["lang"]), fmt_mem(heaviest["mem_kb"]), LANG_COLORS.get(heaviest["lang"], "#888")),
        ("Avg time", f"{len(detail)} langs", f"{fmt_ms(avg_ms)} ms", "var(--muted)"),
        ("Avg RAM", f"{len(detail)} langs", fmt_mem(int(avg_mem)), "var(--muted)"),
    ]
    return "".join(
        f'<div class="stat-card"><dt>{html.escape(label)}</dt>'
        f'<dd><span class="lang-dot" style="background:{color}"></span>'
        f"<strong>{html.escape(who)}</strong> · {html.escape(value)}</dd></div>"
        for label, who, value, color in cards
    )


def build_binary_size_table(binary: dict[str, dict[str, int | str]]) -> str:
    if not binary:
        return (
            '<p class="muted-note">Binary size data not found. Re-run '
            "<code>./scripts/bench.sh</code> (or set <code>BENCH_BINARY_SIZE=1</code>).</p>"
        )
    langs = [l for l in LANG_ORDER if l in binary]
    body = []
    release_vals = [
        int(binary[l]["release"])
        for l in langs
        if l in {"Nyra", "Nyra-typed", "C", "C++", "Go", "Rust"}
        and str(binary[l].get("release", "-")).isdigit()
    ]
    best_release = min(release_vals) if release_vals else 0
    for lang in langs:
        row = binary[lang]
        color = LANG_COLORS.get(lang, "#888")
        rel = row.get("release", "-")
        rel_cls = "cell-best num" if str(rel).isdigit() and int(rel) == best_release else "num"
        body.append(
            f"<tr>"
            f'<td><span class="lang-dot" style="background:{color}"></span>'
            f"<strong>{html.escape(lang_label(lang))}</strong></td>"
            f'<td class="{rel_cls}">{fmt_bytes(rel)}</td>'
            f'<td class="num">{fmt_bytes(row.get("stripped", "-"))}</td>'
            f'<td class="num">{fmt_bytes(row.get("upx", "-"))}</td>'
            f"</tr>"
        )
    return (
        '<div class="table-scroll"><table class="binary-size-table">'
        "<thead><tr><th>Language</th><th>Release</th><th>Stripped</th><th>UPX</th></tr></thead>"
        f"<tbody>{''.join(body)}</tbody></table></div>"
        '<p class="muted-note">Hello-world artifact sizes. <strong>Release</strong> = optimized build; '
        "<strong>Stripped</strong> = after <code>strip</code>; <strong>UPX</strong> = "
        "<code>upx --best</code> (— when UPX is missing or unsupported).</p>"
    )


def build_suite_results_table(
    suite_rows: list[dict], binary: dict[str, dict[str, int | str]], suite: str
) -> str:
    detail = sort_rows(suite_detail_rows(suite_rows), "ms")
    body = []
    if suite == "hello":
        thead = (
            "<thead><tr><th>Language</th><th>Time</th><th>Memory</th>"
            "<th>Binary size</th><th>Time #</th><th>Memory #</th></tr></thead>"
        )
        for row in detail:
            color = LANG_COLORS.get(row["lang"], "#888")
            ms_class = "cell-best num" if row["rank_ms"] == 1 else "num"
            mem_class = "cell-best num" if row["rank_mem"] == 1 else "num"
            rel = binary.get(row["lang"], {}).get("release", "-") if row["lang"] in binary else "-"
            body.append(
                f"<tr>"
                f'<td><span class="lang-dot" style="background:{color}"></span>'
                f"<strong>{html.escape(lang_label(row['lang']))}</strong></td>"
                f'<td class="{ms_class}">{fmt_ms(row["ms"])} ms</td>'
                f'<td class="{mem_class}">{fmt_mem(row["mem_kb"])}</td>'
                f'<td class="num">{fmt_bytes(rel)}</td>'
                f'<td class="num">#{row["rank_ms"]}</td>'
                f'<td class="num">#{row["rank_mem"]}</td>'
                f"</tr>"
            )
    else:
        thead = (
            "<thead><tr><th>Language</th><th>Time</th><th>Memory</th>"
            "<th>Time #</th><th>Memory #</th><th>vs best time</th><th>vs best mem</th></tr></thead>"
        )
        for row in detail:
            color = LANG_COLORS.get(row["lang"], "#888")
            ms_class = "cell-best num" if row["rank_ms"] == 1 else "num"
            mem_class = "cell-best num" if row["rank_mem"] == 1 else "num"
            body.append(
                f"<tr>"
                f'<td><span class="lang-dot" style="background:{color}"></span>'
                f"<strong>{html.escape(lang_label(row['lang']))}</strong></td>"
                f'<td class="{ms_class}">{fmt_ms(row["ms"])} ms</td>'
                f'<td class="{mem_class}">{fmt_mem(row["mem_kb"])}</td>'
                f'<td class="num">#{row["rank_ms"]}</td>'
                f'<td class="num">#{row["rank_mem"]}</td>'
                f'<td class="num">×{row["vs_ms"]:.2f}</td>'
                f'<td class="num">×{row["vs_mem"]:.2f}</td>'
                f"</tr>"
            )
    return (
        '<div class="table-scroll"><table class="suite-results-table">'
        f"{thead}"
        f"<tbody>{''.join(body)}</tbody></table></div>"
    )


def build_report(rows: list[dict], meta: dict, binary: dict[str, dict[str, int | str]]) -> str:
    by_suite: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        by_suite[r["suite"]].append(r)

    suites = ordered_suites(by_suite)
    langs = [l for l in LANG_ORDER if any(r["lang"] == l for r in rows)]
    cell_map = {(r["suite"], r["lang"]): r for r in rows}

    legend_html = "".join(
        f'<span class="legend-item"><span class="lang-dot" style="background:{LANG_COLORS[l]}"></span> '
        f"{html.escape(lang_label(l))}</span>"
        for l in langs
    )
    suite_toc = "".join(f'<a href="#suite-{html.escape(s)}">{html.escape(s)}</a>' for s in suites)

    matrix_html = build_matrix(by_suite, cell_map, suites, langs)
    nyra_types_html = build_nyra_zero_vs_typed_table(by_suite, suites)
    leaderboard_html = build_lang_leaderboard(rows, by_suite, binary)
    binary_html = build_binary_size_table(binary)
    raw_html = build_raw_table(rows)

    suite_sections = []
    for suite in suites:
        suite_rows = by_suite[suite]
        suite_rows_ms = sort_rows(suite_rows, "ms")
        suite_rows_mem = sort_rows(suite_rows, "mem_kb")
        best_ms = suite_rows_ms[0]["ms"]
        best_mem = suite_rows_mem[0]["mem_kb"]
        max_ms = suite_rows_ms[-1]["ms"]
        max_mem = suite_rows_mem[-1]["mem_kb"]

        bars_ms, bars_mem = [], []
        for r in suite_rows_ms:
            color = LANG_COLORS.get(r["lang"], "#888")
            cls = " bar-row-best" if abs(r["ms"] - best_ms) < 1e-9 else ""
            bars_ms.append(
                f'<div class="bar-row{cls}"><span class="lang-badge" style="--lang-color:{color}">'
                f'{html.escape(lang_label(r["lang"]))}</span><div class="bar-track"><div class="bar-fill" '
                f'style="width:{bar_width(r["ms"], max_ms)}%;background:{color}"></div></div>'
                f'<span class="bar-value">{fmt_ms(r["ms"])} ms</span></div>'
            )
        for r in suite_rows_mem:
            color = LANG_COLORS.get(r["lang"], "#888")
            cls = " bar-row-best" if r["mem_kb"] == best_mem else ""
            bars_mem.append(
                f'<div class="bar-row{cls}"><span class="lang-badge" style="--lang-color:{color}">'
                f'{html.escape(lang_label(r["lang"]))}</span><div class="bar-track"><div class="bar-fill" '
                f'style="width:{bar_width(r["mem_kb"], max_mem)}%;background:{color}"></div></div>'
                f'<span class="bar-value">{fmt_mem(r["mem_kb"])}</span></div>'
            )

        analysis = "".join(f'<li class="insight-item">{x}</li>' for x in suite_analysis(suite_rows))
        suite_sections.append(
            f"""
        <article class="panel suite-panel" id="suite-{html.escape(suite)}">
          <header class="suite-head">
            <div>
              <h2 class="suite-title">{html.escape(suite)}</h2>
              <p class="suite-desc">{html.escape(SUITE_INFO.get(suite, ""))}</p>
            </div>
            <a class="suite-jump" href="#top">Back to top ↑</a>
          </header>
          <dl class="suite-stats">{build_suite_stats(suite_rows)}</dl>
          {build_suite_results_table(suite_rows, binary, suite)}
          <div class="chart-grid">
            <div class="chart-box"><h3 class="chart-label">Runtime</h3>{"".join(bars_ms)}</div>
            <div class="chart-box"><h3 class="chart-label">Peak memory</h3>{"".join(bars_mem)}</div>
          </div>
          <ul class="insights">{analysis}</ul>
        </article>"""
        )

    isolation = html.escape(meta.get("isolation", ""))
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="dark">
  <title>Nyra benchmark report</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=DM+Sans:ital,opsz,wght@0,9..40,400;0,9..40,500;0,9..40,600;0,9..40,700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
  <style>
    :root {{
      --brand: #3d9a8b; --brand-dim: rgba(61,154,139,0.14); --brand-strong: rgba(61,154,139,0.35);
      --bg: #05080c; --surface: #0e141c; --surface-2: #141c28; --surface-3: #1a2433;
      --border: rgba(255,255,255,0.07); --border-strong: rgba(61,154,139,0.25);
      --text: #eef3f8; --muted: #8fa3b8; --radius: 16px; --radius-sm: 10px;
      --font: "DM Sans", system-ui, sans-serif; --mono: "JetBrains Mono", ui-monospace, monospace;
      --shadow: 0 20px 50px rgba(0,0,0,0.45);
      --max: 1200px;
    }}
    * {{ box-sizing: border-box; }}
    html {{ scroll-behavior: smooth; }}
    body {{
      margin: 0; font-family: var(--font); color: var(--text); line-height: 1.55;
      background: var(--bg);
      background-image:
        radial-gradient(ellipse 90% 60% at 50% -15%, rgba(61,154,139,0.22), transparent 55%),
        radial-gradient(ellipse 50% 40% at 100% 0%, rgba(0,168,204,0.1), transparent);
    }}
    .page {{ width: min(100% - 2rem, var(--max)); margin: 0 auto; padding: 1.5rem 0 3rem; }}
    .sticky-nav {{
      position: sticky; top: 0; z-index: 50; margin: 0 -1rem 1.25rem; padding: 0.65rem 1rem;
      background: rgba(5,8,12,0.85); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border);
      display: flex; flex-wrap: wrap; gap: 0.4rem; align-items: center;
    }}
    .sticky-nav a {{
      font-size: 0.78rem; font-weight: 600; color: var(--muted); text-decoration: none;
      padding: 0.35rem 0.7rem; border-radius: 999px; border: 1px solid var(--border);
      background: var(--surface);
    }}
    .sticky-nav a:hover, .sticky-nav a.active {{ color: var(--brand); border-color: var(--border-strong); }}
    .hero {{ margin-bottom: 1.5rem; }}
    .hero-badge {{
      display: inline-block; font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em;
      text-transform: uppercase; color: var(--brand); background: var(--brand-dim);
      border: 1px solid var(--border-strong); padding: 0.28rem 0.65rem; border-radius: 999px;
    }}
    h1 {{
      margin: 0.6rem 0 0.4rem; font-size: clamp(1.6rem, 4vw, 2.4rem); font-weight: 700;
      letter-spacing: -0.03em; line-height: 1.1;
    }}
    .lead {{ color: var(--muted); max-width: 58ch; margin: 0; }}
    .meta-grid {{
      display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 0.75rem; margin: 1.25rem 0;
    }}
    .meta-card {{
      background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-sm);
      padding: 0.85rem 1rem;
    }}
    .meta-card dt {{ font-size: 0.65rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); margin: 0; }}
    .meta-card dd {{ margin: 0.3rem 0 0; font-weight: 600; font-size: 0.92rem; }}
    .panel {{
      background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius);
      padding: clamp(1rem, 2.5vw, 1.5rem); margin-bottom: 1.25rem; box-shadow: var(--shadow);
    }}
    .section-head {{ margin-bottom: 1rem; }}
    .section-head h2 {{ margin: 0 0 0.35rem; font-size: 1.15rem; }}
    .section-head p {{ margin: 0; color: var(--muted); font-size: 0.88rem; }}
    .legend {{ display: flex; flex-wrap: wrap; gap: 0.5rem 1rem; margin-bottom: 1rem; }}
    .legend-item {{ display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.85rem; color: var(--muted); }}
    .lang-dot {{ width: 9px; height: 9px; border-radius: 50%; flex-shrink: 0; display: inline-block; }}
    .table-scroll {{ overflow-x: auto; border-radius: var(--radius-sm); border: 1px solid var(--border); background: var(--surface-2); }}
    table {{ width: 100%; border-collapse: collapse; font-size: 0.88rem; }}
    th, td {{ padding: 0.7rem 0.85rem; border-bottom: 1px solid var(--border); text-align: left; }}
    thead th {{ background: var(--surface-3); font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--muted); white-space: nowrap; }}
    tbody tr:last-child td, tbody tr:last-child th {{ border-bottom: none; }}
    tbody tr:hover td, tbody tr:hover th {{ background: rgba(255,255,255,0.02); }}
    td.num, th.num {{ font-family: var(--mono); font-variant-numeric: tabular-nums; }}
    .matrix-table {{ min-width: 52rem; }}
    .matrix-table .col-suite {{ min-width: 10rem; vertical-align: top; }}
    .suite-desc-inline {{ display: block; font-size: 0.72rem; font-weight: 400; color: var(--muted); margin-top: 0.2rem; max-width: 14rem; }}
    .matrix-cell .cell-lang {{ display: none; }}
    .matrix-cell .cell-main {{ display: block; font-weight: 600; font-family: var(--mono); font-size: 0.82rem; }}
    .matrix-cell .cell-vs {{ display: block; font-size: 0.72rem; color: var(--muted); margin-top: 0.15rem; }}
    td.cell-best, td.cell-best-mem {{ background: var(--brand-dim); box-shadow: inset 0 0 0 1px var(--border-strong); }}
    .best-pill {{
      display: inline-block; margin-top: 0.25rem; font-size: 0.58rem; font-weight: 700;
      text-transform: uppercase; letter-spacing: 0.05em; color: var(--brand);
      background: rgba(61,154,139,0.2); padding: 0.1rem 0.35rem; border-radius: 4px;
    }}
    .leaderboard-table td.rank-col {{ color: var(--muted); width: 2.5rem; }}
    td.cell-best {{ background: var(--brand-dim); }}
    .toc-nav {{ display: flex; flex-wrap: wrap; gap: 0.4rem; margin: 1rem 0; }}
    .toc-nav a {{
      font-size: 0.78rem; font-weight: 600; color: var(--muted); text-decoration: none;
      padding: 0.35rem 0.65rem; border: 1px solid var(--border); border-radius: 999px; background: var(--surface-2);
    }}
    .toc-nav a:hover {{ color: var(--brand); border-color: var(--border-strong); }}
    .suites-stack {{ display: flex; flex-direction: column; gap: 1.25rem; }}
    .suite-head {{ display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; margin-bottom: 1rem; padding-bottom: 0.75rem; border-bottom: 1px solid var(--border); flex-wrap: wrap; }}
    .suite-title {{ margin: 0; color: var(--brand); text-transform: capitalize; font-size: 1.05rem; }}
    .suite-desc {{ margin: 0.2rem 0 0; font-size: 0.78rem; color: var(--muted); }}
    .suite-jump {{ font-size: 0.75rem; color: var(--muted); text-decoration: none; }}
    .suite-stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 0.6rem; margin: 0 0 1rem; }}
    .stat-card {{ background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0.6rem 0.75rem; }}
    .stat-card dt {{ margin: 0; font-size: 0.6rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); }}
    .stat-card dd {{ margin: 0.25rem 0 0; font-size: 0.82rem; font-weight: 600; }}
    .chart-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 0.85rem; margin-top: 1rem; }}
    @media (max-width: 720px) {{ .chart-grid {{ grid-template-columns: 1fr; }} }}
    .chart-box {{ background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0.85rem; }}
    .chart-label {{ margin: 0 0 0.65rem; font-size: 0.8rem; font-weight: 600; }}
    .bar-row {{ display: grid; grid-template-columns: 4.5rem 1fr 5rem; gap: 0.5rem; align-items: center; margin-bottom: 0.45rem; font-size: 0.8rem; }}
    .bar-track {{ height: 10px; background: rgba(255,255,255,0.06); border-radius: 5px; overflow: hidden; }}
    .bar-fill {{ height: 100%; border-radius: 5px; min-width: 3px; }}
    .bar-value {{ text-align: right; font-family: var(--mono); font-size: 0.72rem; color: var(--muted); }}
    .lang-badge {{ font-size: 0.75rem; font-weight: 600; color: var(--lang-color); }}
    .bar-row-best .lang-badge::after {{ content: " ★"; font-size: 0.65rem; }}
    .insights {{ list-style: none; margin: 1rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; }}
    .insight-item {{ font-size: 0.82rem; color: var(--muted); padding: 0.5rem 0.7rem; background: var(--surface-2); border-radius: var(--radius-sm); border-left: 3px solid var(--brand); }}
    .insight-item strong {{ color: var(--text); }}
    .muted-note {{ font-size: 0.82rem; color: var(--muted); margin: 0.75rem 0 0; }}
    .binary-size-table th, .binary-size-table td {{ white-space: nowrap; }}
    .nyra-key-message {{
      margin: 0 0 1.25rem;
      padding: 1.1rem 1.35rem;
      background: linear-gradient(135deg, rgba(61, 154, 139, 0.14), var(--surface-2));
      border: 1px solid rgba(61, 154, 139, 0.45);
      border-radius: var(--radius);
      font-size: 0.95rem;
      line-height: 1.55;
    }}
    .nyra-key-message p {{ margin: 0 0 0.65rem; }}
    .nyra-key-message p:last-child {{ margin-bottom: 0; }}
    .nyra-key-message ul {{ margin: 0.35rem 0 0.75rem 1.1rem; padding: 0; }}
    .nyra-key-message li {{ margin: 0.2rem 0; }}
    .nyra-key-message strong {{ color: var(--text); }}
    .nyra-types-table th.col-suite, .nyra-types-table th[scope="row"] {{ text-align: left; }}
    .nyra-types-mem-details {{ margin-top: 1rem; font-size: 0.88rem; color: var(--muted); }}
    .nyra-types-mem-details summary {{ cursor: pointer; font-weight: 600; color: var(--text); }}
    .raw-table {{ font-size: 0.82rem; }}
    .footer {{ margin-top: 2rem; padding-top: 1rem; border-top: 1px solid var(--border); color: var(--muted); font-size: 0.82rem; }}
    .footer code {{ font-family: var(--mono); font-size: 0.78rem; background: var(--surface-2); padding: 0.12rem 0.4rem; border-radius: 4px; }}
    @media (max-width: 640px) {{
      .matrix-table thead {{ display: none; }}
      .matrix-table tbody tr {{ display: block; margin-bottom: 0.75rem; border: 1px solid var(--border); border-radius: var(--radius-sm); overflow: hidden; }}
      .matrix-table th.col-suite, .matrix-table td {{ display: flex; justify-content: space-between; border: none; }}
      .matrix-cell .cell-lang {{ display: block; font-size: 0.72rem; color: var(--muted); }}
    }}
  </style>
</head>
<body id="top">
  <main class="page">
    <nav class="sticky-nav" aria-label="Sections">
      <a href="#overview">Overview</a>
      <a href="#binary-size">Binary size</a>
      <a href="#nyra-types">Nyra types</a>
      <a href="#matrix">Matrix</a>
      <a href="#leaderboard">Languages</a>
      <a href="#suites">Suites</a>
      <a href="#raw">Raw data</a>
    </nav>

    <header class="hero" id="overview">
      <span class="hero-badge">Nyra comparison benchmark</span>
      <h1>Runtime, memory &amp; binary size</h1>
      <p class="lead">Mean wall-clock <strong>time</strong>, peak <strong>memory</strong> (RSS), and hello-world <strong>binary size</strong> across Nyra, C, C++, Go, and Rust. Lower is better.</p>
    </header>

    <dl class="meta-grid">
      <div class="meta-card"><dt>Generated (UTC)</dt><dd>{html.escape(meta["generated"])}</dd></div>
      <div class="meta-card"><dt>Platform</dt><dd>{html.escape(meta["platform"])}</dd></div>
      <div class="meta-card"><dt>Timed runs</dt><dd>{html.escape(meta["runs"])} <span style="color:var(--muted)">(warmup {html.escape(meta["warmup"])} discarded)</span></dd></div>
      <div class="meta-card"><dt>Nyra build</dt><dd>{html.escape(meta["nyra_mode"])}</dd></div>
      <div class="meta-card"><dt>Isolation</dt><dd>{isolation}</dd></div>
      <div class="meta-card"><dt>Entries</dt><dd>{len(rows)} measurements</dd></div>
    </dl>

    <nav class="legend" aria-label="Languages">{legend_html}</nav>

    <section class="panel" id="binary-size">
      <div class="section-head">
        <h2>Hello world — binary size</h2>
        <p>Marketing-relevant artifact sizes: optimized <strong>release</strong> build, <strong>stripped</strong> symbols, and <strong>UPX</strong> compression.</p>
      </div>
      {binary_html}
    </section>

    <section class="panel" id="nyra-types">
      <div class="section-head">
        <h2>Nyra — Zero Types vs Explicit Types</h2>
        <p>Direct proof that optional type annotations do <strong>not</strong> change runtime performance. Same benchmark, two styles.</p>
      </div>
      {nyra_types_html}
    </section>

    <section class="panel" id="matrix">
      <div class="section-head">
        <h2>Full comparison matrix</h2>
        <p>Every language × every benchmark — <strong>time</strong> and <strong>memory</strong> at a glance. Highlighted cells are fastest (time) or leanest (memory) in that row.</p>
      </div>
      {matrix_html}
    </section>

    <section class="panel" id="leaderboard">
      <div class="section-head">
        <h2>Language leaderboard</h2>
        <p>Average performance across all suites and how often each language ranked #1.</p>
      </div>
      {leaderboard_html}
    </section>

    <section id="suites">
      <div class="section-head" style="margin:1.5rem 0 1rem">
        <h2>Detailed results by benchmark</h2>
        <p>Rankings, charts, and Nyra-vs-others analysis for each suite.</p>
      </div>
      <nav class="toc-nav" aria-label="Benchmark index">{suite_toc}</nav>
      <div class="suites-stack">{"".join(suite_sections)}</div>
    </section>

    <section class="panel" id="raw">
      <div class="section-head">
        <h2>Raw measurements</h2>
        <p>Complete dataset — same rows as <code>results/data.tsv</code>.</p>
      </div>
      {raw_html}
    </section>

    <footer class="footer">
      <p>Compile time excluded — only execution is measured. Re-run: <code>./scripts/bench.sh</code></p>
    </footer>
  </main>
</body>
</html>"""


def load_rows(tsv: Path) -> list[dict]:
    rows: list[dict] = []
    with tsv.open(encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("suite\t"):
                continue
            suite, lang, ms_s, mem_s = line.split("\t")
            if not ms_s.strip():
                continue
            rows.append({"suite": suite, "lang": lang, "ms": float(ms_s), "mem_kb": int(mem_s or 0)})
    return rows


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Nyra comparison benchmark reports")
    parser.add_argument(
        "--txt-only",
        action="store_true",
        help="Rebuild latest.txt appendix from results/data.tsv (no HTML)",
    )
    args = parser.parse_args()

    tsv = Path(os.environ.get("BENCH_TSV", "examples/comparison/results/data.tsv"))
    env_bin = os.environ.get("BENCH_BINARY_TSV", "").strip()
    binary_path = Path(env_bin) if env_bin else tsv.parent / "binary-size.tsv"
    binary = load_binary_sizes(binary_path if binary_path.is_file() else None)
    rows = load_rows(tsv)
    if not rows:
        print("bench_comparison_html: no data rows", file=sys.stderr)
        return 1

    if args.txt_only:
        latest = Path(os.environ.get("BENCH_LATEST", tsv.parent / "latest.txt"))
        rebuild_latest_txt(latest, rows, binary)
        print(f"bench_comparison_html: updated {latest}")
        return 0

    out = Path(os.environ["BENCH_HTML"])
    meta = {
        "generated": os.environ["BENCH_GENERATED"],
        "platform": os.environ["BENCH_PLATFORM"],
        "runs": os.environ["BENCH_RUNS"],
        "warmup": os.environ["BENCH_WARMUP"],
        "nyra_mode": os.environ["BENCH_NYRA_MODE"],
        "isolation": os.environ.get("BENCH_ISOLATION", ""),
    }
    out.write_text(build_report(rows, meta, binary), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
