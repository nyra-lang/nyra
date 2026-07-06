"""Interactive wizard — explains what each answer controls; numbered choices only."""
from __future__ import annotations

from .discover import WiredBuiltin, list_wired_builtins, suggest_string_args
from .method_catalog import (
    RECEIVERS,
    default_spec_hints,
    explain_args_choice,
    explain_borrows_choice,
    explain_method_choice,
    explain_receiver_choice,
    explain_returns_choice,
    method_profile,
    normalize_receiver,
    preview_free_fn_call,
)
from .method_catalog import NYRA_TYPE_NAMES
from .monitor_report import print_spec_preview
from .spec import ArgSpec, BuiltinSpec, NyraType

RETURN_TYPES = ["string", "i32", "i64", "f64", "bool", "vec_str", "bytes", "array"]
RECEIVER_NAMES = list(RECEIVERS.keys())


def run_add_wizard() -> BuiltinSpec:
    print("\n" + "═" * 62)
    print("  ADD BUILTIN WIZARD")
    print("  Each step explains what your answer controls in the new method.")
    print("═" * 62)

    receiver = _pick_receiver()
    explain_receiver_choice(receiver)

    method = _ask_text("\nMethod name (snake_case)", required=True)
    explain_method_choice(method, receiver)
    hints = default_spec_hints(method)

    suggested = suggest_string_args(method) if receiver == "string" else []
    args = _pick_args(method, receiver, suggested)

    default_ret = hints.get("returns", "string")
    returns = _pick_one(
        f"Return type  (controls Nyra type of .{method}() result)",
        RETURN_TYPES,
        default=default_ret,
    )
    explain_returns_choice(returns, method)

    c_default = BuiltinSpec.from_cli(
        receiver=receiver,
        method=method,
        arg_specs=[f"{a.name}:{a.nyra_type.value}" for a in args],
        returns=returns,
        c_name=None,
        rt_module=None,
        borrows_receiver=True,
        owned_return=None,
        free_fn_alias=True,
        stable_abi=False,
        abi_since="1.0.0",
    ).c_name
    c_name = _pick_default(
        f"C symbol  (LLVM/runtime function name in stdlib/rt/)",
        c_default,
    )

    rt_default = RECEIVERS[receiver].default_rt
    rt_module = _pick_default(f"Runtime C file  (where C implementation lives)", rt_default)

    default_borrows = hints.get("borrows_receiver", True)
    borrows = _pick_yes_no(
        "Borrows receiver?  (YES = reads string without consuming it)",
        default=default_borrows,
    )
    explain_borrows_choice(borrows)

    stable_abi = _pick_yes_no("Add to stable ABI manifest (docs/abi-manifest.toml)?", default=False)
    abi_since = "1.0.0"
    if stable_abi:
        abi_since = _ask_text("ABI since version", default="1.0.0")

    spec = BuiltinSpec.from_cli(
        receiver=receiver,
        method=method,
        arg_specs=[f"{a.name}:{a.nyra_type.value}" for a in args],
        returns=returns,
        c_name=c_name,
        rt_module=rt_module,
        borrows_receiver=borrows,
        owned_return=None,
        free_fn_alias=True,
        stable_abi=stable_abi,
        abi_since=abi_since,
    )
    _print_final_preview(spec)
    if not _pick_yes_no("Proceed with ADD?", default=True):
        raise SystemExit("Cancelled — no files changed.")
    return spec


