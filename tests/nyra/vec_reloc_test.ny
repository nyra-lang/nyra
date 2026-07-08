// Vec<T> for Move structs (string + scalar columns) — v1.26
import "stdlib/testing.ny"

struct LabelRow {
    label: string
    count: i32
}

struct InnerTag {
    tag: string
    weight: i32
}

struct NestedRow {
    inner: InnerTag
    score: i32
}

struct FlagRow {
    label: string
    active: bool
}

test fn test_vec_reloc_label_row() {
    let mut v: Vec<LabelRow> = Vec_LabelRow_new()
    v = Vec_LabelRow_push(v, LabelRow { label: "alpha", count: 1 })
    v = Vec_LabelRow_push(v, LabelRow { label: "beta", count: 2 })
    assert_eq(Vec_LabelRow_len(v), 2)
    let row = Vec_LabelRow_get(v, 1)
    assert_str_eq(row.label, "beta")
    assert_eq(row.count, 2)
    Vec_LabelRow_free(v)
}

test fn test_vec_reloc_nested_struct() {
    let mut v: Vec<NestedRow> = Vec_NestedRow_new()
    v = Vec_NestedRow_push(v, NestedRow {
        inner: InnerTag { tag: "go", weight: 3 },
        score: 10,
    })
    assert_eq(Vec_NestedRow_len(v), 1)
    let row = Vec_NestedRow_get(v, 0)
    assert_str_eq(row.inner.tag, "go")
    assert_eq(row.inner.weight, 3)
    assert_eq(row.score, 10)
    Vec_NestedRow_free(v)
}

test fn test_vec_reloc_bool_field() {
    let mut v: Vec<FlagRow> = Vec_FlagRow_new()
    v = Vec_FlagRow_push(v, FlagRow { label: "on", active: true })
    v = Vec_FlagRow_push(v, FlagRow { label: "off", active: false })
    assert_eq(Vec_FlagRow_len(v), 2)
    let row = Vec_FlagRow_get(v, 0)
    assert_str_eq(row.label, "on")
    if !row.active {
        assert_eq(1, 0)
    }
    Vec_FlagRow_free(v)
}
