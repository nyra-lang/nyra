# Contributor automation conformance (CONF-CONTRIB-PY)

Python gate for `make contribute`, `builtin_dev`, `contrib_dev`, and batch generators.
**Not** Nyra source — run via `make test-contrib-conformance` (included in CI tier1 on Linux).

## What this contract guarantees

| Invariant | Why |
|-----------|-----|
| All automation `.py` files compile | Broken syntax cannot ship |
| Recipe JSON examples parse to valid specs | `make contribute ARGS='add --config …'` stays usable |
| Batch catalog JSON loads (batch3–6) | `make contribute → Batch` configs are valid |
| ABI manifest templates always emit `tier` | Prevents `runtime_map_matches_manifest` drift |
| `builtin_dev.add` patches manifest + runtime_map together | Same |
| `example_codegen` demos are check-safe shapes | optional-types CI on `examples/builtins/` |
| `docs/abi-manifest.toml` has no duplicate symbol names | `abi_manifest` Rust test |
| Pure Nyra fns (e.g. `pow_i32`) not in ABI map | No missing-C-symbol failures |
| `contribute` / `builtin-dev` CLIs respond to `--help` | Hub entry points import cleanly |
| `manifest_dedupe` is idempotent on a clean tree | Safe to run after batch adds |

## Run locally

```bash
make test-contrib-conformance
# alias:
make test-contrib-py
```

## CI

`.github/workflows/ci.yml` — `tier1 · Linux · contrib-automation` (`test-contrib-conformance`).

## When to extend

Add a check in `make/py/test_contrib_conformance.py` when you:

- Add a new `make/py/contrib_dev/*.py` or batch catalog
- Change ABI wiring in recipes or `builtin_dev/add.py`
- Add example codegen paths that must pass `nyra check`
