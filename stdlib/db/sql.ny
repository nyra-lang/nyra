import "sqlite.ny"
import "postgres.ny"
import "mysql.ny"

struct SqlDb {
    driver: string
    handle: ptr
}

fn Sql_open(driver: string, dsn: string) -> SqlDb {
    if strcmp(driver, "sqlite") == 0 || strcmp(driver, "sqlite3") == 0 {
        let db = Sqlite_open(dsn)
        return SqlDb { driver: driver, handle: db.handle }
    }
    if strcmp(driver, "postgres") == 0 || strcmp(driver, "postgresql") == 0 {
        return SqlDb { driver: driver, handle: postgres_open(dsn) }
    }
    if strcmp(driver, "mysql") == 0 {
        return SqlDb { driver: driver, handle: mysql_open(dsn) }
    }
    print(strcat("sql: unknown driver ", driver))
    return SqlDb { driver: driver, handle: _sqlite_null_handle() }
}

impl SqlDb {
    fn exec(self, sql: string) -> i32 {
        if strcmp(self.driver, "sqlite") == 0 || strcmp(self.driver, "sqlite3") == 0 {
            return sqlite_exec(self.handle, sql)
        }
        if strcmp(self.driver, "postgres") == 0 || strcmp(self.driver, "postgresql") == 0 {
            return postgres_exec(self.handle, sql)
        }
        if strcmp(self.driver, "mysql") == 0 {
            return mysql_exec(self.handle, sql)
        }
        return -1
    }

    fn query_rows(self, sql: string) -> SqliteRowset {
        if strcmp(self.driver, "sqlite") == 0 || strcmp(self.driver, "sqlite3") == 0 {
            return SqliteRowset { handle: sqlite_query_rows(self.handle, sql) }
        }
        return SqliteRowset { handle: _sqlite_null_handle() }
    }

    fn close(self) -> void {
        if strcmp(self.driver, "sqlite") == 0 || strcmp(self.driver, "sqlite3") == 0 {
            sqlite_close(self.handle)
        } else {
            if strcmp(self.driver, "postgres") == 0 || strcmp(self.driver, "postgresql") == 0 {
                postgres_close(self.handle)
            } else {
                if strcmp(self.driver, "mysql") == 0 {
                    nyra_mysql_close(self.handle)
                }
            }
        }
    }
}

extern fn sqlite_exec(handle: ptr, sql: string) -> i32
extern fn sqlite_close(handle: ptr) -> void
extern fn sqlite_query_rows(handle: ptr, sql: string) -> ptr
extern fn _sqlite_null_handle() -> ptr
