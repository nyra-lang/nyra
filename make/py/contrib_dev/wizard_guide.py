"""Step-by-step wizard copy — what/why/tool vs you for each contribute recipe."""
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class WizardStep:
    question: str
    why: str
    tool: str
    you: str
    example: str
    default: str = ""


@dataclass(frozen=True)
class RecipeGuide:
    slug: str
    title: str
    when: str
    steps: tuple[WizardStep, ...]
    tool_files: str
    you_files: str
    verify: str


def print_recipe_intro(guide: RecipeGuide) -> None:
    print("\n" + "═" * 62)
    print(f"  {guide.title.upper()}")
    print("═" * 62)
    print(f"  When to pick this: {guide.when}")
    print("─" * 62)
    print("  Legend for each question:")
    print("    WHY  → why we ask")
    print("    TOOL → what make contribute writes automatically")
    print("    YOU  → what you implement after the tool finishes")
    print("─" * 62)


def print_step(step: WizardStep, *, n: int, total: int) -> None:
    print(f"\n── Step {n}/{total} ──")
    print(f"Q: {step.question}")
    print(f"   WHY  → {step.why}")
    print(f"   TOOL → {step.tool}")
    print(f"   YOU  → {step.you}")
    if step.example:
        print(f"   e.g. {step.example}")


def print_preview(guide: RecipeGuide, *, answers: dict[str, str]) -> None:
    print("\n" + "─" * 62)
    print("  PREVIEW — confirm before writing files")
    print("─" * 62)
    for key, val in answers.items():
        if val:
            print(f"    {key}: {val}")
    print(f"\n  TOOL will create/edit:\n{guide.tool_files}")
    print(f"\n  YOU will implement:\n{guide.you_files}")
    print(f"\n  Verify with:\n    {guide.verify}")
    print("─" * 62)