def run_remove_wizard() -> BuiltinSpec:
    print("\n" + "═" * 62)
    print("  REMOVE BUILTIN WIZARD")
    print("  Pick an existing wired builtin or type a custom name.")
    print("═" * 62 + "\n")

    receiver = _pick_receiver()
    wired = list_wired_builtins(receiver=receiver)

    method: str
    if wired:
        print("\nWired builtins found (via [builtin-dev:…] markers):")
        options = [b.label for b in wired]
        options.append("(type a different name)")
        idx = _pick_index("Select builtin to REMOVE", options, default=0)
        if idx < len(wired):
            chosen: WiredBuiltin = wired[idx]
            method = chosen.method
            print(f"  → selected: {chosen.label}")
        else:
            method = _ask_text("Method name to remove", required=True)
    else:
        print("  (no [builtin-dev:…] markers found for this receiver)")
        method = _ask_text("Method name to remove", required=True)

    spec = BuiltinSpec.from_cli(
        receiver=receiver,
        method=method,
        arg_specs=[],
        returns="string",
        c_name=None,
        rt_module=None,
        borrows_receiver=True,
        owned_return=False,
        free_fn_alias=False,
        stable_abi=False,
        abi_since="1.0.0",
    )
    print_spec_preview(spec, action="REMOVE")
    if not _pick_yes_no("Proceed with REMOVE?", default=False):
        raise SystemExit("Cancelled — no files changed.")
    return spec


def run_patch_wizard() -> tuple[BuiltinSpec, BuiltinSpec]:
    print("\n" + "═" * 62)
    print("  PATCH BUILTIN WIZARD")
    print("  Update an existing method (args, return type, C symbol, behavior wiring).")
    print("  C implementation is preserved when method name stays the same.")
    print("═" * 62 + "\n")

    receiver = _pick_receiver()
    wired = list_wired_builtins(receiver=receiver)
    if not wired:
        print("  No wired builtins found. Use `make add-builtin` first.")
        raise SystemExit(1)

    print("\nWired builtins:")
    options = [b.label for b in wired]
    idx = _pick_index("Select builtin to PATCH", options, default=0)
    chosen = wired[idx]
    old = BuiltinSpec.from_cli(
        receiver=receiver,
        method=chosen.method,
        arg_specs=[],
        returns="string",
        c_name=None,
        rt_module=None,
        borrows_receiver=True,
        owned_return=None,
        free_fn_alias=True,
        stable_abi=False,
        abi_since="1.0.0",
    )
    profile = method_profile(old.method)
    print(f"\n  Current: {old.receiver.value}.{old.method}  C: {old.c_name}")

    print("\nWhat do you want to change?")
    print("  1. Re-wire with catalog defaults (recommended if hand-edited wrong)")
    print("  2. Change arguments only")
    print("  3. Change return type only")
    print("  4. Change C symbol / runtime file")
    print("  5. Full custom re-spec (step through all fields)")
    choice = _pick_index("Patch mode", [str(i) for i in range(1, 6)], default=0)

    hints = default_spec_hints(old.method)
    method = old.method
    args = list(old.args)
    returns = old.returns.value
    c_name = old.c_name
    rt_module = old.rt_module
    borrows = old.borrows_receiver

    if choice == 0 and profile:
        args = [ArgSpec.parse(a) for a in profile.default_args]
        returns = profile.default_returns
        borrows = profile.borrows_receiver
        print("\n  → Applying catalog defaults for ." + method)
    elif choice == 1:
        suggested = suggest_string_args(method) if receiver == "string" else []
        args = _pick_args(method, receiver, suggested)
    elif choice == 2:
        returns = _pick_one("New return type", RETURN_TYPES, default=returns)
    elif choice == 3:
        c_name = _pick_default("C symbol", c_name or f"str_{method}")
        rt_module = _pick_default("Runtime C file", rt_module or RECEIVERS[receiver].default_rt)
    else:
        method = _ask_text("Method name (Enter to keep)", default=method) or method
        suggested = suggest_string_args(method) if receiver == "string" else []
        args = _pick_args(method, receiver, suggested)
        returns = _pick_one("Return type", RETURN_TYPES, default=returns)
        c_name = _pick_default("C symbol", c_name or f"str_{method}")
        rt_module = _pick_default("Runtime C file", rt_module or RECEIVERS[receiver].default_rt)
        borrows = _pick_yes_no("Borrows receiver?", default=borrows)

    new = BuiltinSpec.from_cli(
        receiver=receiver,
        method=method,
        arg_specs=[f"{a.name}:{a.nyra_type.value}" for a in args],
        returns=returns,
        c_name=c_name,
        rt_module=rt_module,
        borrows_receiver=borrows,
        owned_return=None,
        free_fn_alias=True,
        stable_abi=False,
        abi_since="1.0.0",
    )
    print("\n  OLD:")
    print_spec_preview(old, action="replace")
    print("  NEW:")
    print_spec_preview(new, action="apply")
    if not _pick_yes_no("Proceed with PATCH?", default=True):
        raise SystemExit("Cancelled — no files changed.")
    return old, new


