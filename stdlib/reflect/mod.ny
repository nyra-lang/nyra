// Reflection MVP — compile-time struct serde (`{Struct}_json_encode` / `_json_decode`).
// The compiler synthesizes JSON helpers for eligible structs (scalars, strings,
// nested serde structs, `Vec<i32>`, `Vec<string>`, and `Vec<Struct>`).

enum TypeKind {
    I32,
    Bool,
    String,
    Void,
    Unknown,
}

fn typeof_i32(_x: i32) -> TypeKind {
    return TypeKind.I32
}

fn typeof_bool(_x: bool) -> TypeKind {
    return TypeKind.Bool
}

fn typeof_string(_x: string) -> TypeKind {
    return TypeKind.String
}

fn type_name_i32() -> string {
    return "i32"
}

fn type_name_bool() -> string {
    return "bool"
}

fn type_name_string() -> string {
    return "string"
}

fn type_name_vec_i32() -> string {
    return "Vec_i32"
}

fn type_name_hashmap_str_i32() -> string {
    return "HashMap_str_i32"
}