GUIDES: dict[str, RecipeGuide] = {
    "stdlib-pure": RecipeGuide(
        slug="stdlib-pure",
        title="Stdlib Pure Function (Pattern A)",
        when="Nyra-only wrapper or helper in stdlib — no new C runtime.",
        steps=(
            WizardStep(
                "Stdlib module path",
                "Chooses which stdlib file gets your new fn.",
                "Appends a marked fn block to stdlib/<path>.",
                "Implement the fn body inside the marked block.",
                "json/mod.ny",
                "json/mod.ny",
            ),
            WizardStep(
                "Function name",
                "Public Nyra API name (auto-prelude if in stdlib).",
                "Uses this name in fn, tests, and examples.",
                "Keep the name stable — users may call it without import.",
                "decode_user_id",
                "decode_example",
            ),
            WizardStep(
                "Arguments (name:type, comma-separated)",
                "Nyra parameter list for typecheck and docs.",
                "Generates fn signature in stdlib + test imports.",
                "Use in tests to cover edge cases.",
                "json:string, key:string",
                "json:string, key:string",
            ),
            WizardStep(
                "Return type",
                "What the fn returns (i32, string, void, …).",
                "Generates matching fn signature.",
                "Return correct values; fix test expectations.",
                "i32",
                "i32",
            ),
            WizardStep(
                "Wrap existing extern fn (name only, or empty)",
                "If set, tool generates return extern_fn(args) wrapper.",
                "No C changes — reuses existing runtime symbol.",
                "Leave empty if you write custom Nyra logic.",
                "json_get_i32",
                "",
            ),
        ),
        tool_files="""    • stdlib/<module>.ny          — fn stub [contrib-dev:…]
    • tests/nyra/<fn>_test.ny     — zero-types test
    • tests/nyra/<fn>_test.typed.ny
    • examples/<topic>/<fn>.ny    — runnable demo (+ .typed.ny)""",
        you_files="""    • stdlib/<module>.ny          — replace TODO / finish wrapper body
    • tests/nyra/<fn>_test.ny     — real assertions (both .ny files)""",
        verify="nyra test tests/nyra/<fn>_test.ny",
    ),
    "stdlib-extern": RecipeGuide(
        slug="stdlib-extern",
        title="Stdlib Extern + C (Pattern B)",
        when="New C-backed API (I/O, JSON field, crypto, …) in core stdlib.",
        steps=(
            WizardStep(
                "Stdlib module path",
                "Where extern fn is declared for Nyra callers.",
                "Appends extern fn line to stdlib/<path>.",
                "Add friendly Nyra wrappers nearby if needed.",
                "json/mod.ny",
                "json/mod.ny",
            ),
            WizardStep(
                "Function name",
                "C symbol name and Nyra extern fn name.",
                "Registers in runtime_map.rs for linking.",
                "Implement logic in C stub with same name.",
                "json_get_f64",
                "json_get_example",
            ),
            WizardStep(
                "Arguments",
                "Maps to C parameter types in the stub.",
                "Generates extern fn + C signature.",
                "Handle NULL/empty strings safely in C.",
                "json:string, key:string",
                "json:string, key:string",
            ),
            WizardStep(
                "Return type",
                "C return type (char* for string, int for i32, …).",
                "Generates extern + C stub with safe default return.",
                "Replace stub return with real implementation.",
                "f64",
                "i32",
            ),
            WizardStep(
                "C runtime file",
                "Which stdlib/rt/*.c file holds implementation.",
                "Inserts C stub in stdlib/rt/<file>.",
                "Open file, search [contrib-dev:…], implement.",
                "rt_json.c",
                "rt_json.c",
            ),
            WizardStep(
                "Add to stable ABI manifest?",
                "Public C ABI for FFI/embed users.",
                "Appends docs/abi-manifest.toml entry.",
                "Run make gen-abi-header && make gen-bindings-doc.",
                "n (no) for internal/experimental",
                "n",
            ),
        ),
        tool_files="""    • stdlib/<module>.ny                    — extern fn
    • stdlib/rt/<rt>.c                      — C stub
    • compiler/codegen/src/runtime_map.rs   — link symbol
    • docs/abi-manifest.toml                — (optional) stable ABI
    • tests/nyra/<fn>_test.ny (+ .typed.ny)
    • examples/builtins/<topic>/<fn>.ny""",
        you_files="""    • stdlib/rt/<rt>.c                      — C implementation
    • tests/nyra/<fn>_test.ny               — fix expected values""",
        verify="make install-dev && nyra test tests/nyra/<fn>_test.ny",
    ),
    "test-example": RecipeGuide(
        slug="test-example",
        title="Test + Example Pair",
        when="Any user-visible change needs tests + runnable demo.",
        steps=(
            WizardStep(
                "Feature name (snake_case)",
                "Base name for test and example files.",
                "Creates <name>_test.ny and examples/<topic>/<name>.ny.",
                "Write assertions and demo main().",
                "borrow_ref_deref",
                "my_feature",
            ),
            WizardStep(
                "Example topic folder under examples/",
                "Groups demos by area (syntax, net, …).",
                "Writes examples/<topic>/<name>.ny (+ .typed.ny).",
                "Keep demo small and focused.",
                "syntax",
                "syntax",
            ),
            WizardStep(
                "Optional stdlib import path",
                "Pre-import module under test in generated files.",
                "Adds import line to test + example.",
                "Use APIs you are testing.",
                "stdlib/testing.ny",
                "",
            ),
        ),
        tool_files="""    • tests/nyra/<name>_test.ny (+ .typed.ny)
    • examples/<topic>/<name>.ny (+ .typed.ny)""",
        you_files="""    • Same four files — replace TODO / assert_eq(1,1) placeholders""",
        verify="nyra test tests/nyra/<name>_test.ny && nyra run examples/<topic>/<name>.ny",
    ),
    "pkg": RecipeGuide(
        slug="pkg",
        title="NyraPkg Package",
        when="Community package with semver (driver, library) — lives in examples/packages/.",
        steps=(
            WizardStep(
                "Package name",
                "Folder name and pkg import path (ny-foo).",
                "Creates examples/packages/<name>/ layout.",
                "Implement API; publish or use via nyrapkg.",
                "ny-redis",
                "ny-example",
            ),
            WizardStep(
                "Version",
                "Semver in nyra.mod for lockfiles.",
                "Writes version line in nyra.mod.",
                "Bump on breaking changes.",
                "0.1.0",
                "0.1.0",
            ),
            WizardStep(
                "Native link library (or empty)",
                "If set, adds link + rt/*.c shim.",
                "Creates rt/<module>.c stub when link_lib set.",
                "Implement C shims; document in README.",
                "sqlite3",
                "",
            ),
        ),
        tool_files="""    • examples/packages/<name>/nyra.mod
    • examples/packages/<name>/<module>.ny
    • examples/packages/<name>/main.ny
    • examples/packages/<name>/README.md
    • examples/packages/<name>/rt/*.c     — if link_lib set""",
        you_files="""    • <module>.ny — real extern fn + wrappers
    • rt/*.c      — C implementation (if any)
    • main.ny     — smoke test""",
        verify="cd examples/packages/<name> && nyra run main.ny",
    ),
    "cli": RecipeGuide(
        slug="cli",
        title="CLI Command / Flag",
        when="New nyra subcommand or build/run flag.",
        steps=(
            WizardStep(
                "CLI kind (1=subcommand, 2=flag)",
                "Subcommand = nyra foo; flag = nyra build --foo.",
                "Generates matching args_snippet.rs template.",
                "Copy snippet into cli/src/app/args.rs manually.",
                "1",
                "1",
            ),
            WizardStep(
                "Name (snake_case)",
                "Rust module and CLI identifier.",
                "Creates docs/contrib_scaffold/cli_<name>/.",
                "Move command.rs → cli/src/commands/<name>.rs.",
                "fmt_check",
                "my_cmd",
            ),
            WizardStep(
                "Short description",
                "Shows in --help text.",
                "Embeds in clap #[arg] / subcommand doc.",
                "Write clear user-facing help.",
                "Deep format validation for projects",
                "TODO: describe this command",
            ),
        ),
        tool_files="""    • docs/contrib_scaffold/cli_<name>/command.rs
    • docs/contrib_scaffold/cli_<name>/args_snippet.rs
    • docs/contrib_scaffold/cli_<name>/README.md""",
        you_files="""    • cli/src/app/args.rs       — paste args_snippet
    • cli/src/commands/<name>.rs — implement run()
    • cli/src/commands/mod.rs   — pub mod
    • cli/src/app/session.rs    — dispatch match arm""",
        verify="cargo test -p cli && make smoke-cli",
    ),
    "conformance": RecipeGuide(
        slug="conformance",
        title="Conformance Test",
        when="Stable language contract (must pass or must fail compile).",
        steps=(
            WizardStep(
                "Mode (1=pass, 2=fail)",
                "pass = nyra test; fail = nyra check must error.",
                "Creates pass/ or fail/ test file.",
                "Write real contract code.",
                "1",
                "1",
            ),
            WizardStep(
                "Area subdirectory",
                "Groups tests (strings, borrow, edge, …).",
                "Path: tests/conformance/<pass|fail>/<area>/.",
                "Pick existing area when possible.",
                "strings",
                "edge",
            ),
            WizardStep(
                "Test name (snake_case)",
                "File name without .ny.",
                "Creates <name>.ny with conf_* test or main.",
                "Replace placeholder assertions.",
                "string_concat",
                "my_contract",
            ),
            WizardStep(
                "Contract description",
                "Documents what the language guarantees.",
                "Comment in generated file.",
                "Match description in test code.",
                "String concat preserves both operands",
                "TODO: language contract",
            ),
        ),
        tool_files="""    • tests/conformance/pass/<area>/<name>.ny
      — or tests/conformance/fail/<area>/<name>.ny""",
        you_files="""    • Same file — real test fn or failing main()""",
        verify="nyra test tests/conformance/pass/…  OR  nyra check tests/conformance/fail/…",
    ),
    "syntax-scaffold": RecipeGuide(
        slug="syntax-scaffold",
        title="Syntax / Keyword Scaffold",
        when="New keyword or syntax — checklist only (no auto lexer/parser edits).",
        steps=(
            WizardStep(
                "Keyword or syntax name",
                "Token/keyword users will type.",
                "Referenced in CHECKLIST.md + grammar_snippet.json.",
                "Add token in compiler/lexer/.",
                "await",
                "yield",
            ),
            WizardStep(
                "Feature slug (snake_case)",
                "File names for tests/examples/scaffold dir.",
                "Creates syntax_<slug>/ checklist + tests.",
                "Follow checklist pipeline order.",
                "async_select",
                "yield_expr",
            ),
            WizardStep(
                "Short description",
                "What the syntax means semantically.",
                "Written into CHECKLIST.md header.",
                "Implement semantics in typecheck/codegen.",
                "Select among multiple async branches",
                "TODO: describe syntax semantics",
            ),
            WizardStep(
                "Needs expand/ desugar pass?",
                "Sugar before typecheck (like ?? or async).",
                "Checklist step 4 on/off.",
                "Add/adjust compiler/expand/ pass if yes.",
                "y",
                "y",
            ),
            WizardStep(
                "Needs comptime eval?",
                "Compile-time execution of new syntax.",
                "Checklist step 7 on/off.",
                "Touch const_eval/ if yes.",
                "n",
                "n",
            ),
        ),
        tool_files="""    • docs/contrib_scaffold/syntax_<slug>/CHECKLIST.md
    • docs/contrib_scaffold/syntax_<slug>/grammar_snippet.json
    • tests/nyra/<slug>_syntax_test.ny (+ .typed.ny)
    • examples/syntax/<slug>.ny (+ .typed.ny)""",
        you_files="""    • compiler/lexer → parser → ast → expand? → typecheck → codegen
    • grammar/nyra.tmLanguage.json
    • tests + examples from scaffold""",
        verify="make install-dev && cargo test -p compiler && nyra test tests/nyra/<slug>_syntax_test.ny",
    ),
}


def monitor_sections(guide: RecipeGuide) -> tuple[list[str], list[str], list[str]]:
    """Return (why_lines, tool_why, you_why) for monitor footer."""
    why = [f"This recipe: {guide.when}"]
    tool = [
        "The tool only creates marked stubs and wiring — safe to re-run or remove via make contribute-remove.",
        f"Files touched — see TOOL block above. Marker: [contrib-dev:…]",
    ]
    you = [
        "You own all TODO logic, assertions, and compiler changes (syntax/cli).",
        f"Primary edit locations:\n{guide.you_files}",
        f"Verify: {guide.verify}",
    ]
    return why, tool, you
