// Fluent SQL query builder — emits SQL strings for SqliteDb / SqlDb.
// Style mirrors HTTP: qb().select(...).from(...).where(...).to_sql()
import "../strings.ny"

struct SqlQuery {
    columns: string
    table: string
    joins: string
    where_sql: string
    order_sql: string
    limit_n: i32
    distinct: i32
}

fn qb() -> SqlQuery {
    return SqlQuery {
        columns: "*",
        table: "",
        joins: "",
        where_sql: "",
        order_sql: "",
        limit_n: -1,
        distinct: 0,
    }
}

fn qb_from(table: string) -> SqlQuery {
    return qb().from(table)
}

// Quote a SQL string literal (single quotes, escape ' as '').
fn sql_quote(value: string) -> string {
    let n = strlen(value)
    let mut out = "'"
    let mut i = 0
    while i < n {
        let c = char_at(value, i)
        if c == 39 {
            out = strcat(out, "''")
        } else {
            out = strcat(out, char_from_code(c))
        }
        i = i + 1
    }
    return strcat(out, "'")
}

fn SqlQuery_append_where(q: SqlQuery, fragment: string) -> SqlQuery {
    let mut where_sql = q.where_sql
    if strlen(where_sql) == 0 {
        where_sql = fragment
    } else {
        where_sql = strcat(strcat(where_sql, " AND "), fragment)
    }
    return SqlQuery {
        columns: q.columns,
        table: q.table,
        joins: q.joins,
        where_sql: where_sql,
        order_sql: q.order_sql,
        limit_n: q.limit_n,
        distinct: q.distinct,
    }
}

impl SqlQuery {
    fn select(self, columns: string) -> SqlQuery {
        return SqlQuery {
            columns: columns,
            table: self.table,
            joins: self.joins,
            where_sql: self.where_sql,
            order_sql: self.order_sql,
            limit_n: self.limit_n,
            distinct: self.distinct,
        }
    }

    fn distinct(self) -> SqlQuery {
        return SqlQuery {
            columns: self.columns,
            table: self.table,
            joins: self.joins,
            where_sql: self.where_sql,
            order_sql: self.order_sql,
            limit_n: self.limit_n,
            distinct: 1,
        }
    }

    fn from(self, table: string) -> SqlQuery {
        return SqlQuery {
            columns: self.columns,
            table: table,
            joins: self.joins,
            where_sql: self.where_sql,
            order_sql: self.order_sql,
            limit_n: self.limit_n,
            distinct: self.distinct,
        }
    }

    // where(col, op, val) — val is quoted as a string literal.
    fn where(self, col: string, op: string, val: string) -> SqlQuery {
        let frag = strcat(strcat(strcat(col, " "), op), strcat(" ", sql_quote(val)))
        return SqlQuery_append_where(self, frag)
    }

    // where_raw("users.id = posts.user_id") — unquoted fragment.
    fn where_raw(self, fragment: string) -> SqlQuery {
        return SqlQuery_append_where(self, fragment)
    }

    fn and(self, col: string, op: string, val: string) -> SqlQuery {
        return self.where(col, op, val)
    }

    // INNER JOIN table ON <on>
    fn include(self, table: string, on: string) -> SqlQuery {
        let piece = strcat(strcat(strcat(" INNER JOIN ", table), " ON "), on)
        return SqlQuery {
            columns: self.columns,
            table: self.table,
            joins: strcat(self.joins, piece),
            where_sql: self.where_sql,
            order_sql: self.order_sql,
            limit_n: self.limit_n,
            distinct: self.distinct,
        }
    }

    // lookup(related, local_col, foreign_col) — JOIN related ON related.foreign = local
    fn lookup(self, related: string, local_col: string, foreign_col: string) -> SqlQuery {
        let on = strcat(strcat(strcat(related, "."), foreign_col), strcat(" = ", local_col))
        return self.include(related, on)
    }

    // unwind(child, on) — LEFT JOIN to expand related/nested rows (document-style name).
    fn unwind(self, table: string, on: string) -> SqlQuery {
        let piece = strcat(strcat(strcat(" LEFT JOIN ", table), " ON "), on)
        return SqlQuery {
            columns: self.columns,
            table: self.table,
            joins: strcat(self.joins, piece),
            where_sql: self.where_sql,
            order_sql: self.order_sql,
            limit_n: self.limit_n,
            distinct: self.distinct,
        }
    }

    fn order(self, col: string) -> SqlQuery {
        return SqlQuery {
            columns: self.columns,
            table: self.table,
            joins: self.joins,
            where_sql: self.where_sql,
            order_sql: col,
            limit_n: self.limit_n,
            distinct: self.distinct,
        }
    }

    fn order_desc(self, col: string) -> SqlQuery {
        return self.order(strcat(col, " DESC"))
    }

    fn limit(self, n: i32) -> SqlQuery {
        return SqlQuery {
            columns: self.columns,
            table: self.table,
            joins: self.joins,
            where_sql: self.where_sql,
            order_sql: self.order_sql,
            limit_n: n,
            distinct: self.distinct,
        }
    }

    fn to_sql(self) -> string {
        let mut sql = "SELECT "
        if self.distinct == 1 {
            sql = strcat(sql, "DISTINCT ")
        }
        sql = strcat(sql, self.columns)
        sql = strcat(sql, " FROM ")
        sql = strcat(sql, self.table)
        if strlen(self.joins) > 0 {
            sql = strcat(sql, self.joins)
        }
        if strlen(self.where_sql) > 0 {
            sql = strcat(strcat(sql, " WHERE "), self.where_sql)
        }
        if strlen(self.order_sql) > 0 {
            sql = strcat(strcat(sql, " ORDER BY "), self.order_sql)
        }
        if self.limit_n >= 0 {
            sql = strcat(strcat(sql, " LIMIT "), i32_to_string(self.limit_n))
        }
        return sql
    }
}

// --- short write helpers (string SQL) ---

fn sql_insert(table: string, columns: string, values_sql: string) -> string {
    return strcat(
        strcat(strcat(strcat(strcat("INSERT INTO ", table), " ("), columns), ") VALUES ("),
        strcat(values_sql, ")")
    )
}

fn sql_update(table: string, set_sql: string, where_sql: string) -> string {
    let mut sql = strcat(strcat(strcat("UPDATE ", table), " SET "), set_sql)
    if strlen(where_sql) > 0 {
        sql = strcat(strcat(sql, " WHERE "), where_sql)
    }
    return sql
}

fn sql_delete(table: string, where_sql: string) -> string {
    let mut sql = strcat("DELETE FROM ", table)
    if strlen(where_sql) > 0 {
        sql = strcat(strcat(sql, " WHERE "), where_sql)
    }
    return sql
}
