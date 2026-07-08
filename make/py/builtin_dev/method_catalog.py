"""Human-readable catalog — what each builtin choice means and what it affects."""
from __future__ import annotations

from dataclasses import dataclass

from .spec import ArgSpec, BuiltinSpec, NyraType, ReceiverKind


@dataclass(frozen=True)
class MethodProfile:
    summary: str
    example_in: str
    example_out: str
    default_args: list[str]
    default_returns: str
    borrows_receiver: bool
    c_logic_hint: str


@dataclass(frozen=True)
class ReceiverProfile:
    label: str
    aliases: tuple[str, ...]
    call_syntax: str
    files_touched: tuple[str, ...]
    default_rt: str


RECEIVERS: dict[str, ReceiverProfile] = {
    "string": ReceiverProfile(
        label="string",
        aliases=("strings", "str", "text"),
        call_syntax='"hello".method_name(arg)   or   method_name("hello", arg)',
        files_touched=(
            "stdlib/rt/rt_strings.c          — C implementation",
            "stdlib/builtins_string.ny         — .method() wrapper + free fn",
            "stdlib/strings.ny                 — extern fn declaration",
            "compiler/typecheck/…            — type checking for .method()",
            "compiler/codegen/…              — LLVM codegen",
        ),
        default_rt="rt_strings.c",
    ),
    "array": ReceiverProfile(
        label="array",
        aliases=("arrays", "arr", "vec"),
        call_syntax="arr.method_name(arg)",
        files_touched=(
            "compiler/typecheck/array_builtins.rs",
            "compiler/codegen/… (may need manual LLVM)",
        ),
        default_rt="rt_array.c",
    ),
    "bytes": ReceiverProfile(
        label="bytes",
        aliases=("byte", "binary"),
        call_syntax="data.method_name(arg)",
        files_touched=(
            "stdlib/rt/rt_bytes.c",
            "compiler/typecheck/bytes_builtins.rs",
        ),
        default_rt="rt_bytes.c",
    ),
    "free": ReceiverProfile(
        label="free",
        aliases=("function", "fn", "global"),
        call_syntax="method_name(arg1, arg2)   — standalone function, no receiver",
        files_touched=(
            "stdlib/rt/rt_strings.c (or other rt_*.c)",
            "stdlib/strings.ny",
            "compiler/codegen/runtime_map.rs",
        ),
        default_rt="rt_strings.c",
    ),
}

RETURN_PROFILES: dict[str, str] = {
    "string": "Nyra `string` — caller owns result; runtime allocates with str_dup/malloc",
    "i32": "Nyra integer (i32) — used for bool-like checks (.contains → 0/1)",
    "i64": "64-bit integer",
    "f64": "Floating point",
    "bool": "Boolean as i32 (0 or 1)",
    "vec_str": "Vector of strings (.split)",
    "bytes": "Byte buffer",
    "array": "Fixed or inferred array type",
}

KNOWN_METHODS: dict[str, MethodProfile] = {
    "strip_suffix": MethodProfile(
        summary="Remove trailing substring if the string ends with it; otherwise return a copy.",
        example_in='"hamdy.txt".strip_suffix(".txt")',
        example_out='"hamdy"',
        default_args=["suffix:string"],
        default_returns="string",
        borrows_receiver=True,
        c_logic_hint="Compare end of string with suffix; if match, allocate shorter copy.",
    ),
    "strip_prefix": MethodProfile(
        summary="Remove leading substring if the string starts with it; otherwise return a copy.",
        example_in='"prefix_hello".strip_prefix("prefix_")',
        example_out='"hello"',
        default_args=["prefix:string"],
        default_returns="string",
        borrows_receiver=True,
        c_logic_hint="Compare start of string with prefix; if match, return remainder.",
    ),
    "starts_with": MethodProfile(
        summary="Returns 1 if string starts with prefix, else 0.",
        example_in='"hello".starts_with("hel")',
        example_out="1",
        default_args=["prefix:string"],
        default_returns="i32",
        borrows_receiver=True,
        c_logic_hint="strncmp at position 0.",
    ),
    "ends_with": MethodProfile(
        summary="Returns 1 if string ends with suffix, else 0.",
        example_in='"hello".ends_with("lo")',
        example_out="1",
        default_args=["suffix:string"],
        default_returns="i32",
        borrows_receiver=True,
        c_logic_hint="Compare last N chars.",
    ),
    "contains": MethodProfile(
        summary="Returns 1 if substring found, else 0.",
        example_in='"hello".contains("ell")',
        example_out="1",
        default_args=["needle:string"],
        default_returns="i32",
        borrows_receiver=True,
        c_logic_hint="Use strstr or manual scan.",
    ),
    "trim": MethodProfile(
        summary="Remove leading and trailing whitespace.",
        example_in='"  hello  ".trim()',
        example_out='"hello"',
        default_args=[],
        default_returns="string",
        borrows_receiver=True,
        c_logic_hint="Skip spaces at both ends; return new owned string.",
    ),
    "replace": MethodProfile(
        summary="Replace all occurrences of `from` with `to`.",
        example_in='"a-b-c".replace("-", "_")',
        example_out='"a_b_c"',
        default_args=["from:string", "to:string"],
        default_returns="string",
        borrows_receiver=True,
        c_logic_hint="Scan and build new string with replacements.",
    ),
    "split": MethodProfile(
        summary="Split string by separator into a vector of strings.",
        example_in='"a,b,c".split(",")',
        example_out="VecStr handle",
        default_args=["sep:string"],
        default_returns="vec_str",
        borrows_receiver=True,
        c_logic_hint="Use vec_str_push in a loop.",
    ),
}

