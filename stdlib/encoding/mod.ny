import "../strings.ny"

// hex_encode_byte — two-char lowercase hex for 0..255 (MVP table lookup).
fn encoding_hex_digit(n: i32) -> string {
    if n < 10 {
        return i32_to_string(n)
    }
    if n == 10 {
        return "a"
    }
    if n == 11 {
        return "b"
    }
    if n == 12 {
        return "c"
    }
    if n == 13 {
        return "d"
    }
    if n == 14 {
        return "e"
    }
    return "f"
}

fn hex_encode_byte(b: i32) -> string {
    let hi = (b / 16) % 16
    let lo = b % 16
    return strcat(encoding_hex_digit(hi), encoding_hex_digit(lo))
}

fn url_is_unreserved(c: i32) -> i32 {
    if c >= 65 && c <= 90 {
        return 1
    }
    if c >= 97 && c <= 122 {
        return 1
    }
    if c >= 48 && c <= 57 {
        return 1
    }
    if c == 45 || c == 46 || c == 95 || c == 126 {
        return 1
    }
    return 0
}

// url_encode — RFC 3986 percent-encoding (unreserved characters left as-is).
fn url_encode(s: string) -> string {
    let n = strlen(s)
    let mut out = ""
    let mut i = 0
    while i < n {
        let c = char_at(s, i)
        if url_is_unreserved(c) == 1 {
            out = strcat(out, char_from_code(c))
        } else {
            out = strcat(out, strcat("%", hex_encode_byte(c)))
        }
        i = i + 1
    }
    return out
}

fn hex_nibble(c: i32) -> i32 {
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

fn url_decode(s: string) -> string {
    let n = strlen(s)
    let mut out = ""
    let mut i = 0
    while i < n {
        let c = char_at(s, i)
        if c == 43 {
            out = strcat(out, " ")
            i = i + 1
        } else {
            if c == 37 && i + 2 < n {
                let hi = hex_nibble(char_at(s, i + 1))
                let lo = hex_nibble(char_at(s, i + 2))
                if hi >= 0 && lo >= 0 {
                    out = strcat(out, char_from_code(hi * 16 + lo))
                    i = i + 3
                } else {
                    out = strcat(out, "%")
                    i = i + 1
                }
            } else {
                out = strcat(out, char_from_code(c))
                i = i + 1
            }
        }
    }
    return out
}

// base64_encode / base64_decode — see `encoding/base64.ny` (binary-safe decode).
import "base64.ny"

fn b64(s: string) -> string {
    return base64_encode(s)
}

fn b64d(s: string) -> string {
    return base64_decode(s)
}
// [contrib-dev:hex_decode:encoding_mod]
extern fn hex_decode(hex: &string) -> string
// [/contrib-dev:hex_decode:encoding_mod]
