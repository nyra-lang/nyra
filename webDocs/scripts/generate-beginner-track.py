#!/usr/bin/env python3
"""Generate beginner track HTML pages (hub + 8 lessons)."""
from importlib.machinery import SourceFileLoader
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
gp = SourceFileLoader("gp", str(WEBDOCS / "scripts" / "generate-pages.py")).load_module()
shell = gp.shell


def lesson_goals(items):
    lis = "".join(f"<li>{x}</li>" for x in items)
    return f'<div class="lesson-goals"><strong>You will learn</strong><ul>{lis}</ul></div>'


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


def line_table(rows):
    body = "".join(
        f"<tr><td>{n}</td><td><code>{code}</code></td><td>{meaning}</td></tr>"
        for n, code, meaning in rows
    )
    return f"""<table class="line-table">
<thead><tr><th>Line</th><th>Code</th><th>What it means</th></tr></thead>
<tbody>{body}</tbody></table>"""


def lesson_nav(prev_href, prev_label, next_href, next_label):
    prev = (
        f'<a class="lesson-nav-prev" href="{prev_href}">← {prev_label}</a>'
        if prev_href
        else '<span class="lesson-nav-spacer"></span>'
    )
    nxt = (
        f'<a class="lesson-nav-next" href="{next_href}">{next_label} →</a>'
        if next_href
        else '<span class="lesson-nav-spacer"></span>'
    )
    return f"""<nav class="lesson-nav" aria-label="Lesson navigation">
{prev}
<a class="lesson-nav-hub" href="beginner-track.html">Track overview</a>
{nxt}
</nav>"""


def wrap_lesson(num, title, lead, body, prev, next, active):
    footer = lesson_nav(prev[0], prev[1], next[0], next[1])
    return shell(
        f"beginner-{num:02d}",
        f"Lesson {num}: {title} — Nyra Docs",
        f"Lesson {num}: {title}",
        lead,
        body + footer,
        "",
        active,
    )


HUB_BODY = """
      <p class="lesson-meta">8 lessons · ~3–4 hours · no prior programming required</p>
      <div class="callout">
        <strong>Before you start</strong>
        <p>Install Nyra and <code>clang</code>, then confirm you can run a program: <a href="install.html">Installation</a> → <a href="getting-started.html">Getting started</a>.</p>
      </div>
      <ol class="lesson-track-list">
        <li><a href="beginner-01-first-program.html"><strong>Lesson 1</strong> — Your first program</a><span>What code is, <code>fn main</code>, <code>print</code>, running <code>nyra run</code></span></li>
        <li><a href="beginner-02-variables.html"><strong>Lesson 2</strong> — Variables: <code>let</code>, <code>mut</code>, <code>const</code></a><span>Names for values, mutable vs immutable, when to reassign</span></li>
        <li><a href="beginner-03-operators.html"><strong>Lesson 3</strong> — Operators &amp; expressions</a><span>Math, comparisons, combining values</span></li>
        <li><a href="beginner-04-decisions.html"><strong>Lesson 4</strong> — Decisions with <code>if</code></a><span><code>true</code>/<code>false</code>, branching, if-expressions</span></li>
        <li><a href="beginner-05-loops.html"><strong>Lesson 5</strong> — Loops</a><span><code>while</code>, <code>for</code>, counters, repeating work</span></li>
        <li><a href="beginner-06-functions.html"><strong>Lesson 6</strong> — Functions</a><span>Reuse code, parameters, return values</span></li>
        <li><a href="beginner-07-structs-enums.html"><strong>Lesson 7</strong> — Structs &amp; enums</a><span>Group data, named choices, intro <code>match</code></span></li>
        <li><a href="beginner-08-mini-project.html"><strong>Lesson 8</strong> — Mini project: score tracker</a><span>Put it together in one small program</span></li>
      </ol>
      <p>After the track: <a href="dungeon-steps.html">Dungeon Steps</a> (multi-file app) · <a href="language-basics.html">Language basics</a> (reference) · <a href="examples.html">Example apps</a></p>
"""

