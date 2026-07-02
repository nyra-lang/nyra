use errors::{ErrorKind, NyraError, Span, E035_LIFETIME, E036_SEND_SYNC, E037_PARALLEL};

pub fn struct_send_attr_mismatch(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E036_SEND_SYNC,
        ErrorKind::BorrowCheck,
        sp,
        format!("struct `{name}` is marked `Send` but field types are not all `Send`"),
    )
}

pub fn struct_sync_attr_mismatch(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E036_SEND_SYNC,
        ErrorKind::BorrowCheck,
        sp,
        format!("struct `{name}` is marked `Sync` but field types are not all `Sync`"),
    )
}

pub fn capture_not_send(name: &str, context: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E036_SEND_SYNC,
        ErrorKind::BorrowCheck,
        sp,
        format!("cannot use {context}: captured value `{name}` is not `Send`"),
    )
    .note("only `Send` types may cross thread boundaries")
}

pub fn shared_ref_requires_sync(name: &str, context: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E036_SEND_SYNC,
        ErrorKind::BorrowCheck,
        sp,
        format!(
            "cannot use {context}: shared reference `{name}` requires `Sync` inner type",
        ),
    )
    .note("immutable references captured across threads need `T: Sync`")
}

pub fn mut_ref_requires_send(name: &str, context: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E036_SEND_SYNC,
        ErrorKind::BorrowCheck,
        sp,
        format!(
            "cannot use {context}: mutable reference `{name}` requires `Send` inner type",
        ),
    )
    .note("mutable references captured across threads need `T: Send`")
}

pub fn closure_cannot_capture_ref(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E035_LIFETIME,
        ErrorKind::BorrowCheck,
        sp,
        "cannot capture reference in closure; use owned value or copy type",
    )
    .note(format!("captured variable `{name}` has reference type"))
}

pub fn hrtb_lifetime_arity_mismatch(fn_name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E035_LIFETIME,
        ErrorKind::BorrowCheck,
        sp,
        format!(
            "function `{fn_name}` is not compatible with higher-ranked parameter (lifetime arity mismatch)",
        ),
    )
}

pub fn undeclared_lifetime(lt: &str, fn_name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E035_LIFETIME,
        ErrorKind::BorrowCheck,
        sp,
        format!("undeclared lifetime `{lt}` in function `{fn_name}`"),
    )
}

pub fn lifetime_elision_ambiguous(fn_name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E035_LIFETIME,
        ErrorKind::BorrowCheck,
        sp,
        format!(
            "function `{fn_name}` returns a reference but lifetime elision is ambiguous; annotate with explicit lifetimes",
        ),
    )
    .note("when multiple reference parameters exist, specify the return lifetime explicitly, e.g. `fn pick<'a>(a: &'a string, b: &'a string) -> &'a string`")
}

pub fn return_ref_to_local(sp: Span) -> NyraError {
    NyraError::coded(
        E035_LIFETIME,
        ErrorKind::BorrowCheck,
        sp,
        "cannot return reference to local variable; value does not live long enough",
    )
}

pub fn returned_lifetime_too_short(source_lt: &str, expected: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E035_LIFETIME,
        ErrorKind::BorrowCheck,
        sp,
        format!(
            "returned reference lifetime `{source_lt}` does not outlive required lifetime `{expected}`",
        ),
    )
}

pub fn parallel_for_no_break_continue(sp: Span) -> NyraError {
    NyraError::coded(
        E037_PARALLEL,
        ErrorKind::Type,
        sp,
        "`break` / `continue` are not allowed in `parallel for`",
    )
    .note("iterations run concurrently; use a regular `for` loop for early exit")
}

pub fn parallel_for_mutates_outer(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E037_PARALLEL,
        ErrorKind::Type,
        sp,
        format!("`parallel for` cannot mutate outer variable `{name}`"),
    )
    .note("each iteration must be independent; use a local or a regular `for` loop")
}

pub fn struct_cannot_be_copy(name: &str, sp: Span) -> NyraError {
    NyraError::coded(
        E036_SEND_SYNC,
        ErrorKind::Type,
        sp,
        format!("struct `{name}` cannot be `Copy`: not all fields are `Copy` types"),
    )
    .note("remove `#[derive(Copy)]` or use `Clone`/move for heap fields like `string`")
}
