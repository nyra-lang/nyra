#!/usr/bin/env python3
"""Generate new webDocs HTML pages with shared shell."""
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent

NAV = '''<nav>
  <section>
    <div class="nav-label" data-i18n="nav.start">Start</div>
    <ul>
      <li><a href="index.html" data-i18n="nav.overview">Nyra HOME</a></li>
      <li><a href="install.html" data-i18n="nav.install">Installation</a></li>
      <li><a href="learning-path.html" data-i18n="nav.learningPath">Learning path</a></li>
      <li><a href="ai-skill.html" data-i18n="nav.aiSkill">AI skill file</a></li>
      <li><a href="#" id="sidebar-search" data-i18n="nav.search">Search</a></li>
    </ul>
  </section>
  <section>
    <div class="nav-label" data-i18n="nav.learnSection">Learn Nyra</div>
    <ul>
      <li><a href="learn-intro.html" data-i18n="nav.learnIntro">Nyra Intro</a></li>
      <li><a href="learn-get-started.html" data-i18n="nav.learnGetStarted">Nyra Get Started</a></li>
      <li><a href="learn-syntax.html" data-i18n="nav.learnSyntax">Nyra Syntax</a></li>
      <li><a href="learn-output.html" data-i18n="nav.learnOutput">Nyra Output</a></li>
      <li><a href="learn-comments.html" data-i18n="nav.learnComments">Nyra Comments</a></li>
      <li><a href="learn-variables.html" data-i18n="nav.learnVariables">Nyra Variables</a></li>
      <li><a href="learn-data-types.html" data-i18n="nav.learnDataTypes">Nyra Data Types</a></li>
      <li><a href="learn-constants.html" data-i18n="nav.learnConstants">Nyra Constants</a></li>
      <li><a href="learn-operators.html" data-i18n="nav.learnOperators">Nyra Operators</a></li>
      <li><a href="learn-booleans.html" data-i18n="nav.learnBooleans">Nyra Booleans</a></li>
      <li><a href="learn-if-else.html" data-i18n="nav.learnIfElse">Nyra If..Else</a></li>
      <li><a href="learn-match.html" data-i18n="nav.learnMatch">Nyra Match</a></li>
      <li><a href="learn-loops.html" data-i18n="nav.learnLoops">Nyra Loops</a></li>
      <li><a href="learn-while.html" data-i18n="nav.learnWhile">Nyra While Loops</a></li>
      <li><a href="learn-for.html" data-i18n="nav.learnFor">Nyra For Loops</a></li>
      <li><a href="learn-functions.html" data-i18n="nav.learnFunctions">Nyra Functions</a></li>
      <li><a href="learn-scope.html" data-i18n="nav.learnScope">Nyra Scope</a></li>
      <li><a href="learn-strings.html" data-i18n="nav.learnStrings">Nyra Strings</a></li>
      <li><a href="learn-ownership.html" data-i18n="nav.learnOwnership">Nyra Ownership</a></li>
      <li><a href="learn-borrowing.html" data-i18n="nav.learnBorrowing">Nyra Borrowing</a></li>
    </ul>
  </section>
  <section>
    <div class="nav-label" data-i18n="nav.dataStructuresSection">Nyra Data Structures</div>
    <ul>
      <li><a href="learn-data-structures.html" data-i18n="nav.learnDataStructures">Nyra Data Structures</a></li>
      <li><a href="learn-arrays.html" data-i18n="nav.learnArrays">Nyra Arrays</a></li>
      <li><a href="learn-vectors.html" data-i18n="nav.learnVectors">Nyra Vectors</a></li>
      <li><a href="learn-tuples.html" data-i18n="nav.learnTuples">Nyra Tuples</a></li>
      <li><a href="learn-hashmap.html" data-i18n="nav.learnHashMap">Nyra HashMap</a></li>
      <li><a href="learn-structs.html" data-i18n="nav.learnStructs">Nyra Structs</a></li>
      <li><a href="learn-enums.html" data-i18n="nav.learnEnums">Nyra Enums</a></li>
    </ul>
  </section>
  <section>
    <div class="nav-label" data-i18n="nav.languageSection">Advanced</div>
    <ul>
      <li><a href="language-basics.html" data-i18n="nav.basics">Language basics</a></li>
      <li><a href="language.html" data-i18n="nav.syntax">Syntax</a></li>
      <li><a href="types.html" data-i18n="nav.types">Types &amp; data</a></li>
      <li><a href="reference.html" data-i18n="nav.reference">Language reference</a></li>
      <li><a href="keywords.html" data-i18n="nav.keywords">Nyra Keywords</a></li>
      <li><a href="spec.html" data-i18n="nav.spec">Language spec</a></li>
      <li><a href="generics.html" data-i18n="nav.generics">Generics</a></li>
      <li><a href="comptime.html" data-i18n="nav.comptime">Comptime</a></li>
      <li><a href="match.html" data-i18n="nav.match">Match</a></li>
      <li><a href="modules.html" data-i18n="nav.modules">Modules</a></li>
      <li><a href="imports.html" data-i18n="nav.imports">Imports guide</a></li>
      <li><a href="memory.html" data-i18n="nav.memory">Memory &amp; ownership</a></li>
      <li><a href="async.html" data-i18n="nav.async">Async</a></li>
      <li><a href="traits-macros.html" data-i18n="nav.traits">Traits &amp; macros</a></li>
      <li><a href="stdlib.html" data-i18n="nav.stdlib">Standard library</a></li>
      <li><a href="methods.html" data-i18n="nav.methods">Built-in methods</a></li>
      <li><a href="concurrency.html" data-i18n="nav.concurrency">Concurrency</a></li>
      <li><a href="net-http.html">net/http API</a></li>
      <li><a href="os-hardware.html" data-i18n="nav.osHardware">OS, files &amp; hardware</a></li>
    </ul>
  </section>
  <section>
    <div class="nav-label" data-i18n="nav.ecosystem">Ecosystem</div>
    <ul>
      <li><a href="tooling.html" data-i18n="nav.tooling">Toolchain</a></li>
      <li><a href="performance.html" data-i18n="nav.performance">Performance</a></li>
      <li><a href="pgo.html" data-i18n="nav.pgo">PGO</a></li>
      <li><a href="escape-analysis.html" data-i18n="nav.escapeAnalysis">Escape analysis</a></li>
      <li><a href="diagnostics.html" data-i18n="nav.diagnostics">Diagnostics</a></li>
      <li><a href="ffi-abi.html" data-i18n="nav.ffi">FFI &amp; ABI</a></li>
      <li><a href="bindings.html" data-i18n="nav.bindings">Runtime bindings</a></li>
      <li><a href="targets.html" data-i18n="nav.targets">Targets</a></li>
      <li><a href="editor-setup.html" data-i18n="nav.editor">Editor setup</a></li>
      <li><a href="packages.html" data-i18n="nav.packages">NyraPkg</a></li>
      <li><a href="roadmap.html" data-i18n="nav.roadmap">Roadmap</a></li>
      <li><a href="changelog.html" data-i18n="nav.changelog">Changelog</a></li>
      <li><a href="sitemap.html" data-i18n="nav.sitemap">Sitemap</a></li>
    </ul>
  </section>
</nav>'''


