# Extended comparison benchmarks

Language coverage: **memory**, **strings**, **collections**, **algorithms**, **concurrency**.

Nyra runs twice per suite: zero-types (`bench.ny`) and typed (`bench_typed.ny`).

**Fair parity:** every language runs the same algorithm per suite (e.g. `map[int]int` /
`map_i32_i32`, same loop counts, same checksum). Regenerate with `make/py/gen-comparison-extended.py`.

Scale up with `BENCH_SCALE=10` when running `./scripts/bench.sh` (future).

| Suite | Description | Expected |
|-------|-------------|----------|
| `memory_alloc_struct` | malloc/free 500,000 nodes (8 B) | `496337424` |
| `memory_free_struct` | alloc+free 500,000 blocks (16 B) | `999749132` |
| `memory_arena` | bump arena simulation (500,000 allocs) | `3735125` |
| `memory_ownership` | struct pass-by-value (500,000) | `499500000` |
| `strings_concat` | strcat chain (100,000) | `149965` |
| `strings_substring` | substring (100,000) | `800000` |
| `strings_replace` | str_replace (100,000) | `1200000` |
| `strings_split` | split/search (100,000) | `1000000` |
| `strings_utf8` | UTF-8 byte iterate (100,000) | `193200000` |
| `collections_hashmap` | HashMap insert/get (200,000) | `999899867` |
| `collections_hashset` | HashSet insert/contains (200,000) | `200000` |
| `collections_vec_push` | Vec push (500,000) | `249125` |
| `collections_vec_pop` | Vec push+pop (500,000) | `999749132` |
| `collections_sort` | in-place shell sort (50,000) | `24836625` |
| `algorithms_qsort` | quicksort-style partition sum | `24836625` |
| `algorithms_mergesort` | merge sort simulation sum | `24836625` |
| `algorithms_binary_search` | binary search probes | `272238` |
| `algorithms_json_parse` | json_get_i32 (100,000) | `103900000` |
| `algorithms_regex` | regex_is_match (100,000) | `100000` |
| `concurrency_spawn_tasks` | spawn 5,000 tasks | `5000` |
| `concurrency_channel_pingpong` | spawn + channel (500,000) | `999749132` |
| `concurrency_worker_pool` | 4-worker pool (500,000 jobs) | `248996383` |
| `concurrency_parallel_map` | parallel for (200,000) | `83907879` |
