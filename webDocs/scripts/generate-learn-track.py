#!/usr/bin/env python3
"""Generate Learn Nyra pages: short prose → example → Output (English source)."""
from __future__ import annotations

import json
from html import escape
from importlib.machinery import SourceFileLoader
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
gp = SourceFileLoader("gp", str(WEBDOCS / "scripts" / "generate-pages.py")).load_module()
shell = gp.shell
EN_PAGES = json.loads((WEBDOCS / "locales" / "en.json").read_text(encoding="utf-8"))["pages"]


def nav_label(page_key: str) -> str:
    """Short prev/next label from en.json h1 (e.g. Variables, Data Types)."""
    return EN_PAGES[page_key]["h1"]


def out_block(text: str) -> str:
    return (
        '<p class="example-output-label">Output</p>\n'
        f'<pre class="example-output"><code>{escape(text)}</code></pre>'
    )


def ex(title: str, prose: str, code: str, output: str | None = None) -> str:
    pieces = [
        '<section class="doc-ex">',
        f"<h2>{title}</h2>",
        f'<p class="doc-ex-prose">{prose}</p>',
        f"<pre><code>{escape(code)}</code></pre>",
    ]
    if output is not None:
        pieces.append(out_block(output))
    pieces.append("</section>")
    return "\n".join(pieces)


def learn_nav(prev_h, prev_l, next_h, next_l):
    prev = (
        f'<a class="lesson-nav-prev" href="{prev_h}">← {prev_l}</a>'
        if prev_h
        else '<span class="lesson-nav-spacer"></span>'
    )
    nxt = (
        f'<a class="lesson-nav-next" href="{next_h}">{next_l} →</a>'
        if next_h
        else '<span class="lesson-nav-spacer"></span>'
    )
    return f"""<nav class="lesson-nav" aria-label="Learn navigation">
{prev}
<a class="lesson-nav-hub" href="learn-intro.html">Learn overview</a>
{nxt}
</nav>"""


def page(slug: str, body: str, prev, nxt):
    key = f"learn-{slug}"
    meta = EN_PAGES[key]
    h1 = meta["h1"]
    return {
        "file": f"{key}.html",
        "page": key,
        "title": meta.get("metaTitle", f"{h1} — Nyra Docs"),
        "h1": h1,
        "lead": meta["lead"],
        "body": body,
        "active": f"{key}.html",
        "prev": prev,
        "next": nxt,
    }


