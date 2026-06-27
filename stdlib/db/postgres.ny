// PostgreSQL driver — requires `link pq` (libpq) in nyra.mod for native backend.
// DSN: host;port;dbname;user;password

extern fn _postgres_stub_open(dsn: string) -> ptr
extern fn postgres_exec(handle: ptr, sql: string) -> i32
extern fn postgres_close(handle: ptr) -> void

fn postgres_open(dsn: string) -> ptr {
    return _postgres_stub_open(dsn)
}
