extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string
extern fn str_push_char(s: string, ch: i32) -> string
extern fn str_pop(s: string) -> string
extern fn substring(s: &string, start: i32, len: i32) -> string
extern fn char_at(s: &string, i: i32) -> i32
extern fn strcmp(a: &string, b: &string) -> i32

struct TextBuffer {
    text: string
    cursor: i32
    max_len: i32
}

fn TextBuffer_new(max_len: i32) -> TextBuffer {
    return TextBuffer { text: "", cursor: 0, max_len: max_len }
}

fn TextBuffer_len(buf: TextBuffer) -> i32 {
    return strlen(buf.text)
}

fn TextBuffer_insert_char(mut buf: TextBuffer, ch: i32) -> TextBuffer {
    if strlen(buf.text) >= buf.max_len {
        return buf
    }
    let head = substring(buf.text, 0, buf.cursor)
    let tail = substring(buf.text, buf.cursor, strlen(buf.text) - buf.cursor)
    let piece = str_push_char("", ch)
    buf.text = strcat(strcat(head, piece), tail)
    buf.cursor = buf.cursor + 1
    return buf
}

fn TextBuffer_backspace(mut buf: TextBuffer) -> TextBuffer {
    if buf.cursor <= 0 {
        return buf
    }
    let head = substring(buf.text, 0, buf.cursor - 1)
    let tail = substring(buf.text, buf.cursor, strlen(buf.text) - buf.cursor)
    buf.text = strcat(head, tail)
    buf.cursor = buf.cursor - 1
    return buf
}

fn TextBuffer_cursor_left(mut buf: TextBuffer) -> TextBuffer {
    if buf.cursor > 0 {
        buf.cursor = buf.cursor - 1
    }
    return buf
}

fn TextBuffer_cursor_right(mut buf: TextBuffer) -> TextBuffer {
    if buf.cursor < strlen(buf.text) {
        buf.cursor = buf.cursor + 1
    }
    return buf
}

fn TextBuffer_line_col_at(text: string, idx: i32) -> i32 {
    let mut line = 0
    let mut col = 0
    let mut i = 0
    while i < idx && i < strlen(text) {
        if char_at(text, i) == 10 {
            line = line + 1
            col = 0
        } else {
            col = col + 1
        }
        i = i + 1
    }
    return line * 10000 + col
}

fn TextBuffer_index_for_line_col(text: string, line: i32, col: i32) -> i32 {
    let mut cur_line = 0
    let mut cur_col = 0
    let mut i = 0
    let n = strlen(text)
    while i <= n {
        if cur_line == line && cur_col == col {
            return i
        }
        if i >= n {
            break
        }
        if char_at(text, i) == 10 {
            if cur_line == line {
                return i
            }
            cur_line = cur_line + 1
            cur_col = 0
        } else {
            cur_col = cur_col + 1
        }
        i = i + 1
    }
    return n
}

fn TextBuffer_cursor_up(mut buf: TextBuffer) -> TextBuffer {
    if buf.cursor <= 0 {
        return buf
    }
    let packed = TextBuffer_line_col_at(buf.text, buf.cursor)
    let line = packed / 10000
    let col = packed - line * 10000
    if line <= 0 {
        buf.cursor = 0
        return buf
    }
    buf.cursor = TextBuffer_index_for_line_col(buf.text, line - 1, col)
    return buf
}

fn TextBuffer_cursor_down(mut buf: TextBuffer) -> TextBuffer {
    let packed = TextBuffer_line_col_at(buf.text, buf.cursor)
    let line = packed / 10000
    let col = packed - line * 10000
    let next = TextBuffer_index_for_line_col(buf.text, line + 1, col)
    if next > buf.cursor {
        buf.cursor = next
    }
    return buf
}

fn TextBuffer_poll_keys(mut buf: TextBuffer, backspace_pressed: i32, left: i32, right: i32, up: i32, down: i32, ch: i32) -> TextBuffer {
    if backspace_pressed == 1 {
        buf = TextBuffer_backspace(buf)
    }
    if left == 1 {
        buf = TextBuffer_cursor_left(buf)
    }
    if right == 1 {
        buf = TextBuffer_cursor_right(buf)
    }
    if up == 1 {
        buf = TextBuffer_cursor_up(buf)
    }
    if down == 1 {
        buf = TextBuffer_cursor_down(buf)
    }
    if ch > 0 {
        buf = TextBuffer_insert_char(buf, ch)
    }
    return buf
}
