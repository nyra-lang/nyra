import "../map.ny"
import "../strings.ny"
import "../fs.ny"
import "../time/instant.ny"

struct TtlCache {
    entries: HashMap_str_str
    boot: i64
    ttl_ms: i32
    dir: string
    disk: i32
}

fn TtlCache_new(ttl_ms: i32, dir: string, use_disk: i32) -> TtlCache {
    return TtlCache {
        entries: HashMap_str_str_new(),
        boot: instant_now(),
        ttl_ms: ttl_ms,
        dir: dir,
        disk: use_disk
    }
}

fn TtlCache_disk_path(cache: TtlCache, key: string) -> string {
    let digest = sha256_hex(key)
    let prefix = substring(digest, 0, 16)
    return strcat(cache.dir, strcat("/", prefix))
}

fn TtlCache_disk_read(cache: TtlCache, key: string) -> string {
    if cache.disk == 0 {
        return ""
    }
    let path = TtlCache_disk_path(cache, key)
    if file_exists(path) == 0 {
        return ""
    }
    return read_file(path)
}

fn TtlCache_disk_write(cache: TtlCache, key: string, value: string) -> void {
    if cache.disk == 0 {
        return
    }
    let path = TtlCache_disk_path(cache, key)
    write_file(path, value)
}

fn TtlCache_deadline(cache: TtlCache) -> i32 {
    return instant_elapsed_ms(cache.boot) + cache.ttl_ms
}

fn TtlCache_pack(deadline: i32, value: string) -> string {
    return strcat(strcat(i32_to_string(deadline), "|"), value)
}

fn TtlCache_value_from_packed(packed: string) -> string {
    let sep = strstr_pos(packed, "|")
    if sep < 0 {
        return packed
    }
    return substring(packed, sep + 1, strlen(packed) - sep - 1)
}

fn TtlCache_deadline_from_packed(packed: string) -> i32 {
    let sep = strstr_pos(packed, "|")
    if sep < 0 {
        return 0
    }
    let head = substring(packed, 0, sep)
    return str_to_i32(head)
}

fn TtlCache_is_alive(cache: TtlCache, packed: string) -> i32 {
    let deadline = TtlCache_deadline_from_packed(packed)
    let age = instant_elapsed_ms(cache.boot)
    if age > deadline {
        return 0
    }
    return 1
}

fn TtlCache_get(cache: TtlCache, key: string) -> string {
    if cache.entries.contains(key) == 1 {
        let packed = cache.entries.get(key)
        if TtlCache_is_alive(cache, packed) == 1 {
            return TtlCache_value_from_packed(packed)
        }
    }
    if cache.disk == 1 {
        return TtlCache_disk_read(cache, key)
    }
    return ""
}

fn TtlCache_put(cache: TtlCache, key: string, value: string) -> TtlCache {
    map_str_str_retain(cache.entries.handle)
    let packed = TtlCache_pack(TtlCache_deadline(cache), value)
    TtlCache_disk_write(cache, key, value)
    map_str_str_insert(cache.entries.handle, key, packed)
    map_str_str_retain(cache.entries.handle)
    return cache
}

fn TtlCache_has(cache: TtlCache, key: string) -> i32 {
    if cache.entries.contains(key) == 1 {
        let packed = cache.entries.get(key)
        if TtlCache_is_alive(cache, packed) == 1 {
            return 1
        }
    }
    if cache.disk == 1 {
        let path = TtlCache_disk_path(cache, key)
        if file_exists(path) == 1 {
            return 1
        }
    }
    return 0
}
