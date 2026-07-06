from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def repo_path(*parts: str) -> Path:
    return ROOT.joinpath(*parts)


STDLIB = repo_path("stdlib")
TESTS_NYRA = repo_path("tests/nyra")
EXAMPLES = repo_path("examples")
CONFORMANCE = repo_path("tests/conformance")
RUNTIME_MAP = repo_path("compiler/codegen/src/runtime_map.rs")
ABI_MANIFEST = repo_path("docs/abi-manifest.toml")
CLI_COMMANDS = repo_path("cli/src/commands")
CLI_ARGS = repo_path("cli/src/app/args.rs")
PKG_EXAMPLES = repo_path("examples/packages")
SCAFFOLD_DIR = repo_path("docs/contrib_scaffold")
