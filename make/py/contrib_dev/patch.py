"""File patching with [contrib-dev:…] markers — shared helpers for contributor recipes."""
from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

PREFIX = "contrib-dev"


@dataclass
class PatchResult:
    path: Path
    changed: bool
    message: str


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")


def marker_tag(marker: str) -> str:
    return f"[{PREFIX}:{marker}]"


def has_marker(content: str, marker: str) -> bool:
    return marker_tag(marker) in content


def c_function_defined(content: str, name: str) -> bool:
    """True if a C function named `name` is DEFINED (not merely called) in `content`.

    Matches a definition line like `char *str_to_snake_case(const char *s) {`
    while ignoring call sites such as `x = str_to_snake_case(s);`. Used to stop
    two recipes (e.g. Pattern B extern + Built-in Method) from emitting the same
    C symbol twice, which the C compiler rejects as a redefinition.
    """
    pattern = re.compile(
        rf"(?m)^[A-Za-z_][A-Za-z0-9_ \t\*]*\b{re.escape(name)}\s*\([^;{{]*\)\s*\{{"
    )
    return bool(pattern.search(content))


def marker_start(marker: str, *, lang: str = "ny") -> str:
    if lang == "c":
        return f"// {marker_tag(marker)}"
    if lang == "toml":
        return f"# {marker_tag(marker)}"
    if lang == "rust":
        return f"// {marker_tag(marker)}"
    if lang == "md":
        return f"<!-- {marker_tag(marker)} -->"
    return f"// {marker_tag(marker)}"


def marker_end(marker: str, *, lang: str = "ny") -> str:
    if lang == "c":
        return f"// [/{PREFIX}:{marker}]"
    if lang == "toml":
        return f"# [/{PREFIX}:{marker}]"
    if lang == "rust":
        return f"// [/{PREFIX}:{marker}]"
    if lang == "md":
        return f"<!-- [/{PREFIX}:{marker}] -->"
    return f"// [/{PREFIX}:{marker}]"


def wrap_scaffold(body: str, marker: str, *, lang: str = "ny") -> str:
    return "\n".join(
        [
            marker_start(marker, lang=lang),
            body.rstrip(),
            marker_end(marker, lang=lang),
            "",
        ]
    )


def remove_marked_block(content: str, marker: str) -> tuple[str, bool]:
    patterns = [
        re.compile(
            rf"[ \t]*(?:#|//) \[{PREFIX}:{re.escape(marker)}\].*?"
            rf"(?:#|//) \[/{PREFIX}:{re.escape(marker)}\]\n?",
            re.DOTALL,
        ),
        re.compile(
            rf"[ \t]*<!-- \[{PREFIX}:{re.escape(marker)}\] -->.*?"
            rf"<!-- \[/{PREFIX}:{re.escape(marker)}\] -->\n?",
            re.DOTALL,
        ),
    ]
    changed = False
    for pattern in patterns:
        content, n = pattern.subn("", content)
        changed = changed or n > 0
    return content, changed


def insert_before(content: str, needle: str, insertion: str) -> tuple[str, bool]:
    idx = content.find(needle)
    if idx == -1:
        return content, False
    block = insertion if insertion.endswith("\n") else insertion + "\n"
    return content[:idx] + block + content[idx:], True


def add_line_before_anchor(content: str, anchor: str, line: str, *, last: bool = False) -> tuple[str, bool]:
    if line.strip() in content:
        return content, False
    if last:
        idx = content.rfind(anchor)
        if idx == -1:
            return content, False
        block = line if line.endswith("\n") else line + "\n"
        return content[:idx] + block + content[idx:], True
    return insert_before(content, anchor, line)


def upsert_marked_block(path: Path, block: str, marker: str) -> PatchResult:
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        write_text(path, block + "\n")
        return PatchResult(path, True, "created")
    content = read_text(path)
    if has_marker(content, marker):
        return PatchResult(path, False, "already present")
    content, _ = remove_marked_block(content, marker)
    if not content.endswith("\n"):
        content += "\n"
    write_text(path, content + block + "\n")
    return PatchResult(path, True, "inserted")


def write_new_file(path: Path, content: str, marker: str, *, force: bool = False) -> PatchResult:
    if path.exists() and not force:
        if has_marker(read_text(path), marker):
            return PatchResult(path, False, "already present")
        return PatchResult(path, False, "exists (use --force)")
    path.parent.mkdir(parents=True, exist_ok=True)
    write_text(path, content)
    return PatchResult(path, True, "created")


_IMPL_RE = re.compile(r"^\s*impl\s+(\w+)\s*\{", re.MULTILINE)
_METHOD_RE = re.compile(r"^\s*fn\s+(\w+)\s*\(", re.MULTILINE)