LESSON_01 = lesson_goals([
    "What a program is and where execution starts",
    "How to print text to the screen",
    "How to run a <code>.ny</code> file with the Nyra CLI",
]) + """
      <h2>What is a program?</h2>
      <p>A <strong>program</strong> is a list of instructions for the computer. You write instructions in a file; a tool called a <strong>compiler</strong> turns them into something the machine can run. Nyra uses files ending in <code>.ny</code>.</p>
      <h2>Your first file</h2>
      <p>Create a file named <code>hello.ny</code> with this content:</p>
      <pre><code>fn main() {
    print("Hello, Nyra!")
}</code></pre>
""" + line_table([
    ("1", "fn main() {", "Define a function named <code>main</code>. Every Nyra program starts here when you run it."),
    ("2", 'print("Hello, Nyra!")', "Call the built-in <code>print</code> function to show text on the screen."),
    ("3", "}", "End of the <code>main</code> function block."),
]) + try_it("nyra run hello.ny", "Hello, Nyra!", "From the repo you can also run: <code>nyra run examples/syntax/hello.ny</code>") + mistakes([
    "Forgetting quotes around text — strings must be in double quotes: <code>\"Hello\"</code>.",
    "Running from the wrong folder — use the path to your <code>.ny</code> file, or <code>cd</code> into its directory first.",
    "Using <code>nyra run .</code> on a single file with no <code>main.ny</code> in a folder — for one file, pass the file path.",
])

LESSON_02 = lesson_goals([
    "What a variable is (a name for a value)",
    "The difference between <code>let</code>, <code>let mut</code>, and <code>const</code>",
    "What <code>mut</code> means — short for <strong>mutable</strong> (changeable)",
]) + """
      <h2>What is a variable?</h2>
      <p>A <strong>variable</strong> is a name you choose for a value — like putting a label on a box so you can find it later.</p>
      <pre><code>let age = 25
print(age)</code></pre>
      <p>Here <code>age</code> is the name and <code>25</code> is the value. When the program runs, Nyra remembers that <code>age</code> means <code>25</code>.</p>
      <h2><code>let</code> — bind a name (immutable)</h2>
      <p><code>let</code> creates a variable and sets its first value. By default the binding is <strong>immutable</strong>: you must not assign a new value to that name later.</p>
      <pre><code>let score = 10
print(score)   // OK — reading is always allowed
// score = 20  // ERROR — score is not mut</code></pre>
      <h2><code>mut</code> — mutable (changeable)</h2>
      <p><code>mut</code> is short for <strong>mutable</strong>. It means the value stored under that name <em>may change</em> while the program runs.</p>
      <pre><code>let mut lives = 3
lives = lives - 1
print(lives)   // 2</code></pre>
      <p>Use <code>let mut</code> for counters, scores, loop indices — anything you update over time.</p>
      <h2><code>const</code> — compile-time constant</h2>
      <p><code>const</code> names a value that is fixed for the whole program and known when you compile (not computed from user input at runtime).</p>
      <pre><code>const MAX_LIVES = 3
print(MAX_LIVES)</code></pre>
      <table>
        <thead><tr><th></th><th><code>let</code></th><th><code>let mut</code></th><th><code>const</code></th></tr></thead>
        <tbody>
          <tr><td><strong>Can reassign?</strong></td><td>No</td><td>Yes</td><td>No</td></tr>
          <tr><td><strong>When is value set?</strong></td><td>When that line runs</td><td>When that line runs; can change later</td><td>At compile time</td></tr>
          <tr><td><strong>Example</strong></td><td><code>let name = "Sam"</code></td><td><code>let mut gold = 0</code></td><td><code>const MAX = 100</code></td></tr>
        </tbody>
      </table>
      <h2>Side by side</h2>
      <pre><code>const MAX = 100
let title = "Game"
let mut score = 0
score = score + 10
print(score)   // 10</code></pre>
""" + try_it(
    "nyra run examples/comparison/arithmetic/sum.ny",
    "(prints the result of adding two numbers)",
) + mistakes([
    "Writing <code>score = 20</code> without <code>let mut</code> — Nyra reports that you cannot assign to an immutable binding.",
    "Confusing <code>const</code> with <code>let</code> — <code>const</code> is for fixed values shared everywhere, not for values that change once at runtime.",
    "Using <code>mut</code> alone without a value: write <code>let mut x = 0</code> or the shorthand <code>mut x = 0</code> inside functions.",
])

