from __future__ import annotations

from .spec import ArgSpec, BuiltinSpec, NyraType, ReceiverKind


def nyra_type_annotation(ty: NyraType, ref: bool = True) -> str:
    if ty == NyraType.STRING:
        return "&string" if ref else "string"
    if ty == NyraType.I32:
        return "i32"
    if ty == NyraType.I64:
        return "i64"
    if ty == NyraType.F64:
        return "f64"
    if ty == NyraType.BOOL:
        return "i32"
    if ty == NyraType.BYTES:
        return "&bytes" if ref else "bytes"
    if ty == NyraType.PTR:
        return "ptr"
    if ty == NyraType.VEC_STR:
        return "ptr"
    if ty == NyraType.VOID:
        return "void"
    raise ValueError(f"unsupported Nyra type annotation: {ty}")


def c_type(ty: NyraType, *, is_return: bool = False) -> str:
    if ty == NyraType.STRING:
        return "char *" if is_return else "const char *"
    if ty == NyraType.I32 or ty == NyraType.BOOL:
        return "int"
    if ty == NyraType.I64:
        return "long long"
    if ty == NyraType.F64:
        return "double"
    if ty == NyraType.BYTES:
        return "char *" if is_return else "const char *"
    if ty == NyraType.PTR or ty == NyraType.VEC_STR:
        return "void *"
    if ty == NyraType.VOID:
        return "void"
    raise ValueError(f"unsupported C type: {ty}")


def expr_value_ty(ret: NyraType) -> str:
    if ret == NyraType.VEC_STR:
        return "vec_str"
    return llvm_return_ty(ret)


def llvm_return_ty(ret: NyraType) -> str:
    if ret in (NyraType.STRING, NyraType.BYTES, NyraType.VEC_STR):
        return "ptr"
    if ret == NyraType.I32 or ret == NyraType.BOOL:
        return "i32"
    if ret == NyraType.I64:
        return "i64"
    if ret == NyraType.F64:
        return "double"
    if ret == NyraType.VOID:
        return "void"
    if ret == NyraType.PTR:
        return "ptr"
    raise ValueError(f"unsupported LLVM return: {ret}")


def llvm_arg_ty(ty: NyraType) -> str:
    if ty in (NyraType.STRING, NyraType.BYTES):
        return "ptr"
    if ty in (NyraType.I32, NyraType.BOOL):
        return "i32"
    if ty == NyraType.I64:
        return "i64"
    if ty == NyraType.F64:
        return "double"
    if ty == NyraType.PTR:
        return "ptr"
    raise ValueError(f"unsupported LLVM arg: {ty}")


def rust_return_type(ret: NyraType) -> str:
    mapping = {
        NyraType.STRING: "Type::String",
        NyraType.I32: "Type::Integer(ast::IntKind::I32)",
        NyraType.I64: "Type::Integer(ast::IntKind::I64)",
        NyraType.F64: "Type::F64",
        NyraType.BOOL: "Type::Integer(ast::IntKind::I32)",
        NyraType.VEC_STR: "Type::VecStr",
        NyraType.BYTES: "Type::Bytes",
    }
    return mapping[ret]


def marker_start(spec: BuiltinSpec) -> str:
    return f"// [builtin-dev:{spec.marker}]"


def marker_end(spec: BuiltinSpec) -> str:
    return f"// [/builtin-dev:{spec.marker}]"


def toml_marker_start(spec: BuiltinSpec) -> str:
    return f"# [builtin-dev:{spec.marker}]"


def toml_marker_end(spec: BuiltinSpec) -> str:
    return f"# [/builtin-dev:{spec.marker}]"


def c_stub(spec: BuiltinSpec) -> str:
    params: list[str] = []
    if spec.receiver == ReceiverKind.STRING:
        params.append("const char *s")
    for arg in spec.args:
        params.append(f"{c_type(arg.nyra_type)} {arg.name}")
    ret = c_type(spec.returns, is_return=True)
    sig = ", ".join(params) if params else "void"
    lines = [
        "",
        marker_start(spec),
        f"{ret} {spec.c_name}({sig}) {{",
    ]
    if spec.c_body and spec.c_body.strip():
        for line in spec.c_body.rstrip().splitlines():
            lines.append(line if line.startswith("    ") else f"    {line}")
    else:
        lines.append("    /* TODO: implement logic here — this stub returns a safe default. */")
        if spec.returns == NyraType.STRING:
            if spec.receiver == ReceiverKind.STRING:
                lines.append("    if (!s) return NULL;")
                lines.append("    return str_dup(s);")
            else:
                lines.append("    return NULL;")
        elif spec.returns in (NyraType.I32, NyraType.BOOL):
            lines.append("    return 0;")
        elif spec.returns == NyraType.I64:
            lines.append("    return 0;")
        elif spec.returns == NyraType.F64:
            lines.append("    return 0.0;")
    lines.extend([f"}}", marker_end(spec), ""])
    return "\n".join(lines)


