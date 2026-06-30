import "../strings.ny"
import "../vec_str.ny"

extern fn stdin_read_line(prompt: string) -> string

struct Scanner {
    data: string
    pos: i32
    token: string
    done: i32
}

fn Scanner_new(data: string) -> Scanner {
    return Scanner { data: data, pos: 0, token: "", done: 0 }
}

fn Scanner_from_lines(text: string) -> Scanner {
    return Scanner_new(text)
}

fn scanner_find_line_end(data: string, start: i32) -> i32 {
    let n = strlen(data)
    let mut i = start
    while i < n {
        let c = char_at(data, i)
        if c == 10 {
            return i
        }
        i = i + 1
    }
    return n
}

fn Scanner_scan(s: Scanner) -> Scanner {
    if s.done != 0 {
        return Scanner { data: s.data, pos: s.pos, token: "", done: 1 }
    }
    let n = strlen(s.data)
    if s.pos >= n {
        return Scanner { data: s.data, pos: s.pos, token: "", done: 1 }
    }
    let end = scanner_find_line_end(s.data, s.pos)
    let line = substring(s.data, s.pos, end - s.pos)
    let mut next = end
    if next < n && char_at(s.data, next) == 10 {
        next = next + 1
    }
    let mut done = 0
    if next >= n {
        done = 1
    }
    return Scanner { data: s.data, pos: next, token: line, done: done }
}

fn Scanner_text(s: Scanner) -> string {
    return s.token
}

fn Scanner_ok(s: Scanner) -> i32 {
    if s.done != 0 && strlen(s.token) == 0 {
        return 0
    }
    return 1
}

fn ReadLine(prompt: string) -> string {
    return stdin_read_line(prompt)
}

fn ReadString(prompt: string) -> string {
    return stdin_read_line(prompt)
}
