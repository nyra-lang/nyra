#!/usr/bin/env python3
"""Strip or add optional type annotations for webDocs Nyra snippets."""
from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "gen_typed_examples", ROOT / "make" / "py" / "gen-typed-examples.py"
)
_gen = importlib.util.module_from_spec(_spec)
assert _spec and _spec.loader
_spec.loader.exec_module(_gen)
transform_source = _gen.transform_source

TYPE_NAMES = frozenset(
    {
        "i32",
        "i64",
        "u32",
        "u64",
        "f32",
        "f64",
        "bool",
        "char",
        "string",
        "void",
        "ptr",
    }
)


def fix_struct_literal_commas(text: str) -> str:
    """Insert commas between struct literal fields (not struct definitions)."""
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    mode: str | None = None
    depth = 0

    def is_type_rhs(rhs: str) -> bool:
        rhs = rhs.strip().rstrip(",")
        if not rhs:
            return False
        parts = rhs.split()
        if len(parts) == 2 and parts[1] in ("Send", "Copy", "Drop"):
            rhs = parts[0]
        if rhs.startswith("[") or "<" in rhs:
            return True
        if rhs in TYPE_NAMES:
            return True
        return bool(re.match(r"^[A-Z]\w*$", rhs))

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.rstrip("\n\r")
        nl = line[len(stripped) :]

        if mode is None:
            if re.search(r"\bstruct\s+\w+", stripped) and "{" in stripped:
                mode = "def"
                depth = stripped.count("{") - stripped.count("}")
            elif re.search(r"\bstruct\s+\w+\s*$", stripped):
                mode = "def"
                depth = 0
            elif re.search(r"(?<![\w.])\b(return\s+)?[A-Z]\w*\s*\{", stripped) or re.search(
                r"=\s*\{", stripped
            ):
                mode = "lit"
                depth = max(1, stripped.count("{") - stripped.count("}"))

        if "{" in stripped and "}" in stripped and mode != "def":
            new_stripped = stripped
            while True:
                m = re.search(
                    r"(\{[^}\n]*?)(\"(?:\\.|[^\"\\])*\"|\d+(?:\.\d+)?|[A-Za-z_]\w*(?:\([^)]*\))?)\s+([A-Za-z_]\w*\s*:)",
                    new_stripped,
                )
                if not m:
                    break
                new_stripped = (
                    new_stripped[: m.start()]
                    + m.group(1)
                    + m.group(2)
                    + ", "
                    + m.group(3)
                    + new_stripped[m.end() :]
                )
            stripped = new_stripped

        if mode == "lit" and depth > 0:
            fm = re.match(r"^(\s*)([A-Za-z_]\w*)\s*:\s*(.+?)\s*$", stripped)
            if fm and not is_type_rhs(fm.group(3)):
                j = i + 1
                while j < len(lines) and not lines[j].strip():
                    j += 1
                if j < len(lines):
                    nxt = lines[j].rstrip("\n\r")
                    if re.match(r"^\s+[A-Za-z_]\w*\s*:", nxt) and not fm.group(3).rstrip().endswith(
                        ","
                    ):
                        stripped = f"{fm.group(1)}{fm.group(2)}: {fm.group(3).rstrip()},"

        out.append(stripped + nl)

        if mode == "def":
            if depth == 0 and stripped.strip() == "{":
                depth = 1
            elif "{" in stripped:
                depth += stripped.count("{")
            depth -= stripped.count("}")
            if depth <= 0:
                mode = None
                depth = 0
        elif mode == "lit":
            if "{" in stripped:
                if depth == 0:
                    depth = stripped.count("{")
                else:
                    depth += stripped.count("{")
            depth -= stripped.count("}")
            if depth <= 0:
                mode = None
                depth = 0
        i += 1

    return "".join(out)


def _keep_struct_declarations(text: str) -> bool:
    markers = (
        "impl ",
        "repr(",
        "trait ",
        "extern ",
        "..",
        " Send",
        "Drop for",
        "dyn ",
        "import ",
        ".ny",
        "#[",
        "constructor",
        "User(",
        "Point(",
        "Calculator {",
        "Counter {",
        "Counter(",
        "GzFile",
        "FileHandle",
        "UserRecord",
        "fn save(",
        "auto-borrow",
        "desugared to",
    )
    return any(m in text for m in markers)


