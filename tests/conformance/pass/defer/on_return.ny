import "stdlib/testing.ny"

fn noop() -> void {
    let _ = 1
}

fn with_defer_return() -> i32 {
    defer noop()
    return 9
}

fn with_defer_lifo() -> i32 {
    defer noop()
    defer noop()
    return 3
}

test fn conf_defer_001_runs_on_return() {
    assert_eq(with_defer_return(), 9)
}

test fn conf_defer_002_lifo_on_return() {
    assert_eq(with_defer_lifo(), 3)
}
