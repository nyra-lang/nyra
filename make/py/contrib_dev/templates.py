from __future__ import annotations

from builtin_dev.spec import ArgSpec, NyraType
from builtin_dev.templates import c_type, nyra_type_annotation

from .patch import marker_end, marker_start, wrap_scaffold
from .spec import (
    CliKind,
    CliSpec,
    ConformanceMode,
    ConformanceSpec,
    PkgSpec,
    StdlibFnSpec,
    SyntaxSpec,
    TestExampleSpec,
)


def ny_sig_args(args: list[ArgSpec]) -> str:
    return ", ".join(f"{a.name}: {nyra_type_annotation(a.nyra_type)}" for a in args)


def ny_sig(spec: StdlibFnSpec, *, typed: bool = False) -> str:
    ret = "" if spec.returns == NyraType.VOID else f" -> {nyra_type_annotation(spec.returns, ref=False)}"
    return f"fn {spec.fn_name}({ny_sig_args(spec.args)}){ret if typed else ret}"


def extern_line(spec: StdlibFnSpec) -> str:
    ret = "" if spec.returns == NyraType.VOID else f" -> {nyra_type_annotation(spec.returns, ref=False)}"
    return f"extern fn {spec.fn_name}({ny_sig_args(spec.args)}){ret}"


def pure_fn_body(spec: StdlibFnSpec) -> str:
    if spec.pure_body:
        return spec.pure_body
    if spec.wrap_extern:
        call_args = ", ".join(a.name for a in spec.args)
        if spec.returns == NyraType.VOID:
            return f"    {spec.wrap_extern}({call_args})"
        return f"    return {spec.wrap_extern}({call_args})"
    if spec.returns == NyraType.VOID:
        return "    // TODO: implement"
    if spec.returns == NyraType.I32:
        return "    return 0"
    if spec.returns == NyraType.STRING:
        return '    return ""'
    return "    // TODO: implement"


def pure_fn_block(spec: StdlibFnSpec, marker: str) -> str:
    if spec.pure_source:
        return wrap_scaffold(spec.pure_source.rstrip() + "\n", marker)
    ret_ann = "" if spec.returns == NyraType.VOID else f" -> {nyra_type_annotation(spec.returns, ref=False)}"
    lines = [
        marker_start(marker),
        f"fn {spec.fn_name}({ny_sig_args(spec.args)}){ret_ann} {{",
        pure_fn_body(spec),
        "}",
        marker_end(marker),
        "",
    ]
    return "\n".join(lines)


def c_stub(spec: StdlibFnSpec, marker: str) -> str:
    params = [f"{c_type(a.nyra_type)} {a.name}" for a in spec.args]
    ret = c_type(spec.returns, is_return=True)
    sig = ", ".join(params) if params else "void"
    default = "0" if spec.returns in (NyraType.I32, NyraType.BOOL, NyraType.I64) else '""' if spec.returns == NyraType.STRING else ""
    body = "    /* TODO: implement logic — safe default stub. */"
    if spec.returns != NyraType.VOID:
        body += f"\n    return {default};"
    return "\n".join(
        [
            marker_start(marker, lang="c"),
            f"{ret}{spec.fn_name}({sig}) {{",
            body,
            "}",
            marker_end(marker, lang="c"),
            "",
        ]
    )


def runtime_map_line(spec: StdlibFnSpec) -> str:
    assert spec.rt_module
    return f'("{spec.fn_name}", "{spec.rt_module}"),'


def abi_manifest_block(spec: StdlibFnSpec, marker: str) -> str:
    params = [f"{c_type(a.nyra_type)} {a.name}" for a in spec.args]
    ret = c_type(spec.returns, is_return=True)
    sig = ", ".join(params) if params else "void"
    # Keep the manifest a superset of runtime_map (see the
    # `runtime_map_matches_manifest` ABI test). Symbols that did not opt into
    # the stable ABI are recorded as `experimental`, which the test allows while
    # keeping them out of the generated stdlib/nyra_rt.h header.
    tier = "stable" if spec.stable_abi else "experimental"
    return "\n".join(
        [
            marker_start(marker, lang="toml"),
            "[[symbol]]",
            f'name = "{spec.fn_name}"',
            f'c_sig = "{ret}{spec.fn_name}({sig})"',
            f'module = "{spec.rt_module}"',
            f'tier = "{tier}"',
            f'since = "{spec.abi_since}"',
            marker_end(marker, lang="toml"),
            "",
        ]
    )