def shell(page: str, title: str, h1: str, lead: str, body: str, footer: str, active: str) -> str:
    import re
    nav = NAV
    nav = re.sub(
        rf'<a href="{re.escape(active)}"',
        f'<a class="active" href="{active}"',
        nav,
        count=1,
    )

    return f'''<!DOCTYPE html>
<html lang="en" dir="ltr" data-theme="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="theme-color" content="#06090d">
  <meta name="color-scheme" content="dark light">
  <title>{title}</title>
  <link rel="stylesheet" href="css/style.css">
  <link rel="stylesheet" href="css/search.css">
</head>
<body data-page="{page}">
  <input type="checkbox" id="nav-check" class="nav-check" hidden aria-hidden="true">
  <header class="site-header">
    <a class="logo" href="index.html">
      <img src="../assets/Nyrabgremoved.png" alt="Nyra">
    </a>
    <div class="site-toolbar">
      <div class="toolbar-group" role="group" aria-label="Search">
        <button type="button" class="toolbar-btn search-btn" id="search-open" data-i18n-title="ui.searchOpen" title="Search (Ctrl+K)">
          <span aria-hidden="true">⌕</span>
          <kbd>Ctrl+K</kbd>
        </button>
      </div>
      <div class="toolbar-group" role="group" aria-label="Theme">
        <button type="button" class="toolbar-btn" id="theme-toggle" data-i18n-title="ui.themeToggle" title="Toggle theme">
          <span class="theme-icon theme-icon-sun" aria-hidden="true">☀</span>
          <span class="theme-icon theme-icon-moon" aria-hidden="true">☽</span>
        </button>
      </div>
      <div class="toolbar-group lang-switch" role="group" aria-label="Language">
        <button type="button" class="toolbar-btn lang-btn active" data-lang="en" id="lang-en">EN</button>
        <button type="button" class="toolbar-btn lang-btn" data-lang="ar" id="lang-ar">عربي</button>
      </div>
    </div>
    <label for="nav-check" class="nav-toggle" data-i18n-aria-label="ui.menuOpen" aria-label="Open navigation menu">
      <span></span><span></span><span></span>
    </label>
    <span class="tagline" data-i18n="common.tagline">Fast · Safe · Minimal</span>
  </header>
  <label for="nav-check" class="sidebar-backdrop" aria-hidden="true"></label>
  <div class="layout">
    <aside class="sidebar">
{nav}
    </aside>
    <main class="content">
      <h1 data-i18n="pages.{page}.h1">{h1}</h1>
      <p class="lead" data-i18n-html="pages.{page}.lead">{lead}</p>
{body}
      <footer class="site-footer">{footer}</footer>
    </main>
  </div>
    <script src="vendor/lunr.min.js"></script>
    <script src="js/search.js"></script>
    <script src="js/site.js"></script>
</body>
</html>
'''


