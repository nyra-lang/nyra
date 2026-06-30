struct Meta {
    module: string
    clone: i32
}

fn main() {
    let module = "demo.app"
    let clone = 1
    let m = Meta { module: module, clone: clone }
    if clone == 1 {
        print(module)
    }
    print(m.module)
}
