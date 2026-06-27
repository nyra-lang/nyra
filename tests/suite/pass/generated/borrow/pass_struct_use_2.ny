struct Box {
    id: i32
    label: string
}
fn main() {
    let b = Box { id: 2 label: "item" }
    print(b.id)
    print(b.label)
}
