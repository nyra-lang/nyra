import "os/syscall.ny"

fn test_fail(msg: string) -> void {
    print(msg)
    os_exit(1)
}

fn assert_eq(actual: i32, expected: i32) -> void {
    if actual != expected {
        test_fail("assert_eq failed")
    }
}

fn assert_ne(actual: i32, expected: i32) -> void {
    if actual == expected {
        test_fail("assert_ne failed")
    }
}

fn assert_true(cond: i32) -> void {
    if cond == 0 {
        test_fail("assert_true failed")
    }
}

fn assert(cond: i32) -> void {
    assert_true(cond)
}

fn assert_bool(cond: bool) -> void {
    if cond == false {
        test_fail("assert_bool failed")
    }
}

fn assert_str_eq(actual: string, expected: string) -> void {
    if actual != expected {
        test_fail("assert_str_eq failed")
    }
}

// Use with `nyra test` — test fn my_test() { assert_eq(2 + 3, 5) }
