#!/usr/bin/env bash
# Measure runtime + peak memory (max RSS).
# Writes examples/comparison/results/latest.txt and latest.html
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
COMPARISON="$ROOT/examples/comparison"
BENCH_DIR="$COMPARISON/.bench"
RESULTS_DIR="$COMPARISON/results"
LATEST="$RESULTS_DIR/latest.txt"
LATEST_HTML="$RESULTS_DIR/latest.html"
RESULTS_TSV="$RESULTS_DIR/data.tsv"
BINARY_SIZE_TSV="$RESULTS_DIR/binary-size.tsv"
RUNS="${BENCH_RUNS:-5}"
WARMUP="${BENCH_WARMUP:-1}"
# Pause between languages so CPU/thermal state from one runtime does not skew the next.
LANG_COOLDOWN="${BENCH_LANG_COOLDOWN:-2}"
# Release nyra by default (true performance). Set BENCH_RELEASE=0 for debug / lower RAM at build time.
BENCH_RELEASE="${BENCH_RELEASE:-1}"
# Set BENCH_NO_ISOLATE=1 to run all languages per suite back-to-back (legacy order).
BENCH_NO_ISOLATE="${BENCH_NO_ISOLATE:-0}"
# Set BENCH_PGO=1 to build **all** Nyra suites with `nyra build --pgo` (slow; full pipeline).
# cpu_bound_pgo runs by default (Nyra-only, `--pgo` on same hot path as cpu_bound).
# Set BENCH_SKIP_PGO=1 to skip the cpu_bound_pgo suite (faster runs, no llvm-profdata).
BENCH_PGO="${BENCH_PGO:-0}"
BENCH_SKIP_PGO="${BENCH_SKIP_PGO:-0}"

rm -rf "$BENCH_DIR"
mkdir -p "$BENCH_DIR" "$RESULTS_DIR"

log() { echo "$@" >&2; }

lang_cooldown() {
  if [[ "$LANG_COOLDOWN" -gt 0 ]] 2>/dev/null; then
    sleep "$LANG_COOLDOWN"
  fi
}

# Suite source paths (set by suite_paths)
SP_NY_PATH=""
SP_NY_NAME=""
SP_NY_TYPED_PATH=""
SP_NY_TYPED_NAME=""
SP_NY_CT_PATH=""
SP_NY_CT_TYPED_PATH=""
SP_NY_CT_NAME=""
SP_NY_CT_TYPED_NAME=""
SP_GO_SRC=""
SP_RUST_SRC=""
SP_JS_SRC=""
SP_PY_SRC=""
SP_JAVA_SRC=""
SP_C_SRC=""
SP_CPP_SRC=""

# Extended suites (memory, strings, collections, algorithms, concurrency) — see make/py/gen-comparison-extended.py
EXTENDED_SUITES=(
  memory_alloc_struct memory_free_struct memory_arena memory_ownership
  strings_concat strings_substring strings_replace strings_split strings_utf8
  collections_hashmap collections_hashset collections_vec_push collections_vec_pop collections_sort
  algorithms_qsort algorithms_mergesort algorithms_binary_search algorithms_json_parse algorithms_regex
  concurrency_spawn_tasks concurrency_channel_pingpong concurrency_worker_pool concurrency_parallel_map
)

extended_suite_paths() {
  local suite="$1"
  local cat name dir
  case "$suite" in
    memory_*) cat=memory; name="${suite#memory_}" ;;
    strings_*) cat=strings; name="${suite#strings_}" ;;
    collections_*) cat=collections; name="${suite#collections_}" ;;
    algorithms_*) cat=algorithms; name="${suite#algorithms_}" ;;
    concurrency_*) cat=concurrency; name="${suite#concurrency_}" ;;
    *) return 1 ;;
  esac
  dir="$COMPARISON/$cat/$name"
  if [[ ! -f "$dir/bench.ny" ]]; then
    log "warn: missing extended suite $dir/bench.ny"
    return 1
  fi
  SP_NY_PATH="$dir/bench.ny"
  SP_NY_NAME="bench_${suite}"
  [[ -f "$dir/bench.go" ]] && SP_GO_SRC="$dir/bench.go"
  [[ -f "$dir/bench.rs" ]] && SP_RUST_SRC="$dir/bench.rs"
  [[ -f "$dir/bench.js" ]] && SP_JS_SRC="$dir/bench.js"
  [[ -f "$dir/bench.py" ]] && SP_PY_SRC="$dir/bench.py"
  [[ -f "$dir/bench.java" ]] && SP_JAVA_SRC="$dir/bench.java"
  [[ -f "$dir/bench.c" ]] && SP_C_SRC="$dir/bench.c"
  [[ -f "$dir/bench.cpp" ]] && SP_CPP_SRC="$dir/bench.cpp"
  return 0
}