LESSON_03 = lesson_goals([
    "Arithmetic operators: <code>+</code> <code>-</code> <code>*</code>",
    "Comparisons: <code>==</code> <code>!=</code> <code>&lt;</code> <code>&gt;</code>",
    "The difference between an expression and a statement",
]) + """
      <h2>Expressions compute values</h2>
      <p>An <strong>expression</strong> is code that produces a value. You can put it inside <code>print</code> or store it in a variable.</p>
      <pre><code>print(1 + 2)           // 3
let total = 10 + 5
print(total)           // 15</code></pre>
      <h2>Arithmetic</h2>
      <table>
        <thead><tr><th>Operator</th><th>Meaning</th><th>Example</th></tr></thead>
        <tbody>
          <tr><td><code>+</code></td><td>add</td><td><code>3 + 4</code> → 7</td></tr>
          <tr><td><code>-</code></td><td>subtract</td><td><code>10 - 3</code> → 7</td></tr>
          <tr><td><code>*</code></td><td>multiply</td><td><code>6 * 7</code> → 42</td></tr>
        </tbody>
      </table>
      <h2>Comparisons (true or false)</h2>
      <p>Comparison operators return a <code>bool</code>: <code>true</code> or <code>false</code>.</p>
      <pre><code>print(5 == 5)    // true  (equal)
print(5 != 3)    // true  (not equal)
print(2 &lt; 10)    // true
print(2 &gt; 10)    // false</code></pre>
      <h2>Combining ideas</h2>
      <pre><code>fn main() {
    let a = 7
    let b = 3
    print(a + b)
    print(a == b)
}</code></pre>
""" + try_it("nyra run examples/comparison/arithmetic/sum.ny", "(sum of two values)") + mistakes([
    "Using <code>=</code> when you mean compare — use <code>==</code> for equality; single <code>=</code> is for assignment to a <code>mut</code> variable.",
    "Mixing strings and numbers in <code>+</code> without converting — in Nyra, <code>print</code> handles one type at a time; stick to numbers in this lesson.",
])

LESSON_04 = lesson_goals([
    "The <code>bool</code> type: <code>true</code> and <code>false</code>",
    "<code>if</code> / <code>else</code> to run different code paths",
    "If-expressions that produce a value",
]) + """
      <h2>Booleans</h2>
      <p>A <code>bool</code> is either <code>true</code> or <code>false</code>. Comparisons and conditions use booleans.</p>
      <h2>if / else</h2>
      <pre><code>let score = 75

if score >= 60 {
    print("You passed!")
} else {
    print("Try again.")
}</code></pre>
""" + line_table([
    ("1", "let score = 75", "Store the score (immutable — we only read it)."),
    ("2", "if score >= 60 {", "If the condition is true, run the first block."),
    ("3", 'print("You passed!")', "Runs only when score is 60 or higher."),
    ("4", "} else {", "Otherwise run the else block."),
    ("5", 'print("Try again.")', "Runs when score is below 60."),
]) + """
      <h2>If-expression</h2>
      <p>An <code>if</code> can produce a value when both branches return the same type:</p>
      <pre><code>let label = if score >= 90 { "A" } else { "B" }
print(label)</code></pre>
""" + try_it(
    "Create pass.ny with the if/else example above, then: nyra run pass.ny",
    "You passed!",
) + mistakes([
    "Forgetting braces <code>{ }</code> around if bodies — Nyra requires blocks for multi-line if bodies.",
    "Using a number where a bool is expected — the condition after <code>if</code> must be <code>true</code> or <code>false</code>.",
])

LESSON_05 = lesson_goals([
    "<code>while</code> — repeat while a condition stays true",
    "<code>for i in 0..n</code> — repeat for each number in a range",
    "Why loops almost always need <code>let mut</code>",
]) + """
      <h2>Why loops?</h2>
      <p>Loops repeat work without copying the same lines many times. You need a <strong>mutable</strong> counter or accumulator that changes each iteration.</p>
      <h2>while loop</h2>
      <pre><code>let mut i = 0
while i &lt; 3 {
    print(i)
    i = i + 1
}</code></pre>
      <table class="line-table">
        <thead><tr><th>Step</th><th>i</th><th>Output</th></tr></thead>
        <tbody>
          <tr><td>1</td><td>0</td><td>0</td></tr>
          <tr><td>2</td><td>1</td><td>1</td></tr>
          <tr><td>3</td><td>2</td><td>2</td></tr>
          <tr><td>4</td><td>3 — stop</td><td>(loop ends)</td></tr>
        </tbody>
      </table>
      <h2>for loop</h2>
      <pre><code>for j in 0..3 {
    print(j)    // 0, 1, 2
}</code></pre>
      <p><code>0..3</code> means start at 0, stop before 3.</p>
      <h2>Sum example</h2>
      <pre><code>fn main() {
    let mut sum = 0
    for i in 0..5 {
        sum = sum + i
    }
    print(sum)   // 0+1+2+3+4 = 10
}</code></pre>
""" + try_it("nyra run examples/comparison/loop/sum_loop.ny", "(large sum — may take a moment)") + mistakes([
    "Infinite loop — if the counter never increases, <code>while</code> runs forever. Always ensure the condition becomes false.",
    "Using <code>let i = 0</code> then <code>i = i + 1</code> — assignment requires <code>let mut i</code>.",
])

