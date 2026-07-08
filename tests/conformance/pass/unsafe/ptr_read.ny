import "stdlib/testing.ny"

union IpAddr repr(C) {
    v4: i32
}

test fn conf_unsafe_001_union_field_read() {
    unsafe {
        let u = IpAddr { v4: 0x7F000001 }
        assert_eq(u.v4, 2130706433)
    }
}

test fn conf_unsafe_002_raw_ptr_read() {
    let x = 42
    unsafe {
        let p = &x as *i32
        assert_eq(*p, 42)
    }
}