PAGES = [
    page(
        "intro",
        ex(
            "Your first program",
            "<strong>Nyra</strong> is a fast, memory-safe programming language with Rust-inspired ownership, a minimal syntax, and LLVM-backed native compilation.",
            """fn main() {
    print("Hello, Nyra!")
}""",
            "Hello, Nyra!",
        )
        + """
<section class="doc-ex">
  <h2>What you will learn</h2>
  <p class="doc-ex-prose">This tutorial follows a W3Schools-style path: one topic per page, runnable examples, and prev/next navigation. Start with <a href="learn-get-started.html">Get Started</a> after <a href="install.html">Install Nyra</a>.</p>
</section>
""",
        (None, None),
        ("learn-get-started.html", nav_label("learn-get-started")),
    ),
    page(
        "get-started",
        """
<section class="doc-ex">
  <h2>Quick start</h2>
  <p class="doc-ex-prose">1) <a href="install.html">Install Nyra</a> · 2) Create <code>hello.ny</code> · 3) Run <code>nyra run hello.ny</code>.</p>
</section>
"""
        + ex(
            "Run a ready-made example",
            "The run command and its output:",
            """fn main() {
    print("Hello, Nyra!")
}""",
            "Hello, Nyra!",
        ),
        ("learn-intro.html", nav_label("learn-intro")),
        ("learn-syntax.html", nav_label("learn-syntax")),
    ),
    page(
        "syntax",
        ex(
            "Program entry",
            "Nyra uses <strong>curly braces</strong> for blocks. Statements end at newline (no semicolons required). Function names use <code>snake_case</code>; types and structs use <code>PascalCase</code>.",
            """fn main() {
    print("Hello")
}""",
            "Hello",
        ),
        ("learn-get-started.html", nav_label("learn-get-started")),
        ("learn-output.html", nav_label("learn-output")),
    ),
    page(
        "output",
        ex(
            "print",
            "<code>print</code> writes to the terminal and adds a newline after each call.",
            """fn main() {
    print("Hello, Nyra!")
    print(42)
}""",
            "Hello, Nyra!\n42",
        ),
        ("learn-syntax.html", nav_label("learn-syntax")),
        ("learn-comments.html", nav_label("learn-comments")),
    ),
    page(
        "comments",
        ex(
            "Line comments",
            "Everything after <code>//</code> is ignored until end of line.",
            """// this is a comment
let x = 1   // after code
print(x)""",
            "1",
        )
        + ex(
            "Block comments",
            "<code>/* ... */</code> spans multiple lines (non-nested).",
            """/*
 * file header
 */
let total = 1 /* mid-line */ + 2
print(total)""",
            "3",
        ),
        ("learn-output.html", nav_label("learn-output")),
        ("learn-variables.html", nav_label("learn-variables")),
    ),
    page(
        "variables",
        ex(
            "let and let mut",
            "<code>let</code> creates an <strong>immutable</strong> binding. Use <code>let mut</code> when the value must change. Shorthand inside functions: <code>mut x = 0</code>.",
            """fn main() {
    let name = "Sam"
    let mut score = 0
    score = score + 10
    print(score)
}""",
            "10",
        ),
        ("learn-comments.html", nav_label("learn-comments")),
        ("learn-data-types.html", nav_label("learn-data-types")),
    ),
    page(
        "data-types",
        """
<section class="doc-ex">
  <h2>Quick reference</h2>
  <p class="doc-ex-prose">Most-used built-in types:</p>
  <table>
    <thead><tr><th>Type</th><th>Example</th><th>Notes</th></tr></thead>
    <tbody>
      <tr><td><code>i32</code></td><td><code>42</code></td><td>32-bit signed integer (default for literals)</td></tr>
      <tr><td><code>i64</code></td><td><code>10000000000</code></td><td>64-bit signed integer</td></tr>
      <tr><td><code>bool</code></td><td><code>true</code></td><td>Boolean</td></tr>
      <tr><td><code>string</code></td><td><code>"hi"</code></td><td>UTF-8 text (heap-owned when dynamic)</td></tr>
      <tr><td><code>void</code></td><td>—</td><td>No return value</td></tr>
    </tbody>
  </table>
</section>
"""
        + ex(
            "Examples by type",
            "You can write the type explicitly or let the compiler infer it.",
            """fn main() {
    let count = 42
    let flag = true
    let name = "Nyra"
    print(count)
    print(flag)
    print(name)
}""",
            "42\ntrue\nNyra",
        ),
        ("learn-variables.html", nav_label("learn-variables")),
        ("learn-constants.html", nav_label("learn-constants")),
    ),
    page(
        "constants",
        ex(
            "const",
            "<code>const</code> cannot be reassigned and is evaluated at compile time. Unlike <code>let</code>, which creates a runtime binding.",
            """const MAX_LIVES = 3

fn main() {
    print(MAX_LIVES)
}""",
            "3",
        ),
        ("learn-data-types.html", nav_label("learn-data-types")),
        ("learn-operators.html", nav_label("learn-operators")),
    ),
    page(
        "operators",
        """
<section class="doc-ex">
  <h2>Operators</h2>
  <p class="doc-ex-prose"><code>+ - * / %</code> arithmetic · <code>== != &lt; &gt;</code> compare · <code>&amp;&amp; || !</code> logic.</p>
</section>
"""
        + ex(
            "Example",
            "Operation results print as-is.",
            """fn main() {
    print(10 + 5)
    print(10 == 5)
}""",
            "15\nfalse",
        ),
        ("learn-constants.html", nav_label("learn-constants")),
        ("learn-booleans.html", nav_label("learn-booleans")),
    ),
    page(
        "booleans",
        ex(
            "bool",
            "Comparisons and logical operators produce <code>bool</code> values used in <code>if</code> and loops.",
            """fn main() {
    let passed = true
    print(passed)
    print(5 > 3)
}""",
            "true\ntrue",
        ),
        ("learn-operators.html", nav_label("learn-operators")),
        ("learn-if-else.html", nav_label("learn-if-else")),
    ),
    page(
        "if-else",
        ex(
            "if / else",
            "Run one block when the condition is true, otherwise the other.",
            """fn main() {
    let score = 75
    if score >= 60 {
        print("Pass")
    } else {
        print("Fail")
    }
}""",
            "Pass",
        )
        + ex(
            "if as expression",
            "<code>if</code> can return a value directly.",
            """fn main() {
    let score = 95
    let label = if score >= 90 { "A" } else { "B" }
    print(label)
}""",
            "A",
        ),
        ("learn-booleans.html", nav_label("learn-booleans")),
        ("learn-match.html", nav_label("learn-match")),
    ),
    page(
        "match",
        ex(
            "match",
            "Each arm covers one case. Use <code>_</code> for the default.",
            """enum Color { Red Green Blue }

fn main() {
    let c = Color.Red
    let n = match c {
        Color.Red => 1
        Color.Green => 2
        Color.Blue => 3
    }
    print(n)
}""",
            "1",
        ),
        ("learn-if-else.html", nav_label("learn-if-else")),
        ("learn-loops.html", nav_label("learn-loops")),
    ),
    page(
        "loops",
        ex(
            "Sum with for",
            "<code>while</code> repeats while the condition is true. <code>for i in a..b</code> iterates a half-open range.",
            """fn main() {
    let mut sum = 0
    for i in 0..5 {
        sum = sum + i
    }
    print(sum)
}""",
            "10",
        ),
        ("learn-match.html", nav_label("learn-match")),
        ("learn-while.html", nav_label("learn-while")),
    ),
    page(
        "while",
        ex(
            "while",
            "Update the variable inside the loop until the condition becomes false (or you get an infinite loop).",
            """fn main() {
    let mut i = 0
    while i < 3 {
        print(i)
        i = i + 1
    }
}""",
            "0\n1\n2",
        ),
        ("learn-loops.html", nav_label("learn-loops")),
        ("learn-for.html", nav_label("learn-for")),
    ),
    page(
        "for",
        ex(
            "for .. in ..",
            "<code>0..3</code> means from 0 up to (but not including) 3.",
            """fn main() {
    for j in 0..3 {
        print(j)
    }
}""",
            "0\n1\n2",
        ),
        ("learn-while.html", nav_label("learn-while")),
        ("learn-functions.html", nav_label("learn-functions")),
    ),
    page(
        "functions",
        ex(
            "fn",
            "Parameters and return types are optional when inferred; here they are explicit for learning.",
            """fn double(n: i32) -> i32 {
    return n + n
}

fn main() {
    print(double(5))
}""",
            "10",
        )
        + ex(
            "Shorthand body",
            "Single-expression function with <code>=</code>.",
            """fn square(n: i32) -> i32 = n * n

fn main() {
    print(square(4))
}""",
            "16",
        ),
        ("learn-for.html", nav_label("learn-for")),
        ("learn-scope.html", nav_label("learn-scope")),
    ),
    page(
        "scope",
        ex(
            "Block scope",
            "Each <code>{ }</code> block creates a new scope. Bindings declared inside a block are not visible outside it.",
            """fn main() {
    let x = 1
    {
        let y = 2
        print(x)
        print(y)
    }
}""",
            "1\n2",
        ),
        ("learn-functions.html", nav_label("learn-functions")),
        ("learn-strings.html", nav_label("learn-strings")),
    ),
    page(
        "strings",
        ex(
            "String literal",
            "Text between double quotes.",
            """fn main() {
    let greeting = "Hello"
    print(greeting)
}""",
            "Hello",
        )
        + ex(
            "Template strings",
            "Use backticks with <code>{name}</code> to embed values.",
            """fn main() {
    let name = "Nyra"
    print(`Hello, {name}!`)
}""",
            "Hello, Nyra!",
        ),
        ("learn-scope.html", nav_label("learn-scope")),
        ("learn-ownership.html", nav_label("learn-ownership")),
    ),
    page(
        "ownership",
        """
<section class="doc-ex">
  <h2>Quick rule</h2>
  <p class="doc-ex-prose"><code>i32</code> and <code>bool</code> are <strong>Copied</strong>. Heap <code>string</code> values are <strong>Moved</strong> — the previous owner becomes invalid.</p>
</section>
"""
        + ex(
            "Move example",
            "After <code>let b = a</code> on an owned string, do not use <code>a</code>.",
            """fn main() {
    let a = "hello"
    let b = a
    print(b)
}""",
            "hello",
        ),
        ("learn-strings.html", nav_label("learn-strings")),
        ("learn-borrowing.html", nav_label("learn-borrowing")),
    ),
    page(
        "borrowing",
        ex(
            "Mutable borrow",
            "Only one mutable borrow at a time. Finish using it before returning to the owner.",
            """fn main() {
    mut x = 1
    let r = &mut x
    print(*r)
    *r = 5
    print(x)
}""",
            "1\n5",
        )
        + ex(
            "Shared read-only borrow",
            "Multiple <code>&amp;T</code> borrows are allowed together.",
            """fn main() {
    let msg = "hello"
    let r = &msg
    print(*r)
    print(msg)
}""",
            "hello\nhello",
        ),
        ("learn-ownership.html", nav_label("learn-ownership")),
        ("learn-data-structures.html", nav_label("learn-data-structures")),
    ),
    page(
        "data-structures",
        """
<section class="doc-ex">
  <h2>Topic guide</h2>
  <p class="doc-ex-prose">Pick a topic:</p>
  <ul>
    <li><a href="learn-arrays.html">Arrays</a> — fixed size</li>
    <li><a href="learn-vectors.html">Vectors</a> — growable</li>
    <li><a href="learn-tuples.html">Tuples</a> — grouped values</li>
    <li><a href="learn-hashmap.html">HashMap</a> — key → value</li>
    <li><a href="learn-structs.html">Structs</a> — named fields</li>
    <li><a href="learn-enums.html">Enums</a> — fixed variants</li>
  </ul>
</section>
"""
        + ex(
            "Quick tour",
            "Array + tuple + HashMap in one example.",
            """import "stdlib/map"

fn main() {
    let nums = [1, 2, 3]
    print(nums.len())
    let pair = (42, "ok")
    print(pair.0)
    let mut scores = HashMap_str_i32_new()
    scores = scores.insert("ada", 100)
    print(scores.get("ada"))
}""",
            "3\n42\n100",
        ),
        ("learn-borrowing.html", nav_label("learn-borrowing")),
        ("learn-arrays.html", nav_label("learn-arrays")),
    ),
    page(
        "arrays",
        ex(
            "Array",
            "Index starts at 0. All elements share the same type.",
            """fn main() {
    let nums = [1, 2, 3]
    print(nums[0])
    print(nums[1])
}""",
            "1\n2",
        ),
        ("learn-data-structures.html", nav_label("learn-data-structures")),
        ("learn-vectors.html", nav_label("learn-vectors")),
    ),
    page(
        "vectors",
        ex(
            "vec()",
            "Push elements and read length and index. Higher-order helpers like <code>filter</code> ship in v1.45.",
            """fn main() {
    let xs = vec().push(10).push(20).push(30)
    print(xs.len())
    print(xs.get(0))
}""",
            "3\n10",
        ),
        ("learn-arrays.html", nav_label("learn-arrays")),
        ("learn-tuples.html", nav_label("learn-tuples")),
    ),
    page(
        "tuples",
        ex(
            "Tuple",
            "Access fields with <code>.0</code> or destructure with <code>let (a, b) = pair</code>.",
            """fn main() {
    let pair = (1, "hi")
    print(pair.0)
    let (a, b) = pair
    print(a)
}""",
            "1\n1",
        ),
        ("learn-vectors.html", nav_label("learn-vectors")),
        ("learn-hashmap.html", nav_label("learn-hashmap")),
    ),
    page(
        "hashmap",
        ex(
            "HashMap_str_i32",
            "Import <code>stdlib/map</code> then use <code>insert</code> / <code>get</code>.",
            """import "stdlib/map"

fn main() {
    let mut scores = HashMap_str_i32_new()
    scores = scores.insert("score", 100)
    print(scores.get("score"))
}""",
            "100",
        ),
        ("learn-tuples.html", nav_label("learn-tuples")),
        ("learn-structs.html", nav_label("learn-structs")),
    ),
    page(
        "structs",
        ex(
            "struct",
            "Define the shape then create a value with fields.",
            """struct Player {
    hp: i32
    score: i32
}

fn main() {
    let p = Player { hp: 100, score: 0 }
    print(p.hp)
}""",
            "100",
        ),
        ("learn-hashmap.html", nav_label("learn-hashmap")),
        ("learn-enums.html", nav_label("learn-enums")),
    ),
    page(
        "enums",
        ex(
            "enum + match",
            "In v0.2 enums are tag-only (no payloads on variants).",
            """enum Status { Ok Err Pending }

fn main() {
    let s = Status.Ok
    let code = match s {
        Status.Ok => 0
        Status.Err => 1
        Status.Pending => 2
    }
    print(code)
}""",
            "0",
        ),
        ("learn-structs.html", nav_label("learn-structs")),
        (None, None),
    ),
]