LESSON_06 = lesson_goals([
    "Why functions avoid duplicated code",
    "Parameters (inputs) and return values (outputs)",
    "Calling a function you defined",
]) + """
      <h2>Why functions?</h2>
      <p>A <strong>function</strong> is a named piece of code you can run whenever you need it. Change the logic once, every call site benefits.</p>
      <pre><code>fn double(n: i32) -> i32 {
    return n + n
}

fn main() {
    print(double(5))    // 10
    print(double(21))   // 42
}</code></pre>
""" + line_table([
    ("1", "fn double(n: i32) -> i32 {", "Function named <code>double</code>; takes one <code>i32</code> called <code>n</code>; returns <code>i32</code>."),
    ("2", "return n + n", "Send the result back to whoever called <code>double</code>."),
    ("3", "print(double(5))", "Call <code>double</code> with argument <code>5</code>; print the result."),
]) + """
      <h2>Function with no return value</h2>
      <pre><code>fn greet(name: string) {
    print(name)
}

fn main() {
    greet("Nyra")
}</code></pre>
""" + try_it("nyra run examples/syntax/math.ny", "(math helpers demo)") + mistakes([
    "Forgetting <code>return</code> when the function promises a type with <code>-&gt; i32</code>.",
    "Mismatching parameter types — if the function expects <code>i32</code>, do not pass a string.",
])

LESSON_07 = lesson_goals([
    "Structs — group related fields into one type",
    "Enums — a fixed set of named variants",
    "Intro <code>match</code> to pick behavior by variant",
]) + """
      <h2>Struct — group fields</h2>
      <pre><code>struct Player {
    hp: i32
    score: i32
}

fn main() {
    let p = Player { hp: 100, score: 0 }
    print(p.hp)
    print(p.score)
}</code></pre>
      <p>Access fields with a dot: <code>p.score</code>.</p>
      <h2>Enum — named choices</h2>
      <pre><code>enum Color {
    Red
    Green
    Blue
}</code></pre>
      <h2>match — branch on enum</h2>
      <pre><code>fn main() {
    let c = Color.Red
    let n = match c {
        Color.Red => 1
        Color.Green => 2
        Color.Blue => 3
    }
    print(n)   // 1
}</code></pre>
""" + try_it("nyra run examples/language_features/demo.ny", "1") + mistakes([
    "Forgetting a variant in <code>match</code> — list every enum variant in the arms.",
    "Trying to store extra data in enum variants in v0.2 — Nyra enums are tags only (no payloads like <code>Red(255)</code>).",
])

LESSON_08 = lesson_goals([
    "Combine variables, functions, and <code>print</code> in one program",
    "Build a small score tracker step by step",
    "Know where to go next in the docs",
]) + """
      <h2>Mini project: score tracker</h2>
      <p>We will build a program that starts at 0, adds points twice, and prints the final score.</p>
      <h3>Step 1 — start at zero</h3>
      <pre><code>fn main() {
    let mut score = 0
    print(score)
}</code></pre>
      <h3>Step 2 — add a function</h3>
      <pre><code>fn add_points(current: i32, delta: i32) -> i32 {
    return current + delta
}

fn main() {
    let mut score = 0
    score = add_points(score, 10)
    score = add_points(score, 5)
    print(score)   // 15
}</code></pre>
      <h3>Step 3 — optional struct (stretch)</h3>
      <pre><code>struct Game {
    score: i32
}

fn add_points(game: Game, delta: i32) -> Game {
    return Game { score: game.score + delta }
}

fn main() {
    let mut g = Game { score: 0 }
    g = add_points(g, 10)
    g = add_points(g, 5)
    print(g.score)   // 15
}</code></pre>
      <p>Save as <code>tracker.ny</code> and run <code>nyra run tracker.ny</code>.</p>
""" + try_it("nyra run tracker.ny", "15", "After saving step 2 or 3 above.") + """
      <h2>Congratulations</h2>
      <p>You completed the beginner track. Next steps:</p>
      <ul>
        <li><a href="dungeon-steps.html">Dungeon Steps</a> — multi-file game project</li>
        <li><a href="imports.html">Imports guide</a> — split code across files</li>
        <li><a href="memory.html">Memory &amp; ownership</a> — deeper rules (read when ready)</li>
        <li><a href="examples.html">Example apps</a> — more copy-paste projects</li>
      </ul>
""" + mistakes([
    "Skipping <code>let mut</code> on <code>score</code> before reassignment.",
    "Calling <code>add_points</code> but not storing the result — functions return new values; assign back to <code>score</code> or <code>g</code>.",
])

