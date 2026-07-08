#!/usr/bin/env python3
"""Generate examples/**/*.typed.ny from plain .ny siblings (optional explicit types)."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN_DIRS = [
    ROOT / "examples" / "builtins",
    ROOT / "examples" / "syntax",
]


def split_array_inner(inner: str) -> tuple[list[str], list[str]]:
    spreads: list[str] = []
    literals: list[str] = []
    for part in inner.split(","):
        p = part.strip()
        if not p:
            continue
        if p.startswith("..."):
            spreads.append(p[3:].strip())
        else:
            literals.append(p)
    return spreads, literals


def object_field_count(line: str) -> int | None:
    m = re.search(r"=\s*\{([^}]*)\}", line)
    if not m:
        return None
    inner = m.group(1).strip()
    if not inner:
        return 0
    return len([p for p in inner.split(",") if p.strip() and ":" in p])


def infer_literal_type(val: str) -> str:
    if re.match(r"^-?\d+$", val):
        return "i32"
    if re.match(r"^-?\d+\.\d+f32$", val):
        return "f32"
    if re.match(r"^-?\d+\.\d+$", val):
        return "f64"
    if val.startswith('"'):
        return "string"
    if val in ("true", "false"):
        return "bool"
    return "i32"


class LetContext:
    def __init__(self) -> None:
        self.array_lens: dict[str, int] = {}
        self.array_elem_types: dict[str, str] = {}
        self.obj_field_counts: dict[str, int] = {}
        self.obj_field_types: dict[str, str] = {}

    def note_let(self, name: str, line: str) -> None:
        m = re.match(r"\s*let\s+\w+(?::\s*(\[[^\]]+\]))?\s*=\s*(\[.+\])\s*$", line)
        if m:
            ann, rhs = m.groups()
            inner = rhs[1:-1]
            spreads, literals = split_array_inner(inner)
            if ann:
                am = re.match(r"\[(\w+);\s*(\d+)\]", ann)
                if am:
                    self.array_elem_types[name] = am.group(1)
                    self.array_lens[name] = int(am.group(2))
                    return
            if spreads or literals:
                n = len(literals)
                for var in spreads:
                    n += self.array_lens.get(var, self.obj_field_counts.get(var, 1))
                elem = infer_spread_array_elem_type(inner, self)
                self.array_elem_types[name] = elem
                self.array_lens[name] = n
            else:
                self.array_lens[name] = 0
                self.array_elem_types[name] = "i32"
            return

        n_fields = object_field_count(line)
        if n_fields is not None:
            self.obj_field_counts[name] = n_fields
            m2 = re.search(r"=\s*\{([^}]*)\}", line)
            if m2:
                vals = [
                    p.split(":", 1)[1].strip()
                    for p in m2.group(1).split(",")
                    if p.strip() and ":" in p
                ]
                if vals:
                    self.obj_field_types[name] = infer_literal_type(vals[0])


def array_len_literal(line: str, ctx: LetContext | None = None) -> int | None:
    m = re.search(r"=\s*\[([^\]]*)\]", line)
    if not m:
        return None
    inner = m.group(1).strip()
    if not inner:
        return 0
    spreads, literals = split_array_inner(inner)
    if spreads and ctx is not None:
        n = len(literals)
        for var in spreads:
            n += ctx.array_lens.get(var, ctx.obj_field_counts.get(var, 1))
        return n
    return len(spreads) + len(literals)


def infer_elem_type(inner: str) -> str:
    inner = inner.strip()
    if not inner:
        return "i32"
    spreads, literals = split_array_inner(inner)
    if spreads:
        return infer_literal_type(literals[0]) if literals else "i32"
    first = inner.split(",")[0].strip()
    return infer_literal_type(first)


def infer_spread_array_elem_type(inner: str, ctx: LetContext) -> str:
    spreads, literals = split_array_inner(inner.strip())
    for var in spreads:
        if var in ctx.array_elem_types:
            return ctx.array_elem_types[var]
        if var in ctx.obj_field_types:
            return ctx.obj_field_types[var]
    for lit in literals:
        return infer_literal_type(lit)
    return "i32"


def infer_int_literal_type(val: str) -> str:
    try:
        n = int(val, 10)
    except ValueError:
        return "i32"
    if n < -(2**31) or n > 2**31 - 1:
        return "i64"
    return "i32"


def type_let_line(line: str, ctx: LetContext | None = None) -> str:
    stripped = line.strip()
    if ": " in stripped.split("=", 1)[0]:
        return line

    # let x = 3.14159 or 0.5f32
    m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(-?\d+\.\d+(?:f32)?)\s*(?://.*)?$", line)
    if m:
        indent, name, val = m.groups()
        ty = "f32" if val.endswith("f32") else "f64"
        return f"{indent}let {name}: {ty} = {val}\n"

    # let mut sum = 0
    if re.match(r"(\s*)let\s+mut\s+(\w+)\s*=\s*(-?\d+)\s*(?://.*)?$", line):
        m = re.match(r"(\s*)let\s+mut\s+(\w+)\s*=\s*(-?\d+)\s*(?://.*)?$", line)
        assert m
        indent, name, val = m.groups()
        ty = infer_int_literal_type(val)
        return f"{indent}let mut {name}: {ty} = {val}\n"

    # let x = 42  (integer only — not 3.14159)
    if re.match(r"(\s*)let\s+(\w+)\s*=\s*(-?\d+)\s*(?://.*)?$", line):
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(-?\d+)\s*(?://.*)?$", line)
        if m:
            indent, name, val = m.groups()
            ty = infer_int_literal_type(val)
            return f"{indent}let {name}: {ty} = {val}\n"

    # let name = input(...)
    if re.match(r"(\s*)let\s+(\w+)\s*=\s*input\(", line):
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(input\(.+\))\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            return f"{indent}let {name}: string = {rhs}\n"

    # let s = "..."  (literal only — not "foo".split(...))
    if re.match(r'(\s*)let\s+(\w+)\s*=\s*"[^"]*"\s*$', line):
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(.+)\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            return f"{indent}let {name}: string = {rhs}\n"

    # let parts: VecStr = ....split(
    if ".split(" in line and re.match(r"\s*let\s+", line):
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(.+)\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            return f"{indent}let {name}: VecStr = {rhs}\n"

    if re.search(r"=\s*\[", line) and "split(" not in line and "sort()" not in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(\[.+\])\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            inner = rhs[1:-1]
            n = array_len_literal(line, ctx)
            if n is not None:
                if ctx is not None and "..." in inner:
                    elem = infer_spread_array_elem_type(inner, ctx)
                else:
                    elem = infer_elem_type(inner)
                return f"{indent}let {name}: [{elem}; {n}] = {rhs}\n"

    # let sorted = nums.sort()
    if ".sort()" in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(\w+)\.sort\(\)\s*$", line)
        if m:
            indent, name, src = m.groups()
            return f"{indent}let {name}: [i32; 5] = {src}.sort()\n"  # fixed below if needed

    # let parts: VecStr = ....split(
    if ".split(" in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(.+)\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            return f"{indent}let {name}: VecStr = {rhs}\n"
        return line

    # let arr = [..]
    if "date()" in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*date\(\)\s*$", line)
        if m:
            indent, name = m.groups()
            return f"{indent}let {name}: Date = date()\n"

    # let v = Vec_i32_new()
    if "Vec_i32_new()" in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*Vec_i32_new\(\)\s*$", line)
        if m:
            indent, name = m.groups()
            return f"{indent}let {name}: ptr = Vec_i32_new()\n"

    # let out = Array_map(...)
    if "Array_map" in line or "Array_filter" in line or "Array_reduce" in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(.+)\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            if "Array_reduce" in rhs:
                return f"{indent}let {name}: i32 = {rhs}\n"
            if "Array_find" in rhs:
                return f"{indent}let {name}: i32 = {rhs}\n"
            return f"{indent}let {name}: ptr = {rhs}\n"

    # let raw = JSON_stringify
    if "JSON_" in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*(.+)\s*$", line)
        if m:
            indent, name, rhs = m.groups()
            return f"{indent}let {name}: string = {rhs}\n"

    # let copied = clone trimmed
    if line.strip().startswith("let ") and " clone " in line:
        m = re.match(r"(\s*)let\s+(\w+)\s*=\s*clone\s+(\w+)\s*$", line)
        if m:
            indent, name, src = m.groups()
            return f"{indent}let {name}: string = clone {src}\n"

    return line


def fix_sort_array_types(text: str) -> str:
    """Match sorted array length to source array annotation."""
    lines = text.splitlines(keepends=True)
    arr_types: dict[str, str] = {}
    out: list[str] = []
    for line in lines:
        m = re.match(r"\s*let\s+(\w+):\s*(\[[^\]]+\])\s*=", line)
        if m:
            arr_types[m.group(1)] = m.group(2)
        m2 = re.match(r"(\s*)let\s+(\w+):\s*\[i32;\s*5\]\s*=\s*(\w+)\.sort\(\)", line)
        if m2:
            indent, dst, src = m2.groups()
            ty = arr_types.get(src, "[i32; 5]")
            out.append(f"{indent}let {dst}: {ty} = {src}.sort()\n")
            continue
        out.append(line)
    return "".join(out)


def transform_source(text: str) -> str:
    text = re.sub(r"fn main\(\)\s*\{", "fn main() -> void {", text)
    text = re.sub(r"fn main\(\)\s*->\s*void\s*->\s*void", "fn main() -> void", text)
    ctx = LetContext()
    lines: list[str] = []
    for line in text.splitlines(keepends=True):
        if line.lstrip().startswith("let ") or line.lstrip().startswith("let mut "):
            typed = type_let_line(line, ctx)
            lines.append(typed)
            m = re.match(r"\s*let\s+(?:mut\s+)?(\w+)", typed)
            if m:
                ctx.note_let(m.group(1), typed)
        else:
            lines.append(line)
    return fix_sort_array_types("".join(lines))


def typed_path(plain: Path) -> Path:
    return plain.with_name(f"{plain.stem}.typed{plain.suffix}")


def main() -> int:
    written = 0
    for base in SCAN_DIRS:
        if not base.is_dir():
            continue
        for plain in sorted(base.rglob("*.ny")):
            if not plain.is_file() or plain.name.endswith(".typed.ny"):
                continue
            out = typed_path(plain)
            typed = transform_source(plain.read_text(encoding="utf-8"))
            if out.exists() and out.read_text(encoding="utf-8") == typed:
                continue
            out.write_text(typed, encoding="utf-8")
            written += 1
            print(f"wrote {out.relative_to(ROOT)}")
    print(f"done ({written} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
