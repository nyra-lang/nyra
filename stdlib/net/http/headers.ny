// Header map helpers for net/http (request + response).
import "../../map.ny"
import "../../strings.ny"
import "../../vec_str.ny"

fn HeaderMap_new() -> HashMap_str_str {
    return HashMap_str_str_new()
}

fn HeaderMap_set(headers: HashMap_str_str, name: string, value: string) -> HashMap_str_str {
    return headers.insert(name, value)
}

fn HeaderMap_get(headers: HashMap_str_str, name: string) -> string {
    if headers.contains(name) == 1 {
        return headers.get(name)
    }
    // Case-insensitive fallback for common HTTP header names.
    let keys = headers.keys()
    let n = keys.len()
    let needle = str_to_lowercase(name)
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        if strcmp(str_to_lowercase(k), needle) == 0 {
            return headers.get(k)
        }
        i = i + 1
    }
    return ""
}

fn HeaderMap_has(headers: HashMap_str_str, name: string) -> i32 {
    if strlen(HeaderMap_get(headers, name)) > 0 {
        return 1
    }
    if headers.contains(name) == 1 {
        return 1
    }
    return 0
}

fn HeaderMap_format(headers: HashMap_str_str) -> string {
    let keys = headers.keys()
    let n = keys.len()
    let mut out = ""
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        let v = headers.get(k)
        out = strcat(out, strcat(strcat(strcat(k, ": "), v), "\r\n"))
        i = i + 1
    }
    return out
}

fn HeaderMap_parse_line(headers: HashMap_str_str, line: string) -> HashMap_str_str {
    let colon = strstr_pos(line, ":")
    if colon <= 0 {
        return headers
    }
    let name = substring(line, 0, colon)
    let mut value = substring(line, colon + 1, strlen(line) - (colon + 1))
    while strlen(value) > 0 && char_at(value, 0) == 32 {
        value = substring(value, 1, strlen(value) - 1)
    }
    return HeaderMap_set(headers, name, value)
}

fn HeaderMap_parse_raw(raw: string) -> HashMap_str_str {
    let mut headers = HeaderMap_new()
    let sep = strstr_pos(raw, "\r\n\r\n")
    let mut head = clone raw
    if sep >= 0 {
        head = substring(clone raw, 0, sep)
    }
    // Walk lines without overlapping substring views on the same buffer.
    let mut start = 0
    let n = strlen(head)
    let mut first = 1
    while start < n {
        let mut end = start
        while end + 1 < n {
            if char_at(head, end) == 13 && char_at(head, end + 1) == 10 {
                break
            }
            end = end + 1
        }
        if end + 1 >= n {
            // last line (no CRLF) — include the final character at end.
            if first == 0 && n > start {
                let line = substring(clone head, start, n - start)
                headers = HeaderMap_parse_line(headers, line)
            }
            break
        }
        if first == 0 {
            let line = substring(clone head, start, end - start)
            headers = HeaderMap_parse_line(headers, line)
        }
        first = 0
        start = end + 2
    }
    return headers
}
