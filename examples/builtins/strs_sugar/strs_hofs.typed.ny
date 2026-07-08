fn longer_than_3(s: string) -> i32 {
    if s.len() > 3 { return 1 }
    return 0
}

fn main() -> void {
    let names = strs().push("ada").push("nyra").push("bob")
    print(names.joined(","))
    print(names.contains("nyra"))
    print(names.filter(longer_than_3).get(0))
    print(lines("a\nb\nc").len())
}