def test_ny(spec: TestExampleSpec, marker: str) -> str:
    lines = []
    if spec.use_testing:
        lines.append('import "stdlib/testing.ny"')
    if spec.import_path:
        lines.append(f'import "{spec.import_path}"')
    if lines:
        lines.append("")
    lines.extend(
        [
            f"test fn test_{spec.name}() {{",
            "    // TODO: assert behavior",
            "    assert_eq(1, 1)",
            "}",
        ]
    )
    return wrap_scaffold("\n".join(lines), marker)


def test_typed_ny(spec: TestExampleSpec, marker: str) -> str:
    lines = []
    if spec.use_testing:
        lines.append('import "stdlib/testing.ny"')
    if spec.import_path:
        lines.append(f'import "{spec.import_path}"')
    if lines:
        lines.append("")
    lines.extend(
        [
            f"test fn test_{spec.name}_typed() {{",
            "    // TODO: assert behavior (explicit types)",
            "    assert_eq(1, 1)",
            "}",
        ]
    )
    return wrap_scaffold("\n".join(lines), marker)


def example_ny(spec: TestExampleSpec, marker: str) -> str:
    body = "\n".join(
        [
            f"// Demo: {spec.name}",
            "",
            "fn main() {",
            f'    print("TODO: demo", "{spec.name}")',
            "}",
        ]
    )
    return wrap_scaffold(body, marker)


def example_typed_ny(spec: TestExampleSpec, marker: str) -> str:
    body = "\n".join(
        [
            f"// Demo: {spec.name} (explicit types)",
            "",
            "fn main() -> void {",
            f'    print("TODO: demo", "{spec.name}")',
            "}",
        ]
    )
    return wrap_scaffold(body, marker)


def pkg_nyra_mod(spec: PkgSpec, marker: str) -> str:
    lines = [
        f"module example.{spec.module_name}",
        "",
        f"version {spec.version}",
        "",
        "require (",
        ")",
        "",
    ]
    if spec.link_lib:
        lines.append(f"link {spec.link_lib}")
        lines.append(f"link-source {spec.rt_file}")
    return wrap_scaffold("\n".join(lines), marker)


def pkg_api_ny(spec: PkgSpec, marker: str) -> str:
    body = "\n".join(
        [
            f"// NyraPkg: {spec.name}",
            "",
            f"extern fn {spec.module_name}_open(path: string) -> i32",
            f"extern fn {spec.module_name}_close(handle: i32) -> void",
        ]
    )
    return wrap_scaffold(body, marker)


def pkg_main_ny(spec: PkgSpec, marker: str) -> str:
    body = "\n".join(
        [
            f'import "./{spec.module_name}.ny"',
            "",
            "fn main() {",
            f'    print("{spec.name} smoke — TODO")',
            "}",
        ]
    )
    return wrap_scaffold(body, marker)


def pkg_rt_c(spec: PkgSpec, marker: str) -> str:
    sym = spec.module_name
    return "\n".join(
        [
            marker_start(marker, lang="c"),
            f"int {sym}_open(const char *path) {{",
            "    (void)path;",
            "    /* TODO: implement */",
            "    return 0;",
            "}",
            "",
            f"void {sym}_close(int handle) {{",
            "    (void)handle;",
            "}",
            marker_end(marker, lang="c"),
            "",
        ]
    )


def pkg_readme(spec: PkgSpec, marker: str) -> str:
    body = "\n".join(
        [
            f"# {spec.name} (NyraPkg scaffold)",
            "",
            "Generated by `make contribute` → NyraPkg Package.",
            "",
            "## Layout",
            "",
            "```",
            f"{spec.name}/",
            "  nyra.mod",
            f"  {spec.module_name}.ny",
            "  main.ny",
            f"  {spec.rt_file}   # if link_lib set",
            "```",
            "",
            "## Next steps",
            "",
            "1. Implement C shims in `rt/`.",
            "2. Add tests: `nyra test main.ny`.",
            "3. Publish: see `docs/nyrapkg-v1.md`.",
        ]
    )
    return wrap_scaffold(body, marker, lang="md")


def cli_readme(spec: CliSpec, marker: str) -> str:
    body = "\n".join(
        [
            f"# CLI scaffold: `{spec.name}` ({spec.kind.value})",
            "",
            "Files generated under `docs/contrib_scaffold/cli_<name>/`.",
            "",
            "## Wire manually",
            "",
            "1. Copy `args_snippet.rs` into `cli/src/app/args.rs` (`Commands` or `OptFlags`).",
            "2. Add `pub(crate) mod <name>;` in `cli/src/commands/mod.rs`.",
            "3. Move `command.rs` → `cli/src/commands/<name>.rs` and implement `run`.",
            "4. Dispatch in `cli/src/app/session.rs`.",
            "5. Run: `cargo test -p cli` · `make smoke-cli`.",
        ]
    )
    return wrap_scaffold(body, marker, lang="md")


