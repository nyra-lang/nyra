# Changelog — Nyra Documentation Site

Standalone changelog for the **Nyra docs site** ([`nyra-lang/docs`](https://github.com/nyra-lang/docs)).  
This repository is independent from the [Nyra compiler](https://github.com/nyra-lang/nyra). Compiler/stdlib release notes live in the compiler repo [`CHANGELOG.md`](https://github.com/nyra-lang/nyra/blob/main/CHANGELOG.md).

**Versioning:** docs releases use the same pill as `index.html` (e.g. `Documentation v0.1.0`), usually bumped when public messaging or pages change for a compiler release.

---

## Unreleased

**CLI reference expansion**

- [tooling.html](tooling.html) — full CLI TOC; quick-ref with examples; expanded `nyra test` (COMPILE/LINK/PASS walkthrough, `--filter`, `--list-json`); new `nyra toolchain`, `nyra cc`, `nyra bind` sections; help/version.
- [learn-get-started.html](learn-get-started.html) — essential CLI commands + first `test fn`.
- [stdlib.html](stdlib.html) — link from testing helpers to CLI guide.
- [nyra-skill.md](nyra-skill.md) — toolchain / cc / bind / watch / ide.

**Layout & reflect examples**

- [stdlib.html](stdlib.html) — `size_of` / `align_of`, reflect `type_name_*`, `FixedStep`, terminal raw mode examples under Reflect & memory utils.
- [learn-data-types.html](learn-data-types.html) — Type sizes (bytes & bits) with `size_of` example.
- [types.html](types.html) / [memory.html](memory.html) — `size_of` / `align_of` in systems table and Memory → Type layout.
- [methods.html](methods.html) — `size_of` / `align_of` row under Number & math.
- [nyra-skill.md](nyra-skill.md) — layout example for AI assistants.

## v0.1.0 (2026-07-09)

**Stdlib gap-fill documentation**

- [methods.html](methods.html) — batch3–6 string/math/vec/map/sync/FS APIs.
- AI skill (`nyra-skill.md`) and search index rebuilt.
- Examples: `examples/contrib/gap_fill_showcase.ny`, `batch6_showcase.ny`.

## v0.0.1 (2026-07-09)

**Initial documentation release**

- Documentation site for the Nyra programming language — install guide, learn track, stdlib reference, AI skill file, and tooling docs.
- Version baseline reset to **v0.0.1** across compiler and site.
