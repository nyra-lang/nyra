use ast::{Expression, Statement};
use lexer::Lexer;
use parser::Parser;

#[test]
fn parses_multiline_method_chain() {
    let src = r#"fn main() {
    let mut log = StrVec_new()
    log = log
        .push("a")
}"#;
    let (tokens, _) = Lexer::new(src, "t.ny").tokenize();
    let (prog, errors) = Parser::new(tokens).parse();
    assert!(errors.is_empty(), "{errors:?}");
    let Statement::Assign(a) = &prog.functions[0].body.statements[1] else {
        panic!("expected assignment statement");
    };
    let Expression::MethodCall(m) = &a.value else {
        panic!("expected method call, got {:?}", a.value);
    };
    assert_eq!(m.method, "push");
}
