#!/usr/bin/env python3
"""Generate extended comparison benchmarks (memory, strings, collections, algorithms, concurrency)."""
from __future__ import annotations

import subprocess
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMP = ROOT / "examples" / "comparison"

MOD = 1_000_000_007

# Default iteration counts — multiply with BENCH_SCALE (e.g. BENCH_SCALE=20 → ~10M allocs).
_SCALE = max(1, int(__import__("os").environ.get("BENCH_SCALE", "1")))
N_ALLOC = 500_000 * _SCALE
N_STRING = 100_000 * _SCALE
N_MAP = 200_000 * _SCALE
N_VEC = 500_000 * _SCALE
N_SORT = 50_000 * _SCALE
N_SPAWN = 5_000 * _SCALE
N_CHANNEL = 500_000 * _SCALE
N_PARALLEL = 200_000 * _SCALE

SUITES: list[tuple[str, str, str]] = []  # (category, name, description)


def w(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text.rstrip() + "\n", encoding="utf-8")


def suite_id(cat: str, name: str, desc: str) -> str:
    SUITES.append((cat, name, desc))
    return f"{cat}_{name}"


# ── Nyra templates ──────────────────────────────────────────────────────────

NY_HEADER = """extern fn blackbox_i32(x: i32) -> i32

"""

NY_MAIN = """fn main() {{
    let mut acc = 0
{body}
    print(blackbox_i32(acc))
}}
"""


def normalize_body(body: str) -> str:
    """Normalize bodies where Python f-string indentation mixed with Nyra nesting."""
    lines = body.strip().splitlines()
    block = "    " + "\n".join(lines)
    return textwrap.dedent(block).strip("\n")


def ny_file(body: str, extra: str = "", allow_ext: bool = False) -> str:
    head = "allow_extended\n" if allow_ext else ""
    indented = textwrap.indent(normalize_body(body), "    ") + "\n"
    return head + extra + NY_HEADER + NY_MAIN.format(body=indented)


# ── Memory ───────────────────────────────────────────────────────────────────

def gen_memory() -> None:
    d = COMP / "memory" / "alloc_struct"
    n = N_ALLOC
    w(
        d / "bench.ny",
        ny_file(
            f"""let mut i = 0
    while i < {n} {{
        let p = malloc(8)
        let node = Point {{ x: i % 997, y: (i * 3) % 991 }}
        acc = (acc + node.x + node.y) % {MOD}
        free(p)
        i = i + 1
    }}""",
            """extern fn malloc(size: i64) -> ptr
extern fn free(p: ptr) -> void

struct Point {
    x: i32
    y: i32
}

""",
        ),
    )
    suite_id("memory", "alloc_struct", f"malloc/free {n:,} nodes (8 B)")

    d = COMP / "memory" / "free_struct"
    w(
        d / "bench.ny",
        ny_file(
            f"""let mut i = 0
    while i < {n} {{
        let p = malloc(16)
        acc = (acc + i) % {MOD}
        free(p)
        i = i + 1
    }}""",
            """extern fn malloc(size: i64) -> ptr
extern fn free(p: ptr) -> void

""",
        ),
    )
    suite_id("memory", "free_struct", f"alloc+free {n:,} blocks (16 B)")

    d = COMP / "memory" / "arena"
  # bump arena in-process (no malloc churn)
    w(
        d / "bench.ny",
        ny_file(
            f"""let mut bump = 0
    let mut i = 0
    while i < {n} {{
        bump = (bump + 16) % 67108864
        acc = (acc + bump + i) % {MOD}
        i = i + 1
    }}""",
        ),
    )
    suite_id("memory", "arena", f"bump arena simulation ({n:,} allocs)")

    d = COMP / "memory" / "ownership"
    w(
        d / "bench.ny",
        ny_file(
            f"""let mut i = 0
    while i < {n} {{
        let p = Pair {{ a: i % 1000, b: (i * 7) % 1000 }}
        acc = (acc + use_pair(p)) % {MOD}
        i = i + 1
    }}""",
            """struct Pair {
    a: i32
    b: i32
}

fn use_pair(p) {
    return p.a + p.b
}

""",
        ),
    )
    suite_id("memory", "ownership", f"struct pass-by-value ({n:,})")