PAGES = {
    "spec.html": (
        "spec", "Language specification — Nyra Docs", "Language specification",
        "Normative Spec 1.0 freeze (v0.2.0) — what the compiler implements today.",
        '''
      <h2>Spec 1.0 freeze</h2>
      <p><strong>Frozen:</strong> 2026-06-04 with release <strong>v0.2.0</strong>. Parser or ABI breaking changes require an RFC and minor version bump.</p>
      <table>
        <thead><tr><th>Decision</th><th>Resolution</th></tr></thead>
        <tbody>
          <tr><td>Modules</td><td><code>import "path.ny"</code> + <code>module name</code> + <code>nyra.mod</code></td></tr>
          <tr><td>Generics</td><td>Syntax + monomorphization at compile time</td></tr>
          <tr><td>Concurrency</td><td><code>spawn { }</code>, channels, <code>async</code>/<code>await</code></td></tr>
          <tr><td>Memory</td><td>Copy/Move, borrow checker, NLL basics, auto-drop heap strings</td></tr>
          <tr><td>Enums</td><td>Tag-only variants (<code>i32</code>); no ADT payloads</td></tr>
        </tbody>
      </table>
      <h2>Out of scope v0.2</h2>
      <ul>
        <li>Enum variant payloads <code>Color.Red(x)</code></li>
        <li>Full <code>Option&lt;T&gt;</code> / <code>Result&lt;T,E&gt;</code> with stored payloads</li>
        <li><code>?</code> operator</li>
      </ul>
      <h2>Core surface</h2>
      <ul>
        <li><strong>Types:</strong> <code>i32</code>, <code>bool</code>, <code>string</code>, <code>void</code>, <code>struct</code>, <code>enum</code>, <code>[T; N]</code>, <code>[T]</code>, <code>&amp;T</code> / <code>&amp;mut T</code></li>
        <li><strong>Statements:</strong> <code>let</code>, <code>const</code>, <code>let mut</code>, assign, <code>if</code>, <code>while</code>, <code>for x in a..b</code>, <code>return</code>, <code>print</code>, <code>spawn</code>, <code>defer</code>, <code>async fn</code></li>
        <li><strong>Expressions:</strong> literals, calls, methods, field/index, struct/array literals, <code>match</code> (with guards), if-expressions, <code>await</code></li>
        <li><strong>Modules:</strong> <code>import</code>, multi-file <code>main.ny</code></li>
        <li><strong>OO:</strong> <code>impl Type { }</code>, <code>impl Type for Trait { }</code></li>
        <li><strong>FFI:</strong> <code>extern fn</code>, <code>export fn</code></li>
        <li><strong>Toolchain:</strong> <code>nyra run|build|check|test|fmt|diag|lsp|pkg</code></li>
      </ul>
      <h2>Grammar (highlighting only)</h2>
      <p>TextMate grammar: <a href="editor-setup.html">Editor setup</a> · <code>grammar/nyra.tmLanguage.json</code></p>
      <p>See also: <a href="memory.html">Memory</a> · <a href="ffi-abi.html">FFI &amp; ABI</a> · <a href="changelog.html">Changelog</a></p>
''',
        '<a href="reference.html">Language reference →</a>', "spec.html"
    ),
    # ... more pages defined below in extended dict
}