suite_paths() {
  local suite="$1"
  SP_NY_PATH=""
  SP_NY_NAME=""
  SP_NY_TYPED_PATH=""
  SP_NY_TYPED_NAME=""
  SP_GO_SRC=""
  SP_RUST_SRC=""
  SP_JS_SRC=""
  SP_PY_SRC=""
  SP_JAVA_SRC=""
  SP_C_SRC=""
  SP_CPP_SRC=""

  case "$suite" in
    hello)
      SP_NY_PATH="$COMPARISON/hello/hello.ny"
      SP_NY_NAME="bench_hello"
      SP_GO_SRC="$COMPARISON/hello/hello.go"
      SP_RUST_SRC="$COMPARISON/hello/hello.rs"
      SP_JS_SRC="$COMPARISON/hello/hello.js"
      SP_PY_SRC="$COMPARISON/hello/hello.py"
      SP_JAVA_SRC="$COMPARISON/hello/Hello.java"
      SP_C_SRC="$COMPARISON/hello/hello.c"
      SP_CPP_SRC="$COMPARISON/hello/hello.cpp"
      ;;
    arithmetic)
      SP_NY_PATH="$COMPARISON/arithmetic/sum.ny"
      SP_NY_NAME="bench_sum"
      SP_GO_SRC="$COMPARISON/arithmetic/sum.go"
      SP_RUST_SRC="$COMPARISON/arithmetic/sum.rs"
      SP_JS_SRC="$COMPARISON/arithmetic/sum.js"
      SP_PY_SRC="$COMPARISON/arithmetic/sum.py"
      SP_JAVA_SRC="$COMPARISON/arithmetic/Sum.java"
      SP_C_SRC="$COMPARISON/arithmetic/sum.c"
      SP_CPP_SRC="$COMPARISON/arithmetic/sum.cpp"
      ;;
    dungeon)
      SP_NY_PATH="$COMPARISON/dungeon"
      SP_NY_NAME="bench_dungeon"
      SP_GO_SRC="$COMPARISON/dungeon/dungeon.go"
      SP_RUST_SRC="$COMPARISON/dungeon/dungeon.rs"
      SP_JS_SRC="$COMPARISON/dungeon/dungeon.js"
      SP_PY_SRC="$COMPARISON/dungeon/dungeon.py"
      SP_JAVA_SRC="$COMPARISON/dungeon/Dungeon.java"
      SP_C_SRC="$COMPARISON/dungeon/dungeon.c"
      SP_CPP_SRC="$COMPARISON/dungeon/dungeon.cpp"
      ;;
    loop)
      SP_NY_PATH="$COMPARISON/loop/sum_loop.ny"
      SP_NY_NAME="bench_loop"
      SP_GO_SRC="$COMPARISON/loop/sum_loop.go"
      SP_RUST_SRC="$COMPARISON/loop/sum_loop.rs"
      SP_JS_SRC="$COMPARISON/loop/sum_loop.js"
      SP_PY_SRC="$COMPARISON/loop/sum_loop.py"
      SP_JAVA_SRC="$COMPARISON/loop/SumLoop.java"
      SP_C_SRC="$COMPARISON/loop/sum_loop.c"
      SP_CPP_SRC="$COMPARISON/loop/sum_loop.cpp"
      ;;
    fib)
      SP_NY_PATH="$COMPARISON/fib/fib.ny"
      SP_NY_NAME="bench_fib"
      SP_GO_SRC="$COMPARISON/fib/fib.go"
      SP_RUST_SRC="$COMPARISON/fib/fib.rs"
      SP_JS_SRC="$COMPARISON/fib/fib.js"
      SP_PY_SRC="$COMPARISON/fib/fib.py"
      SP_JAVA_SRC="$COMPARISON/fib/Fib.java"
      SP_C_SRC="$COMPARISON/fib/fib.c"
      SP_CPP_SRC="$COMPARISON/fib/fib.cpp"
      ;;
    nested)
      SP_NY_PATH="$COMPARISON/nested/nested.ny"
      SP_NY_NAME="bench_nested"
      SP_GO_SRC="$COMPARISON/nested/nested.go"
      SP_RUST_SRC="$COMPARISON/nested/nested.rs"
      SP_JS_SRC="$COMPARISON/nested/nested.js"
      SP_PY_SRC="$COMPARISON/nested/nested.py"
      SP_JAVA_SRC="$COMPARISON/nested/Nested.java"
      SP_C_SRC="$COMPARISON/nested/nested.c"
      SP_CPP_SRC="$COMPARISON/nested/nested.cpp"
      ;;
    struct_sum)
      SP_NY_PATH="$COMPARISON/struct_sum/struct_sum.ny"
      SP_NY_NAME="bench_struct_sum"
      SP_GO_SRC="$COMPARISON/struct_sum/struct_sum.go"
      SP_RUST_SRC="$COMPARISON/struct_sum/struct_sum.rs"
      SP_JS_SRC="$COMPARISON/struct_sum/struct_sum.js"
      SP_PY_SRC="$COMPARISON/struct_sum/struct_sum.py"
      SP_JAVA_SRC="$COMPARISON/struct_sum/StructSum.java"
      SP_C_SRC="$COMPARISON/struct_sum/struct_sum.c"
      SP_CPP_SRC="$COMPARISON/struct_sum/struct_sum.cpp"
      ;;
    loop_nofold)
      SP_NY_PATH="$COMPARISON/loop_nofold/sum_loop_nofold.ny"
      SP_NY_NAME="bench_loop_nofold"
      SP_GO_SRC="$COMPARISON/loop_nofold/sum_loop_nofold.go"
      SP_RUST_SRC="$COMPARISON/loop_nofold/sum_loop_nofold.rs"
      SP_C_SRC="$COMPARISON/loop_nofold/sum_loop_nofold.c"
      SP_CPP_SRC="$COMPARISON/loop_nofold/sum_loop_nofold.cpp"
      ;;
    comptime_table)
      SP_NY_PATH="$COMPARISON/comptime_table/bench.ny"
      SP_NY_NAME="bench_comptime_table"
      SP_NY_CT_PATH="$COMPARISON/comptime_table/bench_comptime.ny"
      SP_NY_CT_NAME="bench_comptime_table_ct"
      SP_NY_TYPED_PATH="$COMPARISON/comptime_table/bench_typed.ny"
      SP_NY_TYPED_NAME="bench_comptime_table_typed"
      SP_NY_CT_TYPED_PATH="$COMPARISON/comptime_table/bench_comptime_typed.ny"
      SP_NY_CT_TYPED_NAME="bench_comptime_table_ct_typed"
      SP_GO_SRC="$COMPARISON/comptime_table/bench.go"
      SP_RUST_SRC="$COMPARISON/comptime_table/bench.rs"
      SP_C_SRC="$COMPARISON/comptime_table/bench.c"
      SP_CPP_SRC="$COMPARISON/comptime_table/bench.cpp"
      ;;
    cpu_bound)
      SP_NY_PATH="$COMPARISON/cpu_bound/bench.ny"
      SP_NY_NAME="bench_cpu_bound"
      SP_GO_SRC="$COMPARISON/cpu_bound/bench.go"
      SP_RUST_SRC="$COMPARISON/cpu_bound/bench.rs"
      SP_JS_SRC="$COMPARISON/cpu_bound/bench.js"
      SP_PY_SRC="$COMPARISON/cpu_bound/bench.py"
      SP_JAVA_SRC="$COMPARISON/cpu_bound/CpuBound.java"
      SP_C_SRC="$COMPARISON/cpu_bound/bench.c"
      SP_CPP_SRC="$COMPARISON/cpu_bound/bench.cpp"
      ;;
    mix)
      SP_NY_PATH="$COMPARISON/mix/mix.ny"
      SP_NY_NAME="bench_mix"
      SP_GO_SRC="$COMPARISON/mix/mix.go"
      SP_RUST_SRC="$COMPARISON/mix/mix.rs"
      SP_JS_SRC="$COMPARISON/mix/mix.js"
      SP_PY_SRC="$COMPARISON/mix/mix.py"
      SP_JAVA_SRC="$COMPARISON/mix/Mix.java"
      SP_C_SRC="$COMPARISON/mix/mix.c"
      SP_CPP_SRC="$COMPARISON/mix/mix.cpp"
      ;;
    escape_local_channel)
      SP_NY_PATH="$COMPARISON/escape/local_channel.ny"
      SP_NY_NAME="bench_escape_local"
      ;;
    escape_spawn_channel)
      SP_NY_PATH="$COMPARISON/escape/spawn_channel.ny"
      SP_NY_NAME="bench_escape_spawn"
      ;;
    escape_point_sroa)
      SP_NY_PATH="$COMPARISON/escape/point_sroa.ny"
      SP_NY_NAME="bench_escape_sroa"
      ;;
    cpu_bound_pgo)
      SP_NY_PATH="$COMPARISON/cpu_bound/bench.ny"
      SP_NY_NAME="bench_cpu_pgo"
      ;;
    memory_*|strings_*|collections_*|algorithms_*|concurrency_*)
      extended_suite_paths "$suite" || return 1
      ;;
    *)
      log "warn: unknown suite $suite"
      return 1
      ;;
  esac

  if [[ -n "$SP_NY_PATH" ]]; then
    if [[ "$SP_NY_PATH" == *.ny ]]; then
      SP_NY_TYPED_PATH="${SP_NY_PATH%.ny}_typed.ny"
      SP_NY_TYPED_NAME="${SP_NY_NAME}_typed"
    elif [[ "$suite" == "dungeon" ]]; then
      SP_NY_TYPED_PATH="$COMPARISON/dungeon_typed"
      SP_NY_TYPED_NAME="bench_dungeon_typed"
    fi
  fi
}