def _pick_receiver() -> str:
    print("\nReceiver type  (controls which stdlib/compiler files get wired):")
    names = RECEIVER_NAMES
    default = "string"
    default_idx = names.index(default)
    for i, name in enumerate(names, 1):
        p = RECEIVERS[name]
        mark = " (default)" if name == default else ""
        alias_hint = f"  aliases: {', '.join(p.aliases)}" if p.aliases else ""
        print(f"  {i}. {name}{mark}{alias_hint}")
    while True:
        raw = input(f"> pick 1–{len(names)} [Enter={default_idx + 1}]: ").strip()
        if not raw:
            return default
        normalized = normalize_receiver(raw)
        if normalized:
            if normalized != raw.lower() and raw.lower() not in names:
                print(f"  → interpreted {raw!r} as {normalized!r}")
            return normalized
        if raw.isdigit():
            n = int(raw)
            if 1 <= n <= len(names):
                return names[n - 1]
        if raw in names:
            return raw
        print(f"  choose 1–{len(names)} (e.g. string, strings, str)")


def _pick_one(label: str, options: list[str], *, default: str) -> str:
    print(f"\n{label}:")
    default_idx = options.index(default) if default in options else 0
    for i, opt in enumerate(options, 1):
        mark = " (default)" if opt == default else ""
        print(f"  {i}. {opt}{mark}")
    while True:
        raw = input(f"> pick 1–{len(options)} [Enter={default_idx + 1}]: ").strip()
        if not raw:
            return options[default_idx]
        if raw.isdigit():
            n = int(raw)
            if 1 <= n <= len(options):
                return options[n - 1]
        if raw in options:
            return raw
        print(f"  choose 1–{len(options)} or type the value")


def _pick_index(label: str, options: list[str], *, default: int = 0) -> int:
    print(f"\n{label}:")
    for i, opt in enumerate(options, 1):
        mark = " (default)" if i - 1 == default else ""
        print(f"  {i}. {opt}{mark}")
    while True:
        raw = input(f"> pick 1–{len(options)} [Enter={default + 1}]: ").strip()
        if not raw:
            return default
        if raw.isdigit():
            n = int(raw)
            if 1 <= n <= len(options):
                return n - 1
        print(f"  choose 1–{len(options)}")


def _pick_yes_no(prompt: str, *, default: bool) -> bool:
    print(f"\n{prompt}:")
    print(f"  1. Yes{'  (default)' if default else ''}")
    print(f"  2. No{'   (default)' if not default else ''}")
    while True:
        raw = input("> pick 1 or 2 [Enter=default]: ").strip()
        if not raw:
            return default
        if raw in ("1", "y", "yes"):
            return True
        if raw in ("2", "n", "no"):
            return False
        print("  choose 1 or 2")


def _pick_default(label: str, default: str) -> str:
    print(f"\n{label}:")
    print(f"  1. {default}  (default — recommended)")
    print("  2. Type a custom value")
    while True:
        raw = input("> pick 1 or 2 [Enter=1]: ").strip()
        if not raw or raw == "1":
            return default
        if raw == "2":
            return _ask_text(f"Custom value", required=True)
        print("  choose 1 or 2")


