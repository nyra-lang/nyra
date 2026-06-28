struct Point {
    x: i32
    y: i32
}

enum Box {
    Empty
    Val
}

enum Opt {
    None
    Some(i32)
}

const STRUCT_SUM: i32 = comptime {
    let p = Point { x: 3, y: 4 }
    match p {
        Point { x, y } => x + y
    }
}

const FIELD_ACCESS: i32 = comptime {
    let p = Point { x: 10, y: 5 }
    p.x + p.y
}

const TUPLE_SUM: i32 = comptime {
    let pair = (10, 20)
    match pair {
        (a, b) => a + b
    }
}

const ENUM_PAYLOAD: i32 = comptime {
    let v = Box.Val
    match v {
        Box.Empty => 0
        Box.Val => 99
    }
}

const OPT_PAYLOAD: i32 = comptime {
    let o = Opt.Some(42)
    match o {
        Opt.None => 0
        Opt.Some(x) => x
    }
}

fn test_comptime_struct_match() {
    if STRUCT_SUM != 7 {
        print("fail struct match", STRUCT_SUM)
    }
}

fn test_comptime_struct_field() {
    if FIELD_ACCESS != 15 {
        print("fail struct field", FIELD_ACCESS)
    }
}

fn test_comptime_tuple_match() {
    if TUPLE_SUM != 30 {
        print("fail tuple match", TUPLE_SUM)
    }
}

fn test_comptime_enum_unit() {
    if ENUM_PAYLOAD != 99 {
        print("fail enum", ENUM_PAYLOAD)
    }
}

fn test_comptime_enum_payload() {
    if OPT_PAYLOAD != 42 {
        print("fail opt payload", OPT_PAYLOAD)
    }
}
