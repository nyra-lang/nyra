# Nyra toolchain architecture

Guide for contributors: where code lives, how the pipeline fits together, and how we keep files small and readable.

## Design goals

1. **One concern per file** — prefer several 200–400 line modules over a single 2000+ line file.
2. **Pipeline order matches compile order** — lexer → parser → … → codegen mirrors `compiler/driver`.
3. **CLI commands are isolated** — each `nyra` subcommand lives in its own module under `cli/src/commands/`.
4. **Tests stay next to behavior** — unit tests in the same module (`#[cfg(test)]`); integration tests in `compiler/driver/tests/`.

## File size guideline

| Lines | Action |
|-------|--------|
| &lt; 400 | Ideal |
| 400–800 | OK for cohesive modules |
| 800–1200 | Consider splitting on next change |
| &gt; 1200 | Split before adding major features |

## Compiler pipeline

```
.ny sources
    → lexer/          tokens
    → parser/         AST (stmt/, expr/ planned)
    → expand/         desugaring (arrows, nullish, try, …)
    → resolve/        imports, project graph
    → monomorph/      generics → monomorphic AST
    → typecheck/      types (+ ffi.rs, *\_builtins.rs)
    → ownership/      drop plan, escape, NLL, lifetimes
    → borrowck/       use-after-move, borrows
    → const_eval/     const folding
    → codegen/        LLVM IR
    → driver/         orchestration, cache, CLI-facing API
```

**Public API:** `compiler` crate (`compiler/driver`) — `Compiler::compile_file`, `compile_project`, `CompileOptions`.

## Crate map (contributor quick reference)

| Crate | Path | You change this when… |
|-------|------|------------------------|
| `ast` | `compiler/ast/` | New syntax nodes, spans |
| `lexer` | `compiler/lexer/` | Tokens, string/char escapes |
| `parser` | `compiler/parser/` | Grammar, `parse_*` |
| `typecheck` | `compiler/typecheck/` | Types, builtins, diagnostics |
| `codegen` | `compiler/codegen/` | LLVM IR, runtime symbols |
| `cli` | `cli/` | `nyra` commands, flags, link driver |
| `pkg` | `pkg/` | NyraPkg, lockfiles |
| `stdlib/rt` | `stdlib/rt/*.c` | C runtime (`nyra_*` ABI) |

## `cli/` layout

```
cli/src/
  main.rs              # entry only
  app/
    mod.rs             # run() dispatch
    args.rs            # clap: OptFlags, Commands, …
    session.rs         # compile, link, build, run, PGO
  commands/
    mod.rs
    bind.rs            # nyra bind *
    pkg.rs             # nyra pkg *
    check.rs           # check, diag
    ide.rs             # ide goto-def / references
    fmt.rs             # fmt
    test.rs            # test
  link.rs, target.rs, …  # shared infrastructure
```

**Adding a command:** create `commands/foo.rs`, export `pub(crate) fn run(...)`, wire in `app/mod.rs` `match`.

## Native C toolchain (`nyra cc`)

Zig-style foundation for compiling and linking C alongside Nyra — see [`docs/native-cc.md`](native-cc.md).

```
nyra cc --print-toolchain
nyra cc -c vendor/shim.c -o vendor/shim.o
CC=nyra cc make
```

Discovery: `$NYRA_LLVM_BIN`, `$NYRA_HOME/lib/llvm/bin`, then Homebrew/xcrun/PATH.

**Phase 3:** `nyra toolchain install` → `$NYRA_HOME/lib/llvm/bin`.

**Phase 4:** `nyra bind c header.h` — libclang → `vendor/bindings/*.ny` ([c-bindgen.md](c-bindgen.md)).

## `codegen/llvm/` layout

```
compiler/codegen/src/
  lib.rs
  ansi_color.rs        # print color → ANSI (compile-time)
  runtime_map.rs       # nyra_* → rt/*.c
  llvm/
    mod.rs             # Codegen struct, compile pipeline, statements, exprs
    print.rs           # print / write / println / color
    util.rs            # escape_string, LLVM helpers, unit tests
```

**Adding a builtin:** typecheck (`typecheck/`) → codegen (`llvm/print.rs` or expr path) → runtime (`stdlib/rt/`) → `runtime_map.rs` + `docs/abi-manifest.toml`.

## `parser/` (planned split)

`parser/src/lib.rs` is large; target layout:

- `stmt.rs` — `parse_statement`, control flow
- `expr.rs` — `parse_expression`, operators
- `literal.rs` — strings, templates, chars
- `recovery.rs`, `diagnostics.rs` — already split

## Nyra project layout (user apps)

```
myapp/
  main.ny          # entry: fn main()
  nyra.mod         # manifest: module / require / link
  nyra.lock        # pinned versions
  nyra.sum         # checksums
  src/             # application modules
    *.ny
  .nyra/cache/     # fetched packages (after sync / run)
  target/debug/    # build output (gitignored)
```

`nyra pkg init` / `nyrapkg init` scaffolds `main.ny` + `nyra.mod` + empty lockfiles.
`nyra run` / `nyra build` auto-sync `require` packages (fetch missing, prune removed — like Cargo) when `nyrapkg` is available.

## Where to start contributing

| Task | Start here |
|------|------------|
| **Not sure where to go** | [`contributor-map.md`](contributor-map.md) — decision flowchart |
| New `print` / I/O feature | `codegen/llvm/print.rs`, `stdlib/rt/rt_io.c` |
| New syntax | `lexer/`, `parser/`, `ast/`, `compiler/driver/tests/` |
| New CLI flag | `cli/src/app/args.rs`, `cli/src/app/session.rs` |
| Docs | `webDocs/`, `docs/`, run `./scripts/build-webdocs.sh` |
| Package manager | `pkg/src/`, `cli/src/commands/pkg.rs` |

See also [CONTRIBUTING.md](../CONTRIBUTING.md) and [docs/contributor-map.md](contributor-map.md).
