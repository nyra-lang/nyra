# Nyra builtin developer tools (`make/py/builtin_dev/`)

Automations to **add**, **remove**, or **patch** stdlib builtins.
Each run prints a **monitor report**: what the tool did, your tasks, usage examples, and next steps.

## Quick start

```bash
make add-builtin              # interactive wizard (default) — explains each step
make remove-builtin           # interactive remove
make patch-builtin              # update existing builtin wiring

make add-builtin ARGS='--config make/py/builtin_dev/examples/strip_suffix.json'
make patch-builtin ARGS='--method strip_suffix --config make/py/builtin_dev/examples/strip_suffix.json'
```

## File map (name → purpose)

| File | Purpose |
|------|---------|
| `../builtin-dev.py` | Main CLI (`add` / `remove` / `patch`) |
| `../add-builtin.py` | Makefile shortcut → add |
| `../remove-builtin.py` | Makefile shortcut → remove |
| `../patch-builtin.py` | Makefile shortcut → patch |
| `add.py` | Wire ADD across compiler + stdlib + tests + examples |
| `remove.py` | Wire REMOVE (incl. legacy hand-wired code) |
| `wire_patch.py` | Wire PATCH — re-wire + preserve C body when possible |
| `wizard_prompts.py` | Interactive wizard — explains what each answer controls |
| `method_catalog.py` | Known methods: behavior, examples, defaults |
| `monitor_report.py` | Monitor output + usage snippets after add/patch |
| `discover.py` | Scan repo for `[builtin-dev:…]` markers |
| `spec.py` | BuiltinSpec data model |
| `templates.py` | Code templates (C, Rust, Nyra examples/tests) |
| `patch.py` | Safe file patching utilities |
| `paths.py` | Repo path map |
| `examples/*.json` | Ready-made specs |

## Wizard behaviour

Every step explains **what your answer controls**:

1. **Receiver** → which files get wired (`string` accepts aliases: `strings`, `str`)
2. **Method name** → Nyra API `.method()`, C symbol `str_method`, behavior hint from catalog
3. **Arguments** → parameters of `.method(arg)` — NOT return type (typing `string` alone shows a hint)
4. **Return type** → Nyra result type of the method
5. **Preview + confirm** before any file is touched

## Monitor output legend

- **DONE** — files changed automatically
- **YOUR TASKS** — implement C logic, fix test values
- **USAGE** — copy-paste Nyra examples (also written to `examples/builtins/strings/`)
- **NEXT STEPS** — commands to run

## Patch workflow

Use when a method exists but wiring is wrong (args, return type, C symbol):

```bash
make patch-builtin ARGS='-i'
```

Patch removes old wiring, re-adds with new spec, and **preserves your C implementation** when the method name stays the same.