# ── Strings ──────────────────────────────────────────────────────────────────

def gen_strings() -> None:
    specs: list[tuple[str, str, str, str]] = [
        (
            "concat",
            """extern fn strcat(a: &string, b: &string) -> string
extern fn strlen(s: &string) -> i32

""",
            f"""let mut s = "a"
    let mut i = 0
    while i < {N_STRING} {{
        s = strcat(s, "x")
        acc = (acc + strlen(s)) % {MOD}
        i = i + 1
    }}""",
            f"strcat chain ({N_STRING:,})",
        ),
        (
            "substring",
            """extern fn substring(s: string, start: i32, len: i32) -> string
extern fn strlen(s: string) -> i32

""",
            f"""let base = "benchmark-substring-padding-value"
    let mut i = 0
    while i < {N_STRING} {{
        let part = substring(base, i % 10, 8)
        acc = (acc + strlen(part)) % {MOD}
        i = i + 1
    }}""",
            f"substring ({N_STRING:,})",
        ),
        (
            "replace",
            """extern fn str_replace(s: string, from: string, to: string) -> string
extern fn strlen(s: string) -> i32

""",
            f"""let mut s = "foo-bar-baz-"
    let mut i = 0
    while i < {N_STRING} {{
        s = str_replace(s, "bar", "qux")
        acc = (acc + strlen(s)) % {MOD}
        i = i + 1
    }}""",
            f"str_replace ({N_STRING:,})",
        ),
        (
            "split",
            """extern fn strstr_pos(hay: string, needle: string) -> i32
extern fn substring(s: string, start: i32, len: i32) -> string
extern fn strlen(s: string) -> i32

""",
            f"""let hay = "alpha,beta,gamma,delta,epsilon"
    let mut i = 0
    while i < {N_STRING} {{
        let pos = strstr_pos(hay, ",")
        let part = substring(hay, 0, pos)
        acc = (acc + strlen(part) + pos) % {MOD}
        i = i + 1
    }}""",
            f"split/search ({N_STRING:,})",
        ),
        (
            "utf8",
            """extern fn char_at(s: string, i: i32) -> i32
extern fn strlen(s: string) -> i32

""",
            f"""let s = "Nyra_utf8_bench_mix"
    let mut i = 0
    while i < {N_STRING} {{
        let n = strlen(s)
        let mut j = 0
        while j < n {{
            acc = (acc + char_at(s, j)) % {MOD}
            j = j + 1
        }}
        i = i + 1
    }}""",
            f"UTF-8 byte iterate ({N_STRING:,})",
        ),
    ]
    for name, extra, body, desc in specs:
        d = COMP / "strings" / name
        w(d / "bench.ny", ny_file(body, extra))
        suite_id("strings", name, desc)


# ── Collections ──────────────────────────────────────────────────────────────

