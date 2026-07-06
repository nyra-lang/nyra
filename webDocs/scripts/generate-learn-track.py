#!/usr/bin/env python3
"""Generate W3Schools-style Learn Nyra tutorial pages."""
from importlib.machinery import SourceFileLoader
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
gp = SourceFileLoader("gp", str(WEBDOCS / "scripts" / "generate-pages.py")).load_module()
shell = gp.shell


def try_it(cmd, output, note=""):
    extra = f"<p>{note}</p>" if note else ""
    return f"""<div class="callout">
<strong>Try it</strong>
<pre><code>{cmd}</code></pre>
<p><strong>Expected output:</strong></p>
<pre><code>{output}</code></pre>
{extra}
</div>"""


def mistakes(items):
    lis = "".join(f"<li>{x}</li>" for x in items)
    return f'<div class="callout warning"><strong>Common mistakes</strong><ul>{lis}</ul></div>'


def syntax_box(title, code):
    return f"<h2>{title}</h2><pre><code>{code}</code></pre>"


def map_naming_docs():
    """HashMap runtime symbol naming — map_<key>_<value>_<op>."""
    return """
      <h2>Runtime naming: <code>map_&lt;key&gt;_&lt;value&gt;_&lt;op&gt;</code></h2>
      <p>Every hash-map C runtime symbol follows one pattern:</p>
      <pre><code>map_&lt;key_type&gt;_&lt;value_type&gt;_&lt;operation&gt;</code></pre>
      <ul>
        <li><strong>Key type</strong> — first segment after <code>map_</code> (e.g. <code>str</code>, <code>i32</code>)</li>
        <li><strong>Value type</strong> — second segment (e.g. <code>i32</code>, <code>str</code>)</li>
        <li><strong>Operation</strong> — last segment: <code>new</code>, <code>insert</code>, <code>get</code>, <code>contains</code>, <code>remove</code>, <code>keys</code>, <code>free</code>, <code>retain</code></li>
      </ul>
      <p>Examples: <code>map_str_i32_insert</code>, <code>map_str_str_get</code>, <code>map_i32_i32_contains</code>. When key and value share the same type (like Go <code>map[int]int</code>), both appear in the name — <code>map_i32_i32_*</code> is intentional so the ABI stays explicit.</p>
      <p>See the full symbol list in <a href="bindings.html">Runtime bindings</a>.</p>

      <h3>String keys, i32 values — <code>map_str_i32_*</code></h3>
      <pre><code>extern fn map_str_i32_new() -> ptr
extern fn map_str_i32_insert(m: ptr, key: string, value: i32) -> void
extern fn map_str_i32_get(m: ptr, key: string) -> i32
extern fn map_str_i32_contains(m: ptr, key: string) -> i32
extern fn map_str_i32_free(m: ptr) -> void

fn main() {
    let m = map_str_i32_new()
    map_str_i32_insert(m, "score", 100)
    print(map_str_i32_get(m, "score"))
    print(map_str_i32_contains(m, "score"))
    map_str_i32_free(m)
}</code></pre>

      <h3>String keys, string values — <code>map_str_str_*</code></h3>
      <pre><code>extern fn map_str_str_new() -> ptr
extern fn map_str_str_insert(m: ptr, key: string, value: string) -> void
extern fn map_str_str_get(m: ptr, key: string) -> string
extern fn map_str_str_contains(m: ptr, key: string) -> i32
extern fn map_str_str_free(m: ptr) -> void

fn main() {
    let m = map_str_str_new()
    map_str_str_insert(m, "lang", "Nyra")
    print(map_str_str_get(m, "lang"))
    map_str_str_free(m)
}</code></pre>

      <h3>Integer keys and values — <code>map_i32_i32_*</code></h3>
      <pre><code>extern fn map_i32_i32_new() -> ptr
extern fn map_i32_i32_insert(m: ptr, key: i32, value: i32) -> void
extern fn map_i32_i32_get(m: ptr, key: i32) -> i32
extern fn map_i32_i32_contains(m: ptr, key: i32) -> i32
extern fn map_i32_i32_free(m: ptr) -> void

fn main() {
    let m = map_i32_i32_new()
    map_i32_i32_insert(m, 42, 99)
    print(map_i32_i32_get(m, 42))
    print(map_i32_i32_contains(m, 42))
    map_i32_i32_free(m)
}</code></pre>

      <h3>Stdlib wrappers (recommended)</h3>
      <p>Import <code>stdlib/map.ny</code> for <code>HashMap_str_i32</code> and <code>HashMap_str_str</code> — method syntax and automatic cleanup via <code>Drop</code>. The low-level <code>map_*</code> names stay stable for <code>extern fn</code> and FFI.</p>
      <pre><code>import "stdlib/map"

fn main() {
    let mut scores = HashMap_str_i32_new()
    scores = scores.insert("ada", 100)
    print(scores.get("ada"))
}</code></pre>
"""


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


