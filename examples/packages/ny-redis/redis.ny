extern fn redis_connect(host: string, port: i32) -> ptr
extern fn redis_ping(conn: ptr) -> i32
extern fn redis_get(conn: ptr, key: string) -> string
extern fn redis_set(conn: ptr, key: string, value: string, ttl_sec: i32) -> i32
extern fn redis_del(conn: ptr, key: string) -> i32
extern fn redis_lpush(conn: ptr, key: string, value: string) -> i32
extern fn redis_brpop(conn: ptr, key: string, timeout_sec: i32) -> string
extern fn redis_close(conn: ptr) -> void
extern fn redis_free_string(value: string) -> void

const REDIS_QUEUE_KEY = "nyra:queue"

fn Redis_connect(host: string, port: i32) -> ptr {
    return redis_connect(host, port)
}

fn Redis_ping(conn: ptr) -> i32 {
    return redis_ping(conn)
}

fn Redis_get(conn: ptr, key: string) -> string {
    return redis_get(conn, key)
}

fn Redis_set(conn: ptr, key: string, value: string, ttl_sec: i32) -> i32 {
    return redis_set(conn, key, value, ttl_sec)
}

fn Redis_del(conn: ptr, key: string) -> i32 {
    return redis_del(conn, key)
}

fn Redis_lpush(conn: ptr, key: string, value: string) -> i32 {
    return redis_lpush(conn, key, value)
}

fn Redis_brpop(conn: ptr, key: string, timeout_sec: i32) -> string {
    return redis_brpop(conn, key, timeout_sec)
}

fn Redis_close(conn: ptr) -> void {
    redis_close(conn)
}

fn Redis_free_string(value: string) -> void {
    redis_free_string(value)
}
