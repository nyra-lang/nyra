// Metaprogramming helpers — compile-time reflection surface.
// Full codegen: comptime modules, macros, generics monomorph, struct JSON synthesis.

import "../reflect/mod.ny"

// Re-export type-name helpers for generic/comptime routing tables.
fn meta_type_name_i32() -> string {
    return type_name_i32()
}

fn meta_type_name_string() -> string {
    return type_name_string()
}
