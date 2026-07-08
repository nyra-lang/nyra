// nyra test tests/nyra/registry_jsonl_test.ny
import "stdlib/testing.ny"
import "stdlib/json/jsonl.ny"

pub struct RegistryEntry {
    name: string
    version: string
    git_url: string
    git_rev: string
}

fn Registry_entries_from_jsonl(text: string) -> Vec<RegistryEntry> {
    let mut out = Vec_RegistryEntry_new()
    let lines = Json_non_empty_lines(text)
    let mut i = 0
    while i < lines.len() {
        let entry = RegistryEntry_json_decode(lines.get(i))
        out = Vec_RegistryEntry_push(out, entry)
        i = i + 1
    }
    return out
}

fn Registry_entries_from_array(text: string) -> Vec<RegistryEntry> {
    let mut out = Vec_RegistryEntry_new()
    let elems = Json_array_elements(text)
    let mut i = 0
    while i < elems.len() {
        let entry = RegistryEntry_json_decode(elems.get(i))
        out = Vec_RegistryEntry_push(out, entry)
        i = i + 1
    }
    return out
}

fn Registry_pick_highest(entries: Vec<RegistryEntry>) -> RegistryEntry {
    let n = Vec_RegistryEntry_len(entries)
    if n == 0 {
        return RegistryEntry {
            name: "",
            version: "",
            git_url: "",
            git_rev: "",
        }
    }
    let mut best = Vec_RegistryEntry_get(entries, 0)
    let mut i = 1
    while i < n {
        let e = Vec_RegistryEntry_get(entries, i)
        if strcmp(e.version, best.version) > 0 {
            best = e
        }
        i = i + 1
    }
    return best
}

test fn test_registry_jsonl_decode() {
    let text = read_file("tests/fixtures/registry/ny-demo.jsonl")
    let entries = Registry_entries_from_jsonl(text)
    assert_eq(Vec_RegistryEntry_len(entries), 3)
    let best = Registry_pick_highest(entries)
    assert_str_eq(best.version, "2.0.0")
    Vec_RegistryEntry_free(entries)
}

test fn test_registry_json_array_decode() {
    let text = read_file("tests/fixtures/registry/ny-array.json")
    assert_eq(Json_is_array_body(text), 1)
    let entries = Registry_entries_from_array(text)
    assert_eq(Vec_RegistryEntry_len(entries), 2)
    Vec_RegistryEntry_free(entries)
}

extern fn read_file(path: string) -> string
