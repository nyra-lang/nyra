enum Status {
    Ok
    Err
}

const ENUM_CODE = comptime {
    let s = Status.Ok
    match s {
        Status.Ok => 1
        Status.Err => 2
    }
}

const INT_BUCKET = comptime {
    let n = 7
    match n {
        _ if n < 5 => 1
        _ if n < 10 => 2
        _ => 3
    }
}

const BOOL_FLAG = comptime {
    let b = true
    match b {
        true => 10
        false => 0
    }
}

fn test_comptime_match_enum() {
    if ENUM_CODE != 1 {
        print("fail enum match", ENUM_CODE)
    }
}

fn test_comptime_match_int_guard() {
    if INT_BUCKET != 2 {
        print("fail int guard match", INT_BUCKET)
    }
}

fn test_comptime_match_bool() {
    if BOOL_FLAG != 10 {
        print("fail bool match", BOOL_FLAG)
    }
}
