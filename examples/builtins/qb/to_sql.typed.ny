fn main() -> void {
    let sql = qb()
        .select("id, name")
        .from("users")
        .where("active", "=", "1")
        .order("name")
        .limit(10)
        .to_sql()
    print(sql)
}
