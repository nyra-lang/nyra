extern fn str_buf_new() -> ptr
extern fn str_buf_drop(handle: ptr) -> void
extern fn str_buf_append(handle: ptr, piece: &string) -> void
extern fn str_buf_append_char(handle: ptr, ch: i32) -> void
extern fn str_buf_build(handle: ptr) -> string

struct StringBuilder {
    handle: ptr
}

fn StringBuilder_new() -> StringBuilder {
    return StringBuilder { handle: str_buf_new() }
}

fn StringBuilder_push(mut sb: StringBuilder, piece: string) -> StringBuilder {
    str_buf_append(sb.handle, piece)
    return sb
}

fn StringBuilder_push_char(mut sb: StringBuilder, ch: i32) -> StringBuilder {
    str_buf_append_char(sb.handle, ch)
    return sb
}

fn StringBuilder_build(sb: StringBuilder) -> string {
    return str_buf_build(sb.handle)
}

impl Drop for StringBuilder {
    fn drop(mut self) -> void {
        str_buf_drop(self.handle)
    }
}
