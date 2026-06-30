extern fn strcat(a: &string, b: &string) -> string
extern fn str_push_char(s: string, ch: i32) -> string

struct StringBuilder {
    buf: string
}

fn StringBuilder_new() -> StringBuilder {
    return StringBuilder { buf: "" }
}

fn StringBuilder_push(mut sb: StringBuilder, piece: string) -> StringBuilder {
    sb.buf = strcat(clone sb.buf, piece)
    return sb
}

fn StringBuilder_push_char(mut sb: StringBuilder, ch: i32) -> StringBuilder {
    sb.buf = str_push_char(sb.buf, ch)
    return sb
}

fn StringBuilder_build(sb: StringBuilder) -> string {
    return sb.buf
}
