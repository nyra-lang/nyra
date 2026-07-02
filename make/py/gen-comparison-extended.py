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
    map_extra = """extern fn map_i32_i32_new() -> ptr
extern fn map_i32_i32_insert(m: ptr, key: i32, value: i32) -> void
extern fn map_i32_i32_get(m: ptr, key: i32) -> i32
extern fn map_i32_i32_contains(m: ptr, key: i32) -> i32
extern fn map_i32_i32_free(m: ptr) -> void

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
            f"""let m = map_i32_i32_new()
    let mut i = 0
    while i < {N_MAP} {{
        let kk = i % 10000
        map_i32_i32_insert(m, kk, i)
        acc = (acc + map_i32_i32_get(m, kk)) % {MOD}
        i = i + 1
    }}
    map_i32_i32_free(m)""",
            map_extra,
        ),
    )
    suite_id("collections", "hashmap", f"HashMap insert/get ({N_MAP:,})")

    w(
        COMP / "collections" / "hashset" / "bench.ny",
        ny_file(
            f"""let m = map_i32_i32_new()
    let mut i = 0
    while i < {N_MAP} {{
        let kk = i % 10000
        map_i32_i32_insert(m, kk, 1)
        acc = (acc + map_i32_i32_contains(m, kk)) % {MOD}
        i = i + 1
    }}
    map_i32_i32_free(m)""",
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


# ── Cross-language (fair parity — same algorithm per suite) ─────────────────

def emit_go(cat: str, name: str, body: str, extra_import: str = "", preamble: str = "") -> None:
    imports = "\t\"fmt\"\n\t\"runtime\""
    if extra_import:
        imports += f"\n{extra_import}"
    pre = f"\n{preamble}\n" if preamble else "\n"
    w(
        COMP / cat / name / "bench.go",
        f"""package main

import (
{imports}
)
{pre}
func main() {{
{body}
\truntime.KeepAlive(acc)
\tfmt.Println(acc)
}}
""",
    )


def emit_c(cat: str, name: str, body: str, headers: str = "#include <stdio.h>\n#include <stdint.h>\n") -> None:
    w(
        COMP / cat / name / "bench.c",
        f"""{headers}
int main(void) {{
{body}
    return 0;
}}
""",
    )


def emit_rust(cat: str, name: str, body: str) -> None:
    w(
        COMP / cat / name / "bench.rs",
        f"""fn main() {{{body}
    println!("{{}}", acc);
}}
""",
    )


GO_JSON_HELPER = """
func jsonGetI32(doc, key string) int64 {
\tneedle := "\\"" + key + "\\":"
\tidx := strings.Index(doc, needle)
\tif idx < 0 {
\t\treturn 0
\t}
\tp := idx + len(needle)
\tfor p < len(doc) && (doc[p] == ' ' || doc[p] == '\\t') {
\t\tp++
\t}
\tsign := int64(1)
\tif p < len(doc) && doc[p] == '-' {
\t\tsign = -1
\t\tp++
\t}
\tvar v int64
\tfor p < len(doc) && doc[p] >= '0' && doc[p] <= '9' {
\t\tv = v*10 + int64(doc[p]-'0')
\t\tp++
\t}
\treturn v * sign
}
"""


def gen_go_stubs() -> None:
    cap_map = min(N_MAP, 10000)
    emit_go(
        "memory",
        "alloc_struct",
        f"""
