import "stdlib/testing.ny"
import "stdlib/vec.ny"

fn is_even(x: i32) -> i32 {
    if x % 2 == 0 { return 1 }
    return 0
}

test fn test_vec_i32_methods() {
    let v = vec().push(3).push(1).push(2)
    assert_eq(v.sum(), 6)
    assert_eq(v.min_elem(0), 1)
    assert_eq(v.max_elem(0), 3)
    let evens = v.filter(is_even)
    assert_eq(evens.len(), 1)
    assert_eq(v.any(is_even), 1)
    assert_eq(v.all(is_even), 0)
    let taken = v.take(2)
    assert_eq(taken.len(), 2)
    let sorted = vec().push(1).push(3).push(5).push(7)
    assert_eq(sorted.binary_search(5), 2)
    assert_eq(sorted.binary_search(4), -1)
    let mutable = vec().push(3).push(1).push(2)
    let _ = mutable.sort()
    assert_eq(mutable.get(0), 1)
}