REDIRECTS = [
    ("beginner-track.html", "learn-intro.html"),
    ("beginner-01-first-program.html", "learn-output.html"),
    ("beginner-02-variables.html", "learn-variables.html"),
    ("beginner-03-operators.html", "learn-operators.html"),
    ("beginner-04-decisions.html", "learn-if-else.html"),
    ("beginner-05-loops.html", "learn-loops.html"),
    ("beginner-06-functions.html", "learn-functions.html"),
    ("beginner-07-structs-enums.html", "learn-structs.html"),
    ("beginner-08-mini-project.html", "learn-enums.html"),
]


def redirect_html(target: str, label: str) -> str:
    return f"""<!DOCTYPE html>
<html lang="en" dir="ltr">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="refresh" content="0; url={target}">
  <link rel="canonical" href="{target}">
  <title>Moved — {label}</title>
</head>
<body>
  <p>This page moved to <a href="{target}">{target}</a>.</p>
</body>
</html>
"""


def main():
    for p in PAGES:
        prev_h, prev_l = p["prev"]
        next_h, next_l = p["next"]
        body = p["body"] + learn_nav(prev_h, prev_l, next_h, next_l)
        html = shell(p["page"], p["title"], p["h1"], p["lead"], body, "", p["active"])
        (WEBDOCS / p["file"]).write_text(html, encoding="utf-8")
        print(f"wrote {p['file']}")

    for old, new in REDIRECTS:
        label = new.replace(".html", "").replace("learn-", "")
        (WEBDOCS / old).write_text(redirect_html(new, label), encoding="utf-8")
        print(f"wrote redirect {old} -> {new}")


if __name__ == "__main__":
    main()