def extern_ny_line(spec: BuiltinSpec) -> str:
    if spec.receiver == ReceiverKind.STRING:
        params = [f"str: {nyra_type_annotation(NyraType.STRING)}"]
    elif spec.receiver == ReceiverKind.FREE:
        params = []
    else:
        params = []
    for arg in spec.args:
        params.append(f"{arg.name}: {nyra_type_annotation(arg.nyra_type)}")
    ret = nyra_type_annotation(spec.returns, ref=False)
    joined = ", ".join(params)
    return f"extern fn {spec.c_name}({joined}) -> {ret}"


def builtins_wrapper(spec: BuiltinSpec) -> str:
    if spec.receiver == ReceiverKind.STRING:
        recv = f"s: {nyra_type_annotation(NyraType.STRING)}"
        call_args = ["s", *[a.name for a in spec.args]]
    else:
        recv = ""
        call_args = [a.name for a in spec.args]
    arg_parts = [recv] if recv else []
    for arg in spec.args:
        arg_parts.append(f"{arg.name}: {nyra_type_annotation(arg.nyra_type)}")
    ret = nyra_type_annotation(spec.returns, ref=False)
    call = ", ".join(call_args)
    lines = [
        marker_start(spec),
        f"fn {spec.wrapper_fn}({', '.join(arg_parts)}) -> {ret} {{",
        f"    return {spec.c_name}({call})",
        "}",
    ]
    if spec.free_fn_alias and spec.receiver == ReceiverKind.STRING:
        lines.extend(
            [
                "",
                f"fn {spec.method}({', '.join(arg_parts)}) -> {ret} {{",
                f"    return {spec.c_name}({call})",
                "}",
            ]
        )
    lines.append(marker_end(spec))
    return "\n".join(lines)


def typecheck_borrow_entry(spec: BuiltinSpec) -> str:
    return f'"{spec.method}"'


def typecheck_match_arm(spec: BuiltinSpec) -> str:
    n = len(spec.args)
    lines = [marker_start(spec), f'"{spec.method}" => {{']
    if n == 0:
        lines.append("    if !mc.args.is_empty() {")
        lines.append(
            f'        diagnostics::wrong_arity(self, &format!(".{spec.method}"), 0, mc.args.len(), sp.clone());'
        )
        lines.append("    }")
    else:
        lines.append(f"    if mc.args.len() != {n} {{")
        lines.append(
            f'        diagnostics::wrong_arity(self, &format!(".{spec.method}"), {n}, mc.args.len(), sp.clone());'
        )
        lines.append("    } else {")
        for i, arg in enumerate(spec.args):
            if arg.nyra_type == NyraType.STRING:
                lines.append(f"        self.check_string_arg(mc, {i}, env, sp);")
            elif arg.nyra_type == NyraType.I32:
                lines.append(f"        let _arg{i} = self.check_expr(&mc.args[{i}], env);")
                lines.append(
                    f"        if _arg{i} != Type::Integer(ast::IntKind::I32) && _arg{i} != Type::Unknown {{"
                )
                lines.append(
                    f'            diagnostics::wrong_arity(self, &format!(".{spec.method} arg {i}"), 0, 0, sp.clone());'
                )
                lines.append("        }")
        lines.append("    }")
    lines.extend([f"    {rust_return_type(spec.returns)}", "}", marker_end(spec)])
    return "\n".join(lines)


def codegen_util_entry(spec: BuiltinSpec) -> str:
    return f'"{spec.method}"'


