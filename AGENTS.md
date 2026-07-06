# Nyra — agent instructions

Cursor loads **`.cursor/rules/nyra-guidelines.mdc`** automatically (`alwaysApply: true`). Follow it on every compiler, stdlib, CLI, runtime, or docs task without being asked.

## Mandatory checklist

1. Tests (including `make test-all` when appropriate)
2. Example under `examples/`
3. No regressions
4. Update `webDocs/` (+ rebuild skill/search index if needed)
5. Wire new test gates into the root `Makefile`

## More context

| File | Purpose |
|------|---------|
| `docs/contributor-map.md` | What to change → where to go (flowchart + test placement) |
| `docs/make-and-generators.md` | Makefile targets + `make/py/` Python generators (incl. builtin-dev) |
| `.cursor/rules/nyra-guidelines.mdc` | Always-on agent rule |
| `Makefile` | Contributor test/build entry (`make help`) |
| `skills/guidelines.md` | Same checklist (human-readable) |
| `agents/skill.md` | Version bump, changelog, webDocs release workflow |
| `skills/skill.md` | Nyra language reference (`webDocs/nyra-skill.md`) |