def cli_command_stub(spec: CliSpec, marker: str) -> str:
    return "\n".join(
        [
            marker_start(marker, lang="rust"),
            "//! Scaffold — wire into `cli/src/app/args.rs` and `cli/src/app/session.rs`.",
            "",
            f"pub(crate) fn run_{spec.name}() -> i32 {{",
            f'    eprintln!("TODO: implement nyra {spec.name}");',
            "    0",
            "}",
            marker_end(marker, lang="rust"),
            "",
        ]
    )


def cli_args_snippet(spec: CliSpec, marker: str) -> str:
    if spec.kind == CliKind.FLAG:
        return "\n".join(
            [
                marker_start(marker, lang="rust"),
                f"/// {spec.description}",
                f"#[arg(long)]",
                f"pub(crate) {spec.name}: bool,",
                marker_end(marker, lang="rust"),
                "",
            ]
        )
    return "\n".join(
        [
            marker_start(marker, lang="rust"),
            f"/// {spec.description}",
            f"{spec.name.title()} {{",
            "    #[arg(default_value = \".\")]",
            "    file: PathBuf,",
            "},",
            marker_end(marker, lang="rust"),
            "",
        ]
    )


def syntax_checklist(spec: "SyntaxSpec", marker: str) -> str:
    expand_line = "Yes — add/adjust pass in `compiler/expand/`" if spec.needs_expand else "No — skip expand/"
    comptime_line = "Yes — `compiler/const_eval/` + parser/typecheck" if spec.needs_const_eval else "No"
    body = "\n".join(
        [
            f"# Syntax scaffold: `{spec.keyword}` ({spec.feature_name})",
            "",
            spec.description or "TODO: describe the language change.",
            "",
            "## Compiler pipeline checklist",
            "",
            "| Step | Location | Action |",
            "|------|----------|--------|",
            f"| 1 | `compiler/lexer/` | Tokenize `{spec.keyword}` |",
            "| 2 | `compiler/parser/` | Parse new syntax into AST |",
            "| 3 | `compiler/ast/` | Add AST nodes / fields |",
            f"| 4 | `compiler/expand/` | {expand_line} |",
            "| 5 | `compiler/typecheck/` | Type rules + diagnostics |",
            "| 6 | `compiler/codegen/` | LLVM lowering |",
            f"| 7 | `compiler/const_eval/` | {comptime_line} |",
            "| 8 | `grammar/nyra.tmLanguage.json` | Syntax highlighting |",
            "",
            "## Tests & examples",
            "",
            f"- `tests/nyra/{spec.feature_name}_syntax_test.ny` (+ `.typed.ny`)",
            f"- `examples/syntax/{spec.feature_name}.ny` (+ `.typed.ny`)",
            "- Optional: `tests/conformance/pass/…` for stable contract",
            "",
            "## Verify",
            "",
            "```bash",
            "make install-dev",
            f"nyra test tests/nyra/{spec.feature_name}_syntax_test.ny",
            "cargo test -p compiler",
            "make test-preflight",
            "```",
            "",
            "See `docs/architecture.md` and `docs/contributor-map.md`.",
        ]
    )
    return wrap_scaffold(body, marker, lang="md")


def syntax_grammar_hint(spec: "SyntaxSpec", marker: str) -> str:
    body = "\n".join(
        [
            "{",
            f'  "comment": "Add to grammar/nyra.tmLanguage.json — keyword: {spec.keyword}",',
            '  "name": "keyword.control.nyra",',
            f'  "match": "\\\\b{spec.keyword}\\\\b"',
            "}",
        ]
    )
    return wrap_scaffold(body, marker, lang="ny")


def conformance_pass(spec: ConformanceSpec, marker: str) -> str:
    return "\n".join(
        [
            marker_start(marker),
            'import "stdlib/testing.ny"',
            "",
            f"test fn conf_{spec.name}() {{",
            f"    // {spec.description}",
            "    // TODO: assert language contract",
            "    assert_eq(1, 1)",
            "}",
            marker_end(marker),
            "",
        ]
    )


def conformance_fail(spec: ConformanceSpec, marker: str) -> str:
    return "\n".join(
        [
            marker_start(marker),
            f"// {spec.description}",
            "// nyra check must fail (non-zero exit)",
            "",
            "fn main() {",
            "    // TODO: code that must not compile",
            "    let x = 1",
            "    x = 2",
            "}",
            marker_end(marker),
            "",
        ]
    )
