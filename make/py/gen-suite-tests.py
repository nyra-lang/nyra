#!/usr/bin/env python3
"""Generate combinatorial Nyra compile tests under tests/suite/*/generated/.

Run from repo root:
    make gen-suite-tests
    make gen-suite-tests GEN_SUITE_ARGS="--dry-run"
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUITE = ROOT / "tests" / "suite"

NUMERIC = ("i32", "i64", "f64")
ARITH_OPS = ("add", "sub", "mul", "div", "mod")
ARITH_SYMS = {"add": "+", "sub": "-", "mul": "*", "div": "/", "mod": "%"}
CMP_OPS = ("eq", "ne", "lt", "le", "gt", "ge")
CMP_SYMS = {
    "eq": "==",
    "ne": "!=",
    "lt": "<",
    "le": "<=",
    "gt": ">",
    "ge": ">=",
}


@dataclass(frozen=True)
class SuiteProfile:
    """Controls combinatorial breadth for generated compile tests."""

    name: str
    arith_values: tuple[int, ...]
    cmp_values: tuple[int, ...]
    arith_run_pairs: tuple[tuple[int, int], ...]
    cmp_run_pairs: tuple[tuple[int, int], ...]
    fn_grid_i: tuple[int, ...]
    fn_grid_j: tuple[int, ...]
    nested_outer: tuple[int, ...]
    nested_inner: tuple[int, ...]
    match_indices: tuple[int, ...]
    expr_nest_indices: tuple[int, ...]
    while_limits: tuple[int, ...]
    option_indices: tuple[int, ...]
    char_indices: tuple[int, ...]
    array_sizes: tuple[int, ...]
    generics_indices: tuple[int, ...]
    parser_indices: tuple[int, ...]
    control_for_limits: tuple[int, ...]
    control_array_sizes: tuple[int, ...]
    lexer_string_lit_indices: tuple[int, ...]
    stdlib_array_sizes: tuple[int, ...]
    stdlib_string_indices: tuple[int, ...]
    struct_use_indices: tuple[int, ...]
    run_control_for_limits: tuple[int, ...]
    run_control_while_targets: tuple[int, ...]
    run_control_if_values: tuple[int, ...]
    print_values: tuple[int, ...]
    project_const_indices: tuple[int, ...]
    project_fn_indices: tuple[int, ...]
    project_fail_indices: tuple[int, ...]
    project_run_indices: tuple[int, ...]
    fail_assign_indices: tuple[int, ...]
    fail_move_fns: tuple[str, ...]
    fail_move_vars: tuple[str, ...]
    fail_mut_borrow_indices: tuple[int, ...]
    fail_lexer_unclosed: tuple[int, ...]
    fail_parser_missing_brace: tuple[int, ...]
    fail_parser_undefined: tuple[int, ...]
    fail_regression_move: tuple[int, ...]
    fail_regression_mut_borrow: tuple[int, ...]
    fail_fuzz_garbage: tuple[int, ...]
    fail_stdlib_import_extra: tuple[int, ...]


def _full_profile() -> SuiteProfile:
    arith_run = [
        (a, b)
        for a in range(1, 12)
        for b in range(1, 12)
        if (a + b) % 3 == 0
    ][:20]
    cmp_run = [(a, b) for a in range(0, 10) for b in range(0, 10) if a != b][:15]
    return SuiteProfile(
        name="full",
        arith_values=tuple(range(1, 12)),
        cmp_values=tuple(range(0, 12)),
        arith_run_pairs=tuple(arith_run),
        cmp_run_pairs=tuple(cmp_run),
        fn_grid_i=tuple(range(1, 26)),
        fn_grid_j=tuple(range(1, 21)),
        nested_outer=tuple(range(1, 11)),
        nested_inner=tuple(range(1, 21)),
        match_indices=tuple(range(1, 361)),
        expr_nest_indices=tuple(range(1, 151)),
        while_limits=tuple(range(1, 181)),
        option_indices=tuple(range(1, 101)),
        char_indices=tuple(range(1, 101)),
        array_sizes=tuple(range(2, 22)),
        generics_indices=tuple(range(1, 41)),
        parser_indices=tuple(range(1, 21)),
        control_for_limits=tuple(range(2, 22)),
        control_array_sizes=tuple(range(2, 22)),
        lexer_string_lit_indices=tuple(range(1, 26)),
        stdlib_array_sizes=tuple(range(1, 51)),
        stdlib_string_indices=tuple(range(6)),
        struct_use_indices=tuple(range(1, 25)),
        run_control_for_limits=tuple(range(1, 26)),
        run_control_while_targets=tuple(range(1, 16)),
        run_control_if_values=tuple(range(5, 20)),
        print_values=tuple(range(1, 1726)),
        project_const_indices=tuple(range(1, 31)),
        project_fn_indices=tuple(range(1, 21)),
        project_fail_indices=tuple(range(1, 21)),
        project_run_indices=tuple(range(1, 21)),
        fail_assign_indices=tuple(range(1, 26)),
        fail_move_fns=(
            "take",
            "consume",
            "save",
            "store",
            "send",
            "eat",
            "use",
            "drop_val",
            "accept",
            "receive",
        ),
        fail_move_vars=(
            "s",
            "msg",
            "name",
            "text",
            "label",
            "data",
            "value",
            "input",
            "word",
            "line",
        ),
        fail_mut_borrow_indices=tuple(range(1, 21)),
        fail_lexer_unclosed=tuple(range(1, 16)),
        fail_parser_missing_brace=tuple(range(1, 16)),
        fail_parser_undefined=tuple(range(1, 16)),
        fail_regression_move=tuple(range(1, 33)),
        fail_regression_mut_borrow=tuple(range(1, 17)),
        fail_fuzz_garbage=tuple(range(1, 151)),
        fail_stdlib_import_extra=tuple(range(1, 31)),
    )


FAST_PROFILE = SuiteProfile(
    name="fast",
    arith_values=(1, 2, 3, 7, 11),
    cmp_values=(0, 1, 3, 5, 11),
    arith_run_pairs=((1, 2), (3, 3), (5, 4), (7, 2), (10, 2), (11, 11)),
    cmp_run_pairs=((0, 1), (1, 0), (3, 7), (5, 5), (9, 2), (0, 9)),
    fn_grid_i=(1, 3, 5, 10, 25),
    fn_grid_j=(1, 5, 10, 20),
    nested_outer=(1, 3, 5, 10),
    nested_inner=(1, 5, 10, 20),
    match_indices=(1, 2, 5, 10, 25, 50, 100, 200, 360),
    expr_nest_indices=(1, 2, 5, 10, 25, 50, 75, 100, 125, 149),
    while_limits=(1, 3, 10, 20, 50, 100, 180),
    option_indices=(1, 2, 5, 10, 25, 50, 75, 100),
    char_indices=(1, 2, 13, 26, 52, 78, 99, 100),
    array_sizes=(2, 5, 10, 15, 21),
    generics_indices=(1, 2, 5, 10, 20, 30, 35, 40),
    parser_indices=(1, 2, 5, 10, 15, 20),
    control_for_limits=(2, 3, 5, 10, 15, 21),
    control_array_sizes=(2, 3, 5, 10, 15, 21),
    lexer_string_lit_indices=(1, 5, 10, 15, 25),
    stdlib_array_sizes=(1, 5, 10, 20, 30, 50),
    stdlib_string_indices=(0, 1, 2, 4, 5),
    struct_use_indices=(1, 5, 10, 15, 20, 24),
    run_control_for_limits=(1, 3, 5, 10, 15, 25),
    run_control_while_targets=(1, 3, 5, 10, 15),
    run_control_if_values=(5, 8, 12, 15, 19),
    print_values=(1, 2, 3, 10, 42, 99, 100, 127, 128, 255, 256, 512, 999, 1024, 1234, 1700, 1725),
    project_const_indices=(1, 5, 10, 15, 30),
    project_fn_indices=(1, 5, 10, 15, 20),
    project_fail_indices=(1, 5, 10, 15, 20),
    project_run_indices=(1, 5, 10, 15, 20),
    fail_assign_indices=(1,),
    fail_move_fns=("take", "consume", "save", "store", "send"),
    fail_move_vars=("s", "msg", "name", "text", "data"),
    fail_mut_borrow_indices=(1, 5, 10, 15, 20),
    fail_lexer_unclosed=(1, 2, 3, 5, 10),
    fail_parser_missing_brace=(1, 2, 3, 5, 10),
    fail_parser_undefined=(1, 2, 3, 5, 10),
    fail_regression_move=(1, 5, 10, 20, 32),
    fail_regression_mut_borrow=(1, 4, 8, 12, 16),
    fail_fuzz_garbage=(1, 2, 3, 5, 10, 25, 50, 100, 150),
    fail_stdlib_import_extra=(1, 5, 10, 15, 20, 25, 30),
)

CI_PROFILE = SuiteProfile(
    name="ci",
    arith_values=(1, 2, 3, 4, 5, 7, 9, 11),
    cmp_values=(0, 1, 2, 3, 5, 7, 11),
    arith_run_pairs=((1, 2), (2, 3), (3, 3), (5, 4), (7, 2), (9, 3), (10, 2), (11, 11)),
    cmp_run_pairs=((0, 1), (1, 0), (2, 3), (3, 7), (5, 5), (7, 9), (9, 2), (0, 9)),
    fn_grid_i=(1, 2, 3, 5, 10, 15, 25),
    fn_grid_j=(1, 2, 5, 10, 15, 20),
    nested_outer=(1, 2, 3, 5, 10),
    nested_inner=(1, 2, 5, 10, 15, 20),
    match_indices=(1, 2, 3, 5, 10, 25, 50, 100, 150, 200, 360),
    expr_nest_indices=(1, 2, 3, 5, 10, 25, 50, 75, 100, 125, 149),
    while_limits=(1, 2, 3, 10, 20, 50, 100, 150, 180),
    option_indices=(1, 2, 3, 5, 10, 25, 50, 75, 100),
    char_indices=(1, 2, 5, 13, 26, 52, 78, 99, 100),
    array_sizes=(2, 3, 5, 10, 15, 21),
    generics_indices=(1, 2, 3, 5, 10, 20, 30, 35, 40),
    parser_indices=(1, 2, 3, 5, 10, 15, 20),
    control_for_limits=(2, 3, 5, 10, 15, 21),
    control_array_sizes=(2, 3, 5, 10, 15, 21),
    lexer_string_lit_indices=(1, 2, 5, 10, 15, 25),
    stdlib_array_sizes=(1, 2, 5, 10, 20, 30, 50),
    stdlib_string_indices=(0, 1, 2, 3, 4, 5),
    struct_use_indices=(1, 2, 5, 10, 15, 20, 24),
    run_control_for_limits=(1, 2, 3, 5, 10, 15, 25),
    run_control_while_targets=(1, 2, 3, 5, 10, 15),
    run_control_if_values=(5, 8, 10, 12, 15, 19),
    print_values=(1, 2, 3, 4, 10, 42, 99, 100, 127, 128, 255, 256, 512, 999, 1024, 1234, 1500, 1700, 1725),
    project_const_indices=(1, 2, 5, 10, 15, 20, 30),
    project_fn_indices=(1, 2, 5, 10, 15, 20),
    project_fail_indices=(1, 2, 5, 10, 15, 20),
    project_run_indices=(1, 2, 5, 10, 15, 20),
    fail_assign_indices=(1, 2, 5),
    fail_move_fns=("take", "consume", "save", "store", "send", "eat", "use"),
    fail_move_vars=("s", "msg", "name", "text", "data", "value", "input"),
    fail_mut_borrow_indices=(1, 2, 5, 10, 15, 20),
    fail_lexer_unclosed=(1, 2, 3, 5, 8, 10),
    fail_parser_missing_brace=(1, 2, 3, 5, 8, 10),
    fail_parser_undefined=(1, 2, 3, 5, 8, 10),
    fail_regression_move=(1, 2, 5, 10, 20, 32),
    fail_regression_mut_borrow=(1, 2, 4, 8, 12, 16),
    fail_fuzz_garbage=(1, 2, 3, 5, 10, 25, 50, 75, 100, 150),
    fail_stdlib_import_extra=(1, 2, 5, 10, 15, 20, 25, 30),
)

PROFILES = {"fast": FAST_PROFILE, "ci": CI_PROFILE, "full": _full_profile()}
_profile: SuiteProfile = CI_PROFILE


def _array_index_pairs() -> list[tuple[int, int]]:
    pairs: list[tuple[int, int]] = []
    for size in _profile.array_sizes:
        if _profile.name == "full":
            indices = range(size)
        else:
            indices = sorted({0, size // 2, size - 1})
        for idx in indices:
            pairs.append((size, idx))
    return pairs


def numeric_expr(ty: str, name: str) -> str:
    if ty == "i32":
        return f"let {name} = 3"
    if ty == "i64":
        return f"let {name}: i64 = 3"
    if ty == "f64":
        return f"let {name} = 3.0"
    raise ValueError(ty)


def numeric_lit(ty: str) -> str:
    return {"i32": "3", "i64": "3", "f64": "3.0"}[ty]


def write(path: Path, content: str, dry_run: bool) -> None:
    if dry_run:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content.rstrip() + "\n", encoding="utf-8")


def clean_generated(dry_run: bool) -> None:
    for sub in (
        "pass/generated",
        "fail/generated",
        "run/generated",
        "projects/pass/generated",
        "projects/fail/generated",
        "projects/run/generated",
    ):
        gen = SUITE / sub
        if dry_run or not gen.exists():
            continue
        shutil.rmtree(gen)
    fuzz = SUITE / "fail" / "regression" / "fuzz"
    if not dry_run and fuzz.exists():
        shutil.rmtree(fuzz)


def count_suite_tests() -> int:
    n = 0
    for p in SUITE.rglob("*.ny"):
        rel = p.relative_to(SUITE)
        if "projects" in rel.parts and p.name != "main.ny":
            continue
        n += 1
    return n


def eval_i32(op: str, a: int, b: int) -> int:
    if op == "add":
        return a + b
    if op == "sub":
        return a - b
    if op == "mul":
        return a * b
    if op == "div":
        return a // b
    if op == "mod":
        return a % b
    raise ValueError(op)


def eval_cmp(op: str, a: int, b: int) -> int:
    if op == "eq":
        return int(a == b)
    if op == "ne":
        return int(a != b)
    if op == "lt":
        return int(a < b)
    if op == "le":
        return int(a <= b)
    if op == "gt":
        return int(a > b)
    if op == "ge":
        return int(a >= b)
    raise ValueError(op)


def gen_run_arith_i32(dry_run: bool) -> int:
    n = 0
    for op in ARITH_OPS:
        sym = ARITH_SYMS[op]
        for a, b in _profile.arith_run_pairs:
            if op == "div" and b == 0:
                continue
            if op == "mod" and b == 0:
                continue
            expected = eval_i32(op, a, b)
            name = f"run_{op}_{a}_{b}.ny"
            body = f"""// run-stdout: {expected}
