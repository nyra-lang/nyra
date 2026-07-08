import "stdlib/testing.ny"

trait Add {
    fn add(self, other: i32) -> i32
}

struct Counter {
    value: i32
}

impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}

test fn conf_trait_001_static_impl() {
    let c = Counter { value: 5 }
    assert_eq(c.add(3), 8)
}

test fn conf_trait_002_static_second_instance() {
    let c = Counter { value: 10 }
    assert_eq(c.add(2), 12)
}