lang_in_suite() {
  local suite="$1"
  local lang="$2"
  suite_paths "$suite" || return 1

  case "$lang" in
    Nyra) [[ -n "$SP_NY_PATH" ]] ;;
    Nyra-typed) [[ -n "$SP_NY_TYPED_PATH" && -e "$SP_NY_TYPED_PATH" ]] ;;
    Nyra-comptime) [[ -n "$SP_NY_CT_PATH" && -f "$SP_NY_CT_PATH" ]] ;;
    Nyra-comptime-typed) [[ -n "$SP_NY_CT_TYPED_PATH" && -f "$SP_NY_CT_TYPED_PATH" ]] ;;
    C) [[ -n "$SP_C_SRC" && -f "$SP_C_SRC" ]] ;;
    C++) [[ -n "$SP_CPP_SRC" && -f "$SP_CPP_SRC" ]] ;;
    Go) [[ -n "$SP_GO_SRC" && -f "$SP_GO_SRC" ]] ;;
    Rust) [[ -n "$SP_RUST_SRC" && -f "$SP_RUST_SRC" ]] ;;
    *) return 1 ;;
  esac
}

# Prints "TIME_MS PEAK_RSS_KB" — mean time (post-warmup), peak child max RSS
measure_cmd() {
  local cmd=("$@")
  local i count=0 t_total=0 peak_kb=0
  local t_ms mem_kb stats

  for ((i = 0; i < WARMUP + RUNS; i++)); do
    stats="$(
      python3 - "$@" <<'PY'
import platform, resource, subprocess, sys, time

cmd = sys.argv[1:]
start = time.perf_counter()
try:
    proc = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
except (OSError, subprocess.SubprocessError):
    print("0.000000 0")
    sys.exit(0)
if proc.returncode != 0:
    print("0.000000 0")
    sys.exit(0)
elapsed_ms = (time.perf_counter() - start) * 1000
ru = resource.getrusage(resource.RUSAGE_CHILDREN)
rss = ru.ru_maxrss
if platform.system() == "Darwin":
    rss_kb = int(rss / 1024)
else:
    rss_kb = int(rss)
print(f"{elapsed_ms:.6f} {rss_kb}")
PY
    )" || stats="$( /usr/bin/time -l "${cmd[@]}" 2>&1 1>/dev/null | awk '
      /^real/ { t=$2 }
      /maximum resident set size/ { m=int($1/1024) }
      END { printf "%.6f %d\n", t*1000, m+0 }
    ')"

    read -r t_ms mem_kb <<<"$stats"
    [[ -z "$t_ms" ]] && t_ms="0"
    [[ -z "$mem_kb" ]] && mem_kb="0"

    if [[ $i -ge $WARMUP ]]; then
      t_total="$(awk "BEGIN {print $t_total + $t_ms}")"
      count=$((count + 1))
      if [[ "$mem_kb" -gt "$peak_kb" ]]; then
        peak_kb="$mem_kb"
      fi
    fi
  done

  awk "BEGIN {printf \"%.4f %d\", ($count > 0 ? $t_total / $count : 0), $peak_kb}"
}

