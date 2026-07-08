const WHILE_SUM = comptime {
    let mut acc = 0
    let mut i = 0
    while i < 5 {
        acc = acc + i
        i = i + 1
    }
    acc
}

const BREAK_SUM = comptime {
    let mut acc = 0
    for i in 0..100 {
        if i == 10 {
            break
        }
        acc = acc + i
    }
    acc
}

const CONTINUE_EVENS = comptime {
    let mut acc = 0
    for i in 0..6 {
        if i % 2 == 0 {
            continue
        }
        acc = acc + i
    }
    acc
}

fn test_comptime_while() {
    if WHILE_SUM != 10 {
        print("fail while", WHILE_SUM)
    }
}

fn test_comptime_break() {
    if BREAK_SUM != 45 {
        print("fail break", BREAK_SUM)
    }
}

fn test_comptime_continue() {
    if CONTINUE_EVENS != 9 {
        print("fail continue", CONTINUE_EVENS)
    }
}
