#!/usr/bin/env python3
"""Batch scaffold builtins via builtin-dev and contribute.

Usage:
  python3 make/py/builtin_dev/batch_add.py
  python3 make/py/builtin_dev/batch_add.py --batch batch2
  python3 make/py/builtin_dev/batch_add.py --batch all --only string,math
  make batch-add-builtin BATCH=batch3
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MAKE_PY = ROOT / "make" / "py"
BUILTIN_PY = MAKE_PY / "builtin-dev.py"
CONTRIBUTE_PY = MAKE_PY / "contribute.py"
EXAMPLES = Path(__file__).resolve().parent / "examples"

KINDS = (
    "string",
    "math",
    "vec",
    "map",
    "encoding",
    "strconv",
    "format",
    "sync",
    "fs",
    "pure",
)


@dataclass
class RunResult:
    config: Path
    kind: str
    rc: int
    skipped: bool = False


@dataclass
class BatchReport:
    results: list[RunResult] = field(default_factory=list)

    def add(self, config: Path, kind: str, rc: int, *, skipped: bool = False) -> None:
        self.results.append(RunResult(config=config, kind=kind, rc=rc, skipped=skipped))

    def ok(self) -> bool:
        return all(r.rc == 0 for r in self.results)

    def print_summary(self) -> None:
        ok = sum(1 for r in self.results if r.rc == 0 and not r.skipped)
        skip = sum(1 for r in self.results if r.skipped)
        fail = sum(1 for r in self.results if r.rc != 0)
        print("\n=== batch_add summary ===")
        print(f"  ok: {ok}  skipped: {skip}  failed: {fail}")
        for r in self.results:
            status = "SKIP" if r.skipped else ("OK" if r.rc == 0 else "FAIL")
            rel = r.config.relative_to(ROOT) if r.config.is_relative_to(ROOT) else r.config
            print(f"  [{status}] {r.kind:12} {rel}")
        print("=========================\n")


def _glob_configs(folder: Path, pattern: str) -> list[Path]:
    if not folder.is_dir():
        return []
    return sorted(folder.glob(pattern))


def collect_batches(batch_arg: str) -> list[str]:
    if batch_arg == "all":
        names = sorted(p.name for p in EXAMPLES.iterdir() if p.is_dir() and p.name.startswith("batch"))
        return names or ["batch"]
    return [batch_arg]


def collect_configs(batch_names: list[str], only: set[str]) -> dict[str, list[Path]]:
    out: dict[str, list[Path]] = {k: [] for k in KINDS}
    for name in batch_names:
        builtin_dir = EXAMPLES / name
        contrib_dir = MAKE_PY / "contrib_dev" / "examples" / name
        out["string"].extend(_glob_configs(builtin_dir, "*.json"))
        out["math"].extend(_glob_configs(contrib_dir, "math_*.json"))
        out["vec"].extend(_glob_configs(contrib_dir, "vec_*.json"))
        out["map"].extend(_glob_configs(contrib_dir, "map_*.json"))
        out["encoding"].extend(_glob_configs(contrib_dir, "encoding_*.json"))
        out["strconv"].extend(_glob_configs(contrib_dir, "strconv_*.json"))
        out["format"].extend(_glob_configs(contrib_dir, "format_*.json"))
        out["sync"].extend(_glob_configs(contrib_dir, "sync_*.json"))
        out["fs"].extend(_glob_configs(contrib_dir, "fs_*.json"))
        out["pure"].extend(_glob_configs(contrib_dir, "pure_*.json"))
    if "all" in only:
        return out
    return {k: v for k, v in out.items() if k in only}


def run_cmd(cmd: list[str], *, dry_run: bool) -> int:
    print("\n>>>", " ".join(cmd), flush=True)
    if dry_run:
        return 0
    return subprocess.call(cmd, cwd=str(ROOT))


def scaffold_string(cfg: Path, *, force: bool, dry_run: bool) -> int:
    cmd = [sys.executable, str(BUILTIN_PY), "add", "--config", str(cfg)]
    if force:
        cmd.append("--force")
    return run_cmd(cmd, dry_run=dry_run)


def scaffold_contrib(cfg: Path, recipe: str, *, dry_run: bool, no_webdocs: bool, force: bool) -> int:
    cmd = [
        sys.executable,
        str(CONTRIBUTE_PY),
        "add",
        "--recipe",
        recipe,
        "--config",
        str(cfg),
        "--force",
    ]
    if no_webdocs:
        cmd.append("--no-webdocs")
    return run_cmd(cmd, dry_run=dry_run)


def main() -> int:
    parser = argparse.ArgumentParser(description="Batch scaffold Nyra builtins")
    parser.add_argument(
        "--batch",
        default="batch",
        help="batch folder name under examples/ (batch, batch2, batch3, or all)",
    )
    parser.add_argument(
        "--only",
        default="all",
        help="comma-separated: string,math,vec,map,encoding,strconv,format,sync,fs,pure,all",
    )
    parser.add_argument("--force", action="store_true", help="pass --force to builtin-dev add")
    parser.add_argument("--dry-run", action="store_true", help="print commands only")
    parser.add_argument(
        "--no-webdocs",
        action="store_true",
        help="skip webDocs regen after each contribute add (faster; default regens)",
    )
    args = parser.parse_args()

    if not args.no_webdocs:
        os.environ.pop("NYRA_CONTRIBUTE_SKIP_WEBDOCS", None)
    else:
        os.environ.setdefault("NYRA_CONTRIBUTE_SKIP_WEBDOCS", "1")
    only = {x.strip() for x in args.only.split(",") if x.strip()}
    batch_names = collect_batches(args.batch)
    configs = collect_configs(batch_names, only)
    report = BatchReport()

    for cfg in configs.get("string", []):
        if cfg.name == "manifest.json":
            continue
        rc = scaffold_string(cfg, force=args.force, dry_run=args.dry_run)
        report.add(cfg, "string", rc)

    extern_kinds = ("math", "vec", "map", "encoding", "strconv", "format", "sync", "fs")
    for kind in extern_kinds:
        for cfg in configs.get(kind, []):
            if cfg.name == "manifest.json":
                continue
            rc = scaffold_contrib(
                cfg, "stdlib-extern", dry_run=args.dry_run, no_webdocs=args.no_webdocs, force=args.force
            )
            report.add(cfg, kind, rc)

    for cfg in configs.get("pure", []):
        if cfg.name == "manifest.json":
            continue
        rc = scaffold_contrib(
            cfg, "stdlib-pure", dry_run=args.dry_run, no_webdocs=args.no_webdocs, force=args.force
        )
        report.add(cfg, "pure", rc)

    if not report.results:
        print("No configs matched; check --batch and --only", file=sys.stderr)
        return 1

    report.print_summary()
    return 0 if report.ok() else 1


if __name__ == "__main__":
    raise SystemExit(main())
