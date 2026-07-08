#!/usr/bin/env python3
"""Inject cross-language benchmark tables into README.md between BENCH markers."""
from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
README = ROOT / "README.md"
TSV = ROOT / "examples" / "comparison" / "results" / "data.tsv"
LATEST = ROOT / "examples" / "comparison" / "results" / "latest.txt"

START = "<!-- BENCH:START -->"
END = "<!-- BENCH:END -->"

LANG_ORDER = ["Nyra", "Nyra-typed", "C", "C++", "Go", "Rust"]
LANG_DISPLAY = {
    "Nyra": "Nyra (Zero Types)",
    "Nyra-typed": "Nyra (Explicit Types)",
}


def lang_display(lang: str) -> str:
    return LANG_DISPLAY.get(lang, lang)
# README highlights (full matrix in examples/comparison/results/)
SHOW_SUITES = ["cpu_bound", "nested", "loop", "hello"]
SUITE_LABELS = {
    "cpu_bound": "CPU hot loop",
    "nested": "Nested loops",
    "loop": "Linear sum",
    "hello": "Hello I/O",
}


def parse_latest_meta(text: str) -> dict[str, str]:
    meta: dict[str, str] = {}
    for line in text.splitlines():
        if not line.startswith("# "):
            continue
        body = line[2:].strip()
        if body.startswith("Generated:"):
            meta["generated"] = body.split(":", 1)[1].strip()
        elif body.startswith("Platform:"):
            meta["platform"] = body.split(":", 1)[1].strip()
        elif body.startswith("Runs per command:"):
            meta["runs"] = body.split(":", 1)[1].strip()
        elif body.startswith("Nyra build:"):
            meta["nyra_build"] = body.split(":", 1)[1].strip()
    return meta


def fmt_ms(ms: float) -> str:
    if ms >= 1000:
        return f"{ms:,.0f} ms"
    if ms >= 100:
        return f"{ms:.0f} ms"
    return f"{ms:.1f} ms"


def fmt_mem(kb: int) -> str:
    if kb >= 1024:
        return f"{kb / 1024:.1f} MB"
    return f"{kb} KB"


def load_tsv(path: Path) -> list[dict[str, str | float | int]]:
    rows: list[dict[str, str | float | int]] = []
    with path.open(encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split("\t")
            row = dict(zip(header, parts))
            row["ms_mean"] = float(row["ms_mean"])
            row["peak_rss_kb"] = int(row["peak_rss_kb"])
            rows.append(row)
    return rows


def build_section(rows: list[dict], meta: dict[str, str]) -> str:
    by_suite: dict[str, list[dict]] = defaultdict(list)
    for row in rows:
        by_suite[str(row["suite"])].append(row)

    suites = [s for s in SHOW_SUITES if s in by_suite]
    langs = [lang for lang in LANG_ORDER if any(r["language"] == lang for r in rows)]

    generated = meta.get("generated", "unknown")
    platform = meta.get("platform", "unknown")
    runs = meta.get("runs", "5")
    nyra_build = meta.get("nyra_build", "release")

    lines = [
        START,
        "",
        "## Performance benchmarks",
        "",
        "Nyra is compared against **C, C++, Go, and Rust** on the same",
        "programs under [`examples/comparison/`](examples/comparison/). **Lower runtime and RAM are better.**",
        "Compile time is excluded; numbers are mean wall-clock over timed runs.",
        "",
        f"**Last run:** {generated} · **Platform:** {platform} · **Runs:** {runs} · **Nyra:** {nyra_build}",
        "",
        f"**[Interactive report →](examples/comparison/results/latest.html)** ·",
        f"raw data: [`data.tsv`](examples/comparison/results/data.tsv)",
        "",
    ]

    # Main comparison table
    header = "| Language | " + " | ".join(SUITE_LABELS.get(s, s) for s in suites) + " |"
    sep = "|----------|" + "|".join("----------:" for _ in suites) + "|"
    lines.extend([header, sep])

    for lang in langs:
        cells = [lang_display(lang)]
        for suite in suites:
            match = next((r for r in by_suite[suite] if r["language"] == lang), None)
            if match is None:
                cells.append("—")
            else:
                cells.append(fmt_ms(float(match["ms_mean"])))
        lines.append("| " + " | ".join(cells) + " |")

    lines.append("")

    # Nyra vs fastest compiled (C/C++/Go/Rust) on cpu_bound
    if "cpu_bound" in by_suite:
        compiled = {"C", "C++", "Go", "Rust"}
        cpu_rows = [r for r in by_suite["cpu_bound"] if r["language"] in compiled]
        nyra_row = next((r for r in by_suite["cpu_bound"] if r["language"] == "Nyra"), None)
        if cpu_rows and nyra_row:
            best = min(cpu_rows, key=lambda r: float(r["ms_mean"]))
            nyra_ms = float(nyra_row["ms_mean"])
            best_ms = float(best["ms_mean"])
            ratio = nyra_ms / best_ms if best_ms > 0 else 0
            lines.extend(
                [
                    "**cpu_bound snapshot:** Nyra (Zero Types) "
                    f"`{fmt_ms(nyra_ms)}` vs fastest compiled "
                    f"({lang_display(best['language'])} `{fmt_ms(best_ms)}`) — "
                    f"**{ratio:.2f}×** wall time.",
                    "",
                ]
            )

    # Peak RAM on cpu_bound
    if "cpu_bound" in by_suite:
        lines.append("**Peak RAM (cpu_bound):**")
        lines.append("")
        lines.append("| Language | Peak RSS |")
        lines.append("|----------|----------:|")
        for r in sorted(by_suite["cpu_bound"], key=lambda x: int(x["peak_rss_kb"])):
            lines.append(
                f"| {lang_display(r['language'])} | {fmt_mem(int(r['peak_rss_kb']))} |"
            )
        lines.append("")

    lines.extend(
        [
            "```bash",
            "make bench              # full matrix + HTML report",
            "BENCH_QUICK=1 make bench  # CI-friendly subset",
            "```",
            "",
            END,
        ]
    )
    return "\n".join(lines)


def patch_readme(section: str) -> None:
    text = README.read_text(encoding="utf-8")
    if START in text and END in text:
        pattern = re.compile(re.escape(START) + r".*?" + re.escape(END), re.DOTALL)
        new_text = pattern.sub(section, text, count=1)
    else:
        anchor = "## Quick start"
        if anchor not in text:
            print("update-readme-bench: README anchor not found", file=sys.stderr)
            sys.exit(1)
        new_text = text.replace(anchor, section + "\n\n" + anchor, 1)
    README.write_text(new_text, encoding="utf-8")


def main() -> int:
    if not TSV.is_file():
        print(f"update-readme-bench: missing {TSV} — run make bench first", file=sys.stderr)
        return 1
    meta = parse_latest_meta(LATEST.read_text(encoding="utf-8")) if LATEST.is_file() else {}
    rows = load_tsv(TSV)
    patch_readme(build_section(rows, meta))
    print(f"update-readme-bench: updated {README}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
