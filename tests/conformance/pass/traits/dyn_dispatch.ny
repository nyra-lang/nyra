import "stdlib/testing.ny"

trait Scale {
    fn scale(self, n: i32) -> i32
}

struct Twice {
    base: i32
}

impl Scale for Twice {
    fn scale(self, n: i32) -> i32 {
        return self.base * n
    }
}

fn call_scale(g: dyn Scale) -> i32 {
    return g.scale(3)
}

test fn conf_trait_003_dyn_dispatch() {
    let t = Twice { base: 4 }
    assert_eq(call_scale(t as dyn Scale), 12)
}