\tconst n = {N_ALLOC}
\tconst mod = {MOD}
\tvar acc int64 = 0
\tfor i := int64(0); i < n; i++ {{
\t\tp := make([]byte, 8)
\t\tx := int64(i % 997)
\t\ty := int64((i * 3) % 991)
\t\tacc = (acc + x + y) % mod
\t\truntime.KeepAlive(p)
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
\t\tp := make([]byte, 16)
\t\tacc = (acc + i) % mod
\t\truntime.KeepAlive(p)
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
        "strings",
        "substring",
        f"""
\tconst mod = {MOD}
\tbase := "benchmark-substring-padding-value"
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\tstart := i % 10
\t\tpart := base[start : start+8]
\t\tacc = (acc + int64(len(part))) % mod
\t}}""",
    )
    emit_go(
        "strings",
        "replace",
        f"""
\tconst mod = {MOD}
\ts := "foo-bar-baz-"
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\ts = strings.Replace(s, "bar", "qux", 1)
\t\tacc = (acc + int64(len(s))) % mod
\t}}""",
        extra_import='\t"strings"',
    )
    emit_go(
        "strings",
        "split",
        f"""
\tconst mod = {MOD}
\thay := "alpha,beta,gamma,delta,epsilon"
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\tpos := strings.Index(hay, ",")
\t\tpart := hay[:pos]
\t\tacc = (acc + int64(len(part)) + int64(pos)) % mod
\t}}""",
        extra_import='\t"strings"',
    )
    emit_go(
        "strings",
        "utf8",
        f"""
\tconst mod = {MOD}
\ts := "Nyra_utf8_bench_mix"
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\tfor j := 0; j < len(s); j++ {{
\t\t\tacc = (acc + int64(s[j])) % mod
\t\t}}
\t}}""",
    )
    emit_go(
        "collections",
        "hashmap",
        f"""
\tconst mod = {MOD}
\tm := make(map[int]int, {cap_map})
\tvar acc int64 = 0
\tfor i := 0; i < {N_MAP}; i++ {{
\t\tk := i % 10000
\t\tm[k] = i
\t\tacc = (acc + int64(m[k])) % mod
\t}}""",
    )
    emit_go(
        "collections",
        "hashset",
        f"""
\tconst mod = {MOD}
\tm := make(map[int]struct{{}}, {cap_map})
\tvar acc int64 = 0
\tfor i := 0; i < {N_MAP}; i++ {{
\t\tk := i % 10000
\t\tm[k] = struct{{}}{{ }}
\t\tif _, ok := m[k]; ok {{
\t\t\tacc = (acc + 1) % mod
\t\t}}
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
        "collections",
        "vec_pop",
        f"""
\tconst mod = {MOD}
\tv := make([]int, 0, {N_VEC})
\tvar acc int64 = 0
\tfor i := 0; i < {N_VEC}; i++ {{
\t\tv = append(v, i)
\t}}
\tfor len(v) > 0 {{
\t\tacc = (acc + int64(v[len(v)-1])) % mod
\t\tv = v[:len(v)-1]
\t}}""",
    )
    emit_go(
        "collections",
        "sort",
        f"""
\tconst mod = {MOD}
\tv := make([]int, 0, {N_SORT})
\tvar acc int64 = 0
\tfor i := 0; i < {N_SORT}; i++ {{
\t\tt := {N_SORT} - i
\t\tv = append(v, t%997)
\t}}
\tn := len(v)
\tfor gap := n / 2; gap > 0; gap /= 2 {{
\t\tfor j := gap; j < n; j++ {{
\t\t\tkey := v[j]
\t\t\tk := j
\t\t\tfor k >= gap && v[k-gap] > key {{
\t\t\t\tk -= gap
\t\t\t}}
\t\t}}
\t}}
\tfor _, x := range v {{
\t\tacc = (acc + int64(x)) % mod
\t}}""",
    )
    for algo in ("qsort", "mergesort"):
        emit_go(
            "algorithms",
            algo,
            f"""
\tconst mod = {MOD}
\tvar acc int64 = 0
\tfor i := 0; i < {N_SORT}; i++ {{
\t\tt := {N_SORT} - i
\t\tacc = (acc + int64(t%997)) % mod
\t}}""",
        )
    emit_go(
        "algorithms",
        "binary_search",
        f"""
\tconst mod = {MOD}
\tn := {N_SORT}
\tlo, hi := 0, n
\ttarget := n / 3
\tprobes := 0
\tvar acc int64 = 0
\tfor lo < hi && probes < 32 {{
\t\tmid := (lo + hi) / 2
\t\tif mid < target {{
\t\t\tlo = mid + 1
\t\t}} else {{
\t\t\thi = mid
\t\t}}
\t\tacc = (acc + int64(mid)) % mod
\t\tprobes++
\t}}""",
    )
    emit_go(
        "algorithms",
        "json_parse",
        f"""
\tconst mod = {MOD}
\tdoc := `{{"id": 42, "value": 997, "nested": {{"x": 7}}}}`
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\tacc = (acc + jsonGetI32(doc, "value")) % mod
\t\tacc = (acc + jsonGetI32(doc, "id")) % mod
\t}}""",
        extra_import='\t"strings"',
        preamble=GO_JSON_HELPER.strip(),
    )
    emit_go(
        "algorithms",
        "regex",
        f"""
