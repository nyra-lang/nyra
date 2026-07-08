union IpAddr repr(C) {
    v4: i32
}

test fn test_union_read_v4() {
    unsafe {
        let u = IpAddr { v4: 0x7F000001 }
        assert_eq(u.v4, 2130706433)
    }
}

fn main() {
    print(0)
}
