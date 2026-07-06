from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def repo_path(*parts: str) -> Path:
    return ROOT.joinpath(*parts)


STRING_PATHS = {
    "rt_c": repo_path("stdlib/rt/rt_strings.c"),
    "extern_ny": repo_path("stdlib/strings.ny"),
    "ops_ny": repo_path("stdlib/strings/ops.ny"),
    "builtins_ny": repo_path("stdlib/builtins_string.ny"),
    "typecheck": repo_path("compiler/typecheck/src/string_builtins.rs"),
    "codegen_util": repo_path("compiler/codegen/src/llvm/util.rs"),
    "codegen_strings": repo_path("compiler/codegen/src/llvm/strings.rs"),
    "codegen_core": repo_path("compiler/codegen/src/llvm/core.rs"),
    "runtime_map": repo_path("compiler/codegen/src/runtime_map.rs"),
    "ownership_kind": repo_path("compiler/ownership/src/kind.rs"),
    "abi_manifest": repo_path("docs/abi-manifest.toml"),
    "example_dir": repo_path("examples/builtins/strings"),
    "test_dir": repo_path("tests/nyra"),
}

ARRAY_PATHS = {
    "typecheck": repo_path("compiler/typecheck/src/array_builtins.rs"),
    "codegen_collections": repo_path("compiler/codegen/src/llvm/collections.rs"),
    "codegen_expr": repo_path("compiler/codegen/src/llvm/expr.rs"),
}

BYTES_PATHS = {
    "rt_c": repo_path("stdlib/rt/rt_bytes.c"),
    "typecheck": repo_path("compiler/typecheck/src/bytes_builtins.rs"),
    "runtime_map": repo_path("compiler/codegen/src/runtime_map.rs"),
    "codegen_core": repo_path("compiler/codegen/src/llvm/core.rs"),
    "ownership_kind": repo_path("compiler/ownership/src/kind.rs"),
    "abi_manifest": repo_path("docs/abi-manifest.toml"),
}

FREE_PATHS = {
    "rt_c": repo_path("stdlib/rt/rt_strings.c"),
    "extern_ny": repo_path("stdlib/strings.ny"),
    "runtime_map": repo_path("compiler/codegen/src/runtime_map.rs"),
    "codegen_core": repo_path("compiler/codegen/src/llvm/core.rs"),
    "ownership_kind": repo_path("compiler/ownership/src/kind.rs"),
    "abi_manifest": repo_path("docs/abi-manifest.toml"),
    "test_dir": repo_path("tests/nyra"),
}
