from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


@dataclass
class PatchResult:
    path: Path
    changed: bool
    message: str


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")


def has_marker(content: str, marker: str) -> bool:
    return f"[builtin-dev:{marker}]" in content


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


_MARKER_BLOCK_RE = re.compile(
    r"(?:#|//) \[builtin-dev:[^\]]+\].*?(?:#|//) \[/builtin-dev:[^\]]+\]",
    re.DOTALL,
)


def _marker_block_spans(content: str) -> list[tuple[int, int]]:
    return [(m.start(), m.end()) for m in _MARKER_BLOCK_RE.finditer(content)]


def _find_c_function_span(content: str, name: str) -> tuple[int, int] | None:
    """Char span of a C function DEFINITION named `name`, brace-balanced.

    Returns (start-of-line, end-after-closing-brace-and-newline) or None if the
    definition is absent or its braces are unbalanced (in which case we refuse to
    touch it rather than risk corrupting the file). Braces inside string/char
    literals and comments are ignored so nested `{}` in the body are handled.
    """
    m = re.search(
        rf"(?m)^[A-Za-z_][A-Za-z0-9_ \t\*]*\b{re.escape(name)}\s*\([^;{{]*\)\s*\{{",
        content,
    )
    if not m:
        return None
    i = m.end() - 1  # position of the opening '{'
    n = len(content)
    depth = 0
    while i < n:
        ch = content[i]
        nxt = content[i + 1] if i + 1 < n else ""
        if ch == "/" and nxt == "/":  # line comment
            j = content.find("\n", i)
            i = n if j == -1 else j
            continue
        if ch == "/" and nxt == "*":  # block comment
            j = content.find("*/", i + 2)
            i = n if j == -1 else j + 2
            continue
        if ch in ('"', "'"):  # string / char literal
            quote = ch
            i += 1
            while i < n:
                if content[i] == "\\":
                    i += 2
                    continue
                if content[i] == quote:
                    i += 1
                    break
                i += 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                if end < n and content[end] == "\n":
                    end += 1
                return (m.start(), end)
        i += 1
    return None  # unbalanced — do not remove


def remove_c_function_def(content: str, name: str) -> tuple[str, str]:
    """Remove an UNMARKED C function definition named `name`, brace-balanced.

    Never deletes a definition that lives inside a `[builtin-dev:...]` marker
    block (that belongs to some feature and is removed via its own marker), and
    never corrupts multi-brace bodies. Returns (new_content, status) where status
    is one of: "removed", "skipped_marked", "none".
    """
    span = _find_c_function_span(content, name)
    if span is None:
        return content, "none"
    start, end = span
    for ms, me in _marker_block_spans(content):
        if start < me and ms < end:  # overlaps a marker block
            return content, "skipped_marked"
    return content[:start] + content[end:], "removed"


def remove_marked_block(content: str, marker: str) -> tuple[str, bool]:
    pattern = re.compile(
        rf"[ \t]*(?:#|//) \[builtin-dev:{re.escape(marker)}\].*?"
        rf"(?:#|//) \[/builtin-dev:{re.escape(marker)}\]\n?",
        re.DOTALL,
    )
    new_content, n = pattern.subn("", content)
    return new_content, n > 0


def insert_before(content: str, needle: str, insertion: str) -> tuple[str, bool]:
    idx = content.find(needle)
    if idx == -1:
        return content, False
    block = insertion if insertion.endswith("\n") else insertion + "\n"
    return content[:idx] + block + content[idx:], True


def insert_before_last(content: str, needle: str, insertion: str) -> tuple[str, bool]:
    idx = content.rfind(needle)
    if idx == -1:
        return content, False
    block = insertion if insertion.endswith("\n") else insertion + "\n"
    return content[:idx] + block + content[idx:], True


def add_to_matches_in_fn(content: str, fn_needle: str, item: str) -> tuple[str, bool]:
    """Add one `| "item"` entry to the matches! inside a specific function."""
    fn_idx = content.find(fn_needle)
    if fn_idx == -1:
        return content, False
    matches_idx = content.find("matches!(", fn_idx)
    if matches_idx == -1 or matches_idx > fn_idx + 400:
        return content, False
    close = content.find(")", matches_idx)
    if close == -1:
        return content, False
    segment = content[matches_idx:close]
    quoted = f'"{item.strip(chr(34))}"'
    if quoted in segment:
        return content, False
    trimmed = segment.rstrip()
    if trimmed.endswith("|"):
        new_segment = segment + f" {quoted}"
    else:
        new_segment = segment + f" | {quoted}"
    return content[:matches_idx] + new_segment + content[close:], True


def add_to_match_before_default(content: str, default_arm: str, arm: str) -> tuple[str, bool]:
    return insert_before(content, default_arm, arm + "\n            ")


def add_line_before_anchor(content: str, anchor: str, line: str, *, last: bool = False) -> tuple[str, bool]:
    if line.strip() in content:
        return content, False
    if last:
        return insert_before_last(content, anchor, line)
    return insert_before(content, anchor, line)


def add_tuple_line_before(content: str, anchor: str, line: str, *, last: bool = False) -> tuple[str, bool]:
    """Insert one line (e.g. a tuple entry) immediately before an anchor."""
    return add_line_before_anchor(content, anchor, line, last=last)


