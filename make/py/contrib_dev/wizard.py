"""Interactive prompts — step-by-step monitor style (what/why/tool vs you)."""
from __future__ import annotations

import json
from pathlib import Path

from builtin_dev.spec import ArgSpec, NyraType

from .spec import (
    CliKind,
    CliSpec,
    ConformanceMode,
    ConformanceSpec,
    PkgSpec,
    StdlibFnSpec,
    TestExampleSpec,
)
from .suggestions import Suggestion, suggestions_for
from .wizard_guide import GUIDES, RecipeGuide, print_preview, print_recipe_intro, print_step


def prompt(msg: str, default: str = "", suggestions: list[Suggestion] | None = None) -> str:
    suffix = f" [{default}]" if default else ""
    options = suggestions or []
    while True:
        raw = input(f"\n→ {msg}{suffix}: ").strip()
        if raw:
            if raw.isdigit() and options:
                idx = int(raw) - 1
                if 0 <= idx < len(options):
                    return options[idx].value
                print(f"  (pick 1–{len(options)}, or type your own value)")
                continue
            return raw
        if default:
            return default
        print("  (required — type a value, pick a number, or press Enter for default if shown)")


def prompt_yes_no(msg: str, default: bool = False) -> bool:
    hint = "Y/n" if default else "y/N"
    raw = input(f"\n→ {msg} ({hint}): ").strip().lower()
    if not raw:
        return default
    return raw in ("y", "yes", "1", "true")


def prompt_choice(msg: str, choices: dict[str, str]) -> str:
    print(f"\n→ {msg}")
    for key, label in choices.items():
        print(f"    {key}. {label}")
    valid = set(choices)
    while True:
        raw = input("  Choice: ").strip()
        if raw in valid:
            return raw
        print(f"  Enter one of: {', '.join(sorted(valid))}")


def confirm_apply(guide: RecipeGuide, answers: dict[str, str]) -> bool:
    print_preview(guide, answers=answers)
    return prompt_yes_no("Apply scaffold now?", default=True)


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


def _ask_step(step, n: int, total: int) -> str:
    print_step(step, n=n, total=total)
    options = suggestions_for(step.suggest) if getattr(step, "suggest", "") else []
    return prompt(step.question, step.default, suggestions=options)


def run_stdlib_pure_wizard() -> StdlibFnSpec:
    g = GUIDES["stdlib-pure"]
    print_recipe_intro(g)
    ny_module = _ask_step(g.steps[0], 1, len(g.steps))
    fn_name = _ask_step(g.steps[1], 2, len(g.steps))
    args_raw = _ask_step(g.steps[2], 3, len(g.steps))
    returns = parse_returns(_ask_step(g.steps[3], 4, len(g.steps)))
    wrap = _ask_step(g.steps[4], 5, len(g.steps))
    body = None
    if not wrap.strip():
        if prompt_yes_no("Provide custom fn body now? (optional — else TODO stub)", False):
            print("  Enter body lines; finish with empty line:")
            lines = []
            while True:
                line = input("    > ")
                if not line:
                    break
                lines.append(line)
            body = "\n".join(lines) if lines else None
    answers = {
        "module": ny_module,
        "fn": fn_name,
        "args": args_raw,
        "returns": returns.value if hasattr(returns, "value") else str(returns),
        "wrap_extern": wrap or "(none)",
    }
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
    return StdlibFnSpec(
        fn_name=fn_name,
        args=parse_args(args_raw),
        returns=returns,
        ny_module=ny_module,
        wrap_extern=wrap or None,
        pure_body=body,
    )


def run_stdlib_extern_wizard() -> StdlibFnSpec:
    g = GUIDES["stdlib-extern"]
    print_recipe_intro(g)
    from naming_guide import print_extern_naming_legend

    print_extern_naming_legend()
    ny_module = _ask_step(g.steps[0], 1, len(g.steps))
    fn_name = _ask_step(g.steps[1], 2, len(g.steps))
    args_raw = _ask_step(g.steps[2], 3, len(g.steps))
    returns = parse_returns(_ask_step(g.steps[3], 4, len(g.steps)))
    rt_module = _ask_step(g.steps[4], 5, len(g.steps))
    print_step(g.steps[5], n=6, total=len(g.steps))
    stable = prompt_yes_no("Add to stable ABI manifest?", default=False)
    since = "1.0.0"
    if stable:
        since = prompt("ABI since version", "1.0.0")
    from naming_guide import format_extern_name_summary

    answers = {
        "module": ny_module,
        "fn (C symbol + extern)": fn_name,
        "Nyra programmer call": f"{fn_name}(…)",
        "args": args_raw,
        "returns": returns.value if hasattr(returns, "value") else str(returns),
        "rt_module": rt_module,
        "stable_abi": "yes" if stable else "no",
        "naming": " | ".join(format_extern_name_summary(fn_name)),
    }
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
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
    g = GUIDES["test-example"]
    print_recipe_intro(g)
    name = _ask_step(g.steps[0], 1, len(g.steps))
    topic = _ask_step(g.steps[1], 2, len(g.steps))
    import_path = _ask_step(g.steps[2], 3, len(g.steps))
    answers = {"name": name, "topic": topic, "import": import_path or "(none)"}
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
    return TestExampleSpec(
        name=name,
        example_topic=topic,
        with_typed=True,
        use_testing=True,
        import_path=import_path or None,
    )


