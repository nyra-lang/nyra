extern fn str_buf_new() -> ptr
extern fn str_buf_drop(handle: ptr) -> void
extern fn str_buf_append(handle: ptr, piece: &string) -> void
extern fn str_buf_append_char(handle: ptr, ch: i32) -> void
extern fn str_buf_build(handle: ptr) -> string
extern fn strcat(a: &string, b: &string) -> string
extern fn i32_to_string(n: i32) -> string

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

fn sb() -> StringBuilder {
    return StringBuilder_new()
}

fn cat(a: string, b: string) -> string {
    return strcat(a, b)
}

fn cat3(a: string, b: string, c: string) -> string {
    return strcat(strcat(a, b), c)
}

fn cat4(a: string, b: string, c: string, d: string) -> string {
    return strcat(strcat(strcat(a, b), c), d)
}

impl StringBuilder {
    fn push(self, piece: string) -> StringBuilder {
        return StringBuilder_push(self, piece)
    }

    fn push_char(self, ch: i32) -> StringBuilder {
        return StringBuilder_push_char(self, ch)
    }

    fn push_i32(self, n: i32) -> StringBuilder {
        return StringBuilder_push(self, i32_to_string(n))
    }

    fn build(self) -> string {
        return StringBuilder_build(self)
    }
}

impl Drop for StringBuilder {
    fn drop(mut self) -> void {
        str_buf_drop(self.handle)
    }
}
