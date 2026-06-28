# Nyra change guidelines

**Canonical agent rule:** `.cursor/rules/nyra-guidelines.mdc` (`alwaysApply: true` — loaded automatically in Cursor).

**Not sure which folder to edit?** See [`docs/contributor-map.md`](../docs/contributor-map.md).

Any modification or addition to the language, compiler, stdlib, CLI, or runtime must follow:

1. **Tests** — Cover with all supported test types; run `make test-all` for full language verification when appropriate.
2. **Examples** — Add or update an example under `examples/` for new or changed features.
3. **No regressions** — Ensure the change does not break or negatively affect other features.
4. **webDocs** — Update documentation; rebuild skill + search index when needed (see `agents/skill.md`).
5. **Makefile** — New test gates must be wired into the root `Makefile` (`make test-all` dependencies).

For version bumps and detailed webDocs sync, see [`agents/skill.md`](../agents/skill.md).

## Standard library

Nyra targets a **strong, batteries-included** stdlib shipped with the compiler — crypto, databases, serialization, WebSocket, compression, and encoding belong **in-tree**, not behind NyraPkg-only redirects.

When adding or extending stdlib modules:

- **Performance & memory** — primary design goals; prefer native `stdlib/rt/` C with demand-driven linking
- **Types optional** — all six value kinds must work without required annotations
- **Modular files** — keep modules small and well-organized (`stdlib/README.md`)
- **NyraPkg** — community and optional extensions; proven packages may graduate into stdlib
