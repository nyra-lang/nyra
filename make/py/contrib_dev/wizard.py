"""Interactive prompts shared by contributor recipes."""
from __future__ import annotations

import json
from pathlib import Path

from builtin_dev.spec import ArgSpec

from .spec import (
    CliKind,
    CliSpec,
    ConformanceMode,
    ConformanceSpec,
    PkgSpec,
    StdlibFnSpec,
    TestExampleSpec,
)
from builtin_dev.spec import NyraType


def prompt(msg: str, default: str = "") -> str:
    suffix = f" [{default}]" if default else ""
    while True:
        raw = input(f"{msg}{suffix}: ").strip()
        if raw:
            return raw
        if default:
            return default
        print("  (required — press Enter after typing a value)")


def prompt_yes_no(msg: str, default: bool = False) -> bool:
    hint = "Y/n" if default else "y/N"
    raw = input(f"{msg} ({hint}): ").strip().lower()
    if not raw:
        return default
    return raw in ("y", "yes", "1", "true")


def prompt_choice(msg: str, choices: dict[str, str]) -> str:
    print(msg)
    for key, label in choices.items():
        print(f"  {key}. {label}")
    valid = set(choices)
    while True:
        raw = input("Choice: ").strip()
        if raw in valid:
            return raw
        print(f"  Enter one of: {', '.join(sorted(valid))}")


def parse_args(raw: str) -> list[ArgSpec]:
    if not raw.strip():
        return []
    parts = [p.strip() for p in raw.split(",") if p.strip()]
    return [ArgSpec.parse(p) for p in parts]


def parse_returns(raw: str) -> NyraType:
    raw = raw.strip().lower() or "void"
    aliases = {"int": "i32", "integer": "i32", "str": "string", "void": "void", "none": "void"}
    raw = aliases.get(raw, raw)
    if raw == "void":
        return NyraType.VOID
    return NyraType(raw)


def load_json_config(path: str) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def run_stdlib_pure_wizard() -> StdlibFnSpec:
    print("\n── Stdlib Pure Function (Pattern A) ──")
    print("Adds a Nyra `fn` in stdlib/ — no new C unless you wrap an extern.\n")
    ny_module = prompt("Stdlib module path", "json/mod.ny")
    fn_name = prompt("Function name", "decode_example")
    args_raw = prompt("Arguments (comma-separated name:type, or empty)", "json:string, key:string")
    returns = parse_returns(prompt("Return type", "i32"))
    wrap = prompt("Wrap existing extern fn (name only, or empty)", "")
    body = ""
    if not wrap:
        if prompt_yes_no("Provide custom fn body now?", False):
            print("  Enter body lines; finish with empty line:")
            lines = []
            while True:
                line = input("  > ")
                if not line:
                    break
                lines.append(line)
            body = "\n".join(lines) if lines else None
    return StdlibFnSpec(
        fn_name=fn_name,
        args=parse_args(args_raw),
        returns=returns,
        ny_module=ny_module,
        wrap_extern=wrap or None,
        pure_body=body or None,
    )


def run_stdlib_extern_wizard() -> StdlibFnSpec:
    print("\n── Stdlib Extern + C (Pattern B) ──")
    print("Wires extern fn, C runtime stub, and runtime_map.rs.\n")
    ny_module = prompt("Stdlib module path", "json/mod.ny")
    fn_name = prompt("Function name", "json_get_example")
    args_raw = prompt("Arguments", "json:string, key:string")
    returns = parse_returns(prompt("Return type", "i32"))
    rt_module = prompt("C runtime file", "rt_json.c")
    stable = prompt_yes_no("Add to stable ABI manifest?", False)
    since = "1.0.0"
    if stable:
        since = prompt("ABI since version", "1.0.0")
    return StdlibFnSpec(
        fn_name=fn_name,
        args=parse_args(args_raw),
        returns=returns,
        ny_module=ny_module,
        rt_module=rt_module,
        stable_abi=stable,
        abi_since=since,
    )


def run_test_example_wizard() -> TestExampleSpec:
    print("\n── Test + Example Pair ──")
    print("Creates tests/nyra/*_test.ny and examples/<topic>/ pair.\n")
    name = prompt("Feature name (snake_case)", "my_feature")
    topic = prompt("Example topic folder under examples/", "syntax")
    import_path = prompt("Optional stdlib import path", "")
    return TestExampleSpec(
        name=name,
        example_topic=topic,
        with_typed=True,
        use_testing=True,
        import_path=import_path or None,
    )


def run_pkg_wizard() -> PkgSpec:
    print("\n── NyraPkg Package ──")
    print("Scaffolds examples/packages/<name>/ (NyraPkg pattern).\n")
    name = prompt("Package name", "ny-example")
    version = prompt("Version", "0.1.0")
    link = prompt("Native link library (e.g. sqlite3, or empty)", "")
    return PkgSpec(name=name, version=version, link_lib=link or None)