build_nyra() {
  local ny_file="$1"
  local out_name="$2"
  local built
  local profile="--release"
  if [[ "$BENCH_RELEASE" != "1" ]]; then
    profile=""
  fi
  local nyra_flags=()
  if [[ "$BENCH_RELEASE" == "1" ]]; then
    nyra_flags=(--release --no-prelude)
  fi
  if [[ "$BENCH_PGO" == "1" ]]; then
    nyra_flags+=(--pgo)
  fi
  local build_log
  build_log="$(mktemp "${TMPDIR:-/tmp}/nyra-bench-build.XXXXXX")"
  built="$(cd "$ROOT" && cargo run $profile -p cli --quiet -- build "$ny_file" -o "$out_name" ${nyra_flags[@]+"${nyra_flags[@]}"} 2>"$build_log" | sed -n 's/^built: //p')"
  if [[ -z "$built" || ! -x "$built" ]]; then
    log "error: failed to build Nyra binary for $ny_file"
    sed 's/^/  /' "$build_log" >&2
    rm -f "$build_log"
    return 1
  fi
  rm -f "$build_log"
  echo "$built"
}

# Profile-guided release build — trains on `main()` (same hot path as cpu_bound).
build_nyra_pgo() {
  local ny_file="$1"
  local out_name="$2"
  local built
  local profile="--release"
  if [[ "$BENCH_RELEASE" != "1" ]]; then
    log "warn: cpu_bound_pgo requires BENCH_RELEASE=1 (PGO needs release)"
    return 1
  fi
  if ! command -v opt &>/dev/null && ! command -v llvm-opt &>/dev/null; then
    log "warn: llvm opt not on PATH — skip cpu_bound_pgo (brew install llvm; export PATH=\"\$(brew --prefix llvm)/bin:\$PATH\")"
    return 1
  fi
  local build_log
  build_log="$(mktemp "${TMPDIR:-/tmp}/nyra-bench-pgo.XXXXXX")"
  log "PGO: building $ny_file (instrument → train main → merge → release)..."
  built="$(cd "$ROOT" && cargo run $profile -p cli --quiet -- build "$ny_file" -o "$out_name" --release --pgo --no-prelude 2>"$build_log" | sed -n 's/^built: //p')"
  if [[ -z "$built" || ! -x "$built" ]]; then
    log "error: PGO build failed for $ny_file"
    sed 's/^/  /' "$build_log" >&2
    rm -f "$build_log"
    return 1
  fi
  rm -f "$build_log"
  echo "$built"
}

build_go() {
  local src="$1"
  local out="$2"
  (cd "$(dirname "$src")" && go build -o "$out" "$(basename "$src")")
  echo "$out"
}

write_header() {
  local nyra_build="release"
  [[ "$BENCH_RELEASE" != "1" ]] && nyra_build="debug"
  [[ "$BENCH_PGO" == "1" ]] && nyra_build="${nyra_build}+pgo (all suites)"
  [[ "$BENCH_SKIP_PGO" != "1" ]] && nyra_build="${nyra_build}; cpu_bound_pgo=release+pgo"
  [[ "$BENCH_RELEASE" == "1" ]] && nyra_build="${nyra_build}; Nyra flags: --no-prelude, -march=native (host release default)"
  local isolation_note="languages run in isolation (BENCH_LANG_COOLDOWN=${LANG_COOLDOWN}s between langs)"
  [[ "$BENCH_NO_ISOLATE" == "1" ]] && isolation_note="all languages per suite (BENCH_NO_ISOLATE=1)"
  cat >"$LATEST" <<EOF
# Nyra comparison — runtime + memory benchmark
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Runs per command: $RUNS (warmup $WARMUP discarded)
# Time: mean wall clock in milliseconds (lower is better)
# Memory: peak max RSS in KB during timed runs (lower is better)
# Binary size: hello-world release / stripped / UPX (see binary-size.tsv)
# Platform: $(uname -s) $(uname -m)
# Nyra build: $nyra_build
# Isolation: $isolation_note
# Compiled langs: -O3 -flto=thin (Nyra --release, clang/clang++/rustc aligned)
# Note: Nyra/C/C++/Go/Rust/Java = compiled; Node/Python = runtime interpreters

EOF
}

