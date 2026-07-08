// defer runs LIFO (last deferred, first executed).
allow_extended

fn a() {
    print("a")
}

fn b() {
    print("b")
}

fn main() {
    defer b()
    defer a()
    print("body")
}