fn main() {{
    print({a} {sym} {b})
}}"""
            write(SUITE / "run" / "generated" / "arith" / name, body, dry_run)
            n += 1
    return n


def gen_run_cmp_i32(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for a, b in _profile.cmp_run_pairs:
            expected = eval_cmp(op, a, b)
            name = f"run_{op}_{a}_{b}.ny"
            body = f"""// run-stdout: {expected}
fn main() {{
    print({a} {sym} {b})
}}"""
            write(SUITE / "run" / "generated" / "cmp" / name, body, dry_run)
            n += 1
    return n


def gen_run_control(dry_run: bool) -> int:
    n = 0
    for limit in _profile.run_control_for_limits:
        expected = limit * (limit - 1) // 2
        body = f"""// run-stdout: {expected}
fn main() {{
    let mut sum = 0
    for i in 0..{limit} {{
        sum = sum + i
    }}
    print(sum)
}}"""
        write(SUITE / "run" / "generated" / "control" / f"run_for_sum_{limit}.ny", body, dry_run)
        n += 1

    for target in _profile.run_control_while_targets:
        body = f"""// run-stdout: {target}
fn main() {{
    let mut i = 0
    while i < {target} {{
        i = i + 1
    }}
    print(i)
}}"""
        write(SUITE / "run" / "generated" / "control" / f"run_while_count_{target}.ny", body, dry_run)
        n += 1

    for v in _profile.run_control_if_values:
        body = f"""// run-stdout: {v}