def _pick_args(method: str, receiver: str, suggested: list[str]) -> list[ArgSpec]:
    profile = method_profile(method)
    print(f"\nArguments  (parameters of .{method}(…) — NOT the return type):")
    if profile:
        print(f"  Typical for .{method}(): {', '.join(profile.default_args) or '(none)'}")
    if suggested:
        print("  Suggested presets:")
        for i, s in enumerate(suggested, 1):
            print(f"    {i}. Use {s}")
        if len(suggested) > 1:
            print(f"    {len(suggested) + 1}. All: {', '.join(suggested)}")
            print(f"    {len(suggested) + 2}. Custom (enter manually)")
            print(f"    {len(suggested) + 3}. No arguments")
            max_opt = len(suggested) + 3
        else:
            print(f"    2. Custom (enter manually)")
            print(f"    3. No arguments")
            max_opt = 3
        while True:
            raw = input(f"> pick 1–{max_opt} [Enter=1]: ").strip()
            if not raw:
                args = [ArgSpec.parse(s) for s in suggested]
                explain_args_choice(args, method, receiver)
                return args
            if raw.isdigit():
                n = int(raw)
                if 1 <= n <= len(suggested):
                    args = [ArgSpec.parse(suggested[n - 1])]
                    explain_args_choice(args, method, receiver)
                    return args
                if len(suggested) > 1 and n == len(suggested) + 1:
                    args = [ArgSpec.parse(s) for s in suggested]
                    explain_args_choice(args, method, receiver)
                    return args
                custom_n = len(suggested) + 2 if len(suggested) > 1 else 2
                none_n = len(suggested) + 3 if len(suggested) > 1 else 3
                if n == custom_n:
                    args = _manual_args(method)
                    explain_args_choice(args, method, receiver)
                    return args
                if n == none_n:
                    explain_args_choice([], method, receiver)
                    return []
            print(f"  choose 1–{max_opt}")
    else:
        print("  1. No arguments  (default)")
        print("  2. Add arguments manually  (format: name:type)")
        raw = input("> pick 1 or 2 [Enter=1]: ").strip()
        if raw == "2":
            args = _manual_args(method)
            explain_args_choice(args, method, receiver)
            return args
        explain_args_choice([], method, receiver)
        return []


def _manual_args(method: str) -> list[ArgSpec]:
    profile = method_profile(method)
    print("  Enter one argument per line as  name:type  — empty line when done.")
    if profile and profile.default_args:
        print(f"  Hint for .{method}(): try  {profile.default_args[0]}")
    print("  Examples: suffix:string   prefix:string   count:i32")
    print("  ℹ Do NOT enter return type here (e.g. 'string') — that comes later.")
    args: list[ArgSpec] = []
    while True:
        raw = input("  arg> ").strip()
        if not raw:
            if args:
                break
            if profile and profile.default_args:
                use = _pick_yes_no(
                    f"No args entered. Use default {profile.default_args[0]}?",
                    default=True,
                )
                if use:
                    return [ArgSpec.parse(profile.default_args[0])]
            break
        if raw.lower() in NYRA_TYPE_NAMES:
            print(f"  ℹ '{raw}' is a return/param TYPE, not an argument line.")
            print("     Format is  name:type  (e.g. suffix:string). Press Enter when done with args.")
            continue
        try:
            args.append(ArgSpec.parse(raw))
            print(f"     ✓ added arg: {args[-1].name}:{args[-1].nyra_type.value}")
        except ValueError as exc:
            print(f"  ⚠ {exc}")
    return args


def _print_final_preview(spec: BuiltinSpec) -> None:
    print_spec_preview(spec, action="ADD")
    print("\n  📄 Generated files will include:")
    print(f"     • examples/builtins/strings/{spec.method}.ny       (zero-types demo)")
    print(f"     • examples/builtins/strings/{spec.method}.typed.ny (explicit-types demo)")
    print(f"     • tests/nyra/string_{spec.method}_test.ny          (test stub — fix expected values)")
    print("\n  💡 Usage after you implement C logic:")
    for line in preview_free_fn_call_lines(spec):
        print(f"     {line}")


def preview_free_fn_call_lines(spec: BuiltinSpec) -> list[str]:
    from .method_catalog import usage_snippets
    return usage_snippets(spec)


def _ask_text(prompt: str, *, required: bool = False, default: str = "") -> str:
    suffix = f" [{default}]" if default else ""
    while True:
        value = input(f"{prompt}{suffix}: ").strip()
        if not value and default:
            return default
        if value or not required:
            return value
        print("  required")