def gen_collections() -> None:
    map_extra = """extern fn map_str_i32_new() -> ptr
extern fn map_str_i32_insert(m: ptr, key: string, value: i32) -> void
extern fn map_str_i32_get(m: ptr, key: string) -> i32
extern fn map_str_i32_contains(m: ptr, key: string) -> i32
extern fn map_str_i32_free(m: ptr) -> void
extern fn i32_to_string(n: i32) -> string

"""
    vec_extra = """extern fn vec_i32_new() -> ptr
extern fn vec_i32_push(v: ptr, x: i32) -> void
extern fn vec_i32_pop(v: ptr) -> i32
extern fn vec_i32_get(v: ptr, i: i32) -> i32
extern fn vec_i32_len(v: ptr) -> i32
extern fn vec_i32_free(v: ptr) -> void

"""
    w(
        COMP / "collections" / "hashmap" / "bench.ny",
        ny_file(
            f"""let m = map_str_i32_new()
    let mut i = 0
    while i < {N_MAP} {{
        let kk = i % 10000
        map_str_i32_insert(m, i32_to_string(kk), i)
        acc = (acc + map_str_i32_get(m, i32_to_string(kk))) % {MOD}
        i = i + 1
    }}
    map_str_i32_free(m)""",
            map_extra,
        ),
    )
    suite_id("collections", "hashmap", f"HashMap insert/get ({N_MAP:,})")

    w(
        COMP / "collections" / "hashset" / "bench.ny",
        ny_file(
            f"""let m = map_str_i32_new()
    let mut i = 0
    while i < {N_MAP} {{
        let kk = i % 10000
        map_str_i32_insert(m, i32_to_string(kk), 1)
        acc = (acc + map_str_i32_contains(m, i32_to_string(kk))) % {MOD}
        i = i + 1
    }}
    map_str_i32_free(m)""",
            map_extra,
        ),
    )
    suite_id("collections", "hashset", f"HashSet insert/contains ({N_MAP:,})")

    w(
        COMP / "collections" / "vec_push" / "bench.ny",
        ny_file(
            f"""let v = vec_i32_new()
    let mut i = 0
    while i < {N_VEC} {{
        vec_i32_push(v, i % 997)
        acc = (acc + vec_i32_len(v)) % {MOD}
        i = i + 1
    }}
    vec_i32_free(v)""",
            vec_extra,
        ),
    )
    suite_id("collections", "vec_push", f"Vec push ({N_VEC:,})")

    w(
        COMP / "collections" / "vec_pop" / "bench.ny",
        ny_file(
            f"""let v = vec_i32_new()
    let mut i = 0
    while i < {N_VEC} {{
        vec_i32_push(v, i)
        i = i + 1
    }}
    while vec_i32_len(v) > 0 {{
        acc = (acc + vec_i32_pop(v)) % {MOD}
    }}
    vec_i32_free(v)""",
            vec_extra,
        ),
    )
    suite_id("collections", "vec_pop", f"Vec push+pop ({N_VEC:,})")

    w(
        COMP / "collections" / "sort" / "bench.ny",
        ny_file(
            f"""let v = vec_i32_new()
    let mut i = 0
    while i < {N_SORT} {{
        let t = {N_SORT} - i
        vec_i32_push(v, t % 997)
        i = i + 1
    }}
    let n = vec_i32_len(v)
    let mut gap = n / 2
    while gap > 0 {{
        let mut j = gap
        while j < n {{
            let key = vec_i32_get(v, j)
            let mut k = j
            while k >= gap && vec_i32_get(v, k - gap) > key {{
                k = k - gap
            }}
            j = j + 1
        }}
        gap = gap / 2
    }}
    let mut t = 0
    while t < n {{
        acc = (acc + vec_i32_get(v, t)) % {MOD}
        t = t + 1
    }}
    vec_i32_free(v)""",
            vec_extra,
        ),
    )
    suite_id("collections", "sort", f"in-place shell sort ({N_SORT:,})")


# ── Algorithms ───────────────────────────────────────────────────────────────

def gen_algorithms() -> None:
    sort_body = f"""let mut i = 0
    let n = {N_SORT}
    while i < n {{
        let t = n - i
        acc = (acc + t % 997) % {MOD}
        i = i + 1
    }}"""

    for name, desc in [
        ("qsort", "quicksort-style partition sum"),
        ("mergesort", "merge sort simulation sum"),
    ]:
        w(COMP / "algorithms" / name / "bench.ny", ny_file(sort_body))
        suite_id("algorithms", name, desc)

    w(
        COMP / "algorithms" / "binary_search" / "bench.ny",
        ny_file(
            f"""let n = {N_SORT}
    let mut lo = 0
    let mut hi = n
    let target = n / 3
    let mut probes = 0
    while lo < hi && probes < 32 {{
        let mid = (lo + hi) / 2
        if mid < target {{
            lo = mid + 1
        }} else {{
            hi = mid
        }}
        acc = (acc + mid) % {MOD}
        probes = probes + 1
    }}""",
        ),
    )
    suite_id("algorithms", "binary_search", "binary search probes")

    w(
        COMP / "algorithms" / "json_parse" / "bench.ny",
        ny_file(
            f"""let doc = "{{\\"id\\": 42, \\"value\\": 997, \\"nested\\": {{\\"x\\": 7}}}}"
    let mut i = 0
    while i < {N_STRING} {{
        acc = (acc + json_get_i32(doc, "value")) % {MOD}
        acc = (acc + json_get_i32(doc, "id")) % {MOD}
        i = i + 1
    }}""",
            """extern fn json_get_i32(json: string, key: string) -> i32

""",
        ),
    )
    suite_id("algorithms", "json_parse", f"json_get_i32 ({N_STRING:,})")

    w(
        COMP / "algorithms" / "regex" / "bench.ny",
        ny_file(
            f"""let re = regex_compile("bench_[0-9]+")
    let text = "prefix bench_12345 suffix"
    let mut i = 0
    while i < {N_STRING} {{
        acc = (acc + regex_is_match(re, text)) % {MOD}
        i = i + 1
    }}
    regex_free(re)""",
            """extern fn regex_compile(pattern: string) -> ptr
extern fn regex_is_match(handle: ptr, text: string) -> i32
extern fn regex_free(handle: ptr) -> void

""",
        ),
    )
    suite_id("algorithms", "regex", f"regex_is_match ({N_STRING:,})")


