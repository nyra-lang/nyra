# Runtime regression tests (`run/regression/`)

Curated **compile + link + run** guards for bugs that only show up in LLVM codegen,
linkage, or ownership drop — unit/IR-only tests are not enough.

## Rule

When a language/compiler/runtime defect is found (especially inference vs explicit
types, `Option`/`Result` payloads, or malloc/drop crashes):

1. Add a minimal `.ny` here with `// run-stdout: …`
2. Prefer a **matrix** when zero-types works but explicit types fail (or the reverse)
3. Do not land a fix without the regression test

These files are picked up by `compiletest` / `make test-compiletest` / CI `test-all`.

## Current guards

| File | Pattern |
|------|---------|
| `option_string_nullish_*.ny` | `Option<string>` + `??` (zero / explicit / Some / generic / comptime / matrix) |
