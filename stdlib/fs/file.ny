extern fn read_file(path: string) -> string
extern fn read_file_limit(path: string, max_bytes: i32) -> string
extern fn write_file(path: string, content: string) -> i32
extern fn append_file(path: string, content: string) -> i32
extern fn file_exists(path: string) -> i32
extern fn remove_file(path: string) -> i32
extern fn create_dir(path: string) -> i32
extern fn create_dir_all(path: string) -> i32
extern fn remove_dir(path: string) -> i32
extern fn remove_dir_all(path: string) -> i32
extern fn file_size(path: string) -> i64
extern fn copy_file(src: string, dst: string) -> i64
extern fn copy_dir(src: string, dst: string) -> i32
extern fn copy_dir_contents(src: string, dst: string) -> i32
extern fn list_dir(path: string) -> string
extern fn path_is_dir(path: string) -> i32
extern fn os_arg_count() -> i32
extern fn os_arg_at(index: i32) -> string

fn exists(path: string) -> i32 {
    return file_exists(path)
}

fn is_dir(path: string) -> i32 {
    return path_is_dir(path)
}

fn list_dir_entries(path: string) -> StrVec {
    return StrVec_from_lines(list_dir(path))
}
