extern fn blackbox_i32(x: i32) -> i32

test fn test_parallel_any_range() {
    let hit = parallel any for i in 0..16 {
        blackbox_i32(i) == 10
    }
    if !hit {
        assert(0)
    }
}

test fn test_parallel_any_miss() {
    let hit = parallel any for i in 0..4 {
        blackbox_i32(i) == 99
    }
    if hit {
        assert(0)
    }
}

test fn test_parallel_find_range() {
    let idx = parallel find for i in 0..16 {
        blackbox_i32(i) == 7
    }
    assert_eq(idx, 7)
}

test fn test_parallel_find_miss() {
    let idx = parallel find for i in 0..4 {
        blackbox_i32(i) == 99
    }
    assert_eq(idx, -1)
}

test fn test_parallel_all_range() {
    let ok = parallel all for i in 0..8 {
        blackbox_i32(i) >= 0
    }
    if !ok {
        assert(0)
    }
}

test fn test_parallel_all_fail() {
    let ok = parallel all for i in 0..8 {
        blackbox_i32(i) > 0
    }
    if ok {
        assert(0)
    }
}

test fn test_parallel_any_array() {
    let nums = [1, 2, 3, 4]
    let hit = parallel any for n in nums {
        n == 3
    }
    if !hit {
        assert(0)
    }
}

test fn test_parallel_any_max_workers() {
    let hit = parallel(max = 2) any for i in 0..8 {
        blackbox_i32(i) == 5
    }
    if !hit {
        assert(0)
    }
}
