extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string
extern fn strcmp(a: &string, b: &string) -> i32
extern fn char_at(s: &string, i: i32) -> i32
extern fn strstr_pos(hay: &string, needle: &string) -> i32
extern fn substring(s: &string, start: i32, len: i32) -> string
extern fn list_dir(path: string) -> string
extern fn path_is_dir(path: string) -> i32
extern fn file_exists(path: string) -> i32

struct FilePicker {
    cwd: string
    selected: string
    entries: StrVec
}

fn FilePicker_parent(path: string) -> string {
    let slash = strstr_pos(path, "/")
    if slash < 0 {
        return path
    }
    let mut last = slash
    let n = strlen(path)
    let mut i = slash + 1
    while i < n {
        if char_at(path, i) == 47 {
            last = i
        }
        i = i + 1
    }
    if last == 0 {
        return "/"
    }
    return substring(path, 0, last)
}

fn FilePicker_join(base: string, name: string) -> string {
    if strcmp(name, "..") == 0 {
        return FilePicker_parent(base)
    }
    if strcmp(name, ".") == 0 {
        return base
    }
    if char_at(name, 0) == 47 {
        return name
    }
    if strcmp(base, "/") == 0 {
        return strcat("/", name)
    }
    return strcat(strcat(base, "/"), name)
}

fn FilePicker_open(path: string) -> FilePicker {
    let entries = list_dir_entries(path)
    let mut selected = ""
    if entries.len() > 0 {
        selected = entries.get(0)
    }
    return FilePicker { cwd: path, selected: selected, entries: entries }
}

fn FilePicker_refresh(mut p: FilePicker) -> FilePicker {
    p.entries = list_dir_entries(p.cwd)
    if p.entries.len() == 0 {
        p.selected = ""
    }
    return p
}

fn FilePicker_up(mut p: FilePicker) -> FilePicker {
    p.cwd = FilePicker_parent(p.cwd)
    p = FilePicker_refresh(p)
    return p
}

fn FilePicker_pick(mut p: FilePicker, index: i32) -> FilePicker {
    if index < 0 || index >= p.entries.len() {
        return p
    }
    let name = p.entries.get(index)
    let full = FilePicker_join(p.cwd, name)
    if path_is_dir(full) == 1 {
        p.cwd = full
        p = FilePicker_refresh(p)
    } else {
        p.selected = full
    }
    return p
}

fn FilePicker_selected_path(p: FilePicker) -> string {
    if strlen(p.selected) == 0 {
        return ""
    }
    if strstr_pos(p.selected, "/") >= 0 {
        return p.selected
    }
    return FilePicker_join(p.cwd, p.selected)
}
