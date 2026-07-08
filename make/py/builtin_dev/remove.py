from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

from . import patch, templates
from .paths import ARRAY_PATHS, BYTES_PATHS, FREE_PATHS, STRING_PATHS
from .spec import BuiltinSpec, ReceiverKind


def _legacy_c_names(spec: BuiltinSpec) -> list[str]:
    """C symbols that may exist from hand-wiring before builtin-dev."""
    names = [spec.c_name or ""]
    if spec.receiver == ReceiverKind.STRING:
        names.append(f"string_{spec.method}")
    return [n for n in names if n]


@dataclass
class RemoveResult:
    spec: BuiltinSpec
    patches: list[patch.PatchResult] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    user_tasks: list[str] = field(default_factory=list)


def remove_builtin(spec: BuiltinSpec) -> RemoveResult:
    if spec.receiver == ReceiverKind.STRING:
        return _remove_string(spec)
    if spec.receiver == ReceiverKind.ARRAY:
        return _remove_array(spec)
    if spec.receiver == ReceiverKind.BYTES:
        return _remove_bytes(spec)
    if spec.receiver == ReceiverKind.FREE:
        return _remove_free(spec)
    raise ValueError(f"unsupported receiver: {spec.receiver}")


def _strip_marker(path: Path, marker: str, patches: list[patch.PatchResult]) -> None:
    if not path.exists():
        return

    def transform(content: str):
        return patch.remove_marked_block(content, marker)

    patches.append(patch.patch_file(path, transform))


def _strip_legacy_c_defs(path: Path, spec: BuiltinSpec, res: "RemoveResult") -> None:
    """Safely clean up legacy (pre-builtin-dev) C definitions for this method.

    The builtin's own C body lives inside its `[builtin-dev:...]` marker and is
    already removed by `_strip_marker`. Here we only handle hand-wired leftovers,
    and we do so conservatively to avoid destroying an unrelated function that
    merely shares a name (the bug this guards against):

      * `string_<method>` — the documented legacy alias form. Auto-removed
        (brace-balanced), unless it sits inside another feature's marker block.
      * `spec.c_name` (e.g. `str_trim`) — NEVER auto-deleted when found outside
        our marker: it may be a core/hand-wired function of the same name. We
        only warn so the maintainer can decide.
    """
    if not path.exists():
        return
    content = patch.read_text(path)
    changed = False

    if spec.receiver == ReceiverKind.STRING:
        alias = f"string_{spec.method}"
        content, status = patch.remove_c_function_def(content, alias)
        if status == "removed":
            changed = True
        elif status == "skipped_marked":
            res.warnings.append(
                f"Legacy C alias `{alias}` in {path.name} is inside another "
                "builtin-dev marker block — left intact."
            )

    if changed:
        patch.write_text(path, content)
        res.patches.append(patch.PatchResult(path, True, "legacy C alias removed"))
    else:
        res.patches.append(patch.PatchResult(path, False, "no legacy C alias"))

    if spec.c_name and patch.c_function_defined(content, spec.c_name):
        res.warnings.append(
            f"C function `{spec.c_name}` still defined in {path.name} outside this "
            "builtin's marker block — left intact to avoid destroying an unrelated "
            "or hand-wired definition. Remove it manually if it belonged to this "
            "feature."
        )


