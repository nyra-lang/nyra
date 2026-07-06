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
  NYRA_WEBDOCS_JOBS=N   parallel workers (default: 8 unix, 2 windows)
  NYRA_WEBDOCS_TIMEOUT  seconds per snippet (default: 45 unix, 120 windows)
"""
from __future__ import annotations

import argparse
import html as html_mod
import os
import re
import signal
import subprocess
import sys
import tempfile
import threading
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


def resolve_nyra_executable(path: Path) -> Path | None:
    """Return path when it is a real file; on Windows also try ``path`` + ``.exe``."""
    if path.is_file():
        return path
    if not str(path).lower().endswith(".exe"):
        exe = Path(f"{path}.exe")
        if exe.is_file():
            return exe
    return None


def nyra_bin() -> Path:
    env = os.environ.get("NYRA_BIN")
    if env:
        resolved = resolve_nyra_executable(Path(env))
        if resolved is not None:
            return resolved
        return Path(env)
    for candidate in (
        ROOT / "target" / "debug" / "nyra.exe",
        ROOT / "target" / "debug" / "nyra",
    ):
        resolved = resolve_nyra_executable(candidate)
        if resolved is not None:
            return resolved
    return ROOT / "target" / "debug" / "nyra"


def _running_on_windows() -> bool:
    return sys.platform == "win32" or os.name == "nt"


def webdocs_runner_settings() -> tuple[int, int]:
    """Return (parallel_jobs, timeout_seconds) with platform-tuned defaults."""
    if _running_on_windows():
        default_jobs, default_timeout = "2", "120"
    else:
        default_jobs, default_timeout = "8", "45"
    jobs = max(1, int(os.environ.get("NYRA_WEBDOCS_JOBS", default_jobs)))
    timeout = max(5, int(os.environ.get("NYRA_WEBDOCS_TIMEOUT", default_timeout)))
    return jobs, timeout


_ACTIVE_PROCS: list[subprocess.Popen[bytes]] = []
_ACTIVE_LOCK = threading.Lock()


def _register_proc(proc: subprocess.Popen[bytes]) -> None:
    with _ACTIVE_LOCK:
        _ACTIVE_PROCS.append(proc)


def _unregister_proc(proc: subprocess.Popen[bytes]) -> None:
    with _ACTIVE_LOCK:
        try:
            _ACTIVE_PROCS.remove(proc)
        except ValueError:
            pass


def _kill_proc_tree(proc: subprocess.Popen[bytes]) -> None:
    """Kill nyra and any compiled snippet binary in the same process group."""
    if proc.poll() is not None:
        return
    try:
        if sys.platform == "win32":
            subprocess.run(
                ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )
        else:
            os.killpg(proc.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError, OSError):
        try:
            proc.kill()
        except ProcessLookupError:
            pass
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        pass


def _cleanup_all_procs(signum: int | None = None, _frame: object | None = None) -> None:
    with _ACTIVE_LOCK:
        procs = list(_ACTIVE_PROCS)
    for proc in procs:
        _kill_proc_tree(proc)
    if signum is not None:
        raise SystemExit(128 + signum)


def _install_signal_handlers() -> None:
    signal.signal(signal.SIGINT, _cleanup_all_procs)
    if hasattr(signal, "SIGTERM"):
        signal.signal(signal.SIGTERM, _cleanup_all_procs)


def _popen_run_nyra(
    nyra: Path, path: Path, stdout: object, stderr: object
) -> subprocess.Popen[bytes]:
    popen_kwargs: dict[str, object] = {
        "args": [str(nyra), "run", str(path)],
        "cwd": ROOT,
        "stdout": stdout,
        "stderr": stderr,
    }
    if sys.platform == "win32":
        popen_kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        popen_kwargs["start_new_session"] = True
    return subprocess.Popen(**popen_kwargs)


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
        td_path = Path(td)
        path = td_path / "snippet.ny"
        err_path = td_path / "stderr.txt"
        out_path = td_path / "stdout.txt"
        path.write_text(snippet.code + "\n", encoding="utf-8")
        proc: subprocess.Popen[bytes] | None = None
        try:
            # Write child output to files, not pipes — parallel capture_output=True can
            # deadlock on Windows when compiler stderr fills the OS pipe buffer.
            with err_path.open("w", encoding="utf-8", errors="replace") as errf, out_path.open(
                "w", encoding="utf-8", errors="replace"
            ) as outf:
                proc = _popen_run_nyra(nyra, path, outf, errf)
                _register_proc(proc)
                try:
                    proc.wait(timeout=timeout)
                except subprocess.TimeoutExpired:
                    _kill_proc_tree(proc)
                    return snippet, False, f"timeout after {timeout}s"
        finally:
            if proc is not None:
                _unregister_proc(proc)
        stderr = err_path.read_text(encoding="utf-8", errors="replace")
        stdout = out_path.read_text(encoding="utf-8", errors="replace")
        ok = proc.returncode == 0
        expect_fail = snippet_expects_fail(snippet.code, snippet.expect_fail)
        if expect_fail:
            if not ok:
                return snippet, True, ""
            tail = (stdout or stderr)[-400:]
            return snippet, False, f"expected compile/run failure, but succeeded:\n{tail}"
        if ok:
            return snippet, True, ""
        err = stderr or stdout or "(no output)"
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
    jobs, timeout = webdocs_runner_settings()
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

    _install_signal_handlers()

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