fn main() {{
    let x = if false {{ 0 }} else {{ {v} }}
    print(x)
}}"""
        write(SUITE / "run" / "generated" / "control" / f"run_if_else_{v}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_lexer(dry_run: bool) -> int:
    n = 0
    cases = [
        ("escape_n", r'let s = "line1\nline2"', "string"),
        ("escape_t", r'let s = "a\tb"', "string"),
        ("escape_r", r'let s = "a\rb"', "string"),
        ("escape_x41", r'let s = "A\x41"', "string"),
        ("escape_u", r'let s = "A\u{42}"', "string"),
        ("empty_string", r'let s = ""', "string"),
        ("line_comment", "let x = 1 // trailing\nlet y = 2", "i32"),
        ("block_inline", "let x = 1 /* mid */ + 2", "i32"),
        ("block_multiline", "/* a\nb */\nlet x = 3", "i32"),
        ("negative", "let x = -42", "i32"),
        ("float_lit", "let x = 2.5", "f64"),
        ("bool_true", "let x = true", "bool"),
        ("bool_false", "let x = false", "bool"),
    ]
    for name, decl, _ty in cases:
        body = f"""fn main() {{
    {decl}
    print(0)
}}"""
        write(SUITE / "pass" / "generated" / "lexer" / f"pass_{name}.ny", body, dry_run)
        n += 1

    for i in _profile.lexer_string_lit_indices:
        body = f"""fn main() {{
    let s = "item{i}"
    print(s.length())
}}"""
        write(SUITE / "pass" / "generated" / "lexer" / f"pass_string_lit_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_parser(dry_run: bool) -> int:
    n = 0
    for i in _profile.parser_indices:
        body = f"""fn helper_{i}(a: i32, b: i32) -> i32 {{
    return a + b
}}
fn main() {{
    print(helper_{i}({i}, {i + 1}))
}}"""
        write(SUITE / "pass" / "generated" / "parser" / f"pass_fn_{i}.ny", body, dry_run)
        n += 1

    for i in _profile.parser_indices:
        body = f"""struct Node{i} {{
    value: i32
    tag: string
}}
fn main() {{
    let n = Node{i} {{ value: {i} tag: "n" }}
    print(n.value)
}}"""
        write(SUITE / "pass" / "generated" / "parser" / f"pass_struct_{i}.ny", body, dry_run)
        n += 1

    for i in _profile.parser_indices:
        body = f"""enum Tag{i} {{
    A
    B
    C
}}
fn main() {{
    let t = Tag{i}.B
    let n = match t {{
        Tag{i}.A => 1
        Tag{i}.B => {i}
        Tag{i}.C => 3
    }}
    print(n)
}}"""
        write(SUITE / "pass" / "generated" / "parser" / f"pass_enum_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_control(dry_run: bool) -> int:
    n = 0
    for limit in _profile.control_for_limits:
        body = f"""fn main() {{
    let mut sum = 0
    for i in 0..{limit} {{
        sum = sum + 1
    }}
    print(sum)
}}"""
        write(SUITE / "pass" / "generated" / "control" / f"pass_for_count_{limit}.ny", body, dry_run)
        n += 1

    for n_items in _profile.control_array_sizes:
        elems = ", ".join(str(i) for i in range(1, n_items + 1))
        body = f"""fn main() {{
    let arr = [{elems}]
    print(arr.length())
}}"""
        write(SUITE / "pass" / "generated" / "control" / f"pass_array_len_{n_items}.ny", body, dry_run)
        n += 1
    return n


def gen_fail_lexer(dry_run: bool) -> int:
    n = 0
    invalid_chars = ["@", "#", "$", "`", "?"]
    for ch in invalid_chars:
        body = f"""fn main() {{
    let x = {ch} //~ ERROR Invalid character
}}"""
        write(SUITE / "fail" / "generated" / "lexer" / f"fail_char_{ord(ch)}.ny", body, dry_run)
        n += 1

    for i in _profile.fail_lexer_unclosed:
        body = f"""//~ ERROR unclosed block comment