def _impl_body_span(content: str, struct_name: str) -> tuple[int, int] | None:
    match = re.search(rf"impl\s+{re.escape(struct_name)}\s*\{{", content)
    if not match:
        return None
    start = match.end()
    depth = 1
    i = start
    while i < len(content) and depth > 0:
        ch = content[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        if depth == 0:
            return start, i
        i += 1
    return None


def _extract_impl_methods(pure_source: str) -> tuple[str, str] | None:
    all_impls = _extract_all_impl_methods(pure_source)
    if not all_impls:
        return None
    return all_impls[0]


def _extract_all_impl_methods(pure_source: str) -> list[tuple[str, str]]:
    results: list[tuple[str, str]] = []
    source = pure_source.strip()
    for match in _IMPL_RE.finditer(source):
        struct_name = match.group(1)
        subset = source[match.start() :]
        span = _impl_body_span(subset, struct_name)
        if span is None:
            continue
        body_start, body_end = span
        results.append((struct_name, subset[body_start:body_end].rstrip()))
    return results


def _skip_fn_body(lines: list[str], start: int) -> int:
    """Advance past a method body starting at `fn` line `start`."""
    depth = 0
    i = start
    while i < len(lines):
        depth += lines[i].count("{") - lines[i].count("}")
        if depth <= 0 and i > start and "{" in lines[start]:
            return i + 1
        if depth <= 0 and "{" not in lines[start] and lines[i].strip() == "":
            return i + 1
        i += 1
    return i


def _merge_one_impl(content: str, struct_name: str, methods: str) -> tuple[str, bool, str]:
    span = _impl_body_span(content, struct_name)
    if span is None:
        return content, False, f"impl {struct_name} not found"

    body_start, body_end = span
    existing_body = content[body_start:body_end]
    existing_methods = set(_METHOD_RE.findall(existing_body))
    method_lines = methods.splitlines()
    new_chunks: list[str] = []
    chunk: list[str] = []
    i = 0
    while i < len(method_lines):
        line = method_lines[i]
        if line.strip().startswith("fn "):
            name_match = re.match(r"\s*fn\s+(\w+)\s*\(", line)
            if name_match and name_match.group(1) in existing_methods:
                i = _skip_fn_body(method_lines, i)
                if chunk:
                    new_chunks.append("\n".join(chunk).rstrip())
                    chunk = []
                continue
        chunk.append(line)
        i += 1
    if chunk:
        new_chunks.append("\n".join(chunk).rstrip())
    filtered = "\n\n".join(c for c in new_chunks if c.strip())
    if not filtered.strip():
        return content, False, f"all methods already present in impl {struct_name}"

    if not filtered.startswith("\n"):
        filtered = "\n\n" + filtered
    if not filtered.endswith("\n"):
        filtered = filtered + "\n"
    new_content = content[:body_end] + filtered + content[body_end:]
    return new_content, True, f"merged into impl {struct_name}"


def merge_impl_source(path: Path, pure_source: str, marker: str) -> PatchResult:
    """Merge `impl Type { ... }` methods into existing impl blocks when possible."""
    all_impls = _extract_all_impl_methods(pure_source)
    if not all_impls:
        return upsert_marked_block(path, wrap_scaffold(pure_source.rstrip() + "\n", marker), marker)

    if not path.exists():
        return upsert_marked_block(path, wrap_scaffold(pure_source.rstrip() + "\n", marker), marker)

    content = read_text(path)
    if has_marker(content, marker):
        return PatchResult(path, False, "already present")

    changed = False
    messages: list[str] = []
    for struct_name, methods in all_impls:
        content, one_changed, msg = _merge_one_impl(content, struct_name, methods)
        if one_changed:
            changed = True
        messages.append(msg)

    if not changed:
        leftover = [
            (n, m)
            for n, m in all_impls
            if _impl_body_span(content, n) is None
        ]
        if leftover:
            scaffold = "\n\n".join(f"impl {n} {{\n{m}\n}}" for n, m in leftover)
            return upsert_marked_block(path, wrap_scaffold(scaffold + "\n", marker), marker)
        return PatchResult(path, False, "; ".join(messages))

    content, _ = remove_marked_block(content, marker)
    write_text(path, content)
    return PatchResult(path, True, "; ".join(m for m in messages if m.startswith("merged")))


def patch_file(path: Path, transform) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "file not found")
    old = read_text(path)
    new, changed = transform(old)
    if changed:
        write_text(path, new)
    return PatchResult(path, changed, "updated" if changed else "unchanged")


def remove_line_with(content: str, fragment: str) -> tuple[str, bool]:
    lines = content.splitlines(keepends=True)
    new_lines = [ln for ln in lines if fragment not in ln]
    return "".join(new_lines), len(new_lines) != len(lines)


def strip_marker_from_file(path: Path, marker: str) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "not found")

    def transform(content: str) -> tuple[str, bool]:
        new_content, changed = remove_marked_block(content, marker)
        return new_content, changed

    return patch_file(path, transform)


def delete_if_marker_only(path: Path, marker: str) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "not found")
    content = read_text(path)
    if not has_marker(content, marker):
        return PatchResult(path, False, "no marker")
    stripped, _ = remove_marked_block(content, marker)
    if stripped.strip():
        return strip_marker_from_file(path, marker)
    path.unlink()
    return PatchResult(path, True, "deleted")


def append_extern_line(path: Path, line: str, marker: str) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "file not found")

    def transform(content: str) -> tuple[str, bool]:
        if has_marker(content, marker) or line.strip() in content:
            return content, False
        block = "\n".join(
            [
                marker_start(marker),
                line.rstrip(),
                marker_end(marker),
                "",
            ]
        )
        if not content.endswith("\n"):
            content += "\n"
        return content + block, True

    return patch_file(path, transform)
