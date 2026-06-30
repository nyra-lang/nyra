// JSON Lines + JSON array helpers (one object per line, or `[{...},...]`).
import "../strings.ny"
import "../vec_str.ny"

extern fn json_split_array_elements(array_json: string) -> ptr

fn Json_is_array_body(text: string) -> i32 {
    let t = trim(text)
    if strlen(t) == 0 {
        return 0
    }
    return str_starts_with(t, "[")
}

fn Json_array_elements(array_json: string) -> StrVec {
    return StrVec { handle: json_split_array_elements(array_json) }
}

fn Json_non_empty_lines(text: string) -> StrVec {
    let lines = StrVec_from_lines(text)
    let mut out = StrVec_new()
    let mut i = 0
    while i < lines.len() {
        let line = trim(lines.get(i))
        if strlen(line) > 0 && str_starts_with(line, "#") == 0 {
            out = out.push(line)
        }
        i = i + 1
    }
    return out
}
