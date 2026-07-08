fn len(s: &string) -> i32 {
    return strlen(s)
}

fn greet(s: &string) -> void {
    print(s)
}

test fn test_string_auto_borrow() {
    let msg = "hello"
    let n = len(msg)
    assert_eq(n, 5)
    greet(msg)
    print(msg)
}