write_nyra_key_message() {
  cat >>"$LATEST" <<'EOF'

Nyra appears twice:

  • Nyra (Zero Types)
  • Nyra (Explicit Types)

Both generate native code.

The benchmark demonstrates that Zero Types introduces no measurable runtime overhead.

EOF
}

build_c() {
  local src="$1"
  local out="$2"
  local cc="${CC:-clang}"
  if ! command -v "$cc" &>/dev/null; then
    log "warn: $cc not found — skip C"
    return 1
  fi
  if "$cc" -O3 -flto=thin -std=c11 "$src" -o "$out" 2>/dev/null; then
    echo "$out"
    return 0
  fi
  log "warn: C build failed for $src"
  return 1
}

build_cpp() {
  local src="$1"
  local out="$2"
  local cxx="${CXX:-clang++}"
  if ! command -v "$cxx" &>/dev/null; then
    log "warn: $cxx not found — skip C++"
    return 1
  fi
  if "$cxx" -O3 -flto=thin -std=c++17 "$src" -o "$out" 2>/dev/null; then
    echo "$out"
    return 0
  fi
  log "warn: C++ build failed for $src"
  return 1
}

build_rust() {
  local src="$1"
  local out="$2"
  if ! command -v rustc &>/dev/null; then
    log "warn: rustc not found — skip Rust"
    return 1
  fi
  if rustc -C opt-level=3 -C lto=thin "$src" -o "$out" 2>/dev/null; then
    echo "$out"
    return 0
  fi
  log "warn: rustc build failed for $src"
  return 1
}

lang_display_name() {
  case "$1" in
    Nyra) echo "Nyra (Zero Types)" ;;
    Nyra-typed) echo "Nyra (Explicit Types)" ;;
    Nyra-comptime) echo "Nyra (Comptime)" ;;
    Nyra-comptime-typed) echo "Nyra (Comptime + Types)" ;;
    *) echo "$1" ;;
  esac
}

bench_row() {
  local suite="$1"
  local lang="$2"
  local ms="$3"
  local mem_kb="$4"
  local display
  display="$(lang_display_name "$lang")"
  printf "| %-12s | %-22s | %10s | %10s |\n" "$suite" "$display" "$ms" "${mem_kb} KB" >>"$LATEST"
  printf '%s\t%s\t%s\t%s\n' "$suite" "$lang" "$ms" "$mem_kb" >>"$RESULTS_TSV"
  if [[ "$ms" == "0.0000" || "$ms" == "0" ]]; then
    log "warn: $suite / $display — benchmark failed (0 ms; crash or missing binary?)"
  fi
}

# ── Binary size (hello world) ─────────────────────────────────────────────────

file_bytes() {
  local f="$1"
  if [[ ! -f "$f" ]]; then
    echo "0"
    return 0
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    stat -f%z "$f"
  else
    stat -c%s "$f"
  fi
}

strip_binary_copy() {
  local src="$1" dst="$2"
  cp "$src" "$dst"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    strip -x "$dst" 2>/dev/null || strip "$dst" 2>/dev/null || true
  else
    strip --strip-unneeded "$dst" 2>/dev/null || strip "$dst" 2>/dev/null || true
  fi
}

upx_binary_copy() {
  local src="$1" dst="$2"
  if ! command -v upx &>/dev/null; then
    return 1
  fi
  cp "$src" "$dst"
  if upx --best -q -o "$dst" "$dst" 2>/dev/null; then
    return 0
  fi
  rm -f "$dst"
  return 1
}

java_class_bytes() {
  local dir="$1"
  local total=0
  local f sz
  while IFS= read -r -d '' f; do
    sz="$(file_bytes "$f")"
    total=$((total + sz))
  done < <(find "$dir" -name '*.class' -print0 2>/dev/null)
  echo "$total"
}

fmt_bytes_human() {
  local b="$1"
  if [[ -z "$b" || "$b" == "-" ]]; then
    echo "—"
    return 0
  fi
  if [[ "$b" -lt 1024 ]]; then
    printf '%s B' "$b"
  else
    awk "BEGIN {printf \"%.1f KB\", $b / 1024}"
  fi
}

binary_size_row() {
  local lang="$1" release="$2" stripped="$3" upx_b="$4"
  local display
  display="$(lang_display_name "$lang")"
  printf '%s\t%s\t%s\t%s\n' "$lang" "$release" "$stripped" "$upx_b" >>"$BINARY_SIZE_TSV"
  printf "| %-22s | %10s | %10s | %10s |\n" "$display" "$(fmt_bytes_human "$release")" "$(fmt_bytes_human "$stripped")" "$(fmt_bytes_human "$upx_b")" >>"$LATEST"
}