def run_pkg_wizard() -> PkgSpec:
    g = GUIDES["pkg"]
    print_recipe_intro(g)
    name = _ask_step(g.steps[0], 1, len(g.steps))
    version = _ask_step(g.steps[1], 2, len(g.steps))
    link = _ask_step(g.steps[2], 3, len(g.steps))
    answers = {"name": name, "version": version, "link_lib": link or "(none)"}
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
    return PkgSpec(name=name, version=version, link_lib=link or None)


def run_cli_wizard() -> CliSpec:
    g = GUIDES["cli"]
    print_recipe_intro(g)
    print_step(g.steps[0], n=1, total=len(g.steps))
    kind_raw = prompt_choice("", {"1": "Subcommand (nyra my_cmd …)", "2": "Global flag (nyra build --my_flag)"})
    kind = CliKind.SUBCOMMAND if kind_raw == "1" else CliKind.FLAG
    name = _ask_step(g.steps[1], 2, len(g.steps))
    desc = _ask_step(g.steps[2], 3, len(g.steps))
    answers = {"kind": kind.value, "name": name, "description": desc}
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
    return CliSpec(name=name, kind=kind, description=desc)


def run_conformance_wizard() -> ConformanceSpec:
    g = GUIDES["conformance"]
    print_recipe_intro(g)
    print_step(g.steps[0], n=1, total=len(g.steps))
    mode_raw = prompt_choice("", {"1": "pass — nyra test must succeed", "2": "fail — nyra check must error"})
    mode = ConformanceMode.PASS if mode_raw == "1" else ConformanceMode.FAIL
    area = _ask_step(g.steps[1], 2, len(g.steps))
    name = _ask_step(g.steps[2], 3, len(g.steps))
    desc = _ask_step(g.steps[3], 4, len(g.steps))
    answers = {"mode": mode.value, "area": area, "name": name, "description": desc}
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
    return ConformanceSpec(name=name, mode=mode, area=area, description=desc)


def run_syntax_wizard() -> "SyntaxSpec":
    from .spec import SyntaxSpec

    g = GUIDES["syntax-scaffold"]
    print_recipe_intro(g)
    keyword = _ask_step(g.steps[0], 1, len(g.steps))
    feature = _ask_step(g.steps[1], 2, len(g.steps))
    desc = _ask_step(g.steps[2], 3, len(g.steps))
    print_step(g.steps[3], n=4, total=len(g.steps))
    needs_expand = prompt_yes_no("Needs expand/ desugar pass?", default=True)
    print_step(g.steps[4], n=5, total=len(g.steps))
    needs_comptime = prompt_yes_no("Needs comptime eval?", default=False)
    answers = {
        "keyword": keyword,
        "feature": feature,
        "description": desc,
        "needs_expand": str(needs_expand),
        "needs_comptime": str(needs_comptime),
    }
    if not confirm_apply(g, answers):
        raise SystemExit("Cancelled — no files changed.")
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
    print("  WHY  → Undo a scaffold wired by make contribute (marked [contrib-dev:…]).")
    print("  TOOL → Removes markers, deletes scaffold files, cleans runtime_map if needed.")
    print("  YOU  → Search for leftover references; run make test-preflight.\n")
    items = list_wired_contribs()
    if not items:
        raise SystemExit("No [contrib-dev:…] scaffolds found in the repo.")
    for i, item in enumerate(items, 1):
        paths = ", ".join(str(p.name) for p in item.paths[:3])
        extra = "…" if len(item.paths) > 3 else ""
        print(f"  {i}. {item.label}  ({paths}{extra})")
    while True:
        raw = input("\n→ Select number or paste marker: ").strip()
        if raw.isdigit():
            idx = int(raw) - 1
            if 0 <= idx < len(items):
                return items[idx].marker
        for item in items:
            if raw == item.marker or raw == item.label:
                return item.marker
        print("  Invalid selection.")


def spec_from_config(recipe: str, data: dict):
    if recipe in ("stdlib-pure", "1", "stdlib-module"):
        pure_source = data.get("pure_source")
        if pure_source is None and data.get("source_file"):
            pure_source = Path(data["source_file"]).read_text(encoding="utf-8")
        return StdlibFnSpec(
            fn_name=data["fn_name"],
            args=[ArgSpec.parse(a) for a in data.get("args", [])],
            returns=parse_returns(data.get("returns", "void")),
            ny_module=data["ny_module"],
            wrap_extern=data.get("wrap_extern"),
            pure_body=data.get("pure_body"),
            pure_source=pure_source,
            example_topic=data.get("example_topic"),
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