PAGES = [
    {
        "file": "beginner-track.html",
        "page": "beginner-track",
        "title": "Beginner track — Nyra Docs",
        "h1": "Beginner track",
        "lead": "Step-by-step lessons for your first programs in Nyra — no prior experience required.",
        "body": HUB_BODY,
        "footer": '<a href="beginner-01-first-program.html">Start lesson 1 →</a>',
        "active": "beginner-track.html",
        "nav": None,
    },
    {
        "file": "beginner-01-first-program.html",
        "page": "beginner-01",
        "title": "Lesson 1: Your first program — Nyra Docs",
        "h1": "Lesson 1: Your first program",
        "lead": "What a program is, line-by-line through Hello World, and how to run it.",
        "body": LESSON_01,
        "active": "beginner-01-first-program.html",
        "nav": (None, None, "beginner-02-variables.html", "Lesson 2"),
    },
    {
        "file": "beginner-02-variables.html",
        "page": "beginner-02",
        "title": "Lesson 2: Variables — Nyra Docs",
        "h1": "Lesson 2: Variables",
        "lead": "Deep dive into let, mut (mutable), let mut, and const — with examples and mistakes.",
        "body": LESSON_02,
        "active": "beginner-02-variables.html",
        "nav": ("beginner-01-first-program.html", "Lesson 1", "beginner-03-operators.html", "Lesson 3"),
    },
    {
        "file": "beginner-03-operators.html",
        "page": "beginner-03",
        "title": "Lesson 3: Operators — Nyra Docs",
        "h1": "Lesson 3: Operators & expressions",
        "lead": "Arithmetic, comparisons, and expressions that compute values.",
        "body": LESSON_03,
        "active": "beginner-03-operators.html",
        "nav": ("beginner-02-variables.html", "Lesson 2", "beginner-04-decisions.html", "Lesson 4"),
    },
    {
        "file": "beginner-04-decisions.html",
        "page": "beginner-04",
        "title": "Lesson 4: Decisions — Nyra Docs",
        "h1": "Lesson 4: Decisions with if",
        "lead": "Booleans, if/else, and choosing what code runs.",
        "body": LESSON_04,
        "active": "beginner-04-decisions.html",
        "nav": ("beginner-03-operators.html", "Lesson 3", "beginner-05-loops.html", "Lesson 5"),
    },
    {
        "file": "beginner-05-loops.html",
        "page": "beginner-05",
        "title": "Lesson 5: Loops — Nyra Docs",
        "h1": "Lesson 5: Loops",
        "lead": "while and for loops, mutable counters, and tracing iterations.",
        "body": LESSON_05,
        "active": "beginner-05-loops.html",
        "nav": ("beginner-04-decisions.html", "Lesson 4", "beginner-06-functions.html", "Lesson 6"),
    },
    {
        "file": "beginner-06-functions.html",
        "page": "beginner-06",
        "title": "Lesson 6: Functions — Nyra Docs",
        "h1": "Lesson 6: Functions",
        "lead": "Reuse logic with parameters and return values.",
        "body": LESSON_06,
        "active": "beginner-06-functions.html",
        "nav": ("beginner-05-loops.html", "Lesson 5", "beginner-07-structs-enums.html", "Lesson 7"),
    },
    {
        "file": "beginner-07-structs-enums.html",
        "page": "beginner-07",
        "title": "Lesson 7: Structs & enums — Nyra Docs",
        "h1": "Lesson 7: Structs & enums",
        "lead": "Group data in structs, name choices with enums, branch with match.",
        "body": LESSON_07,
        "active": "beginner-07-structs-enums.html",
        "nav": ("beginner-06-functions.html", "Lesson 6", "beginner-08-mini-project.html", "Lesson 8"),
    },
    {
        "file": "beginner-08-mini-project.html",
        "page": "beginner-08",
        "title": "Lesson 8: Mini project — Nyra Docs",
        "h1": "Lesson 8: Mini project",
        "lead": "Build a score tracker and see where to go next.",
        "body": LESSON_08,
        "active": "beginner-08-mini-project.html",
        "nav": ("beginner-07-structs-enums.html", "Lesson 7", "dungeon-steps.html", "Dungeon Steps"),
    },
]


def main():
    for p in PAGES:
        body = p["body"]
        if p.get("nav"):
            prev_h, prev_l, next_h, next_l = p["nav"]
            body = body + lesson_nav(prev_h, prev_l, next_h, next_l)
        html = shell(
            p["page"],
            p["title"],
            p["h1"],
            p["lead"],
            body,
            p.get("footer", ""),
            p["active"],
        )
        (WEBDOCS / p["file"]).write_text(html, encoding="utf-8")
        print(f"wrote {p['file']}")


if __name__ == "__main__":
    main()
