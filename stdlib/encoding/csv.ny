import "../strings.ny"
import "../vec_str.ny"

fn csv_split_row(line: string) -> StrVec {
    let mut out = StrVec_new()
    let n = strlen(line)
    let mut i = 0
    let mut field = ""
    let mut in_quote = 0
    while i < n {
        let c = char_at(line, i)
        if in_quote != 0 {
            if c == 34 {
                if i + 1 < n && char_at(line, i + 1) == 34 {
                    field = strcat(field, "\"")
                    i = i + 2
                } else {
                    in_quote = 0
                    i = i + 1
                }
            } else {
                field = strcat(field, substring(line, i, 1))
                i = i + 1
            }
        } else {
            if c == 34 {
                in_quote = 1
                i = i + 1
            } else {
                if c == 44 {
                    out = out.push(field)
                    field = ""
                    i = i + 1
                } else {
                    field = strcat(field, substring(line, i, 1))
                    i = i + 1
                }
            }
        }
    }
    out = out.push(field)
    return out
}

fn csv_needs_quote(field: string) -> i32 {
    if strstr_pos(field, ",") >= 0 {
        return 1
    }
    if strstr_pos(field, "\"") >= 0 {
        return 1
    }
    if strstr_pos(field, "\n") >= 0 {
        return 1
    }
    return 0
}

fn csv_escape_field(field: string) -> string {
    if csv_needs_quote(field) == 0 {
        return field
    }
    let n = strlen(field)
    let mut out = "\""
    let mut i = 0
    while i < n {
        let ch = substring(field, i, 1)
        if strcmp(ch, "\"") == 0 {
            out = strcat(out, "\"\"")
        } else {
            out = strcat(out, ch)
        }
        i = i + 1
    }
    return strcat(out, "\"")
}

fn csv_format_row(fields: StrVec) -> string {
    let n = fields.len()
    if n == 0 {
        return ""
    }
    let mut out = csv_escape_field(fields.get(0))
    let mut i = 1
    while i < n {
        out = strcat(strcat(out, ","), csv_escape_field(fields.get(i)))
        i = i + 1
    }
    return out
}

fn csv_parse_line(line: string) -> StrVec {
    return csv_split_row(line)
}

fn csv_join_rows(rows: ptr) -> string {
    let n = Vec_str_len(rows)
    if n == 0 {
        return ""
    }
    let mut out = Vec_str_get(rows, 0)
    let mut i = 1
    while i < n {
        out = strcat(out, "\n")
        out = strcat(out, Vec_str_get(rows, i))
        i = i + 1
    }
    return out
}
