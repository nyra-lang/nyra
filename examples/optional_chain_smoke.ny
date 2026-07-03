struct Val { n: i32 }
enum Option { None, Some(Val) }
fn Val_id(v: Val) -> Val { return v }
fn main() {
    let some = Option.Some(Val { n: 5 })
    let empty = Option.None
    let _got: Val = some?.id()
    let _miss: Val = empty?.id()
    print(1)
}
