// Multi-trait dyn A + B — typed
// nyra test tests/nyra/dyn_multi_trait_test.typed.ny

trait Add {
    fn add(self, other: i32) -> i32
}

trait Scale {
    fn scale(self, factor: i32) -> i32
}

struct Counter {
    value: i32
}

impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}

impl Scale for Counter {
    fn scale(self, factor: i32) -> i32 {
        return self.value * factor
    }
}

fn use_both(g: dyn Add + Scale) -> i32 {
    return g.add(1) + g.scale(2)
}

test fn test_dyn_ab_typed() {
    let c: Counter = Counter { value: 10 }
    let result: i32 = use_both(c as dyn Add + Scale)
    assert_eq(result, 31)
}
