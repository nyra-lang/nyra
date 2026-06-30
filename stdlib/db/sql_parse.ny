// SQL subset parser — SELECT / INSERT with WHERE expressions (col op val).

struct SqlAst {
    kind: string
    table: string
    column: string
    columns: StrVec
    values: StrVec
    set_col: string
    set_val: string
    where_col: string
    where_op: string
    where_val: string
}

fn SqlParse_upper(token: string) -> string {
    return str_to_upper(token)
}

fn SqlParse_join_csv(vec: StrVec) -> string {
    let n = vec.len()
    if n == 0 {
        return ""
    }
    let mut out = vec.get(0)
    let mut i = 1
    while i < n {
        out = strcat(out, ", ")
        out = strcat(out, vec.get(i))
        i = i + 1
    }
    return out
}

fn SqlParse_tokenize(sql: string) -> StrVec {
    return String_split_quoted(sql, " ")
}

fn SqlParse_strip_parens(text: string) -> string {
    let n = strlen(text)
    if n < 2 {
        return text
    }
    if char_at(text, 0) != 40 {
        return text
    }
    if char_at(text, n - 1) != 41 {
        return text
    }
    return substring(text, 1, n - 2)
}

fn SqlParse_split_csv(text: string) -> StrVec {
    let inner = SqlParse_strip_parens(text)
    if strstr_pos(inner, ",") < 0 {
        let mut one = StrVec_new()
        return one.push(inner)
    }
    return StrVec { handle: String_split(inner, ",") }
}

fn SqlParse_find_op(text: string) -> string {
    if strstr_pos(text, "!=") >= 0 {
        return "!="
    }
    if strstr_pos(text, ">=") >= 0 {
        return ">="
    }
    if strstr_pos(text, "<=") >= 0 {
        return "<="
    }
    if strstr_pos(text, "=") >= 0 {
        return "="
    }
    if strstr_pos(text, ">") >= 0 {
        return ">"
    }
    if strstr_pos(text, "<") >= 0 {
        return "<"
    }
    return ""
}

fn SqlParse_trim(text: string) -> string {
    let n = strlen(text)
    if n == 0 {
        return text
    }
    let mut start = 0
    while start < n {
        let c = char_at(text, start)
        if c != 32 && c != 9 {
            break
        }
        start = start + 1
    }
    let mut end = n
    while end > start {
        let c = char_at(text, end - 1)
        if c != 32 && c != 9 {
            break
        }
        end = end - 1
    }
    if start == 0 && end == n {
        return text
    }
    return substring(text, start, end - start)
}

fn SqlParse_parse_predicate(predicate: string) -> StrVec {
    let op = SqlParse_find_op(predicate)
    if strlen(op) == 0 {
        let mut out = StrVec_new()
        out = out.push("")
        out = out.push("")
        out = out.push(SqlParse_trim(predicate))
        return out
    }
    let pos = strstr_pos(predicate, op)
    let col = SqlParse_trim(substring(predicate, 0, pos))
    let rest = substring(predicate, pos + strlen(op), strlen(predicate) - pos - strlen(op))
    let mut out = StrVec_new()
    out = out.push(col)
    out = out.push(op)
    out = out.push(SqlParse_trim(rest))
    return out
}

fn SqlParse_error() -> SqlAst {
    return SqlAst {
        kind: "error",
        table: "",
        column: "",
        columns: StrVec_new(),
        values: StrVec_new(),
        set_col: "",
        set_val: "",
        where_col: "",
        where_op: "",
        where_val: ""
    }
}

fn SqlParse_parse_where(tokens: StrVec, start: i32) -> StrVec {
    let n = tokens.len()
    if start < 0 || start >= n {
        let mut empty = StrVec_new()
        empty = empty.push("")
        empty = empty.push("")
        empty = empty.push("")
        return empty
    }
    if strcmp(SqlParse_upper(tokens.get(start)), "WHERE") != 0 {
        let mut empty = StrVec_new()
        empty = empty.push("")
        empty = empty.push("")
        empty = empty.push("")
        return empty
    }
    let mut pred = tokens.get(start + 1)
    let mut k = start + 2
    while k < n {
        pred = strcat(pred, strcat(" ", tokens.get(k)))
        k = k + 1
    }
    return SqlParse_parse_predicate(pred)
}

fn SqlParse_parse_select(tokens: StrVec) -> SqlAst {
    let n = tokens.len()
    if n < 4 {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(0)), "SELECT") != 0 {
        return SqlParse_error()
    }
    let column = tokens.get(1)
    if strcmp(SqlParse_upper(tokens.get(2)), "FROM") != 0 {
        return SqlParse_error()
    }
    let table = tokens.get(3)
    let where_parts = SqlParse_parse_where(tokens, 4)
    return SqlAst {
        kind: "select",
        table: table,
        column: column,
        columns: StrVec_new(),
        values: StrVec_new(),
        set_col: "",
        set_val: "",
        where_col: where_parts.get(0),
        where_op: where_parts.get(1),
        where_val: where_parts.get(2)
    }
}

