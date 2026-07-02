mod diagnostics;
mod diag;
mod lang_features;
mod recovery;
mod parse_program;
mod parse_decl;
mod parse_stmt;
mod parse_expr;
mod parse_util;

use ast::*;
use errors::NyraError;
use lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    pub errors: Vec<NyraError>,
    parsed_enum_names: Vec<String>,
    parsed_struct_names: Vec<String>,
    pending_struct_attrs: StructAttrs,
    pending_fn_attrs: FnAttrs,
}

#[derive(Default)]
struct FnAttrs {
    inline: bool,
    hot: bool,
    cold: bool,
    comptime: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::Lexer;

    #[test]
    fn parses_sum_loop_assignments() {
        let src = r#"fn main() {
    mut sum = 0
    mut i = 0
    let n = 1000
    while i < n {
        sum = sum + i
        i = i + 1
    }
    print(sum)
}"#;
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(program.functions.len(), 1);
        let stmts = &program.functions[0].body.statements;
        let has_assign = stmts.iter().any(|s| match s {
            Statement::Assign(_) => true,
            Statement::While(w) => w
                .body
                .statements
                .iter()
                .any(|inner| matches!(inner, Statement::Assign(_))),
            _ => false,
        });
        assert!(has_assign, "expected assign inside while body");
    }

    #[test]
    fn parses_main_with_print() {
        let src = r#"fn main() {
    let x = 10
    print(x)
}"#;
        let (tokens, _) = Lexer::new(src, "test.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn parses_enum_and_match() {
        let src = r#"enum Color { Red Green }
fn main() {
    let c = Color.Red
    let n = match c {
        Color.Red => 1
        Color.Green => 2
    }
    print(n)
}"#;
        let (tokens, _) = Lexer::new(src, "e.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(program.enums.len(), 1);
        assert_eq!(program.enums[0].name, "Color");
    }

    #[test]
    fn parses_struct_and_impl() {
        let src = r#"struct Point { x: i32 y: i32 }
impl Point {
    fn zero() -> Point {
        Point { x: 0, y: 0 }
    }
}
fn main() { print(0) }"#;
        let (tokens, _) = Lexer::new(src, "s.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(program.structs.len(), 1);
        assert_eq!(program.impls.len(), 1);
    }

    #[test]
    fn parses_for_in_array() {
        let src = r#"fn main() {
    let arr = [1, 2, 3]
    for x in arr {
        print(x)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(program.functions[0]
            .body
            .statements
            .iter()
            .any(|s| matches!(
                s,
                Statement::For(ForStmt {
                    kind: ForKind::Iterable { .. },
                    ..
                })
            )));
    }

    #[test]
    fn parses_array_of_named_struct_literals() {
        let src = r#"struct NumberColor {
    number: i32
    color: string
}

fn main() {
    let collections = [
        NumberColor { number: 1, color: "red" },
        NumberColor { number: 2, color: "blue" },
        NumberColor { number: 3, color: "orange" }
    ]
    for i in collections {
        print(i.number)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "structs.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let Statement::Let(let_stmt) = &program.functions[0].body.statements[0] else {
            panic!("expected let");
        };
        let Expression::ArrayLiteral(al) = &let_stmt.value else {
            panic!("expected array literal, got {:?}", let_stmt.value);
        };
        assert_eq!(al.elems.len(), 3);
        for el in &al.elems {
            assert!(
                matches!(el, Expression::StructLiteral(sl) if sl.name == "NumberColor"),
                "expected NumberColor struct literal, got {el:?}"
            );
        }
    }

    #[test]
    fn parses_array_of_anonymous_struct_literals() {
        let src = r#"fn main() {
    let collections = [
        { number: 1, color: "red" },
        { number: 2, color: "blue" },
        { number: 3, color: "orange" }
    ]
    for i in collections {
        print(i.number)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "anon.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let Statement::Let(let_stmt) = &program.functions[0].body.statements[0] else {
            panic!("expected let");
        };
        let Expression::ArrayLiteral(al) = &let_stmt.value else {
            panic!("expected array literal, got {:?}", let_stmt.value);
        };
        assert_eq!(al.elems.len(), 3);
        for el in &al.elems {
            assert!(
                matches!(el, Expression::StructLiteral(sl) if sl.name.is_empty()),
                "expected anonymous struct literal, got {el:?}"
            );
        }
    }

    #[test]
    fn parses_benchmark_block() {
        let src = r#"fn main() {
    benchmark {
        run()
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(program.functions[0].body.statements.iter().any(|s| {
            matches!(s, Statement::Benchmark(_))
        }));
    }

    #[test]
    fn parses_progress_for() {
        let src = r#"fn main() {
    progress(label = "parser tests") for item in tests {
        run(item)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(program.functions[0].body.statements.iter().any(|s| {
            matches!(s, Statement::For(f) if f.progress.is_some())
        }));
    }

    #[test]
    fn parses_parallel_cpu_percent() {
        let src = r#"fn main() {
    parallel(cpu = 80%) for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let Statement::For(f) = &program.functions[0].body.statements[0] else {
            panic!("expected for");
        };
        let cfg = f.parallel.as_ref().expect("parallel");
        assert!(matches!(cfg.threads, ParallelThreads::CpuPercent(_)));
    }

    #[test]
    fn parses_parallel_for_max_key() {
        let src = r#"fn main() {
    parallel:task(max = 4) for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let stmt = &program.functions[0].body.statements[0];
        let Statement::For(f) = stmt else {
            panic!("expected for");
        };
        let cfg = f.parallel.as_ref().expect("parallel");
        assert_eq!(cfg.kind, SpawnKind::Task);
        assert!(matches!(cfg.threads, ParallelThreads::Max(_)));
    }

    #[test]
    fn parses_parallel_for_max_threads_legacy_emits_note() {
        let src = r#"fn main() {
    parallel(max_threads = 2) for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (mut parser) = Parser::new(tokens);
        let (program, errs) = parser.parse();
        assert!(program.functions[0].body.statements.iter().any(|s| {
            matches!(s, Statement::For(f) if f.parallel.is_some())
        }));
        assert!(
            errs.iter().any(|e| e.message.contains("prefer `max`")),
            "{errs:?}"
        );
    }

    #[test]
    fn parses_parallel_for_with_options() {
        let src = r#"fn main() {
    parallel(max = 4, mode = balanced) for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let stmt = &program.functions[0].body.statements[0];
        let Statement::For(f) = stmt else {
            panic!("expected for");
        };
        let cfg = f.parallel.as_ref().expect("parallel");
        assert!(matches!(cfg.threads, ParallelThreads::Max(_)));
        assert_eq!(cfg.mode, ParallelMode::Balanced);
    }

    #[test]
    fn parses_parallel_for_thread_kind() {
        let src = r#"fn main() {
    parallel:thread(max = 4) for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let stmt = &program.functions[0].body.statements[0];
        let Statement::For(f) = stmt else {
            panic!("expected for");
        };
        let cfg = f.parallel.as_ref().expect("parallel");
        assert_eq!(cfg.kind, SpawnKind::Thread);
        assert!(matches!(cfg.threads, ParallelThreads::Max(_)));
    }

    #[test]
    fn parses_parallel_for_default_task_kind() {
        let src = r#"fn main() {
    parallel for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let stmt = &program.functions[0].body.statements[0];
        let Statement::For(f) = stmt else {
            panic!("expected for");
        };
        let cfg = f.parallel.as_ref().expect("parallel");
        assert_eq!(cfg.kind, SpawnKind::Task);
    }

    #[test]
    fn parses_parallel_for_backend_option() {
        let src = r#"fn main() {
    parallel(backend = thread, threads = 2) for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let stmt = &program.functions[0].body.statements[0];
        let Statement::For(f) = stmt else {
            panic!("expected for");
        };
        let cfg = f.parallel.as_ref().expect("parallel");
        assert_eq!(cfg.kind, SpawnKind::Thread);
    }

    #[test]
    fn parses_parallel_any_for() {
        let src = r#"fn main() {
    let hit = parallel any for i in 0..10 {
        i > 5
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(program.functions[0].body.statements.iter().any(|s| {
            matches!(
                s,
                Statement::Let(l) if matches!(
                    &l.value,
                    Expression::ParallelSearch(ps) if ps.config.op == ParallelOp::Any
                )
            )
        }));
    }

    #[test]
    fn parses_parallel_for() {
        let src = r#"fn main() {
    parallel for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(program.functions[0].body.statements.iter().any(|s| {
            matches!(s, Statement::For(f) if f.parallel.is_some())
        }));
    }

    #[test]
    fn parses_for_range() {
        let src = r#"fn main() {
    for i in 0..10 {
        print(i)
    }
}"#;
        let (tokens, _) = Lexer::new(src, "f.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(program.functions[0]
            .body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::For(_))));
    }

    #[test]
    fn parses_generic_fn_header() {
        let src = r#"fn id<T>(x: T) -> T {
    return x
}
fn main() { print(0) }"#;
        let (tokens, _) = Lexer::new(src, "g.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(program.functions[0].type_params, vec!["T".to_string()]);
    }

    #[test]
    fn parses_multiline_match_arm_arrow() {
        let src = r#"fn main() {
    let n = match 1 {
        1
        => 2
        _ => 0
    }
    print(n)
}"#;
        let (tokens, _) = Lexer::new(src, "m.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(program.functions.len(), 1);
    }

    #[test]
    fn parses_soft_keyword_bindings() {
        let src = r#"fn main() {
    let module = "x"
    let clone = 1
    print(module)
}"#;
        let (tokens, _) = Lexer::new(src, "k.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let main = &program.functions[0];
        let names: Vec<_> = main
            .body
            .statements
            .iter()
            .filter_map(|s| {
                if let Statement::Let(l) = s {
                    Some(l.name.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(names.contains(&"module"));
        assert!(names.contains(&"clone"));
    }

    #[test]
    fn parses_try_in_match_arm_body() {
        let src = r#"fn main() {
    let r = 1
    let n = match r {
        Result.Ok(x) => step(x)?,
        Result.Err(e) => 0,
    }
    print(n)
}"#;
        let (tokens, _) = Lexer::new(src, "m.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let main = &program.functions[0];
        let let_stmt = main.body.statements.iter().find_map(|s| {
            if let Statement::Let(l) = s {
                if l.name == "n" {
                    return Some(l);
                }
            }
            None
        }).expect("let n");
        if let Expression::Match(m) = &let_stmt.value {
            let trailing = ast::block_trailing_expression(&m.arms[0].body);
            assert!(matches!(
                trailing,
                Some(Expression::Unary(ref u)) if u.op == ast::UnaryOp::Try
            ));
        } else {
            panic!("expected match expr, got {:?}", let_stmt.value);
        }
    }

    #[test]
    fn parses_nested_generic_type_vec_vec_i32() {
        let src = r#"fn main() {
    let mut grid: Vec<Vec<i32>> = Vec_Vec_i32_new()
    print(0)
}"#;
        let (tokens, _) = Lexer::new(src, "nested.ny").tokenize();
        let (program, errs) = Parser::new(tokens).parse();
        assert!(errs.is_empty(), "{errs:?}");
        let let_stmt = match &program.functions[0].body.statements[0] {
            Statement::Let(l) => l,
            other => panic!("expected let, got {other:?}"),
        };
        assert!(matches!(
            let_stmt.ty,
            Some(TypeAnnotation::Applied { ref base, .. }) if base == "Vec"
        ));
    }
}
