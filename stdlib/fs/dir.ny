import "../strings.ny"

fn join_path(base: string, part: string) -> string {
    if strlen(base) == 0 {
        return part
    }
    let last = char_at(base, strlen(base) - 1)
    if last == 47 {
        return strcat(base, part)
    }
    return strcat(strcat(base, "/"), part)
}