def codegen_string_method_arm(spec: BuiltinSpec) -> str:
    n = len(spec.args)
    ret_ty = llvm_return_ty(spec.returns)
    reg_name = spec.method.replace(".", "_")
    lines = [marker_start(spec), f'"{spec.method}" => {{']
    for i, arg in enumerate(spec.args):
        lines.append(f"    let arg{i} = self.compile_expr(&mc.args[{i}], env);")
        if arg.nyra_type in (NyraType.STRING, NyraType.BYTES):
            lines.append(f"    let arg{i}_reg = llvm_ptr_reg(&arg{i}.reg);")
        else:
            lines.append(f"    let arg{i}_reg = llvm_value_operand(&arg{i}.reg);")
    lines.append(f'    let reg = self.fresh("{reg_name}");')
    llvm_args = ["ptr {str_reg}"]
    for i, arg in enumerate(spec.args):
        op = llvm_arg_ty(arg.nyra_type)
        lines.append(f"    // arg {i}: {arg.name}")
        llvm_args.append(f"{op} {{arg{i}_reg}}")
    call_args = ", ".join(llvm_args)
    lines.append("    self.emit_runtime_call(")
    lines.append(f'        "{spec.c_name}",')
    lines.append(
        f'        &format!("  %{{reg}} = call {ret_ty} @{spec.c_name}({call_args})"),'
    )
    lines.append("    );")
    lines.append("    ExprValue {")
    lines.append('        reg: format!("%{reg}"),')
    lines.append(f'        ty: "{expr_value_ty(spec.returns)}".into(),')
    lines.append("    }")
    lines.append("}")
    lines.append(marker_end(spec))
    return "\n".join(lines)


def llvm_decl(spec: BuiltinSpec) -> str:
    ret = llvm_return_ty(spec.returns)
    params = ["ptr"]
    for arg in spec.args:
        params.append(llvm_arg_ty(arg.nyra_type))
    if spec.receiver == ReceiverKind.FREE:
        params = [llvm_arg_ty(a.nyra_type) for a in spec.args]
    sig = ", ".join(params)
    return f'("{spec.c_name}", "declare {ret} @{spec.c_name}({sig})"),'


def runtime_map_symbol(spec: BuiltinSpec) -> str:
    return f'("{spec.c_name}", "{spec.rt_module}"),'


def runtime_map_alias(spec: BuiltinSpec) -> str:
    return f'("{spec.method}", "{spec.c_name}"),'


def ownership_owned_entry(spec: BuiltinSpec) -> str:
    return f'"{spec.c_name}",'


def abi_manifest_block(spec: BuiltinSpec) -> str:
    params = []
    if spec.receiver == ReceiverKind.STRING:
        params.append("const char *s")
    for arg in spec.args:
        params.append(f"{c_type(arg.nyra_type)} {arg.name}")
    ret = c_type(spec.returns, is_return=True)
    sig = ", ".join(params) if params else "void"
    # Every runtime_map symbol must appear in the manifest (enforced by the
    # `runtime_map_matches_manifest` ABI test). Non-stable-ABI builtins are
    # recorded as `experimental` so they satisfy that invariant without being
    # required in stdlib/nyra_rt.h (the header only carries stable symbols).
    tier = "stable" if spec.stable_abi else "experimental"
    return "\n".join(
        [
            toml_marker_start(spec),
            "[[symbol]]",
            f'name = "{spec.c_name}"',
            f'c_sig = "{ret} {spec.c_name}({sig})"',
            f'module = "{spec.rt_module}"',
            f'tier = "{tier}"',
            f'since = "{spec.abi_since}"',
            toml_marker_end(spec),
            "",
        ]
    )


def example_ny(spec: BuiltinSpec) -> str:
    if spec.receiver != ReceiverKind.STRING:
        return f'fn main() {{\n    // TODO: demo {spec.method}\n}}\n'
    lines = ["fn main() {"]
    profile_method = spec.method
    # Pick a sample that actually SHOWS the transformation in webDocs. Case /
    # word-splitting methods need a multi-word input, otherwise the gallery
    # renders an example whose output looks identical to the input.
    if "suffix" in spec.method or "prefix" in spec.method:
        sample = '"hamdy.txt"'
    elif spec.method.startswith("to_") or "case" in spec.method:
        sample = '"Hello World"'
    else:
        sample = '"hello"'
    if spec.returns == NyraType.VEC_STR:
        if spec.args:
            arg_vals = []
            for a in spec.args:
                if a.nyra_type == NyraType.STRING:
                    if a.name in ("suffix", "prefix"):
                        arg_vals.append('".txt"')
                    elif a.name == "needle":
                        arg_vals.append('"ell"')
                    elif a.name == "sep":
                        arg_vals.append('","')
                    else:
                        arg_vals.append('"arg"')
                else:
                    arg_vals.append("1")
            lines.append(
                f"    let parts = {sample}.{profile_method}({', '.join(arg_vals)})"
            )
        else:
            sample = '"a b c"' if spec.method == "fields" else sample
            lines.append(f"    let parts = {sample}.{profile_method}()")
        lines.append("    print(parts.len())")
        lines.append("}")
        lines.append("")
        return "\n".join(lines)
    if spec.args:
        arg_vals = []
        for a in spec.args:
            if a.nyra_type == NyraType.STRING:
                if a.name in ("suffix", "prefix"):
                    arg_vals.append('".txt"')
                elif a.name == "needle":
                    arg_vals.append('"ell"')
                elif a.name == "sep":
                    arg_vals.append('","')
                else:
                    arg_vals.append('"arg"')
            else:
                arg_vals.append("1")
        lines.append(f"    print({sample}.{profile_method}({', '.join(arg_vals)}))")
        if spec.free_fn_alias:
            free_args = ", ".join([sample, *arg_vals])
            lines.append(f"    print({spec.method}({free_args}))")
    else:
        lines.append(f"    print({sample}.{profile_method}())")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def example_typed_ny(spec: BuiltinSpec) -> str:
    body = example_ny(spec).replace("fn main()", "fn main() -> void")
    return body


