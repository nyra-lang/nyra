// Barrel for short collection APIs (implemented on the primary types).
// Prefer:
//   dict() / dict_i32() / obj()  — stdlib/json/mod.ny
//   strs() / lines() / .joined() / .filter / .map / .find — stdlib/vec_str.ny
//   vec() / vec_range() / .filter / .map / .find          — stdlib/vec.ny
//   qb().select().from().where().to_sql() / .find(db)    — stdlib/db/query.ny
import "../vec.ny"
import "../vec_str.ny"
import "../json/mod.ny"