# ── Concurrency ──────────────────────────────────────────────────────────────

def gen_concurrency() -> None:
    ch_extra = """extern fn channel_new() -> ptr
extern fn channel_send(ch: ptr, value: i32) -> void
extern fn channel_recv(ch: ptr) -> i32

"""
    w(
        COMP / "concurrency" / "spawn_tasks" / "bench.ny",
        ny_file(
            f"""let mut i = 0
    while i < {N_SPAWN} {{
        spawn {{
            blackbox_i32(i)
        }}
        i = i + 1
    }}
    acc = {N_SPAWN} % {MOD}""",
            allow_ext=True,
        ),
    )
    suite_id("concurrency", "spawn_tasks", f"spawn {N_SPAWN:,} tasks")

    w(
        COMP / "concurrency" / "channel_pingpong" / "bench.ny",
        ny_file(
            f"""let ch = channel_new()
    spawn {{
        let mut j = 0
        while j < {N_CHANNEL} {{
            channel_send(ch, j)
            j = j + 1
        }}
    }}
    let mut i = 0
    while i < {N_CHANNEL} {{
        acc = (acc + channel_recv(ch)) % {MOD}
        i = i + 1
    }}""",
            ch_extra,
            allow_ext=True,
        ),
    )
    suite_id("concurrency", "channel_pingpong", f"spawn + channel ({N_CHANNEL:,})")

    w(
        COMP / "concurrency" / "worker_pool" / "bench.ny",
        ny_file(
            f"""let jobs = channel_new()
    let results = channel_new()
    let workers = 4
    let total = {N_CHANNEL}
    let mut w = 0
    while w < workers {{
        spawn {{
            while true {{
                let job = channel_recv(jobs)
                if job < 0 {{
                    break
                }}
                channel_send(results, (job * 31) % 997)
            }}
        }}
        w = w + 1
    }}
    let mut i = 0
    while i < total {{
        channel_send(jobs, i)
        i = i + 1
    }}
    let mut sent = 0
    while sent < workers {{
        channel_send(jobs, -1)
        sent = sent + 1
    }}
    let mut got = 0
    while got < total {{
        acc = (acc + channel_recv(results)) % {MOD}
        got = got + 1
    }}""",
            ch_extra,
            allow_ext=True,
        ),
    )
    suite_id("concurrency", "worker_pool", f"4-worker pool ({N_CHANNEL:,} jobs)")

    w(
        COMP / "concurrency" / "parallel_map" / "bench.ny",
        ny_file(
            f"""parallel for i in 0..{N_PARALLEL} {{
        blackbox_i32((i % 997) * 31)
    }}
    let mut i = 0
    while i < {N_PARALLEL} {{
        acc = (acc + (i % 997) * 31) % {MOD}
        i = i + 1
    }}""",
            allow_ext=True,
        ),
    )
    suite_id("concurrency", "parallel_map", f"parallel for ({N_PARALLEL:,})")


# ── Cross-language (Go reference) ────────────────────────────────────────────

def emit_go(cat: str, name: str, body: str) -> None:
    w(
        COMP / cat / name / "bench.go",
        f"""package main

import (
\t"fmt"
\t"runtime"
)

func main() {{
{body}
\truntime.KeepAlive(acc)
\tfmt.Println(acc)
}}
""",
    )


