fn use_twice(s: string) -> void {
    print(s)
}

test fn test_string_by_value_moves() {
    let msg = "hello"
    use_twice(msg)
    print(msg)
}
