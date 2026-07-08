#!/usr/bin/env bash
# Verify comparison benchmarks: same checksum across languages (+ Nyra typed).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

NYRA="${NYRA_BIN:-$ROOT/target/debug/nyra}"
if [[ ! -x "$NYRA" ]]; then
  cargo build -p cli -q
  NYRA="$ROOT/target/debug/nyra"
fi

COMP="$ROOT/examples/comparison"

last_line() {
  "$@" 2>/dev/null | grep -vE '^[[:space:]]*(Compiling|Finished)[[:space:]]|^[[:space:]]*nyra[[:space:]]+|^incremental:' | tail -1 | tr -d '\r'
}

nyra_out() {
  "$@" 2>/dev/null | grep -vE '^[[:space:]]*(Compiling|Finished)[[:space:]]|^[[:space:]]*nyra[[:space:]]+|^incremental:'
}

check() {
  local label="$1"
  local want="$2"
  shift 2
  local got
  got="$(last_line "$@")"
  if [[ "$got" != "$want" ]]; then
    echo "check-comparison-parity: FAIL $label — want '$want', got '$got'" >&2
    exit 1
  fi
  echo "  ok $label"
}

echo "check-comparison-parity:"

check nested/nyra         3552224 "$NYRA" run "$COMP/nested/nested.ny"
check nested/nyra-typed   3552224 "$NYRA" run "$COMP/nested/nested_typed.ny"
if command -v rustc &>/dev/null; then
  rustc -O "$COMP/nested/nested.rs" -o /tmp/nyra_cmp_nested 2>/dev/null
  check nested/rust       3552224 /tmp/nyra_cmp_nested
fi
if command -v go &>/dev/null; then
  check nested/go          3552224 go run "$COMP/nested/nested.go"
fi

check fib/nyra            751659594 "$NYRA" run "$COMP/fib/fib.ny"
check fib/nyra-typed      751659594 "$NYRA" run "$COMP/fib/fib_typed.ny"
if command -v rustc &>/dev/null; then
  rustc -O "$COMP/fib/fib.rs" -o /tmp/nyra_cmp_fib 2>/dev/null
  check fib/rust          751659594 /tmp/nyra_cmp_fib
fi

check cpu_bound/nyra      415 "$NYRA" run "$COMP/cpu_bound/bench.ny"
check cpu_bound/nyra-typed 415 "$NYRA" run "$COMP/cpu_bound/bench_typed.ny"
if command -v rustc &>/dev/null; then
  rustc -O "$COMP/cpu_bound/bench.rs" -o /tmp/nyra_cmp_cpu 2>/dev/null
  check cpu_bound/rust    415 /tmp/nyra_cmp_cpu
fi

check struct_sum/nyra     240000000 "$NYRA" run "$COMP/struct_sum/struct_sum.ny"
check struct_sum/nyra-typed 240000000 "$NYRA" run "$COMP/struct_sum/struct_sum_typed.ny"

check hello/nyra          "Hello Nyra" "$NYRA" run "$COMP/hello/hello.ny"
check hello/nyra-typed    "Hello Nyra" "$NYRA" run "$COMP/hello/hello_typed.ny"

if nyra_out "$NYRA" run "$COMP/dungeon" >/tmp/dungeon_out.txt; then
  want="$(tr '\n' '|' </tmp/dungeon_out.txt)"
  got="$(tr '\n' '|' < <(nyra_out "$NYRA" run "$COMP/dungeon_typed"))"
  if [[ "$want" != "$got" ]]; then
    echo "check-comparison-parity: FAIL dungeon typed output mismatch" >&2
    exit 1
  fi
  echo "  ok dungeon/nyra + dungeon_typed"
else
  echo "check-comparison-parity: FAIL dungeon/nyra did not run" >&2
  cat /tmp/dungeon_out.txt >&2
  exit 1
fi

echo "check-comparison-parity: all quick suites match"
