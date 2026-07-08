import "strings.ny"

extern fn read_file(path: string) -> string
extern fn write_file(path: string, content: string) -> i32
extern fn file_exists(path: string) -> i32

fn basename_str(path: string) -> string {
    let n = strlen(path)
    let mut last = -1
    let mut i = 0
    while i < n {
        if char_at(path, i) == 47 {
            last = i
        }
        i = i + 1
    }
    if last < 0 {
        return path
    }
    return substring(path, last + 1, n - last - 1)
}

struct Path {
    value: string
}

fn Path_new(value: string) -> Path {
    return Path { value: value }
}

fn path(value: string) -> Path {
    return Path_new(value)
}

fn Path_join(path: Path, segment: string) -> Path {
    let base = path.value
    let sep = "/"
    let with_sep = strcat(base, sep)
    let joined = strcat(with_sep, segment)
    return Path { value: joined }
}

impl Path {
    fn join(self, segment: string) -> Path {
        return Path_join(self, segment)
    }

    fn extension(self) -> string {
        let dot = strstr_pos(self.value, ".")
        if dot < 0 {
            return ""
        }
        let len = strlen(self.value)
        return substring(self.value, dot + 1, len - dot - 1)
    }

    fn file_name(self) -> string {
        return basename_str(self.value)
    }

    fn basename(self) -> string {
        return basename_str(self.value)
    }

    fn parent(self) -> Path {
        let slash = strstr_pos(self.value, "/")
        if slash < 0 {
            return Path { value: "." }
        }
        if slash == 0 {
            return Path { value: "/" }
        }
        return Path { value: substring(self.value, 0, slash) }
    }

    fn as_string(self) -> string {
        return self.value
    }

    fn read(self) -> string {
        return read_file(self.value)
    }

    fn write(self, content: string) -> i32 {
        return write_file(self.value, content)
    }

    fn exists(self) -> i32 {
        return file_exists(self.value)
    }
}

// PathBuf is an alias-style name for the same MVP type.
struct PathBuf {
    inner: Path
}

fn PathBuf_new(value: string) -> PathBuf {
    return PathBuf { inner: Path_new(value) }
}

impl PathBuf {
    fn join(self, segment: string) -> PathBuf {
        return PathBuf { inner: Path_join(self.inner, segment) }
    }

    fn as_string(self) -> string {
        return self.inner.as_string()
    }
}