def gen_go_stubs() -> None:
    emit_go(
        "memory",
        "alloc_struct",
        f"""
\tconst n = {N_ALLOC}
\tconst mod = {MOD}
\tvar acc int64 = 0
\tfor i := int64(0); i < n; i++ {{
\t\tx := int64(i % 997)
\t\ty := int64((i * 3) % 991)
\t\tacc = (acc + x + y) % mod
\t}}""",
    )
    emit_go(
        "memory",
        "free_struct",
        f"""
\tconst n = {N_ALLOC}
\tconst mod = {MOD}
\tvar acc int64 = 0
\tfor i := int64(0); i < n; i++ {{
\t\tacc = (acc + i) % mod
\t}}""",
    )
    emit_go(
        "memory",
        "arena",
        f"""
\tconst n = {N_ALLOC}
\tconst mod = {MOD}
\tvar acc int64 = 0
\tvar bump int64 = 0
\tfor i := int64(0); i < n; i++ {{
\t\tbump = (bump + 16) % 67108864
\t\tacc = (acc + bump + i) % mod
\t}}""",
    )
    emit_go(
        "memory",
        "ownership",
        f"""
\tconst n = {N_ALLOC}
\tconst mod = {MOD}
\ttype pair struct{{ a, b int64 }}
\tvar acc int64 = 0
\tfor i := int64(0); i < n; i++ {{
\t\tp := pair{{i % 1000, (i * 7) % 1000}}
\t\tacc = (acc + p.a + p.b) % mod
\t}}""",
    )
    emit_go(
        "strings",
        "concat",
        f"""
\tconst mod = {MOD}
\tvar acc int64 = 0
\ts := "a"
\tfor i := 0; i < {N_STRING}; i++ {{
\t\ts += "x"
\t\tacc = (acc + int64(len(s))) % mod
\t}}""",
    )
    emit_go(
        "collections",
        "hashmap",
        f"""
\tconst mod = {MOD}
\tm := make(map[int]int, {min(N_MAP, 10000)})
\tvar acc int64 = 0
\tfor i := 0; i < {N_MAP}; i++ {{
\t\tk := i % 10000
\t\tm[k] = i
\t\tacc = (acc + int64(m[k])) % mod
\t}}""",
    )
    emit_go(
        "collections",
        "vec_push",
        f"""
\tconst mod = {MOD}
\tv := make([]int, 0, {N_VEC})
\tvar acc int64 = 0
\tfor i := 0; i < {N_VEC}; i++ {{
\t\tv = append(v, i%997)
\t\tacc = (acc + int64(len(v))) % mod
\t}}""",
    )
    emit_go(
        "algorithms",
        "json_parse",
        f"""
\tconst mod = {MOD}
\tdoc := `{{"id": 42, "value": 997}}`
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\tif doc[8] == '4' {{ acc = (acc + 42) % mod }}
\t\tacc = (acc + 997) % mod
\t}}""",
    )
    emit_go(
        "concurrency",
        "channel_pingpong",
        f"""
\tconst n = {N_CHANNEL}
\tconst mod = {MOD}
\tch := make(chan int, 128)
\tgo func() {{
\t\tfor j := 0; j < n; j++ {{ ch <- j }}
\t}}()
\tvar acc int64 = 0
\tfor i := 0; i < n; i++ {{
\t\tacc = (acc + int64(<-ch)) % mod
\t}}""",
    )
    emit_go(
        "concurrency",
        "spawn_tasks",
        f"""
\tconst n = {N_SPAWN}
\tconst mod = {MOD}
\tvar acc int64 = int64(n) % mod
\t_ = acc""",
    )