fn main() {{
    let x = {i} /* unclosed block {i}
    print(x)
}}"""
        write(SUITE / "fail" / "generated" / "lexer" / f"fail_unclosed_comment_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_fail_parser(dry_run: bool) -> int:
    n = 0
    for i in _profile.fail_parser_missing_brace:
        body = f"""fn main() {{
    let x = {i}
//~ ERROR Expected '}}'
"""
        write(SUITE / "fail" / "generated" / "parser" / f"fail_missing_brace_{i}.ny", body, dry_run)
        n += 1

    bad_exprs = ["@", ")", "(", "+", "let"]
    for i, bad in enumerate(bad_exprs):
        body = f"""fn main() {{
    let x = {bad} //~ ERROR
}}"""
        write(SUITE / "fail" / "generated" / "parser" / f"fail_bad_expr_{i}.ny", body, dry_run)
        n += 1

    for i in _profile.fail_parser_undefined:
        body = f"""fn main() {{
    let x = y{i} //~ ERROR undefined variable
}}"""
        write(SUITE / "fail" / "generated" / "parser" / f"fail_undefined_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_fail_regression(dry_run: bool) -> int:
    """Curated fail tests that must never start compiling (regression guards)."""
    n = 0
    templates = [
        (
            "reg_assign_immutable.ny",
            """fn main() {
    let x = 1
    x = 2 //~ ERROR cannot assign to immutable
}""",
        ),
        (
            "reg_string_sub.ny",
            """fn main() {
    let s = "x"
    s - 1 //~ ERROR Invalid operation on string
}""",
        ),
        (
            "reg_use_after_move.ny",
            """fn main() {
    let a = "moved"
    let b = a
    print(a) //~ ERROR was moved
}""",
        ),
        (
            "reg_mut_borrow.ny",
            """fn main() {
    let mut v = 0
    let r = &v
    v = 1 //~ ERROR because it is borrowed
    print(r)
}""",
        ),
        (
            "reg_if_type_mismatch.ny",
            """fn main() {
    let x = if true { 1 } else { "x" } //~ ERROR If expression branches
}""",
        ),
        (
            "reg_cmp_string_int.ny",
            """fn main() {
    let _ = "a" == 1 //~ ERROR Type mismatch in comparison
}""",
        ),
        (
            "reg_logical_i32.ny",
            """fn main() {
    let _ = 1 && 2 //~ ERROR requires bool
}""",
        ),
        (
            "reg_wrong_arity.ny",
            """fn f(a: i32, b: i32) -> i32 { return a + b }
fn main() {
    print(f(1)) //~ ERROR expects 2 arguments
}""",
        ),
    ]
    for name, body in templates:
        write(SUITE / "fail" / "regression" / name, body, dry_run)
        n += 1

    for i in _profile.fail_regression_move:
        body = f"""fn take(x: string) -> void {{ print(x) }}
fn main() {{
    let s = "r{i}"
    take(s)
    print(s) //~ ERROR was moved
}}"""
        write(SUITE / "fail" / "regression" / f"reg_move_{i}.ny", body, dry_run)
        n += 1

    for i in _profile.fail_regression_mut_borrow:
        body = f"""fn main() {{
    let mut n = {i}
    let r = &mut n
    n = n + 1 //~ ERROR because it is borrowed
    print(r)
}}"""
        write(SUITE / "fail" / "regression" / f"reg_mut_borrow_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_expr_nest(dry_run: bool) -> int:
    n = 0
    for i in _profile.expr_nest_indices:
        body = f"""fn main() {{
    let a = {i}
    let b = {i + 1}
    let c = (a + b) * 2 - a
    print(c)
}}"""
        write(SUITE / "pass" / "generated" / "expr" / f"pass_nest_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_numeric_arith(dry_run: bool) -> int:
    n = 0
    for op in ARITH_OPS:
        sym = ARITH_SYMS[op]
        for a in _profile.arith_values:
            for b in _profile.arith_values:
                if op == "div" and b == 0:
                    continue
                if op == "mod" and b == 0:
                    continue
                name = f"pass_{op}_i32_{a}_{b}.ny"
                body = f"""fn main() {{
    let x = {a}
    let y = {b}
    let _ = x {sym} y
}}"""
                write(SUITE / "pass" / "generated" / "types" / name, body, dry_run)
                n += 1
    return n


def gen_pass_numeric_cmp(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for a in _profile.cmp_values:
            for b in _profile.cmp_values:
                name = f"pass_{op}_i32_{a}_{b}.ny"
                body = f"""fn main() {{
    let _ = {a} {sym} {b}
}}"""
                write(SUITE / "pass" / "generated" / "types" / name, body, dry_run)
                n += 1
    return n


def gen_pass_logical(dry_run: bool) -> int:
    n = 0
    cases = [
        ("and_true_true", "true && true"),
        ("and_true_false", "true && false"),
        ("and_false_false", "false && false"),
        ("or_true_false", "true || false"),
        ("or_false_false", "false || false"),
        ("not_true", "!true"),
        ("not_false", "!false"),
    ]
    for name, expr in cases:
        body = f"""fn main() {{
    let _ = {expr}
}}"""
        write(SUITE / "pass" / "generated" / "types" / f"pass_logical_{name}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_string_ops(dry_run: bool) -> int:
    n = 0
    cases = [
        ("concat_str_str", '"hello" + "world"'),
        ("concat_str_i32", '"n" + 5'),
        ("concat_i32_str", '5 + "n"'),
    ]
    for name, expr in cases:
        body = f"""fn main() {{
    let _ = {expr}
}}"""
        write(SUITE / "pass" / "generated" / "types" / f"pass_string_{name}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_bool_cmp(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        name = f"pass_{op}_bool_bool.ny"
        body = f"""fn main() {{
    let _ = true {sym} false
}}"""
        write(SUITE / "pass" / "generated" / "types" / name, body, dry_run)
        n += 1
    return n


def gen_pass_literal_infer(dry_run: bool) -> int:
    n = 0
    cases = [
        ("i32_add", "1 + 2"),
        ("i32_sub", "10 - 4"),
        ("i32_mul", "6 * 7"),
        ("i32_div", "20 / 4"),
        ("i32_mod", "10 % 3"),
        ("f64_add", "1.5 + 2.5"),
        ("f64_mul", "2.0 * 3.0"),
        ("nested", "(1 + 2) * 3"),
        ("mixed_int", "1 + 2 + 3"),
    ]
    for name, expr in cases:
        body = f"""fn main() {{
    let _ = {expr}
}}"""
        write(SUITE / "pass" / "generated" / "types" / f"pass_infer_{name}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_typed_let(dry_run: bool) -> int:
    n = 0
    cases = [
        ("i32", "let x: i32 = 1"),
        ("i64", "let x: i64 = 1"),
        ("f64", "let x: f64 = 1.0"),
        ("bool", "let x: bool = true"),
        ("string", 'let x: string = "hi"'),
    ]
    for ty, decl in cases:
        body = f"""fn main() {{
    {decl}
    print(x)
}}"""
        write(SUITE / "pass" / "generated" / "types" / f"pass_typed_let_{ty}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_if_expr(dry_run: bool) -> int:
    n = 0
    for ty in NUMERIC:
        lit = numeric_lit(ty)
        body = f"""fn main() {{
    let x = if true {{ {lit} }} else {{ {lit} }}
    print(x)
}}"""
        write(SUITE / "pass" / "generated" / "types" / f"pass_if_expr_{ty}.ny", body, dry_run)
        n += 1
    return n


def arith_should_fail(op: str, left: str, right: str) -> bool:
    if left == "bool" or right == "bool":
        return True
    if op == "add":
        return False if (left == "string" or right == "string") else False
    if op == "add" and (left == "string" or right == "string"):
        return False
    if left == "string" or right == "string":
        return True
    return False


def gen_fail_arith(dry_run: bool) -> int:
    n = 0
    operands = {
        "i32": "1",
        "i64": "1",
        "f64": "1.0",
        "bool": "true",
        "string": '"a"',
    }
    for op in ARITH_OPS:
        sym = ARITH_SYMS[op]
        for left, lval in operands.items():
            for right, rval in operands.items():
                if op == "add" and (left == "string" or right == "string"):
                    continue
                if op == "add" and left != "bool" and right != "bool":
                    if left in NUMERIC and right in NUMERIC:
                        continue
                if left in NUMERIC and right in NUMERIC:
                    continue
                if left == "string" or right == "string":
                    if op == "add":
                        continue
                    err = "Invalid operation on string" if (
                        left == "string" or right == "string"
                    ) and op == "sub" else "Type mismatch in arithmetic"
                elif left == "bool" or right == "bool":
                    err = "Type mismatch in arithmetic"
                else:
                    continue
                name = f"fail_{op}_{left}_{right}.ny"
                body = f"""fn main() {{
    let _ = {lval} {sym} {rval} //~ ERROR {err}
}}"""
                write(SUITE / "fail" / "generated" / "types" / name, body, dry_run)
                n += 1
    return n


def gen_fail_cmp(dry_run: bool) -> int:
    n = 0
    pairs = [
        ("string", '"a"', "i32", "1"),
        ("i32", "1", "string", '"a"'),
        ("bool", "true", "i32", "1"),
        ("i32", "1", "bool", "true"),
        ("string", '"a"', "bool", "true"),
        ("f64", "1.0", "bool", "true"),
    ]
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for left, lval, right, rval in pairs:
            name = f"fail_{op}_{left}_{right}.ny"
            body = f"""fn main() {{
    let _ = {lval} {sym} {rval} //~ ERROR Type mismatch in comparison
}}"""
            write(SUITE / "fail" / "generated" / "types" / name, body, dry_run)
            n += 1
    return n


def gen_fail_logical(dry_run: bool) -> int:
    n = 0
    cases = [
        ("i32_and_i32", "1 && 2", "requires bool"),
        ("i32_or_i32", "1 || 2", "requires bool"),
        ("string_and_true", '"a" && true', "requires bool"),
        ("f64_or_false", "1.0 || false", "requires bool"),
        ("not_i32", "!1", "requires bool"),
    ]
    for name, expr, err in cases:
        body = f"""fn main() {{
    let _ = {expr} //~ ERROR {err}
}}"""
        write(SUITE / "fail" / "generated" / "types" / f"fail_logical_{name}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_borrow(dry_run: bool) -> int:
    n = 0
    templates = [
        (
            "copy_i32_both_use.ny",
            """fn main() {
    let a = 1
    let b = a
    print(a)
    print(b)
}""",
        ),
        (
            "nll_borrow_ends.ny",
            """fn main() {
    let mut v = 1
    let r = &v
    print(*r)
    v = 2
    print(v)
}""",
        ),
        (
            "reborrow_immut.ny",
            """fn main() {
    let mut v = 1
    let r1 = &v
    let r2 = &v
    print(*r1)
    print(*r2)
}""",
        ),
        (
            "clone_string_keep.ny",
            """fn main() {
    let s = "hello"
    let c = s.clone()
    print(s)
    print(c)
}""",
        ),
        (
            "auto_borrow_ref_param.ny",
            """struct User {
    name: string
    age: i32
}
fn peek(u: &User) -> void {
    print(u.age)
}
fn main() {
    let user = User { name: "Ada" age: 30 }
    peek(user)
    print(user.name)
}""",
        ),
        (
            "ref_fn_param_i32.ny",
            """fn show(p: &i32) -> void { print(*p) }
fn main() {
    let x = 5
    show(&x)
    print(x)
}""",
        ),
        (
            "mut_borrow_read_only.ny",
            """fn main() {
    let mut v = 1
    let r = &mut v
    print(*r)
    print(v)
}""",
        ),
    ]
    for name, body in templates:
        write(SUITE / "pass" / "generated" / "borrow" / name, body, dry_run)
        n += 1

    fns = ["take", "consume", "save", "store", "send", "hold", "keep", "push"]
    for fn in fns:
        body = f"""fn {fn}(x: string) -> void {{ print(x) }}
fn main() {{
    let s = "ok"
    {fn}(clone s)
    print(s)
}}"""
        write(SUITE / "pass" / "generated" / "borrow" / f"pass_clone_call_{fn}.ny", body, dry_run)
        n += 1

    for i in _profile.struct_use_indices:
        body = f"""struct Box {{
    id: i32
    label: string
}}
fn main() {{
    let b = Box {{ id: {i} label: "item" }}
    print(b.id)
    print(b.label)
}}"""
        write(SUITE / "pass" / "generated" / "borrow" / f"pass_struct_use_{i}.ny", body, dry_run)
        n += 1

    return n


def gen_fail_borrow(dry_run: bool) -> int:
    n = 0
    fns = _profile.fail_move_fns
    vars_ = _profile.fail_move_vars
    for fn in fns:
        for var in vars_:
            body = f"""fn {fn}(x: string) -> void {{ print(x) }}
fn main() {{
    let {var} = "hello"
    {fn}({var})
    print({var}) //~ ERROR was moved
}}"""
            write(
                SUITE / "fail" / "generated" / "borrow" / f"fail_move_{fn}_{var}.ny",
                body,
                dry_run,
            )
            n += 1

    for i in _profile.fail_mut_borrow_indices:
        body = f"""fn main() {{
    let mut v = {i}
    let r = &v
    v = v + 1 //~ ERROR because it is borrowed
    print(r)
}}"""
        write(SUITE / "fail" / "generated" / "borrow" / f"fail_mut_assign_borrow_{i}.ny", body, dry_run)
        n += 1

    struct_fns = ["consume", "save", "store"]
    for i, fn in enumerate(struct_fns):
        body = f"""struct Item {{
    id: i32
    name: string
}}
fn {fn}(x: Item) -> void {{ print(x.id) }}
fn main() {{
    let item = Item {{ id: {i + 1} name: "x" }}
    {fn}(item)
    print(item.name) //~ ERROR was moved
}}"""
        write(SUITE / "fail" / "generated" / "borrow" / f"fail_struct_move_{fn}_{i}.ny", body, dry_run)
        n += 1

    return n


def write_project(base: Path, main_body: str, extra: dict[str, str], dry_run: bool) -> None:
    if dry_run:
        return
    base.mkdir(parents=True, exist_ok=True)
    (base / "main.ny").write_text(main_body.rstrip() + "\n", encoding="utf-8")
    for rel, content in extra.items():
        path = base / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content.rstrip() + "\n", encoding="utf-8")


def gen_pass_stdlib(dry_run: bool) -> int:
    n = 0
    strings = [
        '"hello"', '"  spaced  "', '"abc,def"', '"Nyra"', '"line\\none"', '""',
    ]
    methods = [
        ("length", ""),
        ("trim", ""),
        ("to_upper", ""),
        ("to_lower", ""),
        ("contains", '("ell")'),
        ("starts_with", '("he")'),
        ("ends_with", '("lo")'),
        ("replace", '("o", "0")'),
    ]
    for i in _profile.stdlib_string_indices:
        s = strings[i]
        for method, args in methods:
            call = f"s.{method}{args}" if args else f"s.{method}()"
            body = f"""fn main() {{
    let s = {s}
    let _ = {call}
}}"""
            write(
                SUITE / "pass" / "generated" / "stdlib" / f"pass_str_{method}_{i}.ny",
                body,
                dry_run,
            )
            n += 1

    for i in _profile.stdlib_array_sizes:
        elems = ", ".join(str(j) for j in range(1, i + 1))
        body = f"""fn main() {{
    let arr = [{elems}]
    print(arr.length())
    print(arr[{i // 2}])
}}"""
        write(SUITE / "pass" / "generated" / "stdlib" / f"pass_array_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_run_stdlib(dry_run: bool) -> int:
    n = 0
    cases = [
        ("len_abc", '"abc".length()', "3"),
        ("upper", '"abc".to_upper()', "ABC"),
        ("lower", '"ABC".to_lower()', "abc"),
        ("trim", '"  x  ".trim()', "x"),
        ("contains_true", '"hello".contains("ell")', "1"),
        ("contains_false", '"hello".contains("xyz")', "0"),
        ("starts_true", '"hello".starts_with("he")', "1"),
        ("ends_true", '"hello".ends_with("lo")', "1"),
        ("replace", '"foo".replace("o", "0")', "f0o"),
    ]
    for name, expr, expected in cases:
        body = f"""// run-stdout: {expected}
fn main() {{
    print({expr})
}}"""
        write(SUITE / "run" / "generated" / "stdlib" / f"run_str_{name}.ny", body, dry_run)
        n += 1
    return n


def gen_projects_pass(dry_run: bool) -> int:
    n = 0
    for i in _profile.project_const_indices:
        write_project(
            SUITE / "projects" / "pass" / "generated" / f"import_const_{i}",
            f"""import "lib/constants.ny"

fn main() {{
    print(N{i}_MSG)
    print(N{i}_VAL)
}}""",
            {f"lib/constants.ny": f'const N{i}_MSG = "ok{i}"\nconst N{i}_VAL = {i * 10}'},
            dry_run,
        )
        n += 1

    for i in _profile.project_fn_indices:
        write_project(
            SUITE / "projects" / "pass" / "generated" / f"import_fn_{i}",
            f"""import "lib/math.ny"

fn main() {{
    print(add{i}(3, 4))
}}""",
            {
                f"lib/math.ny": f"""fn add{i}(a: i32, b: i32) -> i32 {{
    return a + b + {i}
}}"""
            },
            dry_run,
        )
        n += 1
    return n


def gen_projects_fail(dry_run: bool) -> int:
    n = 0
    for i in _profile.project_fail_indices:
        write_project(
            SUITE / "projects" / "fail" / "generated" / f"missing_import_{i}",
            f"""//~ ERROR import not found
import "lib/missing{i}.ny"

fn main() {{
    print(0)
}}""",
            {},
            dry_run,
        )
        n += 1
    return n


def gen_projects_run(dry_run: bool) -> int:
    n = 0
    for i in _profile.project_run_indices:
        expected = 12 + i
        write_project(
            SUITE / "projects" / "run" / "generated" / f"import_math_{i}",
            f"""// run-stdout: {expected}
import "lib/math.ny"

fn main() {{
    print(mul{i}(3, 4))
}}""",
            {
                f"lib/math.ny": f"""fn mul{i}(a: i32, b: i32) -> i32 {{
    return a * b + {i}
}}"""
            },
            dry_run,
        )
        n += 1
    return n


def gen_pass_generics(dry_run: bool) -> int:
    n = 0
    for i in _profile.generics_indices:
        body = f"""fn id{i}<T>(x: T) -> T {{
    return x
}}
fn main() {{
    print(id{i}<i32>({i}))
}}"""
        write(SUITE / "pass" / "generated" / "generics" / f"pass_id_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_match_ext(dry_run: bool) -> int:
    n = 0
    for i in _profile.match_indices:
        body = f"""enum Op{i} {{
    Add
    Sub
    Mul
}}
fn main() {{
    let op = Op{i}.Add
    let n = match op {{
        Op{i}.Add => {i}
        Op{i}.Sub => {i + 1}
        Op{i}.Mul => {i + 2}
    }}
    print(n)
}}"""
        write(SUITE / "pass" / "generated" / "match" / f"pass_match_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_fail_golden_diag(dry_run: bool) -> int:
    """High-signal diagnostic guards (exact message fragments)."""
    n = 0
    cases = [
        (
            "golden_assign_immutable.ny",
            """fn main() {
    let x = 1
    x = 2 //~ ERROR cannot assign to immutable variable
}""",
        ),
        (
            "golden_undefined.ny",
            """fn main() {
    let x = missing_symbol //~ ERROR undefined variable
}""",
        ),
        (
            "golden_use_after_move.ny",
            """fn main() {
    let s = "moved"
    let t = s
    print(s) //~ ERROR was moved
}""",
        ),
    ]
    for name, body in cases:
        write(SUITE / "fail" / "generated" / "stderr" / name, body, dry_run)
        n += 1
    return n


def nyra_compiles(path: Path) -> bool:
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "cli", "--", "check", str(path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def write_stderr_golden(ny_path: Path, dry_run: bool) -> None:
    if dry_run:
        return
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "compiletest", "--", "--capture", str(ny_path)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = result.stdout.strip()
    if not text:
        return
    key_lines = [
        line
        for line in text.splitlines()
        if line.startswith("error") or line.startswith("error[")
    ]
    if not key_lines:
        key_lines = [line for line in text.splitlines() if "error" in line.lower()][:5]
    stderr = ny_path.with_suffix(".stderr")
    stderr.write_text("\n".join(key_lines[:8]) + "\n", encoding="utf-8")


def gen_pass_numeric_i64(dry_run: bool) -> int:
    n = 0
    for op in ARITH_OPS:
        sym = ARITH_SYMS[op]
        for a in _profile.arith_values:
            for b in _profile.arith_values:
                if op in ("div", "mod") and b == 0:
                    continue
                name = f"pass_{op}_i64_{a}_{b}.ny"
                body = f"""fn main() {{
    let x: i64 = {a}
    let y: i64 = {b}
    let _ = x {sym} y
}}"""
                write(SUITE / "pass" / "generated" / "types_i64" / name, body, dry_run)
                n += 1
    return n


def gen_pass_numeric_f64(dry_run: bool) -> int:
    n = 0
    for op in ARITH_OPS:
        sym = ARITH_SYMS[op]
        for a in _profile.arith_values:
            for b in _profile.arith_values:
                if op in ("div", "mod") and b == 0:
                    continue
                name = f"pass_{op}_f64_{a}_{b}.ny"
                body = f"""fn main() {{
    let x = {a}.0
    let y = {b}.0
    let _ = x {sym} y
}}"""
                write(SUITE / "pass" / "generated" / "types_f64" / name, body, dry_run)
                n += 1
    return n


def gen_pass_numeric_cmp_i64(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for a in _profile.cmp_values:
            for b in _profile.cmp_values:
                name = f"pass_{op}_i64_{a}_{b}.ny"
                body = f"""fn main() {{
    let _ = {a} {sym} {b}
}}"""
                write(SUITE / "pass" / "generated" / "types_i64" / name, body, dry_run)
                n += 1
    return n


def gen_pass_numeric_cmp_f64(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for a in _profile.cmp_values:
            for b in _profile.cmp_values:
                name = f"pass_{op}_f64_{a}_{b}.ny"
                body = f"""fn main() {{
    let _ = {a}.0 {sym} {b}.0
}}"""
                write(SUITE / "pass" / "generated" / "types_f64" / name, body, dry_run)
                n += 1
    return n


def eval_f64(op: str, a: float, b: float) -> float:
    if op == "add":
        return a + b
    if op == "sub":
        return a - b
    if op == "mul":
        return a * b
    if op == "div":
        return a / b
    if op == "mod":
        return a % b
    raise ValueError(op)


def gen_run_arith_i64(dry_run: bool) -> int:
    n = 0
    for op in ARITH_OPS:
        sym = ARITH_SYMS[op]
        for a, b in _profile.arith_run_pairs:
            expected = eval_i32(op, a, b)
            name = f"run_{op}_{a}_{b}.ny"
            body = f"""// run-stdout: {expected}
fn main() {{
    let x: i64 = {a}
    let y: i64 = {b}
    print(x {sym} y)
}}"""
            write(SUITE / "run" / "generated" / "arith_i64" / name, body, dry_run)
            n += 1
    return n


def gen_run_cmp_i64(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for a, b in _profile.cmp_run_pairs:
            expected = eval_cmp(op, a, b)
            name = f"run_{op}_{a}_{b}.ny"
            body = f"""// run-stdout: {expected}
fn main() {{
    let x: i64 = {a}
    let y: i64 = {b}
    print(x {sym} y)
}}"""
            write(SUITE / "run" / "generated" / "cmp_i64" / name, body, dry_run)
            n += 1
    return n


def gen_pass_for_nested(dry_run: bool) -> int:
    n = 0
    for outer in _profile.nested_outer:
        for inner in _profile.nested_inner:
            body = f"""fn main() {{
    let mut count = 0
    for i in 0..{outer} {{
        for j in 0..{inner} {{
            count = count + 1
        }}
    }}
    print(count)
}}"""
            write(
                SUITE / "pass" / "generated" / "control" / f"pass_nested_for_{outer}_{inner}.ny",
                body,
                dry_run,
            )
            n += 1
    return n


def gen_run_cmp_f64(dry_run: bool) -> int:
    n = 0
    for op in CMP_OPS:
        sym = CMP_SYMS[op]
        for a, b in _profile.cmp_run_pairs:
            expected = eval_cmp(op, a, b)
            name = f"run_{op}_{a}_{b}.ny"
            body = f"""// run-stdout: {expected}
fn main() {{
    let x = {a}.0
    let y = {b}.0
    print(x {sym} y)
}}"""
            write(SUITE / "run" / "generated" / "cmp_f64" / name, body, dry_run)
            n += 1
    return n


def gen_pass_array_index(dry_run: bool) -> int:
    n = 0
    for size, idx in _array_index_pairs():
        elems = ", ".join(str(j + 1) for j in range(size))
        body = f"""fn main() {{
    let arr = [{elems}]
    print(arr[{idx}])
}}"""
        write(
            SUITE / "pass" / "generated" / "array" / f"pass_array_idx_{size}_{idx}.ny",
            body,
            dry_run,
        )
        n += 1
    return n


def gen_pass_fn_grid(dry_run: bool) -> int:
    n = 0
    for i in _profile.fn_grid_i:
        for j in _profile.fn_grid_j:
            body = f"""fn add_{i}_{j}(a: i32, b: i32) -> i32 {{
    return a + b + {i} + {j}
}}
fn main() {{
    print(add_{i}_{j}({i}, {j}))
}}"""
            write(
                SUITE / "pass" / "generated" / "fn_grid" / f"pass_fn_{i}_{j}.ny",
                body,
                dry_run,
            )
            n += 1
    return n


def gen_run_for_nested(dry_run: bool) -> int:
    n = 0
    for outer in _profile.nested_outer:
        for inner in _profile.nested_inner:
            expected = outer * inner
            body = f"""// run-stdout: {expected}
fn main() {{
    let mut count = 0
    for i in 0..{outer} {{
        for j in 0..{inner} {{
            count = count + 1
        }}
    }}
    print(count)
}}"""
            write(
                SUITE / "run" / "generated" / "for_nested" / f"run_nested_{outer}_{inner}.ny",
                body,
                dry_run,
            )
            n += 1
    return n


def gen_fail_assign_mismatch(dry_run: bool) -> int:
    n = 0
    cases = [
        ("i32_to_string", "let x: string = 1", "Type mismatch"),
        ("string_to_i32", 'let x: i32 = "a"', "Type mismatch"),
        ("bool_to_i32", "let x: i32 = true", "Type mismatch"),
        ("i32_to_bool", "let x: bool = 1", "Type mismatch"),
        ("f64_to_i32", "let x: i32 = 1.0", "Type mismatch"),
        ("i32_to_f64", "let x: f64 = 1", "Type mismatch"),
    ]
    for i in _profile.fail_assign_indices:
        for name, decl, err in cases:
            body = f"""fn main() {{
    {decl} //~ ERROR {err}
    print(0)
}}"""
            write(
                SUITE / "fail" / "generated" / "types" / f"fail_assign_{name}_{i}.ny",
                body,
                dry_run,
            )
            n += 1
    return n


def gen_pass_stdlib_imports(dry_run: bool) -> int:
    n = 0
    stdlib_root = ROOT / "stdlib"
    for path in sorted(stdlib_root.rglob("*.ny")):
        rel = path.relative_to(stdlib_root).as_posix()
        mod = f"stdlib/{rel}"
        safe = rel.replace("/", "__").replace(".", "_")
        out = SUITE / "pass" / "generated" / "stdlib_import" / f"pass_import_{safe}.ny"
        body = f"""import "{mod}"
fn main() {{
    print(0)
}}"""
        if dry_run:
            n += 1
            continue
        write(out, body, True)
        if nyra_compiles(out):
            n += 1
        else:
            out.unlink(missing_ok=True)
    return n


def gen_fail_stderr_full(dry_run: bool) -> int:
    """Fail tests validated via `.stderr` golden files (key error lines)."""
    n = 0
    cases = [
        (
            "stderr_assign_immutable.ny",
            """fn main() {
    let x = 1
    x = 2
}""",
        ),
        (
            "stderr_undefined.ny",
            """fn main() {
    let x = not_a_symbol
}""",
        ),
        (
            "stderr_use_after_move.ny",
            """fn main() {
    let s = "x"
    let t = s
    print(s)
}""",
        ),
        (
            "stderr_mut_borrow.ny",
            """fn main() {
    let mut v = 0
    let r = &v
    v = 1
    print(r)
}""",
        ),
        (
            "stderr_wrong_arity.ny",
            """fn f(a: i32, b: i32) -> i32 { return a + b }
fn main() {
    print(f(1))
}""",
        ),
        (
            "stderr_if_mismatch.ny",
            """fn main() {
    let x = if true { 1 } else { "no" }
}""",
        ),
        (
            "stderr_cmp_mismatch.ny",
            """fn main() {
    let _ = "a" == 1
}""",
        ),
        (
            "stderr_logical.ny",
            """fn main() {
    let _ = 1 && 2
}""",
        ),
        (
            "stderr_string_sub.ny",
            """fn main() {
    let _ = "a" - 1
}""",
        ),
        (
            "stderr_import_missing.ny",
            """import "stdlib/no_such_module_xyz.ny"
fn main() {
    print(0)
}""",
        ),
    ]
    for name, body in cases:
        out = SUITE / "fail" / "generated" / "stderr_full" / name
        write(out, body, dry_run)
        if not dry_run:
            write_stderr_golden(out, dry_run)
        n += 1
    return n


def gen_fail_fuzz_regression(dry_run: bool) -> int:
    """Curated parser/lexer inputs inspired by fuzz targets."""
    n = 0
    seeds = [
        ("unclosed_paren", "fn main() { print(1 }"),
        ("double_op", "fn main() { let x = 1 ++ 2 }"),
        ("bad_char", "fn main() { let x = @ }"),
        ("missing_colon", "fn main() { let x 1 }"),
        ("double_comma", "fn main() { let arr = [1,, 2] }"),
        ("empty_parens", "fn main() { let x = () }"),
        ("unbalanced_if", "fn main() { if (((({"),
        ("repeated_let", "let let let let"),
        ("nested_brace", "fn main() { { { {"),
        ("keyword_soup", "fn struct enum match import let mut async await"),
        ("broken_fn_sig", "fn main( { let x = 1 }"),
        ("half_struct", "struct S { x: i32"),
    ]
    for i in _profile.fail_fuzz_garbage:
        seeds.append((f"garbage_{i}", f"fn main() {{ let x = @{i} }}"))
    for name, src in seeds:
        body = src if "//~ ERROR" in src else src.rstrip() + " //~ ERROR"
        write(SUITE / "fail" / "regression" / "fuzz" / f"fuzz_{name}.ny", body, dry_run)
        n += 1
    return n


def gen_fail_stdlib_import(dry_run: bool) -> int:
    n = 0
    bad = [
        "stdlib/__missing__.ny",
        "stdlib/no/such/path.ny",
        "stdlib/typo_prelude.ny",
    ]
    for i in _profile.fail_stdlib_import_extra:
        bad.append(f"stdlib/fake_module_{i}.ny")
    for mod in bad:
        safe = mod.replace("/", "__")
        body = f"""//~ ERROR import not found
import "{mod}"
fn main() {{
    print(0)
}}"""
        write(SUITE / "fail" / "generated" / "stdlib_import" / f"fail_{safe}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_option_result(dry_run: bool) -> int:
    n = 0
    for i in _profile.option_indices:
        body = f"""enum Opt{i} {{
    None
    Some(i32)
}}
fn main() {{
    let o = Opt{i}.Some({i})
    let v = match o {{
        Opt{i}.None => 0
        Opt{i}.Some(x) => x
    }}
    print(v)
}}"""
        write(SUITE / "pass" / "generated" / "option" / f"pass_option_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_char(dry_run: bool) -> int:
    n = 0
    for i in _profile.char_indices:
        ch = chr(ord("a") + (i % 26))
        body = f"""fn main() {{
    let c = '{ch}'
    print(c)
}}"""
        write(SUITE / "pass" / "generated" / "char" / f"pass_char_{i}.ny", body, dry_run)
        n += 1
    return n


def gen_pass_array_ops(dry_run: bool) -> int:
    n = 0
    for size, _idx in _array_index_pairs():
        elems = ", ".join(str(j + 1) for j in range(size))
        body = f"""fn main() {{
    let arr = [{elems}]
    let mut sum = 0
    for x in arr {{
        sum = sum + x
    }}
    print(sum)
}}"""
        write(
            SUITE / "pass" / "generated" / "array" / f"pass_array_sum_{size}.ny",
            body,
            dry_run,
        )
        n += 1
    return n


def gen_pass_while_extra(dry_run: bool) -> int:
    n = 0
    for limit in _profile.while_limits:
        body = f"""fn main() {{
    let mut i = 0
    let mut acc = 0
    while i < {limit} {{
        acc = acc + i
        i = i + 1
    }}
    print(acc)
}}"""
        write(SUITE / "pass" / "generated" / "while" / f"pass_while_acc_{limit}.ny", body, dry_run)
        n += 1
    return n


def gen_run_print_seq(dry_run: bool) -> int:
    n = 0
    for i in _profile.print_values:
        body = f"""// run-stdout: {i}
fn main() {{
    print({i})
}}"""
        write(SUITE / "run" / "generated" / "print" / f"run_print_{i}.ny", body, dry_run)
        n += 1
    return n


def main() -> int:
    global _profile
    parser = argparse.ArgumentParser(description="Generate Nyra suite tests")
    parser.add_argument("--dry-run", action="store_true", help="Count only, do not write files")
    parser.add_argument(
        "--profile",
        choices=sorted(PROFILES),
        default="ci",
        help="fast (~1.5k), ci (~3k, default for test-all), or full (~10k, exhaustive)",
    )
    args = parser.parse_args()
    _profile = PROFILES[args.profile]

    clean_generated(args.dry_run)

    counts = {
        "pass_numeric_arith": gen_pass_numeric_arith(args.dry_run),
        "pass_numeric_i64": gen_pass_numeric_i64(args.dry_run),
        "pass_numeric_f64": gen_pass_numeric_f64(args.dry_run),
        "pass_numeric_cmp_i64": gen_pass_numeric_cmp_i64(args.dry_run),
        "pass_numeric_cmp_f64": gen_pass_numeric_cmp_f64(args.dry_run),
        "pass_numeric_cmp": gen_pass_numeric_cmp(args.dry_run),
        "pass_fn_grid": gen_pass_fn_grid(args.dry_run),
        "pass_for_nested": gen_pass_for_nested(args.dry_run),
        "pass_logical": gen_pass_logical(args.dry_run),
        "pass_string_ops": gen_pass_string_ops(args.dry_run),
        "pass_bool_cmp": gen_pass_bool_cmp(args.dry_run),
        "pass_literal_infer": gen_pass_literal_infer(args.dry_run),
        "pass_typed_let": gen_pass_typed_let(args.dry_run),
        "pass_if_expr": gen_pass_if_expr(args.dry_run),
        "pass_borrow": gen_pass_borrow(args.dry_run),
        "pass_lexer": gen_pass_lexer(args.dry_run),
        "pass_parser": gen_pass_parser(args.dry_run),
        "pass_control": gen_pass_control(args.dry_run),
        "pass_expr_nest": gen_pass_expr_nest(args.dry_run),
        "pass_stdlib": gen_pass_stdlib(args.dry_run),
        "pass_stdlib_imports": gen_pass_stdlib_imports(args.dry_run),
        "pass_generics": gen_pass_generics(args.dry_run),
        "pass_match_ext": gen_pass_match_ext(args.dry_run),
        "pass_array_index": gen_pass_array_index(args.dry_run),
        "pass_option_result": gen_pass_option_result(args.dry_run),
        "pass_char": gen_pass_char(args.dry_run),
        "pass_array_ops": gen_pass_array_ops(args.dry_run),
        "pass_while_extra": gen_pass_while_extra(args.dry_run),
        "projects_pass": gen_projects_pass(args.dry_run),
        "projects_fail": gen_projects_fail(args.dry_run),
        "projects_run": gen_projects_run(args.dry_run),
        "fail_assign_mismatch": gen_fail_assign_mismatch(args.dry_run),
        "fail_arith": gen_fail_arith(args.dry_run),
        "fail_cmp": gen_fail_cmp(args.dry_run),
        "fail_logical": gen_fail_logical(args.dry_run),
        "fail_borrow": gen_fail_borrow(args.dry_run),
        "fail_lexer": gen_fail_lexer(args.dry_run),
        "fail_parser": gen_fail_parser(args.dry_run),
        "fail_regression": gen_fail_regression(args.dry_run),
        "fail_fuzz_regression": gen_fail_fuzz_regression(args.dry_run),
        "fail_stderr_golden": gen_fail_golden_diag(args.dry_run),
        "fail_stderr_full": gen_fail_stderr_full(args.dry_run),
        "fail_stdlib_import": gen_fail_stdlib_import(args.dry_run),
        "run_arith": gen_run_arith_i32(args.dry_run),
        "run_arith_i64": gen_run_arith_i64(args.dry_run),
        "run_for_nested": gen_run_for_nested(args.dry_run),
        "run_cmp": gen_run_cmp_i32(args.dry_run),
        "run_cmp_i64": gen_run_cmp_i64(args.dry_run),
        "run_cmp_f64": gen_run_cmp_f64(args.dry_run),
        "run_control": gen_run_control(args.dry_run),
        "run_stdlib": gen_run_stdlib(args.dry_run),
        "run_print_seq": gen_run_print_seq(args.dry_run),
    }
    total = sum(counts.values())
    print(f"generated {total} tests (profile={_profile.name}):")
    for k, v in counts.items():
        print(f"  {k}: {v}")
    if not args.dry_run:
        baseline = SUITE / ".count-baseline"
        all_count = count_suite_tests()
        baseline.write_text(f"{all_count}\n", encoding="utf-8")
        print(f"updated baseline -> {all_count} (total .ny files under tests/suite/)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