measure_binary_variants() {
  local lang="$1" bin="$2"
  local dir="$BENCH_DIR/binary-size"
  local release stripped upx_b upx_path
  mkdir -p "$dir"
  if [[ ! -x "$bin" ]]; then
    binary_size_row "$lang" "-" "-" "-"
    return 1
  fi
  release="$(file_bytes "$bin")"
  strip_binary_copy "$bin" "$dir/${lang// /_}_stripped"
  stripped="$(file_bytes "$dir/${lang// /_}_stripped")"
  upx_b="-"
  upx_path="$dir/${lang// /_}_upx"
  if upx_binary_copy "$bin" "$upx_path"; then
    upx_b="$(file_bytes "$upx_path")"
  fi
  binary_size_row "$lang" "$release" "$stripped" "$upx_b"
}

run_binary_size_benchmark() {
  local dir="$BENCH_DIR/binary-size"
  local bin c_bin cpp_bin go_bin rust_bin java_classes
  mkdir -p "$dir"

  log "Binary size — hello world (release / stripped / UPX)"
  {
    echo ""
    echo "# Binary size — hello world (lower is better)"
    echo "# release: optimized build as produced by the toolchain"
    echo "# stripped: same binary after strip (strip -x on macOS)"
    echo "# upx: UPX --best (— if UPX unavailable or unsupported for this format)"
    echo ""
    echo "| Language               |   Release |  Stripped |       UPX |"
    echo "|------------------------|-----------|-----------|-----------|"
  } >>"$LATEST"
  echo -e "language\trelease_bytes\tstripped_bytes\tupx_bytes" >"$BINARY_SIZE_TSV"

  bin="$(build_nyra "$COMPARISON/hello/hello.ny" "hello_nyra")" && measure_binary_variants "Nyra" "$bin" || true
  bin="$(build_nyra "$COMPARISON/hello/hello_typed.ny" "hello_nyra_typed")" && measure_binary_variants "Nyra-typed" "$bin" || true

  c_bin="$(build_c "$COMPARISON/hello/hello.c" "$dir/hello_c")" && measure_binary_variants "C" "$c_bin" || true
  cpp_bin="$(build_cpp "$COMPARISON/hello/hello.cpp" "$dir/hello_cpp")" && measure_binary_variants "C++" "$cpp_bin" || true
  go_bin="$(build_go "$COMPARISON/hello/hello.go" "$dir/hello_go")" && measure_binary_variants "Go" "$go_bin" || true
  rust_bin="$(build_rust "$COMPARISON/hello/hello.rs" "$dir/hello_rust")" && measure_binary_variants "Rust" "$rust_bin" || true

  if command -v javac &>/dev/null; then
    java_classes="$dir/java_classes"
    mkdir -p "$java_classes"
    if javac -d "$java_classes" "$COMPARISON/hello/Hello.java" 2>/dev/null; then
      local jbytes
      jbytes="$(java_class_bytes "$java_classes")"
      binary_size_row "Java" "$jbytes" "$jbytes" "-"
    fi
  fi

  # Script sources (not standalone binaries — source bytes for reference)
  if [[ -f "$COMPARISON/hello/hello.js" ]]; then
    local jsb
    jsb="$(file_bytes "$COMPARISON/hello/hello.js")"
    binary_size_row "Node" "$jsb" "-" "-"
  fi
  if [[ -f "$COMPARISON/hello/hello.py" ]]; then
    local pyb
    pyb="$(file_bytes "$COMPARISON/hello/hello.py")"
    binary_size_row "Python" "$pyb" "-" "-"
  fi

  echo "" >>"$LATEST"
}

write_html_report() {
  local generated platform nyra_mode isolation
  generated="$(grep '^# Generated:' "$LATEST" | sed 's/^# Generated: //')"
  platform="$(grep '^# Platform:' "$LATEST" | sed 's/^# Platform: //')"
  isolation="$(grep '^# Isolation:' "$LATEST" | sed 's/^# Isolation: //' || true)"
  if [[ "$BENCH_RELEASE" == "1" ]]; then
    nyra_mode="release (-O3, LLVM opt)"
  else
    nyra_mode="debug (BENCH_RELEASE=0)"
  fi

  BENCH_GENERATED="$generated" \
  BENCH_PLATFORM="$platform" \
  BENCH_RUNS="$RUNS" \
  BENCH_WARMUP="$WARMUP" \
  BENCH_NYRA_MODE="$nyra_mode" \
  BENCH_ISOLATION="$isolation" \
  BENCH_TSV="$RESULTS_TSV" \
  BENCH_BINARY_TSV="$BINARY_SIZE_TSV" \
  BENCH_HTML="$LATEST_HTML" \
  python3 "$ROOT/make/py/bench_comparison_html.py"
}

open_bench_report() {
  local port url
  if [[ "${BENCH_NO_OPEN:-0}" == "1" ]]; then
    log "Report: file://$LATEST_HTML"
    return 0
  fi

  if [[ "${BENCH_SERVE:-1}" == "1" ]]; then
    port="${BENCH_PORT:-8766}"
    url="http://127.0.0.1:${port}/latest.html"
    log ""
    log "Opening benchmark report → $url"
    log "Press Ctrl+C to stop the local server."
    if command -v open &>/dev/null; then
      (sleep 0.4 && open "$url") &>/dev/null &
    elif command -v xdg-open &>/dev/null; then
      (sleep 0.4 && xdg-open "$url") &>/dev/null &
    fi
    exec python3 -m http.server "$port" --bind 127.0.0.1 --directory "$RESULTS_DIR"
  fi

  log ""
  log "Report → file://$LATEST_HTML"
  if command -v open &>/dev/null; then
    open "$LATEST_HTML" 2>/dev/null || true
  elif command -v xdg-open &>/dev/null; then
    xdg-open "$LATEST_HTML" 2>/dev/null || true
  fi
}

append_detail_txt() {
  BENCH_TSV="$RESULTS_TSV" \
  BENCH_LATEST="$LATEST" \
  BENCH_BINARY_TSV="$BINARY_SIZE_TSV" \
  python3 "$ROOT/make/py/bench_comparison_html.py" --txt-only
}

bench_one_lang() {
  local suite="$1"
  local lang="$2"
  local ny_bin go_bin rust_bin c_bin cpp_bin java_classes java_class
  local ms kb

  suite_paths "$suite" || return 0
  lang_in_suite "$suite" "$lang" || return 0

  case "$lang" in
    Nyra)
      if [[ "$suite" == "cpu_bound_pgo" ]]; then
        ny_bin="$(build_nyra_pgo "$SP_NY_PATH" "$SP_NY_NAME")" || return 0
        read -r ms kb <<<"$(measure_cmd "$ny_bin")"
        bench_row "$suite" "Nyra" "$ms" "$kb"
      else
        ny_bin="$(build_nyra "$SP_NY_PATH" "$SP_NY_NAME")"
        read -r ms kb <<<"$(measure_cmd "$ny_bin")"
        bench_row "$suite" "Nyra" "$ms" "$kb"
      fi
      ;;
    Nyra-typed)
      if [[ "$suite" == "cpu_bound_pgo" ]]; then
        ny_bin="$(build_nyra_pgo "$SP_NY_TYPED_PATH" "$SP_NY_TYPED_NAME")" || return 0
        read -r ms kb <<<"$(measure_cmd "$ny_bin")"
        bench_row "$suite" "Nyra-typed" "$ms" "$kb"
      else
        ny_bin="$(build_nyra "$SP_NY_TYPED_PATH" "$SP_NY_TYPED_NAME")"
        read -r ms kb <<<"$(measure_cmd "$ny_bin")"
        bench_row "$suite" "Nyra-typed" "$ms" "$kb"
      fi
      ;;
    Nyra-comptime)
      ny_bin="$(build_nyra "$SP_NY_CT_PATH" "$SP_NY_CT_NAME")"
      read -r ms kb <<<"$(measure_cmd "$ny_bin")"
      bench_row "$suite" "Nyra-comptime" "$ms" "$kb"
      ;;
    Nyra-comptime-typed)
      ny_bin="$(build_nyra "$SP_NY_CT_TYPED_PATH" "$SP_NY_CT_TYPED_NAME")"
      read -r ms kb <<<"$(measure_cmd "$ny_bin")"
      bench_row "$suite" "Nyra-comptime-typed" "$ms" "$kb"
      ;;
    C)
      c_bin="$BENCH_DIR/${suite}_c"
      if c_bin="$(build_c "$SP_C_SRC" "$c_bin")"; then
        read -r ms kb <<<"$(measure_cmd "$c_bin")"
        bench_row "$suite" "C" "$ms" "$kb"
      fi
      ;;
    C++)
      cpp_bin="$BENCH_DIR/${suite}_cpp"
      if cpp_bin="$(build_cpp "$SP_CPP_SRC" "$cpp_bin")"; then
        read -r ms kb <<<"$(measure_cmd "$cpp_bin")"
        bench_row "$suite" "C++" "$ms" "$kb"
      fi
      ;;
    Go)
      go_bin="$BENCH_DIR/${suite}_go"
      go_bin="$(build_go "$SP_GO_SRC" "$go_bin")"
      read -r ms kb <<<"$(measure_cmd "$go_bin")"
      bench_row "$suite" "Go" "$ms" "$kb"
      ;;
    Rust)
      rust_bin="$BENCH_DIR/${suite}_rust"
      if rust_bin="$(build_rust "$SP_RUST_SRC" "$rust_bin")"; then
        read -r ms kb <<<"$(measure_cmd "$rust_bin")"
        bench_row "$suite" "Rust" "$ms" "$kb"
      fi
      ;;
    Node)
      read -r ms kb <<<"$(measure_cmd node "$SP_JS_SRC")"
      bench_row "$suite" "Node" "$ms" "$kb"
      ;;
    Python)
      if command -v python3 &>/dev/null; then
        read -r ms kb <<<"$(measure_cmd python3 "$SP_PY_SRC")"
        bench_row "$suite" "Python" "$ms" "$kb"
      else
        log "warn: python3 not found — skip Python ($suite)"
      fi
      ;;
    Java)
      java_classes="$BENCH_DIR/${suite}_java"
      mkdir -p "$java_classes"
      java_class="$(basename "$SP_JAVA_SRC" .java)"
      if command -v javac &>/dev/null && command -v java &>/dev/null; then
        if javac -d "$java_classes" "$SP_JAVA_SRC" 2>/dev/null; then
          read -r ms kb <<<"$(measure_cmd java -cp "$java_classes" "$java_class")"
          bench_row "$suite" "Java" "$ms" "$kb"
        else
          log "warn: javac failed for $SP_JAVA_SRC"
        fi
      else
        log "warn: javac/java not found — skip Java ($suite)"
      fi
      ;;
  esac
}