def normalize_easy_snippet(text: str) -> str:
    """Prefer anonymous object literals in zero-types snippets when struct is not the lesson."""
    if _keep_struct_declarations(text):
        return text

    struct_blocks = list(
        re.finditer(
            r"^struct\s+(\w+)\s*\{[^}]*\}\s*$",
            text,
            flags=re.MULTILINE,
        )
    )
    if not struct_blocks:
        return text

    names = [m.group(1) for m in struct_blocks]
    for name in names:
        text = re.sub(rf"\breturn\s+{name}\s*\{{", "return {", text)
        text = re.sub(rf"=\s*{name}\s*\{{", "= {", text)
        text = re.sub(rf"\b{name}\s*\(", f"__CTOR_{name}(", text)

    for m in reversed(struct_blocks):
        text = text[: m.start()] + text[m.end() :]

    # restore ctor sugar names for docs that show User("Ada") without struct decl
    for name in names:
        text = text.replace(f"__CTOR_{name}(", f"{name}(")

    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip() + "\n"


FIELD_TYPE_HINTS: dict[frozenset[str], str] = {
    frozenset({"name", "age"}): "User",
    frozenset({"value"}): "Calculator",
    frozenset({"x", "y"}): "Point",
}


def fix_doc_snippet_formatting(text: str) -> str:
    """Restore newlines after virtual file labels in import snippets."""
    return re.sub(r"(\.ny)(import |struct |enum |fn )", r"\1\n\2", text)


def _infer_struct_from_fields(fields: set[str]) -> str | None:
    if not fields:
        return None
    for fset, name in FIELD_TYPE_HINTS.items():
        if fields <= fset:
            return name
    return None


def _wants_borrow(fn_name: str, param: str, body: str, easy: str) -> bool:
    if fn_name == "save" or (param == "u" and ".name" in body):
        return True
    if "auto-borrow" in easy or "desugared to" in easy or "rewrites to" in easy:
        return True
    return False


def _match_enum_type(body: str, param: str, enums: set[str]) -> str | None:
    if not re.search(rf"\bmatch\s+{param}\b", body):
        return None
    for en in enums:
        if re.search(rf"\b{en}\.", body):
            return en
    found = re.findall(r"\b(\w+)\.\w+", body)
    for name in found:
        if name[0].isupper():
            return name
    return None


def enrich_trait_signatures(typed: str) -> str:
    """Copy concrete impl signatures onto matching trait method declarations."""
    impl_methods: dict[tuple[str, str], str] = {}
    for m in re.finditer(r"impl\s+(\w+)\s+for\s+\w+\s*\{", typed):
        trait = m.group(1)
        body = _fn_body(typed, m.end() - 1)
        for fm in re.finditer(
            r"fn\s+(\w+)\s*\(([^)]*)\)(\s*->\s*[\w&<>\s]+)?",
            body,
        ):
            ret = fm.group(3) or ""
            if not ret and "print(" in body:
                ret = " -> void"
            impl_methods[(trait, fm.group(1))] = f"({fm.group(2)}){ret}"

    for (trait, method), sig in impl_methods.items():
        typed = re.sub(
            rf"(trait {re.escape(trait)} \{{[\s\S]*?^\s*fn {re.escape(method)})\s*\([^)]*\)(?:\s*->\s*[\w&<>\s]+)?",
            rf"\1{sig}",
            typed,
            count=1,
            flags=re.MULTILINE,
        )
    return typed


def enrich_typed_anonymous_objects(typed: str, easy: str) -> str:
    """Add struct declarations when zero-types tabs use anonymous object literals."""
    if "struct User" in typed:
        return typed
    m = re.search(
        r'let\s+(\w+)\s*=\s*\{\s*name:\s*"[^"]*"\s*,\s*age:\s*\d+\s*\}',
        easy,
    )
    if not m or "u.name" not in easy:
        return typed
    header = "struct User {\n    name: string\n    age: i32\n}\n\n"
    typed = header + typed
    typed = re.sub(
        r'let\s+user\s*=\s*\{',
        "let user: User = User {",
        typed,
    )
    typed = re.sub(
        r'let\s+user\s*:\s*User\s*=\s*\{',
        "let user: User = User {",
        typed,
    )
    return typed


