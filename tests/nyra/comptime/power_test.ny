const TABLE_AT_3 = comptime {
    let mut table = [0; 4]
    let mut i = 0
    while i < 4 {
        table[i] = i * i
        i = i + 1
    }
    table[3]
}

const METHOD = comptime {
    match "POST" {
        "GET" => 1
        "POST" => 2
        _ => 0
    }
}

const LABEL = comptime {
    let a = "comptime"
    let b = "-power"
    a + b
}

const ARR_LEN = comptime {
    let xs = [10, 20, 30]
    xs.len()
}

const LIT_MATCH = comptime {
    let n = 3
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