fn SqlParse_parse_insert(tokens: StrVec) -> SqlAst {
    let n = tokens.len()
    if n < 5 {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(0)), "INSERT") != 0 {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(1)), "INTO") != 0 {
        return SqlParse_error()
    }
    let table = tokens.get(2)
    let mut idx = 3
    let mut columns = StrVec_new()
    if char_at(tokens.get(idx), 0) == 40 {
        columns = SqlParse_split_csv(tokens.get(idx))
        idx = idx + 1
    }
    if idx >= n {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(idx)), "VALUES") != 0 {
        return SqlParse_error()
    }
    idx = idx + 1
    if idx >= n {
        return SqlParse_error()
    }
    let values = SqlParse_split_csv(tokens.get(idx))
    return SqlAst {
        kind: "insert",
        table: table,
        column: "",
        columns: columns,
        values: values,
        set_col: "",
        set_val: "",
        where_col: "",
        where_op: "",
        where_val: ""
    }
}

fn SqlParse_join_from(tokens: StrVec, start: i32, stop_word: string) -> string {
    let n = tokens.len()
    if start >= n {
        return ""
    }
    if strlen(stop_word) > 0 {
        if strcmp(SqlParse_upper(tokens.get(start)), stop_word) == 0 {
            return ""
        }
    }
    let mut out = tokens.get(start)
    let mut i = start + 1
    while i < n {
        if strlen(stop_word) > 0 {
            if strcmp(SqlParse_upper(tokens.get(i)), stop_word) == 0 {
                return out
            }
        }
        out = strcat(out, " ")
        out = strcat(out, tokens.get(i))
        i = i + 1
    }
    return out
}

fn SqlParse_find_token(tokens: StrVec, start: i32, word: string) -> i32 {
    let n = tokens.len()
    let mut i = start
    while i < n {
        if strcmp(SqlParse_upper(tokens.get(i)), word) == 0 {
            return i
        }
        i = i + 1
    }
    return -1
}

fn SqlParse_parse_update(tokens: StrVec) -> SqlAst {
    let n = tokens.len()
    if n < 5 {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(0)), "UPDATE") != 0 {
        return SqlParse_error()
    }
    let table = tokens.get(1)
    if strcmp(SqlParse_upper(tokens.get(2)), "SET") != 0 {
        return SqlParse_error()
    }
    let where_idx = SqlParse_find_token(tokens, 3, "WHERE")
    let set_expr = SqlParse_join_from(tokens, 3, "WHERE")
    let set_parts = SqlParse_parse_predicate(set_expr)
    let where_parts = SqlParse_parse_where(tokens, where_idx)
    return SqlAst {
        kind: "update",
        table: table,
        column: "",
        columns: StrVec_new(),
        values: StrVec_new(),
        set_col: set_parts.get(0),
        set_val: set_parts.get(2),
        where_col: where_parts.get(0),
        where_op: where_parts.get(1),
        where_val: where_parts.get(2)
    }
}

fn SqlParse_parse_delete(tokens: StrVec) -> SqlAst {
    let n = tokens.len()
    if n < 3 {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(0)), "DELETE") != 0 {
        return SqlParse_error()
    }
    if strcmp(SqlParse_upper(tokens.get(1)), "FROM") != 0 {
        return SqlParse_error()
    }
    let table = tokens.get(2)
    let where_parts = SqlParse_parse_where(tokens, 3)
    return SqlAst {
        kind: "delete",
        table: table,
        column: "",
        columns: StrVec_new(),
        values: StrVec_new(),
        set_col: "",
        set_val: "",
        where_col: where_parts.get(0),
        where_op: where_parts.get(1),
        where_val: where_parts.get(2)
    }
}

fn SqlParse_parse(sql: string) -> SqlAst {
    let tokens = SqlParse_tokenize(sql)
    if tokens.len() == 0 {
        return SqlParse_error()
    }
    let head = SqlParse_upper(tokens.get(0))
    if strcmp(head, "SELECT") == 0 {
        return SqlParse_parse_select(tokens)
    }
    if strcmp(head, "INSERT") == 0 {
        return SqlParse_parse_insert(tokens)
    }
    if strcmp(head, "UPDATE") == 0 {
        return SqlParse_parse_update(tokens)
    }
    if strcmp(head, "DELETE") == 0 {
        return SqlParse_parse_delete(tokens)
    }
    return SqlParse_error()
}

fn SqlParse_format_where(ast: SqlAst) -> string {
    if strlen(ast.where_col) == 0 {
        return ""
    }
    return strcat(" WHERE ", strcat(ast.where_col, strcat(" ", strcat(ast.where_op, strcat(" ", ast.where_val)))))
}

fn SqlParse_format(ast: SqlAst) -> string {
    if strcmp(ast.kind, "update") == 0 {
        return strcat("UPDATE ", strcat(ast.table, strcat(" SET ", strcat(ast.set_col, strcat(" = ", strcat(ast.set_val, SqlParse_format_where(ast)))))))
    }
    if strcmp(ast.kind, "delete") == 0 {
        return strcat("DELETE FROM ", strcat(ast.table, SqlParse_format_where(ast)))
    }
    if strcmp(ast.kind, "insert") == 0 {
        let mut cols = ""
        if ast.columns.len() > 0 {
            cols = strcat(" (", strcat(SqlParse_join_csv(ast.columns), ")"))
        }
        return strcat("INSERT INTO ", strcat(ast.table, strcat(cols, strcat(" VALUES (", strcat(SqlParse_join_csv(ast.values), ")")))))
    }
    if strcmp(ast.kind, "select") != 0 {
        return "PARSE_ERROR"
    }
    if strlen(ast.where_col) == 0 {
        return strcat("SELECT ", strcat(ast.column, strcat(" FROM ", ast.table)))
    }
    return strcat("SELECT ", strcat(ast.column, strcat(" FROM ", strcat(ast.table, SqlParse_format_where(ast)))))
}