def _struct_field_map(text: str) -> dict[str, set[str]]:
    out: dict[str, set[str]] = {}
    for m in re.finditer(r"struct\s+(\w+)\s*\{([^}]*)\}", text, re.S):
        fields = {
            fm.group(1)
            for fm in re.finditer(r"^\s*(\w+)\s*:", m.group(2), re.M)
        }
        out[m.group(1)] = fields
    return out


def _enum_names(text: str) -> set[str]:
    return set(re.findall(r"\benum\s+(\w+)", text))


def _fn_body(text: str, start: int) -> str:
    depth = 0
    i = start
    while i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start : i + 1]
        i += 1
    return text[start:]


def _param_only_read(body: str, param: str) -> bool:
    if re.search(rf"\b{param}\s*=", body):
        return False
    if re.search(rf"&mut\s+{param}\b", body):
        return False
    return True


def refine_function_param_types(typed: str, easy: str) -> str:
    structs = _struct_field_map(typed) or _struct_field_map(easy)
    enums = _enum_names(typed) or _enum_names(easy)
    known_returns: dict[str, str] = {}

    for m in re.finditer(
        r"fn\s+(\w+)\s*(?:<[^>]+>)?\s*\([^)]*\)\s*(?:->\s*([\w&<>\s]+?))?\s*\{",
        typed,
    ):
        if m.group(2):
            known_returns[m.group(1)] = m.group(2).strip()

    def infer_param(param: str, fn_name: str, body: str) -> str | None:
        if param == "self":
            return None
        enum_ty = _match_enum_type(body, param, enums)
        if enum_ty:
            return enum_ty
        fields = set(re.findall(rf"\b{param}\.(\w+)", body))
        if fields:
            for sname, sfields in structs.items():
                if fields <= sfields:
                    if _wants_borrow(fn_name, param, body, easy):
                        return f"&{sname}"
                    if _param_only_read(body, param) and fn_name not in (
                        "add_calc",
                        "origin",
                    ):
                        return f"&{sname}"
                    return sname
            hinted = _infer_struct_from_fields(fields)
            if hinted:
                if _wants_borrow(fn_name, param, body, easy):
                    return f"&{hinted}"
                if _param_only_read(body, param) and fn_name not in (
                    "add_calc",
                    "origin",
                ):
                    return f"&{hinted}"
                return hinted
        if re.search(rf"\b(strlen|strstr_pos)\({param}\)", body):
            return "&string"
        # Prefer call-site cast: both(c as dyn Add + Scale) / via_dyn(x as dyn Add)
        cast = re.search(
            rf"\b{re.escape(fn_name)}\([^)]*\bas\s+(dyn\s+\w+(?:\s*\+\s*\w+)*)",
            easy,
        )
        if cast:
            return cast.group(1)
        if re.search(rf"\b{param}\.add\(", body) and re.search(
            rf"\b{param}\.scale\(", body
        ):
            return "dyn Add + Scale"
        if re.search(rf"\b{param}\.add\(", body) and param in ("g", "c"):
            return "dyn Add"
        if fn_name.startswith("health_") and param == "ctx":
            return "RequestContext"
        if param == "data" and "#[no_escape]" in body:
            return "&string"
        return None

    def repl(m: re.Match[str]) -> str:
        prefix, fname, tparams, params, suffix = m.groups()
        if not params.strip():
            return m.group(0)
        fn_start = m.end()
        body = _fn_body(typed, fn_start - 1)
        parts = []
        for p in params.split(","):
            p = p.strip()
            if not p or p == "self":
                parts.append(p)
                continue
            if ":" in p:
                name, ty = p.split(":", 1)
                name = name.strip()
                ty = ty.strip()
                inferred = infer_param(name, fname, body)
                if inferred and ty in ("i32", "string"):
                    parts.append(f"{name}: {inferred}")
                else:
                    parts.append(p)
                continue
            inferred = infer_param(p, fname, body)
            if inferred:
                parts.append(f"{p}: {inferred}")
            elif p in ("a", "b", "x", "y", "n", "i", "j", "acc", "slot", "id", "delta"):
                parts.append(f"{p}: i32")
            elif p in ("s", "name", "key", "msg", "text", "greeting", "label", "path"):
                parts.append(f"{p}: string")
            else:
                parts.append(f"{p}: i32")
        tp = tparams or ""
        return f"{prefix}fn {fname}{tp}({', '.join(parts)}){suffix}"

    return re.sub(
        r"^(\s*)fn\s+(\w+)(\s*<[^>]+>)?\s*\(([^)]*)\)(\s*(?:->\s*[\w&<>\s]+\s*)?[\{=])",
        repl,
        typed,
        flags=re.MULTILINE,
    )


