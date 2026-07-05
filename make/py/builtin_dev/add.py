from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from . import patch, templates
from .paths import ARRAY_PATHS, BYTES_PATHS, FREE_PATHS, STRING_PATHS
from .spec import BuiltinSpec, ReceiverKind


@dataclass
class ActionResult:
    spec: BuiltinSpec
    patches: list[patch.PatchResult] = field(default_factory=list)
    user_tasks: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    def ok(self) -> bool:
        return any(p.changed for p in self.patches)


def add_builtin(spec: BuiltinSpec, *, force: bool = False) -> ActionResult:
    if spec.receiver == ReceiverKind.STRING:
        return _add_string(spec, force=force)
    if spec.receiver == ReceiverKind.ARRAY:
        return _add_array(spec, force=force)
    if spec.receiver == ReceiverKind.BYTES:
        return _add_bytes(spec, force=force)
    if spec.receiver == ReceiverKind.FREE:
        return _add_free(spec, force=force)
    raise ValueError(f"unsupported receiver: {spec.receiver}")


def _add_string(spec: BuiltinSpec, *, force: bool) -> ActionResult:
    res = ActionResult(spec=spec)
    marker = spec.marker
    paths = STRING_PATHS

    res.patches.append(
        patch.upsert_marked_block(paths["rt_c"], templates.c_stub(spec), marker)
    )
    res.user_tasks.append(
        f"Implement C logic in stdlib/rt/{spec.rt_module} (search for [builtin-dev:{marker}])"
    )

    def add_extern(content: str):
        line = templates.extern_ny_line(spec)
        if line in content or has_method(content, spec.c_name):
            return content, False
        if not content.endswith("\n"):
            content += "\n"
        return content + line + "\n", True

    res.patches.append(patch.patch_file(paths["extern_ny"], add_extern))

    res.patches.append(
        patch.upsert_marked_block(paths["builtins_ny"], templates.builtins_wrapper(spec) + "\n", marker)
    )

    borrow_item = templates.typecheck_borrow_entry(spec)
    if spec.borrows_receiver:

        def add_borrow(content: str):
            if f'"{spec.method}"' in content.split("string_method_borrows_receiver")[1].split(")")[0]:
                return content, False
            return patch.add_to_rust_or_chain(
                content,
                "pub fn string_method_borrows_receiver(method: &str) -> bool {\n    matches!(\n        method,",
                f'"{spec.method}"',
            )

        res.patches.append(patch.patch_file(paths["typecheck"], add_borrow))

    def add_typecheck_arm(content: str):
        if templates.marker_start(spec) in content:
            return content, False
        return patch.add_to_match_before_default(
            content,
            "_ => return None,",
            "            " + templates.typecheck_match_arm(spec).replace("\n", "\n            "),
        )

    res.patches.append(patch.patch_file(paths["typecheck"], add_typecheck_arm))

    def add_util(content: str):
        if f'"{spec.method}"' in content:
            return content, False
        return patch.add_to_rust_or_chain(
            content,
            "pub(super) fn is_string_builtin_method(method: &str) -> bool {\n    matches!(\n        method,",
            f'"{spec.method}"',
        )

    res.patches.append(patch.patch_file(paths["codegen_util"], add_util))

    def add_codegen_arm(content: str):
        if templates.marker_start(spec) in content:
            return content, False
        arm = templates.codegen_string_method_arm(spec)
        return patch.insert_before(
            content,
            '_ => ExprValue {',
            "            " + arm.replace("\n", "\n            ") + "\n            ",
        )

    res.patches.append(patch.patch_file(paths["codegen_strings"], add_codegen_arm))

    decl = templates.llvm_decl(spec)

    def add_core_decl(content: str):
        if spec.c_name in content:
            return content, False
        return patch.add_tuple_line_before(content, "        ];", "            " + decl)

    res.patches.append(patch.patch_file(paths["codegen_core"], add_core_decl))

    sym = templates.runtime_map_symbol(spec)

    def add_runtime_map(content: str):
        changed = False
        if f'("{spec.c_name}"' not in content:
            new_content, c = patch.add_tuple_line_before(
                content, "    ])", "        " + sym
            )
            content, changed = new_content, changed or c
        if spec.free_fn_alias:
            alias = templates.runtime_map_alias(spec)
            if alias not in content:
                new_content, c = patch.add_tuple_line_before(
                    content,
                    "    ];",
                    "        " + alias,
                )
                content, changed = new_content, changed or c
        return content, changed

    res.patches.append(patch.patch_file(paths["runtime_map"], add_runtime_map))

    if spec.owned_return:

        def add_owned(content: str):
            entry = templates.ownership_owned_entry(spec)
            if entry in content:
                return content, False
            return patch.add_tuple_line_before(content, "];", "    " + entry)

        res.patches.append(patch.patch_file(paths["ownership_kind"], add_owned))

    if spec.stable_abi:
        res.patches.append(
            patch.upsert_marked_block(
                paths["abi_manifest"], templates.abi_manifest_block(spec), marker
            )
        )
        res.warnings.append("Run: make gen-abi-header && make gen-bindings-doc")

    example = paths["example_dir"] / f"{spec.method}.ny"
    typed = paths["example_dir"] / f"{spec.method}.typed.ny"
    res.patches.append(
        patch.write_new_file(example, templates.example_ny(spec), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(typed, templates.example_typed_ny(spec), marker, force=force)
    )

    test_path = paths["test_dir"] / f"string_{spec.method}_test.ny"
    res.patches.append(
        patch.write_new_file(test_path, templates.test_ny(spec), marker, force=force)
    )
    rel_test = f"tests/nyra/string_{spec.method}_test.ny"
    res.user_tasks.append(f"Fix test expectations in {rel_test}")
    res.user_tasks.append(f"Run: nyra test {rel_test}")

    return res


def _add_array(spec: BuiltinSpec, *, force: bool) -> ActionResult:
    res = ActionResult(spec=spec)
    marker = spec.marker
    paths = ARRAY_PATHS

    if spec.borrows_receiver:
        def add_borrow(content: str):
            return patch.add_to_rust_or_chain(
                content,
                "pub fn array_method_borrows_receiver(method: &str) -> bool {\n    matches!(method,",
                f'"{spec.method}"',
            )

        res.patches.append(patch.patch_file(paths["typecheck"], add_borrow))

    def add_arm(content: str):
        if templates.marker_start(spec) in content:
            return content, False
        return patch.add_to_match_before_default(
            content,
            "_ => None,",
            "            " + templates.array_typecheck_arm(spec).replace("\n", "\n            "),
        )

    res.patches.append(patch.patch_file(paths["typecheck"], add_arm))

    res.warnings.append(
        "Array methods may need manual LLVM codegen in "
        f"{paths['codegen_collections']} or {paths['codegen_expr']} "
        f"if not a pure type-level builtin (e.g. .len())."
    )
    res.user_tasks.append(
        "If the method needs runtime code, add C/LLVM wiring like a string builtin."
    )
    return res


def _add_bytes(spec: BuiltinSpec, *, force: bool) -> ActionResult:
    res = ActionResult(spec=spec)
    marker = spec.marker
    paths = BYTES_PATHS

    if paths["rt_c"].exists():
        res.patches.append(
            patch.upsert_marked_block(paths["rt_c"], templates.c_stub(spec), marker)
        )
        res.user_tasks.append(f"Implement C logic in {paths['rt_c']}")

    def add_return_type(content: str):
        if templates.marker_start(spec) in content:
            return content, False
        return patch.insert_before(
            content,
            "_ => None,",
            "            " + templates.bytes_method_return_arm(spec).replace("\n", "\n            ") + "\n            ",
        )

    res.patches.append(patch.patch_file(paths["typecheck"], add_return_type))
    res.warnings.append("Bytes method codegen may need manual updates in compiler/codegen/.")
    return res


def _add_free(spec: BuiltinSpec, *, force: bool) -> ActionResult:
    res = ActionResult(spec=spec)
    marker = spec.marker
    paths = FREE_PATHS

    res.patches.append(
        patch.upsert_marked_block(paths["rt_c"], templates.c_stub(spec), marker)
    )
    res.user_tasks.append(f"Implement C logic in {paths['rt_c']}")

    def add_extern(content: str):
        line = templates.extern_ny_line(spec)
        if line in content:
            return content, False
        return content + line + "\n", True

    res.patches.append(patch.patch_file(paths["extern_ny"], add_extern))

    decl = templates.llvm_decl(spec)

    def add_core(content: str):
        return patch.add_tuple_line_before(content, "        ];", "            " + decl)

    res.patches.append(patch.patch_file(paths["codegen_core"], add_core))

    sym = templates.runtime_map_symbol(spec)

    def add_map(content: str):
        return patch.add_tuple_line_before(content, "    ])", "        " + sym)

    res.patches.append(patch.patch_file(paths["runtime_map"], add_map))

    if spec.owned_return:

        def add_owned(content: str):
            return patch.add_tuple_line_before(
                content, "];", "    " + templates.ownership_owned_entry(spec)
            )

        res.patches.append(patch.patch_file(paths["ownership_kind"], add_owned))

    test_path = paths["test_dir"] / f"{spec.method}_test.ny"
    res.patches.append(
        patch.write_new_file(test_path, templates.test_ny(spec), marker, force=force)
    )
    return res


def has_method(content: str, name: str) -> bool:
    return name in content
