// SQLite — requires `link sqlite3` in nyra.mod when using this module.
import "query.ny"

struct SqliteDb {
    handle: ptr
}

struct SqliteRowset {
    handle: ptr
}

struct SqliteStmt {
    handle: ptr
    db: ptr
}

extern fn sqlite_open(path: string) -> ptr
extern fn sqlite_exec(handle: ptr, sql: string) -> i32
extern fn sqlite_close(handle: ptr) -> void
extern fn sqlite_last_error(handle: ptr) -> string
extern fn sqlite_prepare(handle: ptr, sql: string) -> ptr
extern fn sqlite_step(stmt: ptr) -> i32
extern fn sqlite_column_count(stmt: ptr) -> i32
extern fn sqlite_column_text(stmt: ptr, col: i32) -> string
extern fn sqlite_finalize(stmt: ptr) -> void
extern fn sqlite_query_rows(handle: ptr, sql: string) -> ptr
extern fn sqlite_rowset_rows(rowset: ptr) -> i32
extern fn sqlite_rowset_cols(rowset: ptr) -> i32
extern fn sqlite_rowset_at(rowset: ptr, row: i32, col: i32) -> string
extern fn sqlite_rowset_free(rowset: ptr) -> void

fn Sqlite_open(path: string) -> SqliteDb {
    return SqliteDb { handle: sqlite_open(path) }
}

impl SqliteDb {
    fn exec(self, sql: string) -> i32 {
        return sqlite_exec(self.handle, sql)
    }

    fn close(self) -> void {
        sqlite_close(self.handle)
    }

    fn last_error(self) -> string {
        return sqlite_last_error(self.handle)
    }

    fn prepare(self, sql: string) -> SqliteStmt {
        return SqliteStmt { handle: sqlite_prepare(self.handle, sql), db: self.handle }
    }

    fn query_rows(self, sql: string) -> SqliteRowset {
        return SqliteRowset { handle: sqlite_query_rows(self.handle, sql) }
    }

    fn query(self, sql: string) -> string {
        let rs = self.query_rows(sql)
        if rs.rows() == 0 {
            return ""
        }
        return rs.at(0, 0)
    }

    fn find(self, q: SqlQuery) -> SqliteRowset {
        return self.query_rows(q.to_sql())
    }
}

impl SqliteStmt {
    fn step(self) -> i32 {
        return sqlite_step(self.handle)
    }

    fn cols(self) -> i32 {
        return sqlite_column_count(self.handle)
    }

    fn col(self, index: i32) -> string {
        return sqlite_column_text(self.handle, index)
    }

    fn finalize(self) -> void {
        sqlite_finalize(self.handle)
    }

    fn last_error(self) -> string {
        return sqlite_last_error(self.db)
    }
}

impl SqliteRowset {
    fn rows(self) -> i32 {
        return sqlite_rowset_rows(self.handle)
    }

    fn cols(self) -> i32 {
        return sqlite_rowset_cols(self.handle)
    }

    fn at(self, row: i32, col: i32) -> string {
        return sqlite_rowset_at(self.handle, row, col)
    }

    fn free(self) -> void {
        sqlite_rowset_free(self.handle)
    }
}