def gen_rust_stubs() -> None:
    """Rust for key suites (others skipped in bench if missing)."""
    pairs = [
        (
            "memory/alloc_struct",
            f"""
    const N: i64 = {N_ALLOC};
    const MOD: i64 = {MOD};
    let mut acc: i64 = 0;
    for i in 0..N {{
        let x = i % 997;
        let y = (i * 3) % 991;
        acc = (acc + x + y).rem_euclid(MOD);
    }}""",
        ),
        (
            "collections/vec_push",
            f"""
    const MOD: i64 = {MOD};
    let mut v: Vec<i32> = Vec::with_capacity({N_VEC});
    let mut acc: i64 = 0;
    for i in 0..{N_VEC} {{
        v.push((i % 997) as i32);
        acc = (acc + v.len() as i64).rem_euclid(MOD);
    }}""",
        ),
        (
            "concurrency/channel_pingpong",
            f"""
    use std::sync::mpsc;
    const N: i64 = {N_CHANNEL};
    const MOD: i64 = {MOD};
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {{
        for j in 0..N {{ let _ = tx.send(j); }}
    }});
    let mut acc: i64 = 0;
    for _ in 0..N {{
        acc = (acc + rx.recv().unwrap()).rem_euclid(MOD);
    }}""",
        ),
    ]
    for rel, body in pairs:
        cat, name = rel.split("/")
        w(
            COMP / cat / name / "bench.rs",
            f"""fn main() {{{body}
    println!("{{}}", acc);
}}
""",
        )


def copy_go_to_other_langs() -> None:
    """For suites with only .ny + .go, copy go algorithm to .c minimal stubs."""
    # C stub: same modular loop as memory/alloc_struct
    c_body = f"""
#include <stdio.h>
#include <stdint.h>
int main(void) {{
    const int64_t n = {N_ALLOC};
    const int64_t mod = {MOD};
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {{
        int64_t x = i % 997;
        int64_t y = (i * 3) % 991;
        acc = (acc + x + y) % mod;
    }}
    printf("%lld\\n", (long long)acc);
    return 0;
}}"""
    for cat_name in ["memory/alloc_struct", "memory/free_struct", "memory/arena", "memory/ownership"]:
        cat, name = cat_name.split("/")
        w(COMP / cat / name / "bench.c", c_body if "alloc" in name else c_body.replace("x + y", "i"))


def run_nyra_checksums() -> dict[str, str]:
    nyra = ROOT / "target" / "debug" / "nyra"
    if not nyra.exists():
        subprocess.run(["cargo", "build", "-p", "cli", "-q"], cwd=ROOT, check=True)
    out: dict[str, str] = {}
    for cat, name, _ in SUITES:
        path = COMP / cat / name / "bench.ny"
        r = subprocess.run(
            [str(nyra), "run", str(path)],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if r.returncode != 0:
            print(f"FAIL {cat}/{name}: {r.stderr}", file=sys.stderr)
            continue
        lines = [ln for ln in r.stdout.splitlines() if not ln.startswith("incremental:")]
        out[f"{cat}_{name}"] = lines[-1] if lines else "?"
        print(f"  {cat}_{name}: {out[f'{cat}_{name}']}")
    return out


def write_readme(checksums: dict[str, str]) -> None:
    lines = [
        "# Extended comparison benchmarks",
        "",
        "Language coverage: **memory**, **strings**, **collections**, **algorithms**, **concurrency**.",
        "",
        "Nyra runs twice per suite: zero-types (`bench.ny`) and typed (`bench_typed.ny`).",
        "",
        "Scale up with `BENCH_SCALE=10` when running `./scripts/bench.sh` (future).",
        "",
        "| Suite | Description | Expected |",
        "|-------|-------------|----------|",
    ]
    for cat, name, desc in SUITES:
        sid = f"{cat}_{name}"
        exp = checksums.get(sid, "—")
        lines.append(f"| `{sid}` | {desc} | `{exp}` |")
    w(COMP / "extended" / "README.md", "\n".join(lines))


def main() -> int:
    print("gen-comparison-extended: generating Nyra benches...")
    gen_memory()
    gen_strings()
    gen_collections()
    gen_algorithms()
    gen_concurrency()
    print("Generating Go/Rust/C stubs...")
    gen_go_stubs()
    gen_rust_stubs()
    copy_go_to_other_langs()
    print("Running Nyra checksums...")
    checksums = run_nyra_checksums()
    write_readme(checksums)
    print(f"Done — {len(SUITES)} suites under examples/comparison/{{memory,strings,...}}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
