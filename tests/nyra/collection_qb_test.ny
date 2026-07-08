import "stdlib/testing.ny"
import "stdlib/vec.ny"
import "stdlib/vec_str.ny"
import "stdlib/db/query.ny"
import "stdlib/strings/ops.ny"

fn gt2(x: i32) -> i32 {
    if x > 2 {
        return 1
    }
    return 0
}

fn is_even(x: i32) -> i32 {
    if x % 2 == 0 {
        return 1
    }
    return 0
}

fn times2(x: i32) -> i32 {
    return x * 2
}

fn add(a: i32, b: i32) -> i32 {
    return a + b
}

fn is_nyra(s: string) -> i32 {
    if strcmp(s, "nyra") == 0 {
        return 1
    }
    return 0
}

fn longer_than_3(s: string) -> i32 {
    if strlen(s) > 3 {
        return 1
    }
    return 0
}

fn to_upper(s: string) -> string {
    return str_to_upper(s)
}

fn test_vec_find_filter_map() {
    let xs = vec().push(1).push(2).push(3).push(4)
    assert_eq(xs.contains(3), 1)
    assert_eq(xs.contains(9), 0)
    assert_eq(xs.includes(2), 1)
    assert_eq(xs.first(-1), 1)
    assert_eq(xs.last(-1), 4)
    assert_eq(xs.find_eq(3, -1), 3)
    assert_eq(xs.index_of(4), 3)
    assert_eq(xs.find(gt2, -1), 3)
    let evens = xs.filter(is_even)
    assert_eq(evens.len(), 2)
    assert_eq(evens.get(0), 2)
    assert_eq(evens.get(1), 4)
    let doubled = xs.map(times2)
    assert_eq(doubled.get(0), 2)
    assert_eq(doubled.get(3), 8)
    assert_eq(xs.reduce(0, add), 10)
}

fn test_strs_find_filter_map() {
    let names = strs().push("ada").push("nyra").push("bob")
    assert_eq(names.contains("nyra"), 1)
    assert_eq(names.includes("zzz"), 0)
    assert_eq(strcmp(names.first(""), "ada"), 0)
    assert_eq(strcmp(names.last(""), "bob"), 0)
    assert_eq(strcmp(names.find_eq("nyra", ""), "nyra"), 0)
    assert_eq(names.index_of("bob"), 2)
    assert_eq(strcmp(names.find(is_nyra, ""), "nyra"), 0)
    let longish = names.filter(longer_than_3)
    assert_eq(longish.len(), 1)
    assert_eq(strcmp(longish.get(0), "nyra"), 0)
    let upper = names.map(to_upper)
    assert_eq(strcmp(upper.get(1), "NYRA"), 0)
    assert_eq(strcmp(names.joined(","), "ada,nyra,bob"), 0)
}

fn test_qb_to_sql() {
    let sql = qb()
        .select("u.name, p.title")
        .from("users u")
        .include("posts p", "p.user_id = u.id")
        .where("u.active", "=", "1")
        .and("u.role", "=", "admin")
        .order("u.name")
        .limit(10)
        .to_sql()
    assert_eq(1, if strstr_pos(sql, "SELECT u.name, p.title FROM users u") >= 0 { 1 } else { 0 })
    assert_eq(1, if strstr_pos(sql, "INNER JOIN posts p ON p.user_id = u.id") >= 0 { 1 } else { 0 })
    assert_eq(1, if strstr_pos(sql, "WHERE u.active = '1' AND u.role = 'admin'") >= 0 { 1 } else { 0 })
    assert_eq(1, if strstr_pos(sql, "ORDER BY u.name") >= 0 { 1 } else { 0 })
    assert_eq(1, if strstr_pos(sql, "LIMIT 10") >= 0 { 1 } else { 0 })

    let lookup_sql = qb_from("users")
        .select("*")
        .lookup("profiles", "users.id", "user_id")
        .to_sql()
    assert_eq(1, if strstr_pos(lookup_sql, "INNER JOIN profiles ON profiles.user_id = users.id") >= 0 { 1 } else { 0 })

    let unwind_sql = qb_from("orders")
        .unwind("items", "items.order_id = orders.id")
        .to_sql()
    assert_eq(1, if strstr_pos(unwind_sql, "LEFT JOIN items ON items.order_id = orders.id") >= 0 { 1 } else { 0 })

    let dist = qb_from("t").distinct().select("a").to_sql()
    assert_eq(1, if strstr_pos(dist, "SELECT DISTINCT a FROM t") >= 0 { 1 } else { 0 })

    assert_eq(strcmp(sql_quote("a'b"), "'a''b'"), 0)
    assert_eq(strcmp(sql_insert("t", "a, b", "'x', 'y'"), "INSERT INTO t (a, b) VALUES ('x', 'y')"), 0)
    assert_eq(strcmp(sql_update("t", "a = 1", "id = 2"), "UPDATE t SET a = 1 WHERE id = 2"), 0)
    assert_eq(strcmp(sql_delete("t", "id = 1"), "DELETE FROM t WHERE id = 1"), 0)
}

fn main() {
    test_vec_find_filter_map()
    test_strs_find_filter_map()
    test_qb_to_sql()
    print("collection+qb sugar ok")
}
