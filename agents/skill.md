# Release workflow — version bump & webDocs sync

Use this when shipping a **user-visible** language, stdlib, CLI, or ABI change.

For the day-to-day contributor checklist see [`CONTRIBUTING.md`](../CONTRIBUTING.md). For mandatory agent rules see [`.cursor/rules/nyra-guidelines.mdc`](../.cursor/rules/nyra-guidelines.mdc).

---

## When to bump the version

Bump `[workspace.package] version` in [`Cargo.toml`](../Cargo.toml) and add a [`CHANGELOG.md`](../CHANGELOG.md) entry **only when at least one applies**:

| Bump | When |
|------|------|
| **Patch** (`1.x.Y`) | Bug fix — correctness, runtime, linker, typecheck |
| **Minor** (`1.Y.0`) | Notable user-facing feature, stdlib addition, ABI/toolchain change |

**Skip version bumps** for: refactors, internal cleanup, comment/docs-only edits (unless docs ship new public behavior), CI/Makefile tweaks, test-only changes, small incremental work.

---

## Release checklist

1. **Version** — bump in root [`Cargo.toml`](../Cargo.toml).
2. **Changelog** — concise entry in [`CHANGELOG.md`](../CHANGELOG.md).
3. **Tests** — `make test-all` (or minimum: `cargo test --workspace` + affected Nyra tests). Cover **zero-types and explicit types**.
4. **Examples** — `examples/` (`feature.ny` + `feature.typed.ny` when both styles apply).
5. **webDocs** — update HTML in [`webDocs/`](../webDocs/) when syntax, stdlib, CLI, or ABI changes:

```bash
node webDocs/scripts/build-nyra-skill.mjs    
node webDocs/scripts/build-search-index.mjs
```

6. **Status** — update [`docs/status.md`](../docs/status.md) when feature depth changes.
7. **Makefile** — wire new test gates into root [`Makefile`](../Makefile) if you add one.

Published docs also live in the [docs repo](https://github.com/nyra-lang/docs) → [nyra-lang.github.io/docs](https://nyra-lang.github.io/docs/).

---

## ABI changes

- **Adding** stable C symbols: [`docs/abi-manifest.toml`](../docs/abi-manifest.toml) → `make gen-abi-header` → update [`compiler/driver/tests/abi_manifest.rs`](../compiler/driver/tests/abi_manifest.rs).
- **Breaking** ABI changes require explicit review and a version bump.

See [`docs/bindings.md`](../docs/bindings.md) and [`docs/abi-policy.md`](../docs/abi-policy.md).

---

## After pulling runtime or compiler changes

```bash
./scripts/updateLang.sh   # or: make install-dev
nyra --version
```