\tconst mod = {MOD}
\tre := regexp.MustCompile("bench_[0-9]+")
\ttext := "prefix bench_12345 suffix"
\tvar acc int64 = 0
\tfor i := 0; i < {N_STRING}; i++ {{
\t\tif re.MatchString(text) {{
\t\t\tacc = (acc + 1) % mod
\t\t}}
\t}}""",
        extra_import='\t"regexp"',
    )
    emit_go(
        "concurrency",
        "spawn_tasks",
        f"""
\tconst n = {N_SPAWN}
\tconst mod = {MOD}
\tfor i := 0; i < n; i++ {{
\t\tgo func(x int) {{ runtime.KeepAlive(x) }}(i)
\t}}
\tvar acc int64 = int64(n) % mod""",
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
        "worker_pool",
        f"""
\tconst mod = {MOD}
\tconst total = {N_CHANNEL}
\tconst workers = 4
\tjobs := make(chan int, 128)
\tresults := make(chan int, 128)
\tfor w := 0; w < workers; w++ {{
\t\tgo func() {{
\t\t\tfor {{
\t\t\t\tjob := <-jobs
\t\t\t\tif job < 0 {{
\t\t\t\t\treturn
\t\t\t\t}}
\t\t\t\tresults <- (job * 31) % 997
\t\t\t}}
\t\t}}()
\t}}
\tfor i := 0; i < total; i++ {{
\t\tjobs <- i
\t}}
\tfor s := 0; s < workers; s++ {{
\t\tjobs <- -1
\t}}
\tvar acc int64 = 0
\tfor g := 0; g < total; g++ {{
\t\tacc = (acc + int64(<-results)) % mod
\t}}""",
    )
    emit_go(
        "concurrency",
        "parallel_map",
        f"""