def add_to_rust_or_chain(content: str, fn_needle: str, item: str) -> tuple[str, bool]:
    """Add one `| "item"` entry to the matches! inside a specific Rust function."""
    return add_to_matches_in_fn(content, fn_needle, item)


def remove_line_with(content: str, fragment: str) -> tuple[str, bool]:
    lines = content.splitlines(keepends=True)
    new_lines = [ln for ln in lines if fragment not in ln]
    return "".join(new_lines), len(new_lines) != len(lines)


def remove_rust_match_arm(content: str, method: str) -> tuple[str, bool]:
    """Remove a `"method" => { ... }` match arm (marked or unmarked)."""
    changed = False
    marked = re.compile(
        rf'[ \t]*// \[builtin-dev:{re.escape(method)}:[^\]]+\].*?'
        rf'// \[/builtin-dev:{re.escape(method)}:[^\]]+\]\n?',
        re.DOTALL,
    )
    content, n1 = marked.subn("", content)
    changed = changed or n1 > 0
    unmarked = re.compile(
        rf'[ \t]*"{re.escape(method)}" => \{{\n(?:[ \t].*\n)*?[ \t]*\}}\n?',
    )
    content, n2 = unmarked.subn("", content)
    changed = changed or n2 > 0
    content, n3 = _cleanup_orphan_match_fragments(content)
    return content, changed or n3


def _cleanup_orphan_match_fragments(content: str) -> tuple[str, bool]:
    """Remove leftover lines from partial match-arm deletes (safe, conservative)."""
    changed = False
    content, n1 = re.subn(
        r"\n[ \t]*else \{\n[ \t]*self\.check_string_arg\(mc, 0, env, sp\);\n[ \t]*\}\n[ \t]*Type::String\n[ \t]*\}\n",
        "\n",
        content,
    )
    changed = changed or n1 > 0
    content, n2 = re.subn(r"\n[ \t]*\}\n[ \t]*\}\n+\n*_ => ", "\n            _ => ", content)
    changed = changed or n2 > 0
    content, n3 = re.subn(r"\n\n_ => ", "\n            _ => ", content)
    changed = changed or n3 > 0
    content, n4 = re.subn(r"\n\n_ => ExprValue", "\n            _ => ExprValue", content)
    return content, changed or n4 > 0


def insert_into_matches(content: str, fn_sig: str, quoted_item: str) -> tuple[str, str]:
    """Insert `| quoted_item` into the `matches!(…)` of the fn found via `fn_sig`.

    Robust to an early-return guard between the fn's `{` and its `matches!(` (e.g.
    `if method.starts_with("String_") { return true; }`), which a plain-anchor
    insert silently misses. `quoted_item` must include the quotes, e.g. `"trim"`.

    Returns (new_content, status) with status one of:
      * "added"     — item inserted
      * "present"   — item already in the matches! arm
      * "not_found" — fn or its matches! block could not be located
    """
    fn_idx = content.find(fn_sig)
    if fn_idx == -1:
        return content, "not_found"
    matches_idx = content.find("matches!(", fn_idx)
    if matches_idx == -1:
        return content, "not_found"
    close = content.find(")", matches_idx)
    if close == -1:
        return content, "not_found"
    segment = content[matches_idx:close]
    if quoted_item in segment:
        return content, "present"
    trimmed = segment.rstrip()
    if trimmed.endswith("|"):
        new_segment = trimmed + f" {quoted_item}"
    else:
        new_segment = trimmed + f" | {quoted_item}"
    new_segment += segment[len(trimmed):]
    return content[:matches_idx] + new_segment + content[close:], "added"


def remove_or_chain_item(content: str, item: str) -> tuple[str, bool]:
    quoted = f'"{item.strip(chr(34))}"'
    patterns = [
        rf"\s*\|\s*{re.escape(quoted)}",
        rf"{re.escape(quoted)}\s*\|\s*",
        re.escape(quoted),
    ]
    for pat in patterns:
        new_content, n = re.subn(pat, "", content, count=1)
        if n:
            return new_content, True
    return content, False


def append_block(path: Path, block: str, marker: str) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "file not found")
    content = read_text(path)
    if has_marker(content, marker):
        return PatchResult(path, False, "already present")
    if not content.endswith("\n"):
        content += "\n"
    write_text(path, content + block + ("\n" if not block.endswith("\n") else ""))
    return PatchResult(path, True, "appended")


def upsert_marked_block(path: Path, block: str, marker: str) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "file not found")
    content = read_text(path)
    if has_marker(content, marker):
        return PatchResult(path, False, "already present")
    content, _ = remove_marked_block(content, marker)
    if not content.endswith("\n"):
        content += "\n"
    write_text(path, content + block + "\n")
    return PatchResult(path, True, "inserted")


def write_new_file(path: Path, content: str, marker: str, force: bool = False) -> PatchResult:
    if path.exists() and not force:
        if has_marker(read_text(path), marker):
            return PatchResult(path, False, "already present")
        return PatchResult(path, False, "exists (use --force)")
    path.parent.mkdir(parents=True, exist_ok=True)
    write_text(path, content)
    return PatchResult(path, True, "created")


def patch_file(path: Path, transform) -> PatchResult:
    if not path.exists():
        return PatchResult(path, False, "file not found")
    old = read_text(path)
    new, changed = transform(old)
    if changed:
        write_text(path, new)
    return PatchResult(path, changed, "updated" if changed else "unchanged")