NYRA_TYPE_NAMES = frozenset(t.value for t in NyraType if t != NyraType.VOID)


def normalize_receiver(raw: str) -> str | None:
    key = raw.strip().lower()
    if key in RECEIVERS:
        return key
    for name, profile in RECEIVERS.items():
        if key in profile.aliases:
            return name
    return None


def method_profile(method: str) -> MethodProfile | None:
    return KNOWN_METHODS.get(method.strip().lower())


def explain_receiver_choice(name: str) -> None:
    p = RECEIVERS[name]
    print(f"\n  📌 You chose: {p.label}")
    print(f"     Call syntax : {p.call_syntax}")
    print(f"     Runtime file: stdlib/rt/{p.default_rt}")
    print("     Files the tool will wire:")
    for f in p.files_touched:
        print(f"       • {f}")


def explain_method_choice(method: str, receiver: str) -> None:
    profile = method_profile(method)
    c_guess = f"str_{method}" if receiver == "string" else f"(auto from {method})"
    print(f"\n  📌 Nyra method name (programmer code): {method}")
    print(f"     Call in Nyra     : \"value\".{method}(…)")
    print(f"     C symbol (rt/*.c): {c_guess}  ← implement logic here, not the method name")
    print(f"     Nyra API         : .{method}(…) on {receiver}")
    if profile:
        print(f"     Behavior       : {profile.summary}")
        print(f"     Example        : {profile.example_in}  →  {profile.example_out}")
        print(f"     C hint         : {profile.c_logic_hint}")
    else:
        print("     Behavior       : (custom — you implement logic in the C stub)")
        print(f"     Example        : \"value\".{method}(…)  — shape depends on args you pick next")


def explain_args_choice(args: list[ArgSpec], method: str, receiver: str) -> None:
    print("\n  📌 Arguments determine the Nyra + C function signature:")
    if args:
        for a in args:
            print(f"       • {a.name}: {a.nyra_type.value}  →  parameter in .{method}({a.name}) and C fn")
    else:
        print(f"       • (none)  →  .{method}() takes no extra parameters")
    print(f"     Full call: {preview_method_call(method, args, receiver)}")


def explain_returns_choice(returns: str, method: str) -> None:
    hint = RETURN_PROFILES.get(returns, returns)
    profile = method_profile(method)
    print(f"\n  📌 Return type: {returns}")
    print(f"     Meaning: {hint}")
    if profile and profile.default_returns != returns:
        print(f"     ⚠ usual for .{method}() is `{profile.default_returns}` — you picked `{returns}`")


def explain_borrows_choice(borrows: bool) -> None:
    if borrows:
        print("\n  📌 Borrows receiver: YES")
        print("     Caller keeps the original string; method only reads it (like .contains).")
    else:
        print("\n  📌 Borrows receiver: NO (moves/consumes)")
        print("     Original string may be invalidated after call (like .pop).")


def preview_method_call(method: str, args: list[ArgSpec], receiver: str) -> str:
    profile = method_profile(method)
    if profile and profile.example_in:
        return profile.example_in
    arg_str = ", ".join(_sample_arg(a) for a in args)
    if receiver == "string":
        sample = '"hello.txt"' if "suffix" in method or "prefix" in method else '"hello"'
        if args:
            return f"{sample}.{method}({arg_str})"
        return f"{sample}.{method}()"
    return f"{method}({arg_str})" if args else f"{method}()"


def preview_free_fn_call(spec: BuiltinSpec) -> str:
    profile = method_profile(spec.method)
    if spec.method in ("strip_suffix", "strip_prefix"):
        a0 = _sample_arg(spec.args[0]) if spec.args else '".txt"'
        return f'{spec.method}("hamdy.txt", {a0})'
    if profile:
        return profile.example_in.replace(f".{spec.method}", f"{spec.method}(") + ")"
    args = ", ".join(_sample_arg(a) for a in spec.args)
    recv = '"hello"' if spec.receiver == ReceiverKind.STRING else "value"
    parts = [recv, *(_sample_arg(a) for a in spec.args)]
    return f"{spec.method}({', '.join(parts)})"


def default_spec_hints(method: str) -> dict:
    profile = method_profile(method)
    if not profile:
        return {}
    return {
        "args": profile.default_args,
        "returns": profile.default_returns,
        "borrows_receiver": profile.borrows_receiver,
    }


def usage_snippets(spec: BuiltinSpec) -> list[str]:
    """Nyra usage lines shown in monitor after add/patch."""
    lines: list[str] = []
    profile = method_profile(spec.method)
    if spec.receiver == ReceiverKind.STRING:
        method_call = preview_method_call(
            spec.method,
            spec.args,
            spec.receiver.value,
        )
        lines.append(f"  // method call (zero types)")
        lines.append(f"  let result = {method_call}")
        if spec.free_fn_alias:
            lines.append("")
            lines.append(f"  // free function alias")
            lines.append(f"  let result2 = {preview_free_fn_call(spec)}")
        if profile:
            lines.append("")
            lines.append(f"  // expected: {profile.example_out}")
    else:
        lines.append(f"  let result = {preview_method_call(spec.method, spec.args, spec.receiver.value)}")
    return lines


def _sample_arg(arg: ArgSpec) -> str:
    if arg.nyra_type == NyraType.STRING:
        if arg.name in ("suffix", "prefix", "needle", "sep", "from"):
            return '".txt"' if "suffix" in arg.name or arg.name == "from" else '"x"'
        return '"arg"'
    if arg.nyra_type in (NyraType.I32, NyraType.BOOL):
        return "1"
    if arg.nyra_type == NyraType.I64:
        return "1"
    return "arg"
