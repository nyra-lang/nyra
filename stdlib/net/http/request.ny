import "../../strings.ny"
import "types.ny"

fn method_from_line(line: string) -> i32 {
    if strlen(line) < 3 {
        return 0
    }
    if strcmp(substring(line, 0, 3), "GET") == 0 {
        return METHOD_GET
    }
    if strlen(line) >= 4 {
        if strcmp(substring(line, 0, 4), "POST") == 0 {
            return METHOD_POST
        }
        if strcmp(substring(line, 0, 4), "HEAD") == 0 {
            return METHOD_HEAD
        }
    }
    if strlen(line) >= 3 {
        if strcmp(substring(line, 0, 3), "PUT") == 0 {
            return METHOD_PUT
        }
    }
    if strlen(line) >= 6 {
        if strcmp(substring(line, 0, 6), "DELETE") == 0 {
            return METHOD_DELETE
        }
    }
    if strlen(line) >= 7 {
        if strcmp(substring(line, 0, 7), "OPTIONS") == 0 {
            return METHOD_OPTIONS
        }
    }
    if strlen(line) >= 5 {
        if strcmp(substring(line, 0, 5), "PATCH") == 0 {
            return METHOD_PATCH
        }
    }
    return 0
}

fn path_from_line(line: string) -> string {
    let sp1 = strstr_pos(line, " ")
    if sp1 < 0 {
        return "/"
    }
    let start = sp1 + 1
    let rest = substring(line, start, strlen(line) - start)
    let sp2 = strstr_pos(rest, " ")
    if sp2 < 0 {
        return rest
    }
    let path_q = substring(rest, 0, sp2)
    let q = strstr_pos(path_q, "?")
    if q < 0 {
        return path_q
    }
    return substring(path_q, 0, q)
}

fn query_from_line(line: string) -> string {
    let sp1 = strstr_pos(line, " ")
    if sp1 < 0 {
        return ""
    }
    let start = sp1 + 1
    let rest = substring(line, start, strlen(line) - start)
    let sp2 = strstr_pos(rest, " ")
    if sp2 < 0 {
        return ""
    }
    let path_q = substring(rest, 0, sp2)
    let q = strstr_pos(path_q, "?")
    if q < 0 {
        return ""
    }
    return substring(path_q, q + 1, strlen(path_q) - (q + 1))
}

fn first_line(raw: string) -> string {
    let nl = strstr_pos(raw, "\r\n")
    if nl < 0 {
        return raw
    }
    return substring(raw, 0, nl)
}

fn header_value(raw: string, name: string) -> string {
    let needle = strcat(strcat(name, ": "), "")
    let pos = strstr_pos(raw, needle)
    if pos < 0 {
        return ""
    }
    let start = pos + strlen(needle)
    let rest = substring(raw, start, strlen(raw) - start)
    let end = strstr_pos(rest, "\r\n")
    if end < 0 {
        return rest
    }
    return substring(rest, 0, end)
}

fn wants_keep_alive(raw: string) -> i32 {
    let conn = header_value(raw, "Connection")
    if strstr_pos(conn, "keep-alive") >= 0 {
        return 1
    }
    if strstr_pos(conn, "Keep-Alive") >= 0 {
        return 1
    }
    return 0
}

fn is_chunked_transfer(raw: string) -> i32 {
    let te = header_value(raw, "Transfer-Encoding")
    if strstr_pos(te, "chunked") >= 0 {
        return 1
    }
    return 0
}

fn http_hex_nibble(c: i32) -> i32 {
    if c >= 48 && c <= 57 {
        return c - 48
    }
    if c >= 97 && c <= 102 {
        return c - 97 + 10
    }
    if c >= 65 && c <= 70 {
        return c - 65 + 10
    }
    return -1
}

fn str_to_i32_hex(s: string) -> i32 {
    let n = strlen(s)
    let mut i = 0
    let mut v = 0
    while i < n {
        let c = char_at(s, i)
        // Skip whitespace; stop at chunk-extension separator ';'.
        if c == 32 || c == 9 {
            i = i + 1
            continue
        }
        if c == 59 {
            break
        }
        let d = http_hex_nibble(c)
        if d < 0 {
            break
        }
        v = v * 16 + d
        i = i + 1
    }
    return v
}

fn decode_chunked_body(body: string) -> string {
    let mut out = ""
    let mut rest = body
    while strlen(rest) > 0 {
        let nl = strstr_pos(rest, "\r\n")
        if nl < 0 {
            break
        }
        let size_hex = substring(rest, 0, nl)
        let size = str_to_i32_hex(size_hex)
        if size <= 0 {
            break
        }
        let chunk_start = nl + 2
        if chunk_start + size > strlen(rest) {
            break
        }
        let chunk = substring(rest, chunk_start, size)
        out = strcat(out, chunk)
        rest = substring(rest, chunk_start + size + 2, strlen(rest) - (chunk_start + size + 2))
    }
    return out
}

fn body_from_raw(raw: string) -> string {
    let sep = strstr_pos(raw, "\r\n\r\n")
    if sep < 0 {
        return ""
    }
    let body = substring(raw, sep + 4, strlen(raw) - (sep + 4))
    if is_chunked_transfer(raw) == 1 {
        return decode_chunked_body(body)
    }
    return body
}

fn method_name(method: i32) -> string {
    if method == METHOD_GET { return "GET" }
    if method == METHOD_POST { return "POST" }
    if method == METHOD_PUT { return "PUT" }
    if method == METHOD_DELETE { return "DELETE" }
    if method == METHOD_PATCH { return "PATCH" }
    if method == METHOD_HEAD { return "HEAD" }
    if method == METHOD_OPTIONS { return "OPTIONS" }
    return "GET"
}

fn RequestContext_from_raw(raw: string) -> RequestContext {
    let line = first_line(raw)
    return RequestContext {
        method: method_from_line(line),
        path: path_from_line(line),
        body: body_from_raw(raw),
        query: query_from_line(line),
        raw: raw,
        params: HashMap_str_str_new(),
    }
}

fn route_key(method: i32, path: string) -> string {
    let m = method_name(method)
    let prefix = strcat(m, ":")
    return strcat(prefix, path)
}