def _test_arg_value(arg: ArgSpec) -> str:
    if arg.nyra_type == NyraType.STRING:
        if arg.name in ("suffix", "prefix"):
            return '".txt"'
        if arg.name in ("sep", "needle", "from"):
            return '","'
        if arg.name == "pad":
            return '"0"'
        return '"x"'
    if arg.nyra_type == NyraType.I32 and arg.name in ("width", "n", "count"):
        return "2"
    return "1"


def _test_assert_line(spec: BuiltinSpec, var: str = "result") -> str:
    if spec.returns == NyraType.STRING:
        return f'    assert_str_eq({var}, "")  // TODO: set expected after C impl'
    if spec.returns in (NyraType.I32, NyraType.BOOL):
        return f'    assert_eq({var}, 0)  // TODO: set expected after C impl'
    if spec.returns == NyraType.VEC_STR:
        return f'    assert_eq({var}.len(), 0)  // TODO: set expected after C impl'
    return f"    // TODO: assert {var}"


def test_ny(spec: BuiltinSpec) -> str:
    test_name = f"test_{spec.receiver.value}_{spec.method}"
    if spec.receiver == ReceiverKind.STRING:
        imports = [
            'import "stdlib/testing.ny"',
            'import "stdlib/strings.ny"',
            'import "stdlib/builtins_string.ny"',
            "",
        ]
        arg_vals = [_test_arg_value(a) for a in spec.args]
        args_suffix = f"({', '.join(arg_vals)})" if arg_vals else "()"
        body_lines = [
            f"test fn {test_name}() {{",
            '    let s = "hamdy.txt"',
            f"    let result = s.{spec.method}{args_suffix}",
            _test_assert_line(spec),
        ]
        if spec.free_fn_alias and spec.args:
            body_lines.extend([
                f"    let result2 = {spec.method}(s, {', '.join(arg_vals)})",
                _test_assert_line(spec, "result2"),
            ])
        body_lines.extend(["}", ""])
        return "\n".join(imports + body_lines)
    return f'import "stdlib/testing.ny"\n\ntest fn {test_name}() {{\n    // TODO\n}}\n'


def array_typecheck_arm(spec: BuiltinSpec) -> str:
    n = len(spec.args)
    lines = [marker_start(spec), f'"{spec.method}" => {{']
    if n == 0:
        lines.append("    if !mc.args.is_empty() {")
        lines.append(
            f'        diagnostics::wrong_arity(self, &format!(".{spec.method}"), 0, mc.args.len(), sp.clone());'
        )
        lines.append("    }")
    else:
        lines.append(f"    if mc.args.len() != {n} {{")
        lines.append(
            f'        diagnostics::wrong_arity(self, &format!(".{spec.method}"), {n}, mc.args.len(), sp.clone());'
        )
        lines.append("    }")
    if spec.returns == NyraType.ARRAY:
        lines.append("    Some(Type::Array {")
        lines.append("        elem: elem.clone(),")
        lines.append("        len: Some(n),")
        lines.append("    })")
    else:
        lines.append(f"    Some({rust_return_type(spec.returns)})")
    lines.extend(["}", marker_end(spec)])
    return "\n".join(lines)


def bytes_method_return_arm(spec: BuiltinSpec) -> str:
    return "\n".join(
        [
            marker_start(spec),
            f'"{spec.method}" => Some({rust_return_type(spec.returns)}),',
            marker_end(spec),
        ]
    )
