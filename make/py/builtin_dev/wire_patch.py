"""Patch an existing builtin — update wiring while preserving C implementation when possible."""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

from . import patch, templates
from .add import ActionResult, add_builtin
from .paths import STRING_PATHS
from .remove import RemoveResult, remove_builtin
from .spec import BuiltinSpec


@dataclass
class PatchResult:
    old_spec: BuiltinSpec
    new_spec: BuiltinSpec
    remove: RemoveResult
    add: ActionResult
    preserved_c: bool = False
    warnings: list[str] = field(default_factory=list)

    def ok(self) -> bool:
        return any(p.changed for p in self.add.patches)


def extract_c_implementation(path: Path, marker: str) -> str | None:
    if not path.exists():
        return None
    content = path.read_text(encoding="utf-8")
    block_re = re.compile(
        rf"(?:#|//) \[builtin-dev:{re.escape(marker)}\].*?(?:#|//) \[/builtin-dev:{re.escape(marker)}\]",
        re.DOTALL,
    )
    m = block_re.search(content)
    if not m:
        return None
    body_re = re.compile(r"\{(.*)\}", re.DOTALL)
    body = body_re.search(m.group(0))
    if not body:
        return None
    lines = [ln for ln in body.group(1).splitlines() if "TODO: implement logic" not in ln]
    text = "\n".join(lines).strip()
    return text or None


def patch_builtin(old: BuiltinSpec, new: BuiltinSpec) -> PatchResult:
    rt_path = STRING_PATHS["rt_c"] if old.receiver.value == "string" else None
    saved_c = None
    if rt_path and old.method == new.method and old.receiver == new.receiver:
        saved_c = extract_c_implementation(rt_path, old.marker)

    remove_res = remove_builtin(old)
    add_res = add_builtin(new, force=True)

    preserved = False
    if saved_c and rt_path:

        def inject_body(content: str):
            start = f"// [builtin-dev:{new.marker}]"
            if start not in content:
                return content, False
            stub = templates.c_stub(new)
            body_re = re.compile(r"(\{)(.*?)(\})", re.DOTALL)
            new_block = body_re.sub(lambda m: m.group(1) + "\n" + saved_c + "\n" + m.group(3), stub, count=1)
            content, _ = patch.remove_marked_block(content, new.marker)
            if not content.endswith("\n"):
                content += "\n"
            return content + new_block + "\n", True

        pr = patch.patch_file(rt_path, inject_body)
        if pr.changed:
            preserved = True
            add_res.patches.append(pr)
            add_res.user_tasks = [
                t for t in add_res.user_tasks if "Implement C logic" not in t
            ]
            add_res.user_tasks.insert(
                0,
                f"Review preserved C logic in stdlib/rt/{new.rt_module} ([builtin-dev:{new.marker}])",
            )

    res = PatchResult(
        old_spec=old,
        new_spec=new,
        remove=remove_res,
        add=add_res,
        preserved_c=preserved,
    )
    if old.method != new.method:
        res.warnings.append(
            f"Method renamed {old.method} → {new.method}: update any manual references."
        )
    if not preserved and saved_c:
        res.warnings.append("Could not auto-preserve C body — re-implement in the new stub.")
    return res
