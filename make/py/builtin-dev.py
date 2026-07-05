#!/usr/bin/env python3
"""Nyra builtin developer CLI — add/remove/patch stdlib methods with monitor output."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_MAKE_PY = Path(__file__).resolve().parent
if str(_MAKE_PY) not in sys.path:
    sys.path.insert(0, str(_MAKE_PY))

from builtin_dev.add import ActionResult, add_builtin
from builtin_dev.monitor_report import print_add_monitor, print_patch_monitor, print_remove_monitor
from builtin_dev.remove import RemoveResult, remove_builtin
from builtin_dev.spec import BuiltinSpec
from builtin_dev.wire_patch import PatchResult, patch_builtin
from builtin_dev.wizard_prompts import run_add_wizard, run_patch_wizard, run_remove_wizard


def build_spec_from_args(ns: argparse.Namespace, *, for_remove: bool = False) -> BuiltinSpec:
    config = getattr(ns, "config", None)
    if config:
        data = json.loads(Path(config).read_text(encoding="utf-8"))
        return BuiltinSpec.from_cli(
            receiver=data["receiver"],
            method=data["method"],
            arg_specs=data.get("args", []),
            returns=data.get("returns", "string"),
            c_name=data.get("c_name"),
            rt_module=data.get("rt_module"),
            borrows_receiver=data.get("borrows_receiver", True),
            owned_return=data.get("owned_return"),
            free_fn_alias=data.get("free_fn_alias", True),
            stable_abi=data.get("stable_abi", False),
            abi_since=data.get("abi_since", "1.0.0"),
        )
    if ns.interactive or not ns.method:
        return run_remove_wizard() if for_remove else run_add_wizard()
    if for_remove:
        return BuiltinSpec.from_cli(
            receiver=ns.receiver,
            method=ns.method,
            arg_specs=[],
            returns="string",
            c_name=ns.c_name,
            rt_module=ns.rt_module,
            borrows_receiver=True,
            owned_return=False,
            free_fn_alias=False,
            stable_abi=False,
            abi_since="1.0.0",
        )
    return BuiltinSpec.from_cli(
        receiver=ns.receiver,
        method=ns.method,
        arg_specs=ns.arg or [],
        returns=ns.returns,
        c_name=ns.c_name,
        rt_module=ns.rt_module,
        borrows_receiver=not ns.moves_receiver,
        owned_return=ns.owned_return,
        free_fn_alias=not ns.no_free_alias,
        stable_abi=ns.stable_abi,
        abi_since=ns.abi_since,
    )


def cmd_add(ns: argparse.Namespace) -> int:
    spec = build_spec_from_args(ns)
    result = add_builtin(spec, force=ns.force)
    print_add_monitor(result)
    return 0 if result.ok() else 1


def cmd_remove(ns: argparse.Namespace) -> int:
    spec = build_spec_from_args(ns, for_remove=True)
    result = remove_builtin(spec)
    print_remove_monitor(result)
    return 0 if any(p.changed for p in result.patches) else 1


def cmd_patch(ns: argparse.Namespace) -> int:
    if ns.config:
        new = build_spec_from_args(ns)
        if not ns.method:
            print("error: --method required with --config for patch", file=sys.stderr)
            return 2
        old = BuiltinSpec.from_cli(
            receiver=ns.receiver,
            method=ns.method,
            arg_specs=[],
            returns="string",
            c_name=ns.c_name,
            rt_module=ns.rt_module,
            borrows_receiver=True,
            owned_return=False,
            free_fn_alias=False,
            stable_abi=False,
            abi_since="1.0.0",
        )
    elif ns.interactive or not ns.method:
        old, new = run_patch_wizard()
    else:
        print("error: use --interactive or --config for patch", file=sys.stderr)
        return 2
    result = patch_builtin(old, new)
    print_patch_monitor(result)
    return 0 if result.ok() else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Nyra builtin monitor — add/remove/patch stdlib methods. "
            "Shows what each choice controls and what you should do next."
        )
    )
    sub = parser.add_subparsers(dest="command", required=True)

    add_p = sub.add_parser("add", help="Wire a new builtin (compiler + stdlib + tests + examples)")
    add_p.add_argument("--interactive", "-i", action="store_true", help="Wizard — explains each step")
    add_p.add_argument("--config", "-c", help="JSON spec (see make/py/builtin_dev/examples/)")
    add_p.add_argument("--receiver", "-t", default="string", choices=["string", "array", "bytes", "free"])
    add_p.add_argument("--method", "-m", help="Method name in snake_case (e.g. strip_suffix)")
    add_p.add_argument("--arg", "-a", action="append", default=[], help="Argument as name:type")
    add_p.add_argument("--returns", "-r", default="string", help="Return type")
    add_p.add_argument("--c-name", help="C symbol (default: str_<method> for strings)")
    add_p.add_argument("--rt-module", help="Runtime source file (default: rt_strings.c)")
    add_p.add_argument("--moves-receiver", action="store_true", help="Receiver is moved, not borrowed")
    add_p.add_argument("--no-free-alias", action="store_true", help="Skip free fn alias in builtins_string.ny")
    add_p.add_argument("--stable-abi", action="store_true", help="Append to docs/abi-manifest.toml")
    add_p.add_argument("--abi-since", default="1.0.0")
    add_p.add_argument("--owned-return", action="store_true", help="Force OWNED_EXTERN_RETURNS entry")
    add_p.add_argument("--force", action="store_true", help="Overwrite example/test stubs if present")
    add_p.set_defaults(func=cmd_add)

    rm_p = sub.add_parser("remove", help="Remove a builtin wired by this tool")
    rm_p.add_argument("--interactive", "-i", action="store_true", help="Wizard — pick from wired builtins")
    rm_p.add_argument("--receiver", "-t", default="string", choices=["string", "array", "bytes", "free"])
    rm_p.add_argument("--method", "-m", help="Method name to remove")
    rm_p.add_argument("--c-name", help="C symbol if non-default")
    rm_p.add_argument("--rt-module", help="Ignored; kept for spec parity")
    rm_p.set_defaults(func=cmd_remove)

    patch_p = sub.add_parser("patch", help="Update an existing builtin (preserves C code when possible)")
    patch_p.add_argument("--interactive", "-i", action="store_true", help="Wizard — pick what to change")
    patch_p.add_argument("--config", "-c", help="New JSON spec (requires --method for old builtin name)")
    patch_p.add_argument("--receiver", "-t", default="string", choices=["string", "array", "bytes", "free"])
    patch_p.add_argument("--method", "-m", help="Existing method name to patch")
    patch_p.add_argument("--c-name", help="C symbol if non-default")
    patch_p.add_argument("--rt-module", help="Runtime module")
    patch_p.set_defaults(func=cmd_patch)

    ns = parser.parse_args(argv)
    try:
        return ns.func(ns)
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
