// FS ergonomics — short aliases (avoid colliding with libc read/write).
import "file.ny"

fn slurp(path: string) -> string {
    return read_file(path)
}

fn spit(path: string, content: string) -> i32 {
    return write_file(path, content)
}

fn spit_append(path: string, content: string) -> i32 {
    return append_file(path, content)
}

fn make_dir(path: string) -> i32 {
    return create_dir(path)
}

fn make_dir_all(path: string) -> i32 {
    return create_dir_all(path)
}

fn rm(path: string) -> i32 {
    return remove_file(path)
}

fn rmdir(path: string) -> i32 {
    return remove_dir(path)
}

fn rm_rf(path: string) -> i32 {
    return remove_dir_all(path)
}

fn ls(path: string) -> StrVec {
    return list_dir_entries(path)
}
