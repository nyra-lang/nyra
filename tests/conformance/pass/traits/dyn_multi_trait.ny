// CONF: multi-trait dyn A + B dispatch
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

fn main() {
    let c = Counter { value: 10 }
    assert_eq(use_both(c as dyn Add + Scale), 31)
}