def _fix_return_types(typed: str, easy: str) -> str:
    structs = _struct_field_map(typed) or _struct_field_map(easy)

    def repl(m: re.Match[str]) -> str:
        fname = m.group(1)
        body = _fn_body(typed, m.end() - 1)
        if "->" in m.group(0):
            return m.group(0)
        if fname == "main":
            return m.group(0)
        ret = infer_fn_return(body)
        if ret:
            return m.group(0).replace(") {", f") -> {ret} {{", 1)
        if re.search(rf"\breturn\s+({'|'.join(structs.keys())})\s*\{{", body):
            for sname in structs:
                if re.search(rf"\breturn\s+{sname}\s*\{{", body):
                    return m.group(0).replace(") {", f") -> {sname} {{", 1)
        if "return Point" in body:
            return m.group(0).replace(") {", ") -> Point {", 1)
        if "return Calculator" in body:
            return m.group(0).replace(") {", ") -> Calculator {", 1)
        return m.group(0)

    return re.sub(
        r"fn\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)\s*\{",
        repl,
        typed,
    )


def strip_optional_types(text: str) -> str:
    """Canonical easy (without types) snippet for docs."""
    text = (
        text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", '"')
        .replace("&amp;", "&")
    )

    text = re.sub(r"\bfn\s+main\(\)\s*->\s*void\s*\{", "fn main() {", text)
    text = re.sub(
        r"(\bfn\s+\w+(?:<[^>]*>)?\s*\([^)]*\))\s*->\s*[^\{=\n]+(\s*[\{=])",
        r"\1\2",
        text,
    )
    # Trait method signatures (no `{` on the same line)
    text = re.sub(
        r"(^\s*fn\s+\w+\s*\([^)]*\))\s*->\s*[\w&<>\[\]]+\s*$",
        r"\1",
        text,
        flags=re.MULTILINE,
    )

    def strip_params(m: re.Match[str]) -> str:
        inner = m.group(1)
        if ":" not in inner:
            return m.group(0)
        parts = []
        depth = 0
        cur = ""
        for ch in inner:
            if ch in "([":
                depth += 1
            elif ch in ")]":
                depth -= 1
            if ch == "," and depth == 0:
                parts.append(cur.strip())
                cur = ""
            else:
                cur += ch
        if cur.strip():
            parts.append(cur.strip())
        cleaned = []
        for p in parts:
            p = p.strip()
            if not p or p == "self":
                cleaned.append(p)
                continue
            if ":" in p:
                name, ty = p.split(":", 1)
                name = name.strip()
                ty = ty.strip()
                # Keep type-parameter and reference annotations needed for runnable snippets.
                if re.match(r"^[A-Z]\w*$", ty) or re.match(r"^&(?:mut\s+)?[A-Z]\w*$", ty):
                    cleaned.append(f"{name}: {ty}")
                else:
                    cleaned.append(name)
            else:
                cleaned.append(p)
        return "(" + ", ".join(cleaned) + ")"

    # Only strip parameter type annotations on fn/method signatures — not call arguments
    # (call strings like `"https://host"` and `{"k":"v"}` contain `:` and must stay intact).
    text = re.sub(
        r"^(\s*(?:fn|async fn)\s+\w+(?:<[^>]*>)?\s*\([^)]*\))",
        lambda m: re.sub(r"\(([^()]*)\)", strip_params, m.group(1)),
        text,
        flags=re.MULTILINE,
    )
    text = re.sub(r"(\blet\s+mut\s+\w+)\s*:\s*[^=]+=", r"\1 =", text)
    text = re.sub(r"(\blet\s+\w+)\s*:\s*[^=]+=", r"\1 =", text)

    # generic call-site args only (not `fn id<T>(`) — keep `<T>` at call sites so snippets compile
    # (e.g. `id<i32>(42)`); inference-only `id(42)` is not required for the easy tab.

    text = re.sub(r"(\blet\s+\w+)\s*:\s*\([^)]+\)\s*=", r"\1 =", text)
    return text.rstrip() + ("\n" if text.endswith("\n") else "")


