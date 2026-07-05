#!/usr/bin/env python3
"""Compile and run runnable Nyra snippets embedded in webDocs HTML.

Extracts:
  - NYRA_SNIPPET code-tab pairs (easy + optional typed panel)
  - Plain <pre><code> blocks with fn main on learn-*.html pages

Markers (HTML comment immediately before NYRA_SNIPPET_START):
  <!-- NYRA_SNIPPET_EXPECT_FAIL -->  — nyra run must fail (error demos)
  <!-- NYRA_SNIPPET_SKIP -->         — skip (multi-file / not self-contained)

Snippets with non-stdlib `import` paths are auto-skipped (need a project directory).

Env:
  NYRA_BIN              path to nyra (default: target/debug/nyra)
  NYRA_WEBDOCS_TYPED=1  also validate typed panels (default: easy only)
  NYRA_WEBDOCS_JOBS=N   parallel workers (default: 8)
  NYRA_WEBDOCS_TIMEOUT  seconds per snippet (default: 45)
"""
from __future__ import annotations

import argparse
import html as html_mod
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WEB = ROOT / "webDocs"

SKIP_PAGES = frozenset({"stdlib.html", "bindings.html"})

SNIP_BLOCK = re.compile(
    r"(<!--\s*NYRA_SNIPPET_EXPECT_FAIL\s*-->\s*)?"
    r"(<!--\s*NYRA_SNIPPET_SKIP\s*-->\s*)?"
    r"<!-- NYRA_SNIPPET_START -->(.*?)<!-- NYRA_SNIPPET_END -->",
    re.S,
)
EASY_PANEL = re.compile(
    r'data-panel="easy"[^>]*><pre><code>(.*?)</code></pre>', re.S
)
TYPED_PANEL = re.compile(
    r'data-panel="typed"[^>]*><pre><code>(.*?)</code></pre>', re.S
)
PLAIN_PRE = re.compile(r"<pre><code>(.*?)</code></pre>", re.S)
HAS_MAIN = re.compile(r"\bfn\s+main\b")
IMPORT_RE = re.compile(r'import\s+"([^"]+)"')


@dataclass(frozen=True)
class Snippet:
    id: str
    source: str
    code: str
    expect_fail: bool
    explicit_skip: bool


def external_imports(code: str) -> list[str]:
    return [
        path
        for path in IMPORT_RE.findall(code)
        if not path.startswith("stdlib/")
    ]


def is_runnable_in_isolation(code: str) -> bool:
    return not external_imports(code)


def unescape_code(raw: str) -> str:
    return html_mod.unescape(raw).strip()


def extract_snippets(html_path: Path, include_typed: bool) -> list[Snippet]:
    html = html_path.read_text(encoding="utf-8")
    out: list[Snippet] = []
    seen_codes: set[str] = set()
    panel_counts: dict[str, int] = {}

    def add(panel: str, code: str, expect_fail: bool, explicit_skip: bool) -> None:
        if not HAS_MAIN.search(code):
            return
        key = code
        if key in seen_codes:
            return
        seen_codes.add(key)
        panel_counts[panel] = panel_counts.get(panel, 0) + 1
        n = panel_counts[panel]
        sid = f"{html_path.name}:{panel}" if n == 1 else f"{html_path.name}:{panel}#{n}"
        out.append(
            Snippet(
                id=sid,
                source=str(html_path.relative_to(ROOT)),
                code=code,
                expect_fail=expect_fail,
                explicit_skip=explicit_skip,
            )
        )

    for m in SNIP_BLOCK.finditer(html):
        expect_fail = m.group(1) is not None
        explicit_skip = m.group(2) is not None
        block = m.group(3)
        em = EASY_PANEL.search(block)
        if em:
            add("easy", unescape_code(em.group(1)), expect_fail, explicit_skip)
        if include_typed:
            tm = TYPED_PANEL.search(block)
            if tm:
                add("typed", unescape_code(tm.group(1)), expect_fail, explicit_skip)

    if html_path.name.startswith("learn-"):
        for m in PLAIN_PRE.finditer(html):
            code = unescape_code(m.group(1))
            add("plain", code, False, False)

    return out


def nyra_bin() -> Path:
    env = os.environ.get("NYRA_BIN")
    if env:
        return Path(env)
    return ROOT / "target" / "debug" / "nyra"


def snippet_expects_fail(code: str, explicit: bool) -> bool:
    """True when snippet is an intentional error demo (HTML marker or // ERROR on code)."""
    if explicit:
        return True
    for line in code.splitlines():
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        if re.search(r"//\s*ERROR\b", line):
            return True
    return False


