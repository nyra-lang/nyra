// nyra test tests/nyra/struct_serde_vec_struct_test.ny
import "stdlib/testing.ny"
import "stdlib/json/mod.ny"

struct Item {
    label: string
    score: i32
}

struct Bag {
    name: string
    items: Vec<Item>
}

test fn test_vec_struct_json_roundtrip() {
    let mut items = Vec_Item_new()
    items = Vec_Item_push(items, Item { label: "a", score: 1 })
    items = Vec_Item_push(items, Item { label: "b", score: 2 })
    let bag = Bag { name: "test", items: items }
    let json = Bag_json_encode(bag)
    let bag2 = Bag_json_decode(json)
    assert_str_eq(bag2.name, "test")
    assert_eq(Vec_Item_len(bag2.items), 2)
    let second = Vec_Item_get(bag2.items, 1)
    assert_str_eq(second.label, "b")
    assert_eq(second.score, 2)
    Vec_Item_free(bag2.items)
}