def infer_fn_return(body: str) -> str | None:
    struct_ret = re.search(r"\breturn\s+([A-Z]\w*)\s*\{", body)
    if struct_ret:
        return struct_ret.group(1)
    if re.search(r"\breturn\s+[^;\n]+", body):
        if re.search(r'return\s+"', body):
            return "string"
        if re.search(r"return\s+true|return\s+false", body):
            return "bool"
        if re.search(r"return\s+\d+\.\d+", body):
            return "f64"
        return "i32"
    return None


def add_explicit_types(text: str) -> str:
    """Typed tab — explicit annotations where inferrable."""
    text = (
        text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", '"')
        .replace("&amp;", "&")
    )
    easy = strip_optional_types(text)
    text = transform_source(easy)

    generic_fns = {m.group(1) for m in re.finditer(r"\bfn\s+(\w+)\s*<", easy)}
    text = re.sub(
        r"^(\s*)fn\s+(\w+)<T>\s*\((\w+)\)\s*\{",
        r"\1fn \2<T>(\3: T) -> T {",
        text,
        flags=re.MULTILINE,
    )

    def add_param_types(m: re.Match[str]) -> str:
        prefix, name, tparams, params, suffix = m.groups()
        if not params.strip():
            return m.group(0)
        parts = [p.strip() for p in params.split(",")]
        typed_parts = []
        for p in parts:
            if ":" in p or p == "self":
                typed_parts.append(p)
            elif p in ("a", "b", "x", "y", "n", "i", "j", "acc"):
                typed_parts.append(f"{p}: i32")
            elif p in ("s", "name", "key", "msg", "text", "greeting"):
                typed_parts.append(f"{p}: string")
            else:
                typed_parts.append(f"{p}: i32")
        tp = tparams or ""
        return f"{prefix}fn {name}{tp}({', '.join(typed_parts)}){suffix}"

    text = re.sub(
        r"^(\s*)fn\s+(\w+)(\s*<[^>]+>)?\s*\(([^)]*)\)(\s*(?:->\s*[\w\[\]]+\s*)?[\{=])",
        add_param_types,
        text,
        flags=re.MULTILINE,
    )

    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match(
            r"^(\s*)fn\s+(\w+)\s*(<[^>]*>)?\s*\(([^)]*)\)\s*\{\s*$",
            line.rstrip("\n"),
        )
        if (
            m
            and m.group(2) != "main"
            and "->" not in line
            and m.group(2) not in generic_fns
        ):
            body_lines = [line]
            depth = line.count("{") - line.count("}")
            j = i + 1
            while j < len(lines) and depth > 0:
                body_lines.append(lines[j])
                depth += lines[j].count("{") - lines[j].count("}")
                j += 1
            body = "".join(body_lines)
            ret = infer_fn_return(body)
            if ret:
                line = line.replace(") {", f") -> {ret} {{", 1)
            out.append(line)
            out.extend(body_lines[1:])
            i = j
            continue
        out.append(line)
        i += 1
    text = "".join(out)

    for fn in generic_fns:
        text = re.sub(rf"\b{re.escape(fn)}\((\d+)\)", rf"{fn}<i32>(\1)", text)
        text = re.sub(rf'\b{re.escape(fn)}\("', rf'{fn}<string>("', text)

    struct_names = set(re.findall(r"\bstruct\s+(\w+)", text))
    for s in struct_names:
        text = re.sub(
            rf"(\blet\s+\w+)\s*=\s*{re.escape(s)}\s*\{{",
            rf"\1: {s} = {s} {{",
            text,
        )
        text = re.sub(
            rf"(\blet\s+\w+)\s*=\s*{re.escape(s)}\(",
            rf"\1: {s} = {s}(",
            text,
        )

    text = refine_function_param_types(text, easy)
    text = _fix_return_types(text, easy)
    text = enrich_typed_anonymous_objects(text, easy)
    text = enrich_trait_signatures(text)
    text = fix_struct_literal_commas(text)
    return text.rstrip() + ("\n" if text.endswith("\n") else "")


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: snippet-types.py strip|add -", file=sys.stderr)
        return 2
    mode = sys.argv[1]
    src = sys.stdin.read() if sys.argv[2] == "-" else Path(sys.argv[2]).read_text(encoding="utf-8")
    if mode == "strip":
        print(strip_optional_types(src), end="")
    elif mode == "add":
        print(add_explicit_types(src), end="")
    else:
        print(f"unknown mode: {mode}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
