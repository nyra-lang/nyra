import "../os/env.ny"

fn env_get(name: string) -> string {
    return os_getenv(name)
}

extern fn rt_os_setenv(name: string, value: string) -> i32

fn env_set(name: string, value: string) -> i32 {
    return rt_os_setenv(name, value)
}

fn env_has(name: string) -> i32 {
    let v = os_getenv(name)
    if strlen(v) > 0 {
        return 1
    }
    return 0
}

fn env(name: string) -> string {
    return env_get(name)
}

fn env_or(name: string, fallback: string) -> string {
    let v = env_get(name)
    if strlen(v) > 0 {
        return v
    }
    return fallback
}

extern fn strlen(s: &string) -> i32