# Extend PAGES with remaining content (abbreviated for script size - full content inline)
PAGES.update({
    "ffi-abi.html": ("ffi-abi", "FFI &amp; ABI — Nyra Docs", "FFI &amp; ABI",
        "Stable C ABI since v0.4.0 — manifest, header, and heap string ownership at the boundary.",
        '''<h2>Stable since v0.4.0</h2><ul><li><code>export fn</code> — unmangled C names; boundary typecheck</li><li><code>repr(C)</code> on structs at FFI</li><li>Boundary types: <code>i32</code>, <code>i64</code>, <code>u32</code>, <code>bool</code>, <code>string</code>, <code>ptr</code>, <code>void</code>, fn callbacks</li><li><code>stdlib/nyra_rt.h</code> generated from <code>docs/abi-manifest.toml</code></li></ul>
<h2>Five-step cookbook (inbound FFI)</h2><ol><li>Declare C API: <code>extern fn foo(x: ptr) -&gt; i32</code></li><li>Link native libs: <code>nyra build . --link-lib sqlite3 --link-search-path /opt/homebrew/lib</code></li><li>Or in <code>nyra.mod</code>: <code>link sqlite3</code> and <code>link -L /path</code></li><li>Build: <code>nyra build .</code></li><li>See <code>examples/ffi/call_libc/</code></li></ol>
<h2>Outbound (Nyra cdylib)</h2><p><code>nyra build lib.ny -o mylib --cdylib</code> — hosts call <code>export fn</code>; free returned <code>string</code> with <code>free</code>. CI: <code>scripts/abi-roundtrip.sh</code>. Examples: <code>examples/ffi/export_greet/</code>, <code>examples/ffi/hello_from_rust/</code>.</p>
<h2>Allocator</h2><p>Strings from <code>export fn</code>, <code>read_file</code>, or <code>strcat</code> are heap-owned. Nyra code: <strong>auto-drop</strong> at scope end. FFI callers outside Nyra must call <code>free</code>.</p>
<h2>SemVer</h2><p>0.4.x may add symbols; 1.x may break signatures after RFC. See <code>docs/abi-policy.md</code>.</p>''',
        '<a href="spec.html">Language spec →</a>', "ffi-abi.html"),
    "performance.html": ("performance", "Performance — Nyra Docs", "Performance toolchain",
        "LLVM opt, release flags, LTO, PGO, and benchmarks.",
        '''<h2>Pipeline</h2><pre class="pipeline">.ny → LLVM IR → opt -O3 → clang [-flto] + nyra_rt.c</pre>
<h2>CLI flags</h2><table><thead><tr><th>Flag</th><th>Effect</th></tr></thead><tbody>
<tr><td><code>--release</code></td><td>-O3, LLVM opt, thin LTO</td></tr>
<tr><td><code>--opt LEVEL</code></td><td>Clang 0–3</td></tr>
<tr><td><code>--lto</code> / <code>--lto-full</code></td><td>Thin / full LTO</td></tr>
<tr><td><code>--no-llvm-opt</code></td><td>Skip opt pass</td></tr>
<tr><td><code>--pgo</code></td><td>Automated profile-guided optimization — see <a href="pgo.html">PGO guide</a></td></tr>
<tr><td><code>--pgo-generate</code> / <code>--pgo-use</code></td><td>Manual Clang PGO (low-level)</td></tr>
<tr><td><code>--native-cpu</code></td><td>-march=native</td></tr>
<tr><td><code>--verbose</code> / <code>-v</code></td><td>Print escape-analysis report — see <a href="escape-analysis.html">Escape analysis</a></td></tr></tbody></table>
<pre><code>nyra build --release examples/syntax/math.ny -o bench
nyra build --pgo examples/comparison/cpu_bound/bench.ny
nyra build --verbose .   # stack promotion, LocalChannel, no_escape
./scripts/bench.sh</code></pre>
<p>Full PGO walkthrough: <a href="pgo.html">Profile-Guided Optimization (PGO)</a>. Stack/lock optimizations: <a href="escape-analysis.html">Escape analysis</a>.</p>''',
        '<a href="tooling.html">Toolchain →</a>', "performance.html"),
    "diagnostics.html": ("diagnostics", "Diagnostics — Nyra Docs", "Diagnostics",
        "Structured errors, borrow messages, and <code>nyra diag --json</code>.",
        '''<h2>nyra diag</h2><pre><code>nyra diag . --json</code></pre>
<h2>Common borrow errors</h2><table><thead><tr><th>Message</th><th>Cause</th></tr></thead><tbody>
<tr><td>Use of moved value</td><td>Move-type (string) used after move</td></tr>
<tr><td>Cannot borrow as mutable</td><td><code>&amp;mut</code> aliasing conflict</td></tr>
<tr><td>cannot return reference to local</td><td>Dangling <code>&amp;</code> return</td></tr>
<tr><td>manual free warning</td><td>Double-free risk with auto-drop</td></tr></tbody></table>
<pre><code>error: Use of moved value 'a'
  --> main.ny:4:11
   |
 4 |     print(a)
   |           ^</code></pre>''',
        '<a href="memory.html">Memory →</a>', "diagnostics.html"),
    "changelog.html": ("changelog", "Changelog — Nyra Docs", "Changelog",
        "Spec freeze history and release notes.",
        '''<h2>v0.5.0 (2026-06-05)</h2><ul>
<li><strong>Unsafe memory</strong> — <code>unsafe { }</code>, typed <code>*T</code>, casts, pointer arithmetic</li>
<li><strong><code>no_std</code></strong> — directive or <code>--no-std</code>; <code>--freestanding</code> builds</li>
<li><strong>Stdlib</strong> — <code>stdlib/core/mem.ny</code>, <code>stdlib/os.ny</code></li>
<li><strong>Inline asm</strong> — <code>asm "..."</code> in <code>unsafe</code></li></ul>
<h2>v0.4.0 (2026-06-05)</h2><ul>
<li>Stable C ABI; expanded FFI boundary types</li></ul>
<h2>Spec 1.0 frozen (2026-06-04, v0.2.0)</h2><ul>
<li>Spec header frozen; RFC for parser breaks</li>
<li>Enum tag-only documented</li>
<li>async, traits, macros, defer, lifetimes, wasm, NyraPkg registry</li>
<li>Copy/Move ownership + auto-drop heap strings</li>
<li><code>time_start</code> / <code>mem_start</code> builtins</li></ul>
<p>See <a href="roadmap.html">Roadmap</a> for future phases.</p>''',
        '<a href="spec.html">Language spec →</a>', "changelog.html"),
    "generics.html": ("generics", "Generics — Nyra Docs", "Generics",
        "Type parameters and monomorphization in v0.2.",
        '''<pre><code>fn id&lt;T&gt;(x: T) -> T {
    return x
}

fn main() {
    print(id&lt;i32&gt;(42))
}</code></pre>
<p>Monomorphization generates specialized functions at compile time. Generic calls use <code>ident &lt; Type &gt; (</code> syntax.</p>''',
        '<a href="spec.html">Language spec →</a>', "generics.html"),
    "match.html": ("match", "Match — Nyra Docs", "Match expressions",
        "Exhaustiveness, guards, and tag-only enums.",
        '''<pre><code>let n = match color {
    Color.Red => 1
    Color.Green => 2
    Color.Blue => 3
}</code></pre>
<h2>Guards</h2><pre><code>Color.Red if x > 0 => 1</code></pre>
<h2>Option / Result</h2><p>Built-in enum <strong>tags</strong> only — no stored payloads in v0.2:</p>
<pre><code>let s = Option.Some
let r = Result.Ok</code></pre>''',
        '<a href="language.html">Syntax →</a>', "match.html"),
    "async.html": ("async", "Async — Nyra Docs", "Async &amp; await",
        "Async functions and await expressions (MVP).",
        '''<pre><code>async fn fetch() -> i32 {
    return await handle
}</code></pre>
<p>Runtime stubs: <code>async_run</code>, <code>await</code>. Full scheduler evolving. Wasm target has limited async support.</p>''',
        '<a href="concurrency.html">Concurrency →</a>', "async.html"),
    "traits-macros.html": ("traits-macros", "Traits &amp; macros — Nyra Docs", "Traits &amp; macros",
        "Trait definitions, impl for, and macro expansion.",
        '''<pre><code>trait Show {
    fn show(self) -> void
}

impl Calculator for Show {
    fn show(self) -> void { print(self.value) }
}</code></pre>
<p>Macros are parsed and expanded in a dedicated compiler pass before typecheck.</p>''',
        '<a href="spec.html">Language spec →</a>', "traits-macros.html"),
    "targets.html": ("targets", "Compilation targets — Nyra Docs", "Compilation targets",
        "Native binaries, cross-compilation, and WebAssembly.",
        '''<h2>Native (default)</h2><p>clang + <code>nyra_rt.c</code> — full stdlib, timing, memory, channels.</p>
<h2>Cross-compilation</h2>
<p>Build for another OS: <code>nyra build . --release --for windows|linux|macos</code>. Artifacts under <code>target/&lt;triple&gt;/release/</code>. See full table in repo <code>webDocs/targets.html</code>.</p>
<h2>wasm32-wasi</h2><pre><code>nyra build --for wasm app.ny -o app.wasm</code></pre>
<p>Links <code>nyra_rt_wasi.c</code>. Timing/memory no-ops; spawn limited.</p>''',
        '<a href="performance.html">Performance →</a>', "targets.html"),
    "editor-setup.html": ("editor-setup", "Editor setup — Nyra Docs", "Editor setup",
        "VS Code / Cursor syntax highlighting for <code>.ny</code> files.",
        '''<p>Grammar file: <code>grammar/nyra.tmLanguage.json</code></p>
<pre><code>nyra-syntax/
  package.json
  syntaxes/nyra.tmLanguage.json</code></pre>
<p>Copy from repo or use raw URL from grammar README. This is highlighting only — see <a href="spec.html">Language spec</a> for semantics.</p>''',
        '<a href="tooling.html">Toolchain →</a>', "editor-setup.html"),
    "integration.html": ("integration", "Integration guides — Nyra Docs", "Integration guides",
        "Embedding Nyra in other stacks (non-normative).",
        '''<ul>
<li><strong>Tauri sidecar</strong> — ship <code>nyra build</code> binary as sidecar</li>
<li><strong>mini-http</strong> — experimental HTTP hello patterns</li>
<li><strong>Wasm</strong> — <a href="targets.html">wasm32-wasi</a> target</li>
<li><strong>FFI</strong> — <a href="ffi-abi.html">export fn</a> from Nyra, <code>extern</code> from C</li></ul>''',
        '<a href="ffi-abi.html">FFI &amp; ABI →</a>', "integration.html"),
    "learning-path.html": ("learning-path", "Learning path — Nyra Docs", "Learning path",
        "Structured path from zero to a full multi-file app.",
        '''        <ol>
<li><a href="install.html">Install</a> → <a href="learn-get-started.html">Get Started</a></li>
<li><a href="learn-intro.html">Learn Nyra</a> — W3Schools-style tutorial (27 topics)</li>
<li><a href="language-basics.html">Language basics</a> → <a href="language.html">Syntax</a> → <a href="types.html">Types</a> → <a href="reference.html">Reference</a></li>
<li><a href="memory.html">Memory &amp; ownership</a> → <a href="stdlib.html">Standard library</a> → <a href="os-hardware.html">OS, files &amp; hardware</a></li>
<li><a href="examples.html">Examples</a> → <a href="spec.html">Language spec</a></li>
<li><a href="generics.html">Generics</a> → <a href="async.html">Async</a> → <a href="ffi-abi.html">FFI</a></li>
<li><a href="dungeon-steps.html">Dungeon Steps</a> — capstone project</li></ol>
<p>AI assistants: download <a href="nyra-skill.md">nyra-skill.md</a> — full syntax, types, loops, imports, and anti-hallucination rules.</p>''',
        '<a href="getting-started.html">Getting started →</a>', "learning-path.html"),
    "ai-skill.html": ("ai-skill", "AI skill file — Nyra Docs", "AI skill file",
        "Download <code>nyra-skill.md</code> so AI models know Nyra v0.2 semantics.",
        '''<div class="callout">
<a class="btn btn-primary" href="nyra-skill.md" download>Download nyra-skill.md</a>
<button type="button" class="btn btn-ghost" id="copy-skill">Copy to clipboard</button>
</div>
<h2>Cursor</h2><p>Add to Project Rules or attach <code>@nyra-skill.md</code> in chat.</p>
<h2>ChatGPT / Claude</h2><p>Upload to Project Knowledge or paste into Custom Instructions.</p>
<h2>System prompt</h2><pre><code>Follow nyra-skill.md as the sole Nyra reference. Do not invent syntax.</code></pre>
<script>
document.getElementById('copy-skill')?.addEventListener('click', function() {
  fetch('nyra-skill.md').then(r=>r.text()).then(t=>navigator.clipboard.writeText(t));
});
</script>''',
        '<a href="index.html">Overview →</a>', "ai-skill.html"),
    "sitemap.html": ("sitemap", "Sitemap — Nyra Docs", "Sitemap",
        "All documentation pages.",
        '''<table><thead><tr><th>Page</th><th>Topic</th></tr></thead><tbody>
<tr><td><a href="index.html">Overview</a></td><td>Start here</td></tr>
<tr><td><a href="getting-started.html">Getting started</a></td><td>First project</td></tr>
<tr><td><a href="install.html">Installation</a></td><td>Install guide</td></tr>
<tr><td><a href="learning-path.html">Learning path</a></td><td>Curriculum</td></tr>
<tr><td><a href="ai-skill.html">AI skill</a></td><td>nyra-skill.md for LLMs</td></tr>
<tr><td><a href="language-basics.html">Language basics</a></td><td>Variables, loops, structs, imports</td></tr>
<tr><td><a href="language.html">Syntax</a></td><td>Core syntax</td></tr>
<tr><td><a href="types.html">Types</a></td><td>Types &amp; data</td></tr>
<tr><td><a href="reference.html">Reference</a></td><td>Operators, literals, quick lookup</td></tr>
<tr><td><a href="keywords.html">Keywords</a></td><td>Reserved words, <code>@</code> raw identifiers, <code>#[…]</code> attributes</td></tr>
<tr><td><a href="spec.html">Spec</a></td><td>Normative spec</td></tr>
<tr><td><a href="generics.html">Generics</a></td><td>Monomorphization</td></tr>
<tr><td><a href="match.html">Match</a></td><td>Pattern matching</td></tr>
<tr><td><a href="memory.html">Memory</a></td><td>Ownership</td></tr>
<tr><td><a href="async.html">Async</a></td><td>async/await</td></tr>
<tr><td><a href="traits-macros.html">Traits</a></td><td>Traits &amp; macros</td></tr>
<tr><td><a href="stdlib.html">Stdlib</a></td><td>API reference</td></tr>
<tr><td><a href="concurrency.html">Concurrency</a></td><td>spawn &amp; channels</td></tr>
<tr><td><a href="examples.html">Examples</a></td><td>Sample apps</td></tr>
<tr><td><a href="os-hardware.html">OS, files &amp; hardware</a></td><td>Filesystem, platform, FFI, bridge</td></tr>
<tr><td><a href="tooling.html">Toolchain</a></td><td>CLI</td></tr>
<tr><td><a href="performance.html">Performance</a></td><td>Release flags, LTO</td></tr>
<tr><td><a href="pgo.html">PGO</a></td><td>Automated profile-guided optimization</td></tr>
<tr><td><a href="escape-analysis.html">Escape analysis</a></td><td>Stack promotion, LocalChannel, #[no_escape]</td></tr>
<tr><td><a href="diagnostics.html">Diagnostics</a></td><td>Errors</td></tr>
<tr><td><a href="ffi-abi.html">FFI</a></td><td>ABI policy</td></tr>
<tr><td><a href="targets.html">Targets</a></td><td>Native &amp; Wasm</td></tr>
<tr><td><a href="editor-setup.html">Editor</a></td><td>Grammar</td></tr>
<tr><td><a href="packages.html">NyraPkg</a></td><td>Packages</td></tr>
<tr><td><a href="roadmap.html">Roadmap</a></td><td>Status</td></tr>
<tr><td><a href="changelog.html">Changelog</a></td><td>History</td></tr>
</tbody></table>''',
        '<a href="index.html">Overview →</a>', "sitemap.html"),
})


def main():
    for filename, (page, title, h1, lead, body, footer, active) in PAGES.items():
        html = shell(page, title, h1, lead, body, footer, active)
        (WEBDOCS / filename).write_text(html, encoding='utf-8')
        print(f'wrote {filename}')


if __name__ == '__main__':
    main()
