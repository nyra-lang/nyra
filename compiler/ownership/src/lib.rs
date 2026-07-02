mod context;
mod copy_attrs;
mod diag;
mod drop;
mod escape;
mod kind;
mod lifetime;
mod no_escape;
mod nll;
mod parallel_for;
mod send_sync;
mod subtype;

use ast::Program;

pub use copy_attrs::check_copy_attrs;
pub use context::OwnershipCtx;
pub use drop::DropPlan;
pub use escape::{analyze_escapes, EscapePlan, EscapeState};
pub use kind::{callee_returns_owned, ownership_of, OwnershipKind};
pub use no_escape::check_no_escape;
pub use nll::{
    arrow_has_captures, arrow_to_block, collect_arrow_captures, collect_captures,
    compute_last_uses,
};
pub use parallel_for::{block_has_break, check_parallel_for_body, collect_assigned_in_block};
pub use send_sync::{
    check_parallel_for_captures, check_program as check_send_sync_program, check_spawn_captures,
    check_sync_closure_captures, is_send, is_sync, thread_safety_of, thread_safety_of_struct,
    ThreadSafety,
};
pub use subtype::{lifetime_outlives, unify_hrtb_call};

pub fn analyze_program(program: &Program) -> (OwnershipCtx, DropPlan) {
    let ctx = OwnershipCtx::from_program(program);
    let plan = drop::plan_drops(program, &ctx);
    (ctx, plan)
}

pub fn check_lifetimes(program: &Program, ctx: &OwnershipCtx, errors: &mut Vec<errors::NyraError>) {
    lifetime::check_program(program, ctx, errors);
}

pub fn check_send_sync(program: &Program, ctx: &OwnershipCtx, errors: &mut Vec<errors::NyraError>) {
    send_sync::check_program(program, ctx, errors);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast::Program;
    use types::Type;

    fn parse_program(src: &str) -> Program {
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        parser::Parser::new(tokens).parse().0
    }

    #[test]
    fn string_is_move_ownership() {
        let program = parse_program(r#"fn main() { let s = "hi" }"#);
        let (ctx, _) = analyze_program(&program);
        assert_eq!(ctx.kind_of(&Type::String), OwnershipKind::Move);
    }

    #[test]
    fn i32_is_copy_ownership() {
        let program = parse_program(r#"fn main() { let n = 42 }"#);
        let (ctx, _) = analyze_program(&program);
        assert_eq!(ctx.kind_of(&Type::Integer(ast::IntKind::I32)), OwnershipKind::Copy);
    }

    #[test]
    fn drop_plan_tracks_string_binding() {
        let program = parse_program(
            r#"extern fn read_file(path: string) -> string
fn main() {
    let s = read_file("/tmp/x")
    print(s)
}"#,
        );
        let (_, plan) = analyze_program(&program);
        assert!(plan.is_owned_in("main", "s"));
    }

    #[test]
    fn send_sync_i32_is_send() {
        let program = parse_program(r#"fn main() { print(0) }"#);
        let (ctx, _) = analyze_program(&program);
        assert!(is_send(&Type::Integer(ast::IntKind::I32), &ctx));
        assert!(is_sync(&Type::Integer(ast::IntKind::I32), &ctx));
    }

    #[test]
    fn explicit_send_marker_rejects_non_send_fields() {
        let program = parse_program(
            r#"struct Bad Send {
    next: &Bad
}
fn main() {}"#,
        );
        let (ctx, _) = analyze_program(&program);
        let mut errors = Vec::new();
        check_send_sync(&program, &ctx, &mut errors);
        assert!(
            errors.iter().any(|e| e.message.contains("marked `Send`")),
            "expected Send marker validation error, got: {errors:?}"
        );
    }

    #[test]
    fn drop_plan_tracks_composite_struct_binding() {
        let program = parse_program(
            r#"extern fn read_file(path: string) -> string
struct Packet { id: i32 body: string }
fn main() {
    let p = Packet { id: 1 body: read_file("/tmp/x") }
    print(p.id)
}"#,
        );
        let (_, plan) = analyze_program(&program);
        assert!(plan.is_composite_struct_in("main", "p"));
    }

    #[test]
    fn owned_extern_return_from_declaration() {
        let program = parse_program(
            r#"extern fn my_load() -> string
fn main() {
    let s = my_load()
    print(s)
}"#,
        );
        let (ctx, plan) = analyze_program(&program);
        assert!(ctx.callee_returns_owned("my_load"));
        assert!(plan.is_owned_in("main", "s"));
    }
}