def page(slug, title, lead, body, prev, nxt):
    return {
        "file": f"learn-{slug}.html",
        "page": f"learn-{slug}",
        "title": f"{title} — Nyra Docs",
        "h1": title,
        "lead": lead,
        "body": body,
        "active": f"learn-{slug}.html",
        "prev": prev,
        "next": nxt,
    }


PAGES = [
    page(
        "intro",
        "Nyra Intro",
        "What Nyra is, why it exists, and how this tutorial is organized.",
        """
      <p><strong>Nyra</strong> is a fast, memory-safe programming language with Rust-inspired ownership, a minimal syntax, and LLVM-backed native compilation.</p>
""" + syntax_box(
            "Your first program",
            """fn main() {
    print("Hello, Nyra!")
}""",
        ) + try_it("nyra run examples/syntax/hello.ny", "Hello, Nyra!") + """
      <h2>What you will learn</h2>
      <p>This tutorial follows a W3Schools-style path: one topic per page, runnable examples, and prev/next navigation. Start with <a href="learn-get-started.html">Get Started</a> or jump to any sidebar topic.</p>
      <ul>
        <li><strong>Basics</strong> — variables, types, control flow, functions, strings</li>
        <li><strong>Memory</strong> — ownership and borrowing (Nyra's safety model)</li>
        <li><strong>Data structures</strong> — arrays, vectors, tuples, hash maps, structs, enums</li>
      </ul>
      <p>Prerequisites: <a href="install.html">Install Nyra</a> and <code>clang</code>.</p>
""",
        (None, None),
        ("learn-get-started.html", "Nyra Get Started"),
    ),
    page(
        "get-started",
        "Nyra Get Started",
        "Install Nyra, create your first project, and run a program.",
        """
      <h2>Quick start</h2>
      <ol>
        <li><a href="install.html">Install Nyra</a> (macOS, Linux, or Windows)</li>
        <li>Create <code>hello.ny</code> with a <code>fn main()</code> entry point</li>
        <li>Run: <code>nyra run hello.ny</code></li>
      </ol>
      <p>See also <a href="getting-started.html">Getting started guide</a> for project layout with <code>main.ny</code> and <code>nyra.mod</code>.</p>
""" + try_it("nyra run examples/syntax/hello.ny", "Hello, Nyra!"),
        ("learn-intro.html", "Nyra Intro"),
        ("learn-syntax.html", "Nyra Syntax"),
    ),
    page(
        "syntax",
        "Nyra Syntax",
        "Blocks, statements, indentation, and naming conventions.",
        syntax_box(
            "Program entry",
            """fn main() {
    print("Hello")
}""",
        )
        + """
      <p>Nyra uses <strong>curly braces</strong> for blocks. Statements end at newline (no semicolons required). Function names use <code>snake_case</code>; types and structs use <code>PascalCase</code>.</p>
      <p>Full cheat sheet: <a href="language.html">Syntax reference</a> · <a href="reference.html">Keywords</a></p>
""",
        ("learn-get-started.html", "Nyra Get Started"),
        ("learn-output.html", "Nyra Output"),
    ),
    page(
        "output",
        "Nyra Output",
        "Print text and numbers to the console with <code>print</code>.",
        syntax_box(
            "print",
            """fn main() {
    print("Hello, Nyra!")
    print(42)
}""",
        )
        + try_it("nyra run examples/syntax/hello.ny", "Hello, Nyra!")
        + mistakes([
            "Forgetting quotes around text — strings use double quotes.",
            "Passing too many types in one call — use separate <code>print</code> calls or template strings.",
        ]),
        ("learn-syntax.html", "Nyra Syntax"),
        ("learn-comments.html", "Nyra Comments"),
    ),
    page(
        "comments",
        "Nyra Comments",
        "Document code with line and block comments.",
        syntax_box(
            "Line comments",
            """// This is a comment
let x = 1   // comment after code
print(x)""",
        )
        + syntax_box(
            "Block comments",
            """/*
 * File header
 */
let total = 1 /* inline */ + 2
print(total)""",
        )
        + """
      <p>Nyra supports <code>//</code> line comments and C-style <code>/* ... */</code> block comments (non-nested). Unclosed <code>/*</code> is a lexer error.</p>
""",
        ("learn-output.html", "Nyra Output"),
        ("learn-variables.html", "Nyra Variables"),
    ),
    page(
        "variables",
        "Nyra Variables",
        "Bind names to values with <code>let</code> and <code>let mut</code>.",
        syntax_box(
            "let and let mut",
            """let name = "Sam"
let mut score = 0
score = score + 10
print(score)""",
        )
        + """
      <p><code>let</code> creates an <strong>immutable</strong> binding. Use <code>let mut</code> when the value must change. Shorthand inside functions: <code>mut x = 0</code>.</p>
"""
        + try_it("nyra run examples/comparison/arithmetic/sum.ny", "(arithmetic demo)")
        + mistakes([
            "Assigning to a <code>let</code> binding without <code>mut</code>.",
            "Confusing <code>let</code> with <code>const</code> — see <a href=\"learn-constants.html\">Constants</a>.",
        ]),
        ("learn-comments.html", "Nyra Comments"),
        ("learn-data-types.html", "Nyra Data Types"),
    ),
    page(
        "data-types",
        "Nyra Data Types",
        "Built-in types: integers, booleans, strings, and more.",
        """
      <table>
        <thead><tr><th>Type</th><th>Example</th><th>Notes</th></tr></thead>
        <tbody>
          <tr><td><code>i32</code></td><td><code>42</code></td><td>32-bit signed integer (default for literals)</td></tr>
          <tr><td><code>i64</code></td><td><code>10000000000</code></td><td>64-bit signed integer</td></tr>
          <tr><td><code>u32</code></td><td><code>42</code></td><td>32-bit unsigned integer</td></tr>
          <tr><td><code>bool</code></td><td><code>true</code>, <code>false</code></td><td>Boolean</td></tr>
          <tr><td><code>string</code></td><td><code>"hello"</code></td><td>UTF-8 text (heap-owned when dynamic)</td></tr>
          <tr><td><code>void</code></td><td>—</td><td>No return value</td></tr>
        </tbody>
      </table>
""" + syntax_box(
            "Examples by type",
            """fn main() {
    let count: i32 = 42
    let big: i64 = 1_000_000_000
    let flag: bool = true
    let name: string = "Nyra"
    print(count)
    print(big)
    print(flag)
    print(name)
}""",
        ) + try_it("nyra run examples/syntax/hello.ny", "Hello, Nyra!") + """
      <p>Annotate types explicitly: <code>let age: i32 = 25</code>. See <a href="types.html">Types &amp; data</a> for structs, arrays, and references.</p>
""",
        ("learn-variables.html", "Nyra Variables"),
        ("learn-constants.html", "Nyra Constants"),
    ),
    page(
        "constants",
        "Nyra Constants",
        "Fixed compile-time values with <code>const</code>.",
        syntax_box(
            "const",
            """const MAX_LIVES = 3
const PI = 3

fn main() {
    print(MAX_LIVES)
}""",
        )
        + """
      <table>
        <thead><tr><th></th><th><code>let</code></th><th><code>let mut</code></th><th><code>const</code></th></tr></thead>
        <tbody>
          <tr><td>Can reassign?</td><td>No</td><td>Yes</td><td>No</td></tr>
          <tr><td>When set?</td><td>Runtime</td><td>Runtime</td><td>Compile time</td></tr>
        </tbody>
      </table>
""",
        ("learn-data-types.html", "Nyra Data Types"),
        ("learn-operators.html", "Nyra Operators"),
    ),
    page(
        "operators",
        "Nyra Operators",
        "Arithmetic and comparison operators.",
        """
      <table>
        <thead><tr><th>Operator</th><th>Meaning</th><th>Example</th></tr></thead>
        <tbody>
          <tr><td><code>+ - * / %</code></td><td>Arithmetic</td><td><code>3 + 4</code> → 7</td></tr>
          <tr><td><code>== != &lt; &gt; &lt;= &gt;=</code></td><td>Compare</td><td><code>5 == 5</code> → true</td></tr>
          <tr><td><code>&amp;&amp; || !</code></td><td>Logic</td><td><code>true &amp;&amp; false</code> → false</td></tr>
        </tbody>
      </table>
""" + syntax_box("Example", """fn main() {
    print(10 + 5)
    print(10 == 5)
}""") + try_it("nyra run examples/syntax/math.ny", "(math demo)"),
        ("learn-constants.html", "Nyra Constants"),
        ("learn-booleans.html", "Nyra Booleans"),
    ),
    page(
        "booleans",
        "Nyra Booleans",
        "The <code>bool</code> type: <code>true</code> and <code>false</code>.",
        syntax_box(
            "bool",
            """let passed = true
let failed = false
print(passed)
print(5 > 3)""",
        )
        + """
      <p>Comparisons and logical operators produce <code>bool</code> values used in <code>if</code> and loops.</p>
""",
        ("learn-operators.html", "Nyra Operators"),
        ("learn-if-else.html", "Nyra If..Else"),
    ),
    page(
        "if-else",
        "Nyra If..Else",
        "Branch on conditions with <code>if</code> and <code>else</code>.",
        syntax_box(
            "if / else",
            """let score = 75
if score >= 60 {
    print("Pass")
} else {
    print("Fail")
}""",
        )
        + syntax_box(
            "If-expression",
            """let label = if score >= 90 { "A" } else { "B" }
print(label)""",
        )
        + mistakes([
            "Using <code>=</code> instead of <code>==</code> in conditions.",
            "Mismatched types in if-expression branches.",
        ]),
        ("learn-booleans.html", "Nyra Booleans"),
        ("learn-match.html", "Nyra Match"),
    ),
    page(
        "match",
        "Nyra Match",
        "Pattern match on enums and values.",
        syntax_box(
            "match",
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
        )
        + """
      <p>Guards: <code>Color.Red if x > 0 => 1</code>. Wildcard: <code>_ => 0</code>. See <a href="match.html">Match reference</a>.</p>
"""
        + try_it("nyra run examples/language_features/demo.ny", "1"),
        ("learn-if-else.html", "Nyra If..Else"),
        ("learn-loops.html", "Nyra Loops"),
    ),
    page(
        "loops",
        "Nyra Loops",
        "Overview of <code>while</code> and <code>for</code> loops.",
        """
      <p>Nyra has two loop forms:</p>
      <ul>
        <li><code>while condition { ... }</code> — repeat while true</li>
        <li><code>for i in start..end { ... }</code> — iterate a half-open range</li>
      </ul>
      <p>Details: <a href="learn-while.html">While loops</a> · <a href="learn-for.html">For loops</a></p>
""" + syntax_box("Sum with for", """fn main() {
    let mut sum = 0
    for i in 0..5 {
        sum = sum + i
    }
    print(sum)
}"""),
        ("learn-match.html", "Nyra Match"),
        ("learn-while.html", "Nyra While Loops"),
    ),
    page(
        "while",
        "Nyra While Loops",
        "Repeat while a condition is true.",
        syntax_box(
            "while",
            """let mut i = 0
while i < 3 {
    print(i)
    i = i + 1
}""",
        )
        + try_it("nyra run examples/comparison/loop/sum_loop_small.ny", "499500")
        + mistakes(["Infinite loop — ensure the condition eventually becomes false."]),
        ("learn-loops.html", "Nyra Loops"),
        ("learn-for.html", "Nyra For Loops"),
    ),
    page(
        "for",
        "Nyra For Loops",
        "Iterate over numeric ranges.",
        syntax_box(
            "for .. in ..",
            """for j in 0..3 {
    print(j)   // 0, 1, 2
}""",
        )
        + """
      <p><code>0..3</code> means from 0 up to (but not including) 3. Use <code>let mut</code> for accumulators outside the loop.</p>
""",
        ("learn-while.html", "Nyra While Loops"),
        ("learn-functions.html", "Nyra Functions"),
    ),
    page(
        "functions",
        "Nyra Functions",
        "Define reusable functions with parameters and return types.",
        syntax_box(
            "fn",
            """fn double(n: i32) -> i32 {
    return n + n
}

fn main() {
    print(double(5))
}""",
        )
        + syntax_box(
            "Shorthand body",
            "fn square(n: i32) -> i32 = n * n",
        )
        + try_it("nyra run examples/syntax/math.ny", "(math helpers)"),
        ("learn-for.html", "Nyra For Loops"),
        ("learn-scope.html", "Nyra Scope"),
    ),
    page(
        "scope",
        "Nyra Scope",
        "Where variables are visible — block scope in Nyra.",
        syntax_box(
            "Block scope",
            """fn main() {
    let x = 1
    {
        let y = 2
        print(x)   // ok — outer scope
        print(y)
    }
    // print(y)  // error — y not in scope
}""",
        )
        + """
      <p>Each <code>{ }</code> block creates a new scope. Bindings declared inside a block are not visible outside it.</p>
""",
        ("learn-functions.html", "Nyra Functions"),
        ("learn-strings.html", "Nyra Strings"),
    ),
    page(
        "strings",
        "Nyra Strings",
        "Text literals, concatenation, and template strings.",
        syntax_box(
            "String literal",
            'let greeting = "Hello"\nprint(greeting)',
        )
        + syntax_box(
            "Template strings",
            """let name = "Nyra"
print(`Hello, {name}!`)""",
        )
        + try_it("nyra run examples/syntax/template_strings.ny", "(template output)")
        + """
      <p>Stdlib: <code>import "stdlib/strings.ny"</code> for <code>strlen</code>, <code>strcat</code>, etc.</p>
""",
        ("learn-scope.html", "Nyra Scope"),
        ("learn-ownership.html", "Nyra Ownership"),
    ),
    page(
        "ownership",
        "Nyra Ownership",
        "Copy vs Move — how Nyra tracks who owns heap data.",
        """
      <table>
        <thead><tr><th>Type</th><th>Kind</th><th>Behavior</th></tr></thead>
        <tbody>
          <tr><td><code>i32</code>, <code>bool</code>, enums</td><td><strong>Copy</strong></td><td>Both bindings stay valid after assign</td></tr>
          <tr><td><code>string</code> (heap)</td><td><strong>Move</strong></td><td>Single owner; source invalidated on move</td></tr>
          <tr><td><code>struct</code></td><td>Copy or Move</td><td>Move if any field is Move</td></tr>
        </tbody>
      </table>
""" + syntax_box("Move example", """fn main() {
    let a = "hello"
    let b = a
    // print(a)  // error: use of moved value
    print(b)
}""") + """
      <p>Owned heap strings are <strong>auto-dropped</strong> at scope end. Deep dive: <a href="memory.html">Memory &amp; ownership</a>.</p>
""",
        ("learn-strings.html", "Nyra Strings"),
        ("learn-borrowing.html", "Nyra Borrowing"),
    ),
    page(
        "borrowing",
        "Nyra Borrowing",
        "Temporary access with <code>&amp;T</code> and <code>&amp;mut T</code>.",
        syntax_box(
            "Mutable borrow",
            """fn main() {
    mut x = 1
    let r = &mut x
    print(*r)
    *r = 5
    print(x)
}""",
        )
        + syntax_box(
            "Shared read-only borrow",
            """fn main() {
    let msg = "hello"
    let r = &msg
    print(*r)
    print(msg)
}""",
        )
        + """
      <p>Borrows expire at the <strong>last use</strong> (non-lexical lifetimes). Rules:</p>
      <ul>
        <li>Many <code>&amp;T</code> OR one <code>&amp;mut T</code> at a time</li>
        <li>Cannot use a moved value after move</li>
        <li>Cannot return <code>&amp;local</code></li>
      </ul>
""",
        ("learn-ownership.html", "Nyra Ownership"),
        ("learn-data-structures.html", "Nyra Data Structures"),
    ),
    page(
        "data-structures",
        "Nyra Data Structures",
        "Overview of arrays, vectors, tuples, maps, structs, and enums.",
        """
      <ul>
        <li><a href="learn-arrays.html">Arrays</a> — fixed-size <code>[T; N]</code></li>
        <li><a href="learn-vectors.html">Vectors</a> — growable <code>Vec&lt;T&gt;</code></li>
        <li><a href="learn-tuples.html">Tuples</a> — <code>(T, U)</code> grouped values</li>
        <li><a href="learn-hashmap.html">HashMap</a> — key-value maps</li>
        <li><a href="learn-structs.html">Structs</a> — named field groups</li>
        <li><a href="learn-enums.html">Enums</a> — fixed variant sets</li>
      </ul>
""" + syntax_box(
            "Quick tour",
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
        ) + try_it("nyra run examples/syntax/hashmap.ny", "100\ntrue", note="HashMap runnable demo; combine with array and tuple snippets from sibling lessons.") + """
""",
        ("learn-borrowing.html", "Nyra Borrowing"),
        ("learn-arrays.html", "Nyra Arrays"),
    ),
    page(
        "arrays",
        "Nyra Arrays",
        "Fixed-size arrays and indexing.",
        syntax_box(
            "Array",
            """fn main() {
    let nums: [i32; 3] = [1, 2, 3]
    print(nums[0])
    print(nums[1])
}""",
        )
        + try_it("nyra run examples/syntax/arrays.ny", "1\n2")
        + mistakes(["Index must be <code>i32</code>.", "All elements must have the same type."]),
        ("learn-data-structures.html", "Nyra Data Structures"),
        ("learn-vectors.html", "Nyra Vectors"),
    ),
    page(
        "vectors",
        "Nyra Vectors",
        "Growable arrays with <code>Vec&lt;T&gt;</code>.",
        syntax_box(
            "Vec (runtime API)",
            """extern fn vec_i32_new() -> ptr
extern fn vec_i32_push(v: ptr, x: i32) -> void
extern fn vec_i32_len(v: ptr) -> i32
extern fn vec_i32_get(v: ptr, i: i32) -> i32
extern fn vec_i32_free(v: ptr) -> void

fn main() {
    let handle = vec_i32_new()
    vec_i32_push(handle, 10)
    vec_i32_push(handle, 20)
    print(vec_i32_len(handle))
    print(vec_i32_get(handle, 0))
    vec_i32_free(handle)
}""",
        )
        + try_it("nyra run examples/syntax/vectors.ny", "2\n10")
        + """
      <p>Import <code>stdlib/vec.ny</code> for <code>Vec_i32</code> and <code>Vec_str</code> wrappers.</p>
""",
        ("learn-arrays.html", "Nyra Arrays"),
        ("learn-tuples.html", "Nyra Tuples"),
    ),
    page(
        "tuples",
        "Nyra Tuples",
        "Group values with tuple types and literals.",
        syntax_box(
            "Tuple",
            """fn main() {
    let pair: (i32, string) = (1, "hi")
    print(pair.0)
    let (a, b) = pair
    print(a)
}""",
        )
        + try_it("nyra run examples/syntax/tuples.ny", "1\n1"),
        ("learn-vectors.html", "Nyra Vectors"),
        ("learn-hashmap.html", "Nyra HashMap"),
    ),
    page(
        "hashmap",
        "Nyra HashMap",
        "Key-value maps: runtime naming map_<key>_<value>_<op> and stdlib wrappers.",
        map_naming_docs()
        + syntax_box(
            "Runnable example",
            """extern fn map_str_i32_new() -> ptr
extern fn map_str_i32_insert(m: ptr, key: string, value: i32) -> void
extern fn map_str_i32_get(m: ptr, key: string) -> i32
extern fn map_str_i32_contains(m: ptr, key: string) -> i32
extern fn map_str_i32_free(m: ptr) -> void

fn main() {
    let handle = map_str_i32_new()
    map_str_i32_insert(handle, "score", 100)
    print(map_str_i32_get(handle, "score"))
    print(map_str_i32_contains(handle, "score"))
    map_str_i32_free(handle)
}""",
        )
        + try_it("nyra run examples/syntax/hashmap.ny", "100\ntrue"),
        ("learn-tuples.html", "Nyra Tuples"),
        ("learn-structs.html", "Nyra Structs"),
    ),
    page(
        "structs",
        "Nyra Structs",
        "Define custom types with named fields.",
        syntax_box(
            "struct",
            """struct Player {
    hp: i32
    score: i32
}

fn main() {
    let p = Player { hp: 100, score: 0 }
    print(p.hp)
}""",
        )
        + """
      <p>Methods: <code>impl Player { fn heal(self) -> void { ... } }</code>. See <a href="traits-macros.html">Traits</a>.</p>
""",
        ("learn-hashmap.html", "Nyra HashMap"),
        ("learn-enums.html", "Nyra Enums"),
    ),
    page(
        "enums",
        "Nyra Enums",
        "Named variants and pattern matching.",
        syntax_box(
            "enum + match",
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
        )
        + """
      <p>v0.2 enums are <strong>tag-only</strong> (no payloads). Next: <a href="methods.html">Built-in methods</a> · <a href="reference.html">Language reference</a>.</p>
"""
        + try_it("nyra run examples/language_features/demo.ny", "1"),
        ("learn-structs.html", "Nyra Structs"),
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
<html lang="en">
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
