//! Static explanations for stable diagnostic codes (`nyra explain E003`).

/// One entry returned by [`explain`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainEntry {
    pub code: &'static str,
    pub title: &'static str,
    pub explanation: &'static str,
    pub example_bad: Option<&'static str>,
    pub example_good: Option<&'static str>,
}

const ENTRIES: &[ExplainEntry] = &[
    ExplainEntry {
        code: "E001",
        title: "import not found",
        explanation: "An `import \"path\"` could not be resolved to a file on disk or in the package cache.",
        example_bad: Some("import \"missing/module.ny\""),
        example_good: Some("import \"stdlib/io.ny\"  // or a path relative to the project root"),
    },
    ExplainEntry {
        code: "E002",
        title: "undefined name",
        explanation: "A variable, function, or type name is used but was never declared in scope.",
        example_bad: Some("print(unknown)"),
        example_good: Some("fn greet() { print(\"hi\") }\nfn main() { greet() }"),
    },
    ExplainEntry {
        code: "E003",
        title: "type mismatch",
        explanation: "An expression's type does not match what the context expects (parameter, return, assignment, or operator).",
        example_bad: Some("fn f(x: i32) {}\nfn main() { f(\"text\") }"),
        example_good: Some("fn f(x: i32) {}\nfn main() { f(42) }"),
    },
    ExplainEntry {
        code: "E004",
        title: "cannot infer type",
        explanation: "The compiler could not infer a type for a binding or expression. Add an explicit annotation.",
        example_bad: Some("let x = []"),
        example_good: Some("let x: [i32] = []\n// or: let x = [1, 2, 3]"),
    },
    ExplainEntry {
        code: "E005",
        title: "unknown struct",
        explanation: "A struct literal or type name refers to a struct that is not defined or not in scope.",
        example_bad: Some("let p = Person { name: \"Ada\" }"),
        example_good: Some("struct Person { name: string }\nfn main() { let p = Person { name: \"Ada\" } }"),
    },
    ExplainEntry {
        code: "E006",
        title: "immutable assignment",
        explanation: "A binding declared with `let` cannot be reassigned. Use `var` for mutable bindings.",
        example_bad: Some("let x = 1\nx = 2"),
        example_good: Some("var x = 1\nx = 2"),
    },
    ExplainEntry {
        code: "E007",
        title: "wrong arity",
        explanation: "A function was called with the wrong number of arguments.",
        example_bad: Some("fn add(a: i32, b: i32) -> i32 { a + b }\nfn main() { add(1) }"),
        example_good: Some("fn add(a: i32, b: i32) -> i32 { a + b }\nfn main() { add(1, 2) }"),
    },
    ExplainEntry {
        code: "E008",
        title: "wrong argument type",
        explanation: "A specific argument position has the wrong type, even when the call arity is correct.",
        example_bad: Some("fn log(n: i32) {}\nfn main() { log(true) }"),
        example_good: Some("fn log(n: i32) {}\nfn main() { log(42) }"),
    },
    ExplainEntry {
        code: "E009",
        title: "invalid assignment target",
        explanation: "The left-hand side of an assignment is not a valid l-value (variable, field, or index).",
        example_bad: Some("1 = x"),
        example_good: Some("var x = 0\nx = 1"),
    },
    ExplainEntry {
        code: "E010",
        title: "borrow while assigned",
        explanation: "A mutable borrow conflicts with an existing assignment or mutable borrow of the same binding.",
        example_bad: Some("var x = 1\nlet a = &mut x\nx = 2"),
        example_good: Some("var x = 1\nx = 2\nlet a = &x"),
    },
    ExplainEntry {
        code: "E011",
        title: "use while borrowed",
        explanation: "A value is used while an active borrow still holds a reference to it.",
        example_bad: Some("let x = 1\nlet r = &x\nprint(x)"),
        example_good: Some("let x = 1\nlet r = &x\nprint(r)"),
    },
    ExplainEntry {
        code: "E012",
        title: "use after move",
        explanation: "A move-type value was moved (into a call, assignment, or closure) and then used again.",
        example_bad: Some("fn take(s: string) {}\nfn main() {\n    let name = \"Ada\"\n    take(name)\n    print(name)\n}"),
        example_good: Some("fn take(s: string) {}\nfn main() {\n    let name = \"Ada\"\n    take(clone name)\n    print(name)\n}"),
    },
    ExplainEntry {
        code: "E013",
        title: "undefined function",
        explanation: "A function call refers to a name that is not declared or not in scope.",
        example_bad: Some("fn main() { greet() }"),
        example_good: Some("fn greet() {}\nfn main() { greet() }"),
    },
    ExplainEntry {
        code: "E014",
        title: "unknown field",
        explanation: "A field access or struct literal uses a field name that does not exist on the type.",
        example_bad: Some("struct Point { x: i32 }\nfn main() { let p = Point { x: 1, y: 2 } }"),
        example_good: Some("struct Point { x: i32, y: i32 }\nfn main() { let p = Point { x: 1, y: 2 } }"),
    },
    ExplainEntry {
        code: "E015",
        title: "operator type mismatch",
        explanation: "An arithmetic, comparison, logical, or bitwise operator was applied to operands of incompatible types.",
        example_bad: Some("fn main() { let x = \"a\" + \"b\" }"),
        example_good: Some("fn main() { let x = 1 + 2 }"),
    },
    ExplainEntry {
        code: "E016",
        title: "unsafe required",
        explanation: "The operation requires an `unsafe` block because it can violate memory safety.",
        example_bad: Some("fn main() { let p = alloc(4); *p = 1 }"),
        example_good: Some("unsafe fn main() { let p = alloc(4); *p = 1 }"),
    },
    ExplainEntry {
        code: "E017",
        title: "not callable",
        explanation: "The callee expression does not have a function type and cannot be called.",
        example_bad: Some("fn main() { let x = 1; x() }"),
        example_good: Some("fn f() {}\nfn main() { f() }"),
    },
    ExplainEntry {
        code: "E018",
        title: "unknown method",
        explanation: "A method call uses a name that is not defined on the receiver's type or trait impl.",
        example_bad: Some("fn main() { let s = \"hi\"; s.unknown() }"),
        example_good: Some("fn main() { let s = \"hi\"; print(s) }"),
    },
    ExplainEntry {
        code: "E019",
        title: "bool condition required",
        explanation: "An `if`, `while`, or match guard condition must be `bool`.",
        example_bad: Some("if 1 { print(1) }"),
        example_good: Some("if true { print(1) }"),
    },
    ExplainEntry {
        code: "E020",
        title: "invalid control flow",
        explanation: "`break` and `continue` may only appear inside `while` or `for` loops.",
        example_bad: Some("fn main() { break }"),
        example_good: Some("fn main() { while true { break } }"),
    },
    ExplainEntry {
        code: "E021",
        title: "platform unsupported",
        explanation: "The feature is not available on this target (e.g. `no_std` or wasm32).",
        example_bad: Some("#![no_std]\nfn main() { print(1) }"),
        example_good: Some("fn main() { print(1) }"),
    },
    ExplainEntry {
        code: "E022",
        title: "return type mismatch",
        explanation: "A `return` expression does not match the function's declared or inferred return type.",
        example_bad: Some("fn f() -> i32 { return \"x\" }"),
        example_good: Some("fn f() -> i32 { return 1 }"),
    },
    ExplainEntry {
        code: "E023",
        title: "match error",
        explanation: "A `match` has an unknown variant, non-exhaustive arms, or a pattern that does not fit the scrutinee type.",
        example_bad: Some("enum E { A }\nfn f(x: E) { match x { E::B => {} } }"),
        example_good: Some("enum E { A }\nfn f(x: E) { match x { E::A => {} } }"),
    },
    ExplainEntry {
        code: "E024",
        title: "for-in error",
        explanation: "A `for` loop iterates over an incompatible type or combines unsupported modifiers.",
        example_bad: Some("fn main() { for x in 1.5 { print(x) } }"),
        example_good: Some("fn main() { for x in 0..3 { print(x) } }"),
    },
    ExplainEntry {
        code: "E025",
        title: "destructure error",
        explanation: "Tuple destructuring syntax is invalid or does not match the value shape.",
        example_bad: Some("fn main() { let mut (a, b) = (1, 2) }"),
        example_good: Some("fn main() { let (a, b) = (1, 2) }"),
    },
    ExplainEntry {
        code: "E026",
        title: "block must produce value",
        explanation: "A block used as an expression must end with a value-producing expression or `return`.",
        example_bad: Some("fn main() { let x = { let a = 1 } }"),
        example_good: Some("fn main() { let x = { let a = 1; a } }"),
    },
    ExplainEntry {
        code: "E027",
        title: "integer out of range",
        explanation: "An integer literal does not fit in the annotated or inferred integer type.",
        example_bad: Some("fn main() { let x: u8 = 300 }"),
        example_good: Some("fn main() { let x: u8 = 255 }"),
    },
    ExplainEntry {
        code: "E028",
        title: "borrow active",
        explanation: "A closure or concurrent construct was created while references to local variables are still active.",
        example_bad: None,
        example_good: None,
    },
    ExplainEntry {
        code: "E029",
        title: "move while borrowed",
        explanation: "A value was moved while an active borrow still references it.",
        example_bad: None,
        example_good: None,
    },
    ExplainEntry {
        code: "E030",
        title: "manual free",
        explanation: "Calling `free` on an owned Nyra value risks double-free because Nyra drops values automatically.",
        example_bad: Some("fn main() { let s = \"hi\"; free(s) }"),
        example_good: Some("fn main() { let s = \"hi\"; print(s) }"),
    },
    ExplainEntry {
        code: "E031",
        title: "array or indexing error",
        explanation: "An array operation or index expression is invalid: wrong index type, heterogeneous elements, unsupported method, or incompatible spread.",
        example_bad: Some("fn main() { let xs = [1, \"two\"]; print(xs[0]) }"),
        example_good: Some("fn main() { let xs = [1, 2]; print(xs[0]) }"),
    },
    ExplainEntry {
        code: "E032",
        title: "enum error",
        explanation: "An enum constructor, variant, or pattern does not match the declared enum definition.",
        example_bad: None,
        example_good: None,
    },
    ExplainEntry {
        code: "E033",
        title: "invalid cast or trait conversion",
        explanation: "A cast, `await`, or trait-object conversion is not allowed for the given types or bounds.",
        example_bad: Some("fn main() { let n: i32 = 1; let s: string = n as string }"),
        example_good: Some("fn main() { let n: i32 = 1; let s: string = \"1\" }"),
    },
    ExplainEntry {
        code: "E034",
        title: "FFI boundary error",
        explanation: "An exported or extern function uses types or generics that are not allowed at the FFI boundary.",
        example_bad: Some("export fn take(s: string) -> i32 { return 0 }  // missing repr(C) struct"),
        example_good: Some("struct Point repr(C) { x: i32 }\nexport fn get_x(p: Point) -> i32 { return p.x }"),
    },
    ExplainEntry {
        code: "E035",
        title: "lifetime error",
        explanation: "A reference outlives its source, lifetimes are undeclared or ambiguous, or a closure captures a reference illegally.",
        example_bad: Some("fn bad() -> &string { let s = \"hi\"; return &s }"),
        example_good: Some("fn ok() -> string { let s = \"hi\"; return s }"),
    },
    ExplainEntry {
        code: "E036",
        title: "Send or Sync error",
        explanation: "A value crosses a thread boundary or is shared across threads without satisfying `Send` or `Sync` requirements.",
        example_bad: None,
        example_good: None,
    },
    ExplainEntry {
        code: "E037",
        title: "parallel for error",
        explanation: "`parallel for` iterations must be independent: no `break`/`continue` and no mutation of outer variables.",
        example_bad: Some("fn main() { let mut sum = 0; parallel for i in 0..3 { sum = sum + i } }"),
        example_good: Some("fn main() { parallel for i in 0..3 { print(i) } }"),
    },
    ExplainEntry {
        code: "E038",
        title: "comptime evaluation error",
        explanation: "A `comptime` function or block violated compile-time evaluation rules (e.g. I/O, spawn, or unsupported operations).",
        example_bad: Some("comptime fn bad() { print(1) }"),
        example_good: Some("comptime fn ok() -> i32 { return 1 + 2 }"),
    },
    ExplainEntry {
        code: "W001",
        title: "extended tier feature",
        explanation: "The code uses an Extended-tier feature (async, traits, spawn, defer, etc.) while `--deny-extended` is active.",
        example_bad: Some("async fn fetch() { }  // with --deny-extended"),
        example_good: Some("fn fetch() { }  // Core tier, or remove --deny-extended"),
    },
    ExplainEntry {
        code: "W002",
        title: "unused import",
        explanation: "An import binding is never used. Remove it or run `nyra pkg prune`.",
        example_bad: Some("import \"stdlib/io.ny\"\nfn main() { print(1) }"),
        example_good: Some("fn main() { print(1) }"),
    },
    ExplainEntry {
        code: "W003",
        title: "unused variable",
        explanation: "A local binding is never read. Prefix with `_` or remove it.",
        example_bad: Some("fn main() {\n    let unused = 42\n    print(1)\n}"),
        example_good: Some("fn main() {\n    let _unused = 42\n    print(1)\n}"),
    },
    ExplainEntry {
        code: "P001",
        title: "anonymous object literal",
        explanation: "Nyra does not support JavaScript-style `{ key: value }` object literals. Declare a struct first.",
        example_bad: Some("let p = { name: \"Ada\" }"),
        example_good: Some("struct Person { name: string }\nlet p = Person { name: \"Ada\" }"),
    },
    ExplainEntry {
        code: "P002",
        title: "standalone block expression",
        explanation: "A bare `{ ... }` block was used where an expression was expected in an invalid context.",
        example_bad: Some("let x = { 1, 2 }"),
        example_good: Some("let x = { 1 + 2 }"),
    },
    ExplainEntry {
        code: "P003",
        title: "expected parameter name",
        explanation: "A function parameter list is missing a parameter name or has invalid syntax.",
        example_bad: Some("fn f(i32) {}"),
        example_good: Some("fn f(x: i32) {}"),
    },
    ExplainEntry {
        code: "P004",
        title: "expected `)` after parameters",
        explanation: "The parameter list of a function is not closed with `)`.",
        example_bad: Some("fn f(a: i32 { }"),
        example_good: Some("fn f(a: i32) { }"),
    },
    ExplainEntry {
        code: "P005",
        title: "expected `(` after function name",
        explanation: "A function declaration is missing `(` after its name.",
        example_bad: Some("fn main { }"),
        example_good: Some("fn main() { }"),
    },
    ExplainEntry {
        code: "P006",
        title: "invalid expression",
        explanation: "The parser could not build a valid expression. Often a cascade from an earlier syntax error.",
        example_bad: Some("let x = \nlet y = 1"),
        example_good: Some("let x = 1\nlet y = 2"),
    },
    ExplainEntry {
        code: "P007",
        title: "unexpected `{` in expression",
        explanation: "A `{` appeared where an expression was expected. Check struct literals vs blocks.",
        example_bad: Some("let x = if true { 1 } else { }"),
        example_good: Some("let x = if true { 1 } else { 0 }"),
    },
    ExplainEntry {
        code: "P008",
        title: "expected `}` to close block",
        explanation: "A block, function, or struct body is missing a closing `}`.",
        example_bad: Some("fn main() {\n    print(1)"),
        example_good: Some("fn main() {\n    print(1)\n}"),
    },
    ExplainEntry {
        code: "P009",
        title: "expected `{` to start block",
        explanation: "A function or control-flow construct is missing `{` to open its body.",
        example_bad: Some("fn main() print(1)"),
        example_good: Some("fn main() { print(1) }"),
    },
    ExplainEntry {
        code: "P010",
        title: "expected top-level item",
        explanation: "The parser found tokens that do not form a valid top-level declaration.",
        example_bad: Some("x = 1"),
        example_good: Some("fn main() { var x = 1 }"),
    },
    ExplainEntry {
        code: "P011",
        title: "expected `)` after arguments",
        explanation: "A function call or grouping is missing a closing `)`.",
        example_bad: Some("print(1"),
        example_good: Some("print(1)"),
    },
    ExplainEntry {
        code: "P012",
        title: "expected `=>` in arrow function",
        explanation: "An arrow function parameter list must be followed by `=>`.",
        example_bad: Some("let f = (x) { x + 1 }"),
        example_good: Some("let f = (x) => { x + 1 }"),
    },
    ExplainEntry {
        code: "P013",
        title: "expected `)`",
        explanation: "A parenthesized group or call is missing a closing `)`.",
        example_bad: Some("if (true { }"),
        example_good: Some("if true { }"),
    },
    ExplainEntry {
        code: "P014",
        title: "expected `]`",
        explanation: "An array literal or index expression is missing a closing `]`.",
        example_bad: Some("let a = [1, 2"),
        example_good: Some("let a = [1, 2]"),
    },
    ExplainEntry {
        code: "P015",
        title: "prefer `max` in `parallel(...)`",
        explanation: "The `cores`, `max_threads`, and `max_workers` keys are accepted but deprecated; use `max = N` to cap workers.",
        example_bad: Some("parallel(max_threads = 4) for i in 0..10 { }"),
        example_good: Some("parallel(max = 4) for i in 0..10 { }"),
    },
    ExplainEntry {
        code: "P099",
        title: "unexpected syntax",
        explanation: "An unclassified parser error. Fix earlier errors first; this may be a cascade.",
        example_bad: None,
        example_good: None,
    },
    ExplainEntry {
        code: "L001",
        title: "invalid token",
        explanation: "The lexer encountered a character or token sequence that is not valid Nyra syntax.",
        example_bad: Some("let x = @invalid"),
        example_good: Some("let x = 42"),
    },
    ExplainEntry {
        code: "L002",
        title: "unclosed literal or comment",
        explanation: "A string, character literal, or block comment was opened but not closed.",
        example_bad: Some("/* comment without end"),
        example_good: Some("/* comment */"),
    },
    ExplainEntry {
        code: "L003",
        title: "invalid numeric literal",
        explanation: "A number literal has invalid syntax, separators, or overflows.",
        example_bad: Some("let x = 0x"),
        example_good: Some("let x = 0xFF"),
    },
    ExplainEntry {
        code: "L004",
        title: "invalid attribute",
        explanation: "An attribute or macro invocation has invalid syntax.",
        example_bad: Some("#[derive]"),
        example_good: Some("#[derive(Clone)]"),
    },
];

/// Look up a stable diagnostic code (case-insensitive).
pub fn explain(code: &str) -> Option<&'static ExplainEntry> {
    let upper = code.to_ascii_uppercase();
    ENTRIES.iter().find(|e| e.code == upper)
}

/// All known stable diagnostic codes, sorted.
pub fn list_codes() -> Vec<&'static str> {
    ENTRIES.iter().map(|e| e.code).collect()
}

/// Format an explanation for terminal output.
pub fn format_explain(entry: &ExplainEntry) -> String {
    let mut out = format!("{} — {}\n\n{}\n", entry.code, entry.title, entry.explanation);
    if let Some(bad) = entry.example_bad {
        out.push_str("\nExample (incorrect):\n");
        out.push_str(bad);
        out.push('\n');
    }
    if let Some(good) = entry.example_good {
        out.push_str("\nExample (correct):\n");
        out.push_str(good);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explain_known_code() {
        let e = explain("e003").expect("E003");
        assert_eq!(e.code, "E003");
        assert!(e.title.contains("type mismatch"));
    }

    #[test]
    fn explain_unknown_code() {
        assert!(explain("E999").is_none());
    }

    #[test]
    fn list_codes_non_empty() {
        assert!(list_codes().len() >= 20);
    }
}