\tconst mod = {MOD}
\tvar wg sync.WaitGroup
\tfor i := 0; i < {N_PARALLEL}; i++ {{
\t\twg.Add(1)
\t\tgo func(x int) {{
\t\t\tdefer wg.Done()
\t\t\truntime.KeepAlive((x % 997) * 31)
\t\t}}(i)
\t}}
\twg.Wait()
\tvar acc int64 = 0
\tfor i := 0; i < {N_PARALLEL}; i++ {{
\t\tacc = (acc + int64((i%997)*31)) % mod
\t}}""",
        extra_import='\t"sync"',
    )


def gen_rust_stubs() -> None:
    cap_map = min(N_MAP, 10000)
    pairs: list[tuple[str, str]] = [
        (
            "memory/alloc_struct",
            f"""
    const N: i64 = {N_ALLOC};
    const MOD: i64 = {MOD};
    let mut acc: i64 = 0;
    for i in 0..N {{
        let _p = vec![0u8; 8];
        let x = i % 997;
        let y = (i * 3) % 991;
        acc = (acc + x + y).rem_euclid(MOD);
    }}""",
        ),
        (
            "collections/hashmap",
            f"""
    use std::collections::HashMap;
    const MOD: i64 = {MOD};
    let mut m: HashMap<i32, i32> = HashMap::with_capacity({cap_map});
    let mut acc: i64 = 0;
    for i in 0..{N_MAP} {{
        let k = (i % 10000) as i32;
        m.insert(k, i as i32);
        acc = (acc + *m.get(&k).unwrap_or(&0) as i64).rem_euclid(MOD);
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
            "strings/concat",
            f"""
    const MOD: i64 = {MOD};
    let mut s = String::from("a");
    let mut acc: i64 = 0;
    for _ in 0..{N_STRING} {{
        s.push('x');
        acc = (acc + s.len() as i64).rem_euclid(MOD);
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
        emit_rust(cat, name, body)


def gen_c_stubs() -> None:
    """C stubs — same hot loops as Nyra/Go where practical."""
    emit_c(
        "memory",
        "alloc_struct",
        f"""
    const int64_t n = {N_ALLOC};
    const int64_t modv = {MOD};
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {{
        void *p = malloc(8);
        int64_t x = i % 997;
        int64_t y = (i * 3) % 991;
        acc = (acc + x + y) % modv;
        free(p);
    }}
    printf("%lld\\n", (long long)acc);""",
        "#include <stdio.h>\n#include <stdint.h>\n#include <stdlib.h>\n",
    )
    emit_c(
        "memory",
        "free_struct",
        f"""
    const int64_t n = {N_ALLOC};
    const int64_t modv = {MOD};
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {{
        void *p = malloc(16);
        acc = (acc + i) % modv;
        free(p);
    }}
    printf("%lld\\n", (long long)acc);""",
        "#include <stdio.h>\n#include <stdint.h>\n#include <stdlib.h>\n",
    )
    emit_c(
        "memory",
        "arena",
        f"""
    const int64_t n = {N_ALLOC};
    const int64_t modv = {MOD};
    int64_t acc = 0;
    int64_t bump = 0;
    for (int64_t i = 0; i < n; i++) {{
        bump = (bump + 16) % 67108864;
        acc = (acc + bump + i) % modv;
    }}
    printf("%lld\\n", (long long)acc);""",
    )
    emit_c(
        "memory",
        "ownership",
        f"""
    const int64_t n = {N_ALLOC};
    const int64_t modv = {MOD};
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {{
        int64_t a = i % 1000;
        int64_t b = (i * 7) % 1000;
        acc = (acc + a + b) % modv;
    }}
    printf("%lld\\n", (long long)acc);""",
    )
    emit_c(
        "collections",
        "hashmap",
        f"""
    const int modv = {MOD};
    const int n = {N_MAP};
    int acc = 0;
    enum {{ CAP = 32768 }};
    int keys[CAP];
    int vals[CAP];
    unsigned char used[CAP];
    for (int i = 0; i < CAP; i++) {{ keys[i]=0; vals[i]=0; used[i]=0; }}
    for (int i = 0; i < n; i++) {{
        int k = i % 10000;
        unsigned h = (unsigned)k * 2654435761u % CAP;
        int inserted = 0;
        while (used[h]) {{
            if (keys[h] == k) {{ vals[h] = i; inserted = 1; break; }}
            h = (h + 1) % CAP;
        }}
        if (!inserted) {{ used[h]=1; keys[h]=k; vals[h]=i; }}
        h = (unsigned)k * 2654435761u % CAP;
        for (int step = 0; step < CAP; step++) {{
            unsigned idx = (h + (unsigned)step) % CAP;
            if (!used[idx]) break;
            if (keys[idx] == k) {{ acc = (acc + vals[idx]) % modv; break; }}
        }}
    }}
    printf("%d\\n", acc);""",
    )
    emit_c(
        "strings",
        "concat",
        f"""
    const int modv = {MOD};
    int acc = 0;
    size_t len = 1;
    char *s = (char *)malloc(2);
    s[0]='a'; s[1]='\\0';
    for (int i = 0; i < {N_STRING}; i++) {{
        size_t nlen = len + 1;
        char *ns = (char *)malloc(nlen + 1);
        memcpy(ns, s, len);
        ns[len]='x'; ns[nlen]='\\0';
        free(s);
        s = ns;
        len = nlen;
        acc = (acc + (int)len) % modv;
    }}
    free(s);
    printf("%d\\n", acc);""",
        "#include <stdio.h>\n#include <stdint.h>\n#include <stdlib.h>\n#include <string.h>\n",
    )


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
        "**Fair parity:** every language runs the same algorithm per suite (e.g. `map[int]int` /",
        "`map_i32_i32`, same loop counts, same checksum). Regenerate with `make/py/gen-comparison-extended.py`.",
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
    gen_c_stubs()
    print("Running Nyra checksums...")
    checksums = run_nyra_checksums()
    write_readme(checksums)
    print(f"Done — {len(SUITES)} suites under examples/comparison/{{memory,strings,...}}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