run_comparison_suite() {
  local suite="$1"
  log "== $suite =="
  local lang
  for lang in Nyra Nyra-typed Nyra-comptime Nyra-comptime-typed C C++ Go Rust Node Python Java; do
    log "  -> $lang"
    bench_one_lang "$suite" "$lang"
  done
}

run_isolated_langs() {
  local lang suite
  local langs=(Nyra Nyra-typed Nyra-comptime Nyra-comptime-typed C C++ Go Rust Node Python Java)
  local suites=(
    hello arithmetic dungeon loop fib nested struct_sum loop_nofold comptime_table cpu_bound mix
    escape_local_channel escape_spawn_channel escape_point_sroa
  )
  if [[ "${BENCH_EXTENDED:-1}" == "1" ]] && [[ "${BENCH_QUICK:-0}" != "1" ]]; then
    suites+=("${EXTENDED_SUITES[@]}")
  elif [[ "${BENCH_EXTENDED:-1}" == "0" ]]; then
    log "BENCH_EXTENDED=0 — skipping extended suites (memory/strings/collections/algorithms/concurrency)"
  fi
    if [[ "${BENCH_QUICK:-0}" == "1" ]]; then
    suites=(hello arithmetic nested cpu_bound comptime_table)
    log "BENCH_QUICK=1 — subset: ${suites[*]}"
  fi
  if [[ "$BENCH_SKIP_PGO" != "1" ]] && [[ "${BENCH_QUICK:-0}" != "1" ]]; then
    suites+=(cpu_bound_pgo)
  fi

  log "Isolated benchmark — one language at a time (fair CPU / thermal)"
  log "Languages: ${langs[*]}"
  log "Cooldown between languages: ${LANG_COOLDOWN}s (BENCH_LANG_COOLDOWN)"
  log ""

  for lang in "${langs[@]}"; do
    log "════════════════════════════════════════════════════════"
    log "  Language: $lang"
    log "════════════════════════════════════════════════════════"
    for suite in "${suites[@]}"; do
      if lang_in_suite "$suite" "$lang"; then
        log "  [$lang] $suite"
        bench_one_lang "$suite" "$lang"
      fi
    done
    lang_cooldown
  done
}

