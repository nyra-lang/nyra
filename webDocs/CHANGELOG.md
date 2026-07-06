# Changelog — Nyra Documentation Site

Standalone changelog for the **Nyra docs site** ([`nyra-lang/docs`](https://github.com/nyra-lang/docs)).  
This repository is independent from the [Nyra compiler](https://github.com/nyra-lang/nyra). Compiler/stdlib release notes live in the compiler repo [`CHANGELOG.md`](https://github.com/nyra-lang/nyra/blob/main/CHANGELOG.md).

**Versioning:** docs releases use the same pill as `index.html` (e.g. `Documentation v1.36.x`), usually bumped when public messaging or pages change for a compiler release.

---

## v1.40.0 (2026-07-03)

**Official errors and async runtime**

- **Updated** — `stdlib.html`, `async.html`, `bindings.html`, and `nyra-skill.md` for the official `stdlib/error.ny` and `stdlib/async/mod.ny` paths.
- **Added** — `changelog.html` entry v1.40.0.
- **Regenerated** — `search-index.json`.

---

## v1.39.0 (2026-06-30)

**nyrapkg split — package manager documentation**

- **Updated** — `packages.html`: nyrapkg as standalone tool ([github.com/nyra-lang/pkg](https://github.com/nyra-lang/pkg)); split from `nyra pkg` (build/prune/c/bind only)
- **Updated** — `install.html`, `getting-started.html`, `examples.html`, `tooling.html`, `imports.html`, `language-vs-ecosystem.html`, `c-bindgen.html`, `backend.html`, `stdlib.html`, `bindings.html`, `modules.html`
- **Updated** — `locales/en.json`, `locales/ar.json`: nav + packages page strings (`nyrapkg`)
- **Updated** — `_includes/sidebar-nav.html` (synced to all pages)
- **Added** — `changelog.html` entry v1.39.0
- **Regenerated** — `search-index.json`

---

## v1.36.18 (2026-06-28)

**Production-ready status — remove MVP / pre-production banner**

- **Updated** — `index.html` hero banner and footer status: **Production-ready — Core + Stable Extended** (aligned with compiler [`docs/status.md`](https://github.com/nyra-lang/nyra/blob/main/docs/status.md))
- **Updated** — Result section on home page: `?` operator and Stable Extended error handling
- **Updated** — `roadmap.html` callout: production-ready tier; remaining gates (multi-trait `dyn`, exotic generic serde)
- **Updated** — `ai-skill.html`: status callout, Result section, system-prompt guardrails
- **Updated** — `learn-enums.html`, `async.html`, `reference.html`: Stable Extended wording (async, traits, `?`)
- **Updated** — `locales/en.json`, `locales/ar.json`: banner, status, Result, roadmap strings
- **Updated** — `nyra-skill.md`: v1.36 production-ready tier, traits section (Stable Extended)
- **Updated** — `css/style.css`: `hero-status-banner` styling
- **Regenerated** — `search-index.json`

---

## v1.41.0 (2026-07-06)

**Language-only docs — remove framework pages**

- **Removed** — App/framework guides: `dungeon-steps.html`, `backend.html`, `examples.html`, `enterprise.html`, `integration.html`, `language-vs-ecosystem.html`
- **Updated** — Sidebar nav: stdlib API links (`net-http.html`, `os-hardware.html`) under Advanced; no separate Guides section
- **Updated** — Learn track: runnable examples on intro, data types, borrowing, data structures; enums links to `methods.html`
- **Updated** — Home page, learning path, getting started, sitemap, `stdlib.html`, `net-http.html`, `nyra-skill.md`
- **Regenerated** — learn pages via `generate-learn-track.py`; `search-index.json`

---

## v1.36.12 (2026-06-27)

**Docs sync with compiler v1.36.12**

- **Updated** — `changelog.html` entry for stdlib HTTP and workspace patterns

---

## Earlier history

Combined language + docs release notes before this file existed are kept in [`changelog.html`](changelog.html) (HTML page, compiler-centric entries through v1.36.12).
