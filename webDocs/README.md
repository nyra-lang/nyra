# Nyra Documentation

Official static documentation site for **[Nyra](https://github.com/nyra-lang/nyra)** — a systems programming language that combines Go-like simplicity, Rust-like memory safety, and LLVM-native performance.

This folder is the **docs site source**. It is designed to live in its own repository ([`nyra-lang/docs`](https://github.com/nyra-lang/docs)) and deploy as a standalone GitHub Pages site, while staying in sync with the compiler repo when Nyra language behavior changes.

---

## What is Nyra?

**Nyra** (`.ny` source files) is a programming language in active development:

| Pillar | Target |
|--------|--------|
| **Ease of writing** | Go — small keyword set, flat learning curve, readable control flow |
| **Memory safety** | Rust — ownership, borrowing, compile-time checks (no GC) |
| **Speed** | C++ — zero-cost abstractions, LLVM `opt` + native codegen |
| **Tooling** | One `nyra` CLI — fmt, test, build, check, pkg, LSP |
| **Types** | **Optional by default** — write without annotations like Go/JS; inference fills types in; annotate only when the compiler cannot infer |

Nyra ships a **batteries-included stdlib** (collections, FS, HTTP/TCP, crypto, databases, serialization, and more) compiled in-tree with the language. **Core** and **Stable Extended** are production-ready; see [`roadmap.html`](roadmap.html) and [compiler status](https://github.com/nyra-lang/nyra/blob/main/docs/status.md).

**Compiler & toolchain:** [github.com/nyra-lang/nyra](https://github.com/nyra-lang/nyra)  
**Install guide:** [`install.html`](install.html) (mirrors [`install.md`](https://github.com/nyra-lang/nyra/blob/main/install.md) in the compiler repo)

---

## About this site

| Property | Detail |
|----------|--------|
| **Format** | Static HTML — no build step required to *view* pages |
| **Search** | Lunr full-text index (`search-index.json`) — **Ctrl+K** / **⌘K** |
| **Themes** | Dark / light toggle |
| **Locales** | English + Arabic UI strings (`locales/en.json`, `locales/ar.json`) |
| **AI reference** | [`nyra-skill.md`](nyra-skill.md) — canonical language summary for Cursor, Claude, ChatGPT |

Every code example that supports it shows **Without types** and **With types** tabs so readers can learn Nyra in either style.

---

## Site map

Navigation follows six top-level sections (see [`_includes/sidebar-nav.html`](_includes/sidebar-nav.html)). Use this table to find the right page quickly.

### Start

Onboarding, install, and meta docs.

| Page | File | Purpose |
|------|------|---------|
| Home | `index.html` | Overview, design pillars, quick links |
| Installation | `install.html` | Platform install, PATH, first `nyra run` |
| Learning path | `learning-path.html` | Recommended order through the learn track |
| AI skill file | `ai-skill.html` | Download / use `nyra-skill.md` with assistants |
| Getting started | `getting-started.html` | First steps after install |

### Learn Nyra

Beginner-friendly lessons — one concept per page, progressive difficulty.

| Topic | Pages |
|-------|-------|
| Basics | `learn-intro`, `learn-get-started`, `learn-syntax`, `learn-output`, `learn-comments` |
| Values & logic | `learn-variables`, `learn-data-types`, `learn-constants`, `learn-operators`, `learn-booleans` |
| Control flow | `learn-if-else`, `learn-match`, `learn-loops`, `learn-while`, `learn-for` |
| Functions & scope | `learn-functions`, `closures`, `learn-scope` |
| Strings & memory | `learn-strings`, `learn-ownership`, `learn-borrowing` |

**Beginner track** (alternate path): `beginner-track.html` + `beginner-01-first-program.html` … `beginner-08-mini-project.html`.

### Nyra Data Structures

Structured data after the core learn track.

| Page | Topic |
|------|-------|
| `learn-data-structures.html` | Overview |
| `learn-arrays.html` | Arrays |
| `learn-vectors.html` | Vectors |
| `learn-tuples.html` | Tuples |
| `learn-hashmap.html` | Hash maps |
| `learn-structs.html` | Structs |
| `learn-enums.html` | Enums |

### Advanced

Language reference depth — syntax, semantics, stdlib surface, concurrency.

| Page | Topic |
|------|-------|
| `language-basics.html` | Core concepts recap |
| `language.html` | Full syntax |
| `types.html` | Types & data model |
| `reference.html` | Quick language reference |
| `spec.html` | Normative language spec |
| `generics.html` | Generics |
| `match.html` | Pattern matching |
| `modules.html` / `imports.html` | Modules & imports |
| `memory.html` | Memory, ownership, `defer`, custom `Drop` |
| `async.html` | Async / await (Extended) |
| `traits-macros.html` | Traits & macros (Extended) |
| `stdlib.html` | Standard library inventory |
| `methods.html` | Built-in methods gallery |
| `concurrency.html` | Tasks, channels, `spawn` |

### Stdlib API reference

Language/stdlib modules with runnable examples in docs.

| Page | Topic |
|------|-------|
| `net-http.html` | `net/http` API reference |
| `os-hardware.html` | Files, process, hardware-facing APIs |

### Ecosystem

Toolchain, performance, FFI, packaging, project status.

| Page | Topic |
|------|-------|
| `tooling.html` | CLI, fmt, test, conformance (`CONF-*`) |
| `performance.html` | Release builds, LTO, optimization |
| `pgo.html` | Profile-guided optimization |
| `escape-analysis.html` | Escape analysis pass |
| `diagnostics.html` | Errors, warnings, LSP |
| `ffi-abi.html` | FFI & stable ABI |
| `c-bindgen.html` | C bindings & `nyra pkg c` |
| `bindings.html` | Runtime symbol map |
| `targets.html` | Cross-compilation targets |
| `editor-setup.html` | VS Code / editor integration |
| `packages.html` | NyraPkg |
| `roadmap.html` | Roadmap & maturity tiers |
| `changelog.html` | Documentation / language changelog |
| `sitemap.html` | Full page index |

**Full machine-readable sitemap:** [`sitemap.xml`](sitemap.xml)

---

## Repository layout

```
webDocs/
├── index.html              # Home
├── *.html                  # Documentation pages
├── nyra-skill.md           # Canonical AI language reference (edit this)
├── CHANGELOG.md            # Docs site release notes (this repo)
├── search-index.json       # Generated Lunr index
├── sitemap.xml
├── assets/                 # Logo, images
├── css/                    # style.css, search.css
├── js/                     # site.js, search.js, typed-transform.js
├── locales/                # en.json, ar.json (UI strings)
├── vendor/                 # lunr.min.js
├── _includes/              # Shared header + sidebar fragments
└── scripts/
    ├── build-search-index.mjs    # Regenerate search-index.json
    ├── build-nyra-skill.mjs      # Sync nyra-skill → compiler repo skills/
    ├── build-builtin-snippets.mjs
    ├── capture-builtin-outputs.mjs
    ├── builtin-outputs.json
    ├── embed-all-code-tabs.mjs
    ├── generate-pages.py         # Scaffold new HTML pages
    ├── generate-learn-track.py
    ├── sync-nav.py               # Propagate sidebar to all pages
    └── patch-html-search.py
```

---

## Local preview

From this directory (repo root when standalone):

```bash
# Optional: refresh search index after editing HTML or nyra-skill.md
node scripts/build-search-index.mjs

# Serve static files
python3 -m http.server 8080
# → http://localhost:8080
```

When nested inside the compiler monorepo, the full build (code tabs, stdlib snippets, skill sync) runs via:

```bash
# From Nyra repo root
make build-webdocs
```

---

## Build scripts

| Script | Output | Standalone docs repo | Needs compiler repo |
|--------|--------|----------------------|---------------------|
| `build-search-index.mjs` | `search-index.json` | ✅ | — |
| `build-nyra-skill.mjs` | `skills/skill.md` (parent) | ✅ (local file only) | For agent sync |
| `embed-all-code-tabs.mjs` | Patches HTML code blocks | — | ✅ (examples + `.typed.ny` siblings) |
| `capture-builtin-outputs.mjs` | `builtin-outputs.json` (stdout from `examples/builtins/`) | run before gallery rebuild when examples change | ✅ |
| `build-builtin-snippets.mjs` | `stdlib.html` + `methods.html` gallery | needs `builtin-outputs.json` for methods output | ✅ |
| `sync-nav.py` | Updates sidebar in all pages | ✅ | — |
| `generate-pages.py` | New page from template | ✅ | — |

After changing HTML content or `nyra-skill.md`, always run **`build-search-index.mjs`** before commit.

---

## Deployment (GitHub Pages)

This folder lives inside the **[Nyra compiler repo](https://github.com/nyra-lang/nyra)** at `webDocs/` (not repo root). GitHub Pages serves **`webDocs/` contents as the site root** — not `Cargo.toml`, `compiler/`, etc.

### Automatic (GitHub Actions)

Workflow: [`.github/workflows/pages.yml`](../.github/workflows/pages.yml)

| Trigger | Action |
|---------|--------|
| Push to `main` | Deploy when `webDocs/**` (or build scripts) change |
| Manual | **Actions → Deploy webDocs to GitHub Pages → Run workflow** |

**One-time repo setup:**

1. **Settings → Pages → Build and deployment → Source:** **GitHub Actions**
2. After first successful run, open the deployment URL from the workflow summary.

`webDocs/.nojekyll` disables Jekyll so `_includes/`, `vendor/`, and static assets are served as-is.

### Local preview

```bash
cd webDocs && python3 -m http.server 8080
# open http://localhost:8080/index.html
```

No server runtime in production — only static HTML, CSS, JS, and `search-index.json`.

---

## Contributing

### Docs-only changes

Edit the relevant `.html` page (or locale JSON for UI strings). Regenerate search:

```bash
node scripts/build-search-index.mjs
```

### Language / stdlib / CLI changes

These originate in the **[Nyra compiler repository](https://github.com/nyra-lang/nyra)**. When syntax, stdlib, or toolchain behavior changes:

1. Update the matching pages (see table in [Site map](#site-map)).
2. Update [`nyra-skill.md`](nyra-skill.md) — the single source of truth for AI assistants.
3. Run `node scripts/build-search-index.mjs`.
4. Bump the docs version pill on `index.html` / `changelog.html` to match the compiler release.
5. Add an entry to [`CHANGELOG.md`](CHANGELOG.md) for every docs-site release.

Compiler contributors: see [`agents/skill.md`](https://github.com/nyra-lang/nyra/blob/main/agents/skill.md) in the main repo for the full release checklist.

### Adding a new page

```bash
python3 scripts/generate-pages.py   # scaffold from template
python3 scripts/sync-nav.py         # add link in _includes/sidebar-nav.html first
node scripts/build-search-index.mjs
# Add entry to sitemap.html and sitemap.xml
```

---

## Related links

| Resource | URL |
|----------|-----|
| Nyra compiler & stdlib | [github.com/nyra-lang/nyra](https://github.com/nyra-lang/nyra) |
| Docs repository | [github.com/nyra-lang/docs](https://github.com/nyra-lang/docs) |
| Language status | [docs/status.md](https://github.com/nyra-lang/nyra/blob/main/docs/status.md) |
| AI language reference | [`nyra-skill.md`](nyra-skill.md) |

---

<p align="center"><em>Fast like a panther. Sharp by design.</em></p>