def _remove_string(spec: BuiltinSpec) -> RemoveResult:
    res = RemoveResult(spec=spec)
    marker = spec.marker
    paths = STRING_PATHS

    _strip_marker(paths["rt_c"], marker, res.patches)

    _strip_legacy_c_defs(paths["rt_c"], spec, res)
    _strip_marker(paths["builtins_ny"], marker, res.patches)
    _strip_marker(paths["abi_manifest"], marker, res.patches)

    def strip_externs(content: str):
        changed = False
        for c_name in _legacy_c_names(spec):
            content, c = patch.remove_line_with(content, f"extern fn {c_name}")
            changed = changed or c
        return content, changed

    res.patches.append(patch.patch_file(paths["extern_ny"], strip_externs))

    def strip_typecheck(content: str):
        content, c1 = patch.remove_marked_block(content, marker)
        content, c2 = patch.remove_rust_match_arm(content, spec.method)
        content, c3 = patch.remove_or_chain_item(content, f'"{spec.method}"')
        return content, c1 or c2 or c3

    res.patches.append(patch.patch_file(paths["typecheck"], strip_typecheck))
    if "borrowck" in paths:
        res.patches.append(
            patch.patch_file(
                paths["borrowck"],
                lambda c: patch.remove_or_chain_item(c, f'"{spec.method}"'),
            )
        )
    res.patches.append(
        patch.patch_file(
            paths["codegen_util"],
            lambda c: patch.remove_or_chain_item(c, f'"{spec.method}"'),
        )
    )

    def strip_codegen(content: str):
        content, c1 = patch.remove_marked_block(content, marker)
        content, c2 = patch.remove_rust_match_arm(content, spec.method)
        return content, c1 or c2

    res.patches.append(patch.patch_file(paths["codegen_strings"], strip_codegen))

    c_names = _legacy_c_names(spec)
    fragments: list[str] = []
    for c_name in c_names:
        fragments.extend([
            f'("{c_name}"',
            f'("{spec.method}", "{c_name}")',
            f'declare ptr @{c_name}',
            f'"{c_name}",',
        ])
    fragments.append(templates.ownership_owned_entry(spec))
    for path in [paths["codegen_core"], paths["runtime_map"], paths["ownership_kind"]]:
        for fragment in fragments:
            res.patches.append(
                patch.patch_file(path, lambda c, f=fragment: patch.remove_line_with(c, f))
            )

    # Legacy hand-wired wrapper (e.g. String_stripSuffix calling string_strip_suffix)
    def strip_legacy_wrapper(content: str):
        pascal = "".join(p[:1].upper() + p[1:] for p in spec.method.split("_") if p)
        wrapper = f"fn String_{pascal[0].lower()}{pascal[1:]}" if pascal else ""
        if not wrapper or wrapper not in content:
            return content, False
        # Remove fn block before any [builtin-dev] block for same method
        pattern = re.compile(
            rf"{re.escape(wrapper)}\([^\)]*\)[^{{]*\{{[^}}]*\}}\n?",
        )
        new_content, n = pattern.subn("", content, count=1)
        return new_content, n > 0

    res.patches.append(patch.patch_file(paths["builtins_ny"], strip_legacy_wrapper))

    for p in [
        paths["example_dir"] / f"{spec.method}.ny",
        paths["example_dir"] / f"{spec.method}.typed.ny",
        paths["test_dir"] / f"string_{spec.method}_test.ny",
    ]:
        if p.exists():
            p.unlink()
            res.patches.append(patch.PatchResult(p, True, "deleted"))

    res.warnings.append("Review docs/bindings.md and webDocs if ABI was published.")
    return res


def _remove_array(spec: BuiltinSpec) -> RemoveResult:
    res = RemoveResult(spec=spec)
    marker = spec.marker

    def strip(content: str):
        content, c1 = patch.remove_marked_block(content, marker)
        content, c2 = patch.remove_or_chain_item(content, f'"{spec.method}"')
        return content, c1 or c2

    res.patches.append(patch.patch_file(ARRAY_PATHS["typecheck"], strip))
    return res


def _remove_bytes(spec: BuiltinSpec) -> RemoveResult:
    res = RemoveResult(spec=spec)
    marker = spec.marker
    _strip_marker(BYTES_PATHS["rt_c"], marker, res.patches)
    res.patches.append(
        patch.patch_file(BYTES_PATHS["typecheck"], lambda c: patch.remove_marked_block(c, marker))
    )
    return res


def _remove_free(spec: BuiltinSpec) -> RemoveResult:
    res = RemoveResult(spec=spec)
    marker = spec.marker
    paths = FREE_PATHS
    _strip_marker(paths["rt_c"], marker, res.patches)

    res.patches.append(
        patch.patch_file(
            paths["extern_ny"],
            lambda c: patch.remove_line_with(c, f"extern fn {spec.c_name}"),
        )
    )
    for path in [paths["codegen_core"], paths["runtime_map"], paths["ownership_kind"]]:
        res.patches.append(
            patch.patch_file(path, lambda c, s=spec: patch.remove_line_with(c, s.c_name))
        )
    test = paths["test_dir"] / f"{spec.method}_test.ny"
    if test.exists():
        test.unlink()
        res.patches.append(patch.PatchResult(p=test, changed=True, message="deleted"))
    return res
