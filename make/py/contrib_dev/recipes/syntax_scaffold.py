"""Recipe: new syntax / keyword scaffold (checklist + tests — no auto lexer/parser edits)."""
from __future__ import annotations

from .. import patch, templates
from ..paths import EXAMPLES, SCAFFOLD_DIR, TESTS_NYRA
from ..spec import RecipeResult, SyntaxSpec, TestExampleSpec


def apply(spec: SyntaxSpec, *, force: bool = False) -> RecipeResult:
    marker = spec.marker
    root = SCAFFOLD_DIR / f"syntax_{spec.feature_name}"
    res = RecipeResult(
        title="Syntax Scaffold",
        recipe="syntax-scaffold",
        marker=marker,
        patches=[],
    )

    res.patches.append(
        patch.write_new_file(
            root / "CHECKLIST.md", templates.syntax_checklist(spec, marker), marker, force=force
        )
    )
    res.patches.append(
        patch.write_new_file(
            root / "grammar_snippet.json",
            templates.syntax_grammar_hint(spec, marker),
            marker,
            force=force,
        )
    )

    test_spec = TestExampleSpec(
        name=f"{spec.feature_name}_syntax",
        example_topic="syntax",
        use_testing=True,
    )
    test_path = TESTS_NYRA / f"{spec.feature_name}_syntax_test.ny"
    typed_path = TESTS_NYRA / f"{spec.feature_name}_syntax_test.typed.ny"
    res.patches.append(
        patch.write_new_file(test_path, templates.test_ny(test_spec, marker), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(typed_path, templates.test_typed_ny(test_spec, marker), marker, force=force)
    )

    ex_spec = TestExampleSpec(name=spec.feature_name, example_topic="syntax", use_testing=False)
    ex_path = EXAMPLES / "syntax" / f"{spec.feature_name}.ny"
    ex_typed = EXAMPLES / "syntax" / f"{spec.feature_name}.typed.ny"
    res.patches.append(
        patch.write_new_file(ex_path, templates.example_ny(ex_spec, marker), marker, force=force)
    )
    res.patches.append(
        patch.write_new_file(ex_typed, templates.example_typed_ny(ex_spec, marker), marker, force=force)
    )

    res.user_tasks = [
        f"Follow docs/contrib_scaffold/syntax_{spec.feature_name}/CHECKLIST.md",
        f"Implement lexer → parser → ast → typecheck → codegen for `{spec.keyword}`",
        f"Update grammar/nyra.tmLanguage.json (hint in grammar_snippet.json)",
        f"Fill tests/nyra/{spec.feature_name}_syntax_test.ny",
        "make install-dev && cargo test -p compiler",
    ]
    res.warnings.append("Syntax changes are NOT auto-wired — checklist + stubs only.")
    return res