def run_cli_wizard() -> CliSpec:
    print("\n── CLI Command / Flag ──")
    print("Generates scaffold under docs/contrib_scaffold/ — wire manually.\n")
    kind_raw = prompt_choice(
        "CLI kind:",
        {"1": "Subcommand", "2": "Global flag on build/run"},
    )
    kind = CliKind.SUBCOMMAND if kind_raw == "1" else CliKind.FLAG
    name = prompt("Name (snake_case)", "my_cmd")
    desc = prompt("Short description", "TODO: describe this command")
    return CliSpec(name=name, kind=kind, description=desc)


def run_conformance_wizard() -> ConformanceSpec:
    print("\n── Conformance Test ──")
    print("Creates tests/conformance/pass/ or fail/ contract test.\n")
    mode_raw = prompt_choice("Mode:", {"1": "pass (nyra test)", "2": "fail (nyra check must error)"})
    mode = ConformanceMode.PASS if mode_raw == "1" else ConformanceMode.FAIL
    area = prompt("Area subdirectory", "edge")
    name = prompt("Test name (snake_case)", "my_contract")
    desc = prompt("Contract description", "TODO: language contract")
    return ConformanceSpec(name=name, mode=mode, area=area, description=desc)


def run_syntax_wizard() -> "SyntaxSpec":
    from .spec import SyntaxSpec

    print("\n── Syntax / Keyword Scaffold ──")
    print("Checklist + test/example stubs — does NOT edit lexer/parser automatically.\n")
    keyword = prompt("Keyword or syntax name", "yield")
    feature = prompt("Feature slug (snake_case)", "yield_expr")
    desc = prompt("Short description", "TODO: describe syntax semantics")
    needs_expand = prompt_yes_no("Needs expand/ desugar pass?", True)
    needs_comptime = prompt_yes_no("Needs comptime eval?", False)
    return SyntaxSpec(
        keyword=keyword,
        feature_name=feature,
        description=desc,
        needs_expand=needs_expand,
        needs_const_eval=needs_comptime,
    )


def run_remove_wizard(*, title: str = "Remove scaffold") -> str:
    from .discover import list_wired_contribs

    print(f"\n── {title} ──")
    items = list_wired_contribs()
    if not items:
        raise SystemExit("No [contrib-dev:…] scaffolds found in the repo.")
    for i, item in enumerate(items, 1):
        paths = ", ".join(str(p.name) for p in item.paths[:3])
        extra = "…" if len(item.paths) > 3 else ""
        print(f"  {i}. {item.label}  ({paths}{extra})")
    while True:
        raw = input("Select number or paste marker: ").strip()
        if raw.isdigit():
            idx = int(raw) - 1
            if 0 <= idx < len(items):
                return items[idx].marker
        for item in items:
            if raw == item.marker or raw == item.label:
                return item.marker
        print("  Invalid selection.")


def spec_from_config(recipe: str, data: dict):
    if recipe in ("stdlib-pure", "1"):
        return StdlibFnSpec(
            fn_name=data["fn_name"],
            args=[ArgSpec.parse(a) for a in data.get("args", [])],
            returns=parse_returns(data.get("returns", "void")),
            ny_module=data["ny_module"],
            wrap_extern=data.get("wrap_extern"),
            pure_body=data.get("pure_body"),
        )
    if recipe in ("stdlib-extern", "2"):
        return StdlibFnSpec(
            fn_name=data["fn_name"],
            args=[ArgSpec.parse(a) for a in data.get("args", [])],
            returns=parse_returns(data.get("returns", "void")),
            ny_module=data["ny_module"],
            rt_module=data.get("rt_module"),
            stable_abi=data.get("stable_abi", False),
            abi_since=data.get("abi_since", "1.0.0"),
        )
    if recipe in ("test-example", "4"):
        return TestExampleSpec(
            name=data["name"],
            example_topic=data.get("example_topic", "syntax"),
            with_typed=data.get("with_typed", True),
            use_testing=data.get("use_testing", True),
            import_path=data.get("import_path"),
        )
    if recipe in ("pkg", "5"):
        return PkgSpec(
            name=data["name"],
            version=data.get("version", "0.1.0"),
            module_name=data.get("module_name"),
            link_lib=data.get("link_lib"),
            rt_file=data.get("rt_file"),
        )
    if recipe in ("cli", "6"):
        kind = CliKind(data.get("kind", "subcommand"))
        return CliSpec(name=data["name"], kind=kind, description=data.get("description", "TODO"))
    if recipe in ("conformance", "7"):
        mode = ConformanceMode(data.get("mode", "pass"))
        return ConformanceSpec(
            name=data["name"],
            mode=mode,
            area=data.get("area", "edge"),
            description=data.get("description", "TODO"),
        )
    if recipe in ("syntax-scaffold", "8"):
        from .spec import SyntaxSpec

        return SyntaxSpec(
            keyword=data["keyword"],
            feature_name=data["feature_name"],
            description=data.get("description", "TODO"),
            needs_expand=data.get("needs_expand", True),
            needs_const_eval=data.get("needs_const_eval", False),
        )
    raise ValueError(f"unknown recipe in config: {recipe}")