def run_snippet(snippet: Snippet, nyra: Path, timeout: int) -> tuple[Snippet, bool, str]:
    with tempfile.TemporaryDirectory(prefix="nyra-webdocs-") as td:
        path = Path(td) / "snippet.ny"
        path.write_text(snippet.code + "\n", encoding="utf-8")
        try:
            proc = subprocess.run(
                [str(nyra), "run", str(path)],
                cwd=ROOT,
                capture_output=True,
                text=True,
                errors="replace",
                timeout=timeout,
            )
        except subprocess.TimeoutExpired:
            return snippet, False, f"timeout after {timeout}s"
        ok = proc.returncode == 0
        expect_fail = snippet_expects_fail(snippet.code, snippet.expect_fail)
        if expect_fail:
            if not ok:
                return snippet, True, ""
            tail = (proc.stdout or proc.stderr)[-400:]
            return snippet, False, f"expected compile/run failure, but succeeded:\n{tail}"
        if ok:
            return snippet, True, ""
        err = proc.stderr or proc.stdout or "(no output)"
        return snippet, False, err[-600:]


def read_manifest_ids(path: Path) -> set[str]:
    ids: set[str] = set()
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if " #" in line:
            line = line.split(" #", 1)[0].strip()
        ids.add(line)
    return ids


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--typed",
        action="store_true",
        help="also run typed code-tab panels (default: easy only)",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="list snippet ids and exit",
    )
    parser.add_argument(
        "--filter",
        default="",
        help="substring filter on snippet id",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=None,
        help="only run snippet ids listed in this file (one per line, # comments ok)",
    )
    parser.add_argument(
        "--write-manifest",
        type=Path,
        default=None,
        help="write ids of passing snippets to this file (updates baseline)",
    )
    args = parser.parse_args()

    include_typed = args.typed or os.environ.get("NYRA_WEBDOCS_TYPED") == "1"
    jobs = max(1, int(os.environ.get("NYRA_WEBDOCS_JOBS", "8")))
    timeout = max(5, int(os.environ.get("NYRA_WEBDOCS_TIMEOUT", "45")))
    nyra = nyra_bin()
    if not nyra.is_file():
        print(f"check-webdocs-snippets: missing nyra binary: {nyra}", file=sys.stderr)
        return 2

    snippets: list[Snippet] = []
    for path in sorted(WEB.glob("*.html")):
        if path.name in SKIP_PAGES:
            continue
        snippets.extend(extract_snippets(path, include_typed))

    if args.filter:
        snippets = [s for s in snippets if args.filter in s.id]

    if args.manifest is not None:
        if not args.manifest.is_file():
            print(f"check-webdocs-snippets: manifest not found: {args.manifest}", file=sys.stderr)
            return 2
        manifest_ids = read_manifest_ids(args.manifest)
        all_ids = {s.id for s in snippets}
        missing = manifest_ids - all_ids
        if missing:
            print(
                "check-webdocs-snippets: manifest entries not found in webDocs:",
                file=sys.stderr,
            )
            for mid in sorted(missing):
                print(f"  - {mid}", file=sys.stderr)
            return 2
        snippets = [s for s in snippets if s.id in manifest_ids]

    if args.list:
        for s in snippets:
            flag = " expect-fail" if s.expect_fail else ""
            print(f"{s.id}{flag}")
        print(f"check-webdocs-snippets: {len(snippets)} runnable snippet(s)")
        return 0

    if not snippets:
        print("check-webdocs-snippets: no runnable snippets found")
        return 0

    failures: list[tuple[Snippet, str]] = []
    passed = 0
    skipped = 0
    runnable: list[Snippet] = []
    for s in snippets:
        if s.explicit_skip:
            skipped += 1
            continue
        if not is_runnable_in_isolation(s.code):
            skipped += 1
            continue
        runnable.append(s)

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = {
            pool.submit(run_snippet, s, nyra, timeout): s for s in runnable
        }
        for fut in as_completed(futures):
            snippet, ok, msg = fut.result()
            if ok:
                passed += 1
            else:
                failures.append((snippet, msg))

    failures.sort(key=lambda x: x[0].id)
    print(
        f"check-webdocs-snippets: {passed} passed, {len(failures)} failed, "
        f"{skipped} skipped ({len(runnable)} run, {len(snippets)} total, "
        f"typed={'yes' if include_typed else 'no'})"
    )
    for snippet, msg in failures:
        print(f"\nFAIL {snippet.id} ({snippet.source})", file=sys.stderr)
        print(msg.strip(), file=sys.stderr)

    if args.write_manifest is not None:
        args.write_manifest.parent.mkdir(parents=True, exist_ok=True)
        ids = sorted(s.id for s in runnable if s.id not in {f[0].id for f in failures})
        args.write_manifest.write_text("\n".join(ids) + "\n", encoding="utf-8")
        print(f"check-webdocs-snippets: wrote {len(ids)} id(s) to {args.write_manifest}")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
