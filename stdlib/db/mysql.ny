// MySQL driver — requires `link mysqlclient` in nyra.mod for native backend.
// DSN: host;port;dbname;user;password

extern fn _mysql_stub_open(dsn: string) -> ptr
extern fn mysql_exec(handle: ptr, sql: string) -> i32
extern fn nyra_mysql_close(handle: ptr) -> void

fn mysql_open(dsn: string) -> ptr {
    return _mysql_stub_open(dsn)
}
