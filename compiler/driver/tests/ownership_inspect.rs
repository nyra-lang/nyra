use borrowck::{check_program_inspect, InspectQuery};
use compiler::{CompileOptions, CompileStage, Compiler};
use expand::{coerce_auto_borrow, expand_program, finish_async_desugar};
use const_eval::fold_program_consts;
use lexer::Lexer;
use monomorph::monomorphize_program;
use ownership::analyze_program;
use parser::Parser;
use typecheck::TypeChecker;

const SAMPLE: &str = r#"fn main() {
    let name = "Ada"
    let r = &name
    print(r)
}"#;

fn program_after_pipeline(src: &str, file: &str) -> ast::Program {
    let (tokens, _) = Lexer::new(src, file).tokenize();
    let (mut program, _) = Parser::new(tokens).parse();
    expand_program(&mut program);
    monomorphize_program(&mut program);
    coerce_auto_borrow(&mut program);
    fold_program_consts(&mut program);
    let mut tc = TypeChecker::new();
    tc.check_program(&program);
    tc.apply_inferred_signatures(&mut program);
    finish_async_desugar(&mut program, &tc);
    program
}

#[test]
fn inspect_binding_after_full_pipeline() {
    let program = program_after_pipeline(SAMPLE, "t.ny");
    let (ctx, _) = analyze_program(&program);
    let query = InspectQuery {
        file: "t.ny".into(),
        line: 4,
        name: "name".into(),
    };
    let mut errors = vec![];
    let report = check_program_inspect(&program, &ctx, &mut errors, Some(&query))
        .expect("inspect report");
    assert_eq!(report.name, "name");
    assert!(!report.borrowed_by.is_empty());
}

#[test]
fn inspect_binding_via_driver() {
    let query = InspectQuery {
        file: "t.ny".into(),
        line: 4,
        name: "name".into(),
    };
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        inspect_query: Some(query),
        no_prelude: true,
        ..CompileOptions::default()
    };
    let out = Compiler::compile_source(SAMPLE, "t.ny", &options).unwrap();
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let report = out
        .inspect_report
        .expect("expected inspect report from driver");
    assert_eq!(report.name, "name");
    assert!(!report.borrowed_by.is_empty(), "{:?}", report.borrowed_by);
}
