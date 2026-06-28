const TABLE_AT_3: i32 = comptime {
    let mut table: [i32; 4] = [0; 4]
    let mut i: i32 = 0
    while i < 4 {
        table[i] = i * i
        i = i + 1
    }
    table[3]
}

const METHOD: i32 = comptime {
    match "POST" {
        "GET" => 1
        "POST" => 2
        _ => 0
    }
}

const LABEL: string = comptime {
    let a: string = "comptime"
    let b: string = "-power"
    a + b
}

const ARR_LEN: i32 = comptime {
    let xs: [i32; 3] = [10, 20, 30]
    xs.len()
}

const LIT_MATCH: i32 = comptime {
    let n: i32 = 3
    match n {
        3 => 99
        _ => 0
    }
}

fn test_comptime_lookup_table() {
    if TABLE_AT_3 != 9 {
        print("fail lookup", TABLE_AT_3)
    }
}

fn test_comptime_string_match() {
    if METHOD != 2 {
        print("fail method", METHOD)
    }
}

fn test_comptime_string_concat() {
    if LABEL != "comptime-power" {
        print("fail label", LABEL)
    }
}

fn test_comptime_len() {
    if ARR_LEN != 3 {
        print("fail len", ARR_LEN)
    }
}

fn test_comptime_int_literal_match() {
    if LIT_MATCH != 99 {
        print("fail lit match", LIT_MATCH)
    }
}