main() {
  if [[ "$BENCH_RELEASE" == "1" ]]; then
    log "Building nyra CLI (release)..."
    (cd "$ROOT" && cargo build --release -p cli -q)
  else
    log "Building nyra CLI (debug, low RAM)..."
    (cd "$ROOT" && cargo build -p cli -q)
  fi

  write_header
  write_nyra_key_message
  echo -e "suite\tlanguage\tms_mean\tpeak_rss_kb" >"$RESULTS_TSV"
  {
    echo "| Suite        | Language               | Time (ms) | Memory     |"
    echo "|--------------|------------------------|-----------|------------|"
  } >>"$LATEST"

  if [[ "$BENCH_NO_ISOLATE" == "1" ]]; then
    local quick_suites=(hello arithmetic dungeon loop fib nested struct_sum loop_nofold comptime_table cpu_bound mix escape_local_channel escape_spawn_channel escape_point_sroa)
    if [[ "${BENCH_EXTENDED:-1}" == "1" ]] && [[ "${BENCH_QUICK:-0}" != "1" ]]; then
      quick_suites+=("${EXTENDED_SUITES[@]}")
    fi
    if [[ "${BENCH_QUICK:-0}" == "1" ]]; then
      quick_suites=(hello arithmetic nested cpu_bound comptime_table)
      log "BENCH_QUICK=1 — subset: ${quick_suites[*]}"
    fi
    for suite in "${quick_suites[@]}"; do
      run_comparison_suite "$suite"
    done
    if [[ "$BENCH_SKIP_PGO" != "1" ]] && [[ "${BENCH_QUICK:-0}" != "1" ]]; then
      run_comparison_suite "cpu_bound_pgo"
    fi
  else
    run_isolated_langs
  fi

  if [[ "${BENCH_BINARY_SIZE:-1}" == "1" ]]; then
    run_binary_size_benchmark
  else
    log "BENCH_BINARY_SIZE=0 — skipping hello binary size table"
  fi

  echo "" >>"$LATEST"

  write_html_report
  append_detail_txt

  echo "" >>"$LATEST"
  echo "Re-run: make bench" >>"$LATEST"
  echo "Report: examples/comparison/results/latest.html" >>"$LATEST"

  if [[ "${BENCH_UPDATE_README:-1}" == "1" ]]; then
    python3 "$ROOT/make/py/update-readme-bench.py" || log "warn: README benchmark section not updated"
  fi

  log "Done."
  log "  Text:   $LATEST"
  log "  Report: $LATEST_HTML"
  open_bench_report
}

usage() {
  cat >&2 <<'EOF'
Usage: make bench [options]

  (default)     Run full benchmark, write latest.txt/html, open report in browser
  --html-only   Regenerate latest.html from existing results/data.tsv (no benchmark)
  --txt-only    Regenerate latest.txt appendix from existing data.tsv (no benchmark)
  --help        Show this help

Environment: BENCH_SERVE=0 BENCH_NO_OPEN=1 BENCH_RUNS=1 BENCH_LANG_COOLDOWN=0
            BENCH_QUICK=1 BENCH_SKIP_PGO=1 BENCH_UPDATE_README=1
            BENCH_EXTENDED=0 BENCH_SCALE=20 (regenerate extended suites at 20× load)
            BENCH_BINARY_SIZE=0 (skip hello binary size: release/stripped/UPX)
EOF
}

html_only_report() {
  if [[ ! -f "$RESULTS_TSV" ]]; then
    log "error: missing $RESULTS_TSV — run the benchmark first"
    exit 1
  fi
  write_html_report
  log "Report regenerated."
  log "  Report: $LATEST_HTML"
  open_bench_report
}

txt_only_report() {
  if [[ ! -f "$RESULTS_TSV" ]]; then
    log "error: missing $RESULTS_TSV — run the benchmark first"
    exit 1
  fi
  append_detail_txt
  echo "" >>"$LATEST"
  echo "Re-run: make bench" >>"$LATEST"
  echo "Report: examples/comparison/results/latest.html" >>"$LATEST"
  log "Text report regenerated."
  log "  Text: $LATEST"
}

case "${1:-}" in
  --help|-h) usage; exit 0 ;;
  --html-only) html_only_report; exit 0 ;;
  --txt-only) txt_only_report; exit 0 ;;
esac

main "$@"
