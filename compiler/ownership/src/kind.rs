use std::collections::HashMap;
use types::{StructInfo, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipKind {
    Copy,
    Move,
}

impl OwnershipKind {
    pub fn is_copy(self) -> bool {
        matches!(self, OwnershipKind::Copy)
    }

    pub fn is_move(self) -> bool {
        matches!(self, OwnershipKind::Move)
    }
}

pub fn ownership_of(
    ty: &Type,
    structs: &HashMap<String, StructInfo>,
    drop_structs: &std::collections::HashSet<String>,
) -> OwnershipKind {
    match ty {
        Type::Integer(_)
        | Type::F32
        | Type::F64
        | Type::Char
        | Type::Bool
        | Type::Enum(_)
        | Type::Void
        | Type::Unknown
        | Type::Handle
        | Type::VecStr
        | Type::Ptr
        | Type::RawPtr { .. } => OwnershipKind::Copy,
        Type::String => OwnershipKind::Move,
        Type::Ref { .. } | Type::Generic(_) | Type::ForAll { .. } | Type::FnPtr { .. } => {
            OwnershipKind::Copy
        }
        Type::Array { elem, .. } => ownership_of(elem, structs, drop_structs),
        Type::Tuple { elems } => {
            for e in elems {
                if ownership_of(e, structs, drop_structs).is_move() {
                    return OwnershipKind::Move;
                }
            }
            OwnershipKind::Copy
        }
        Type::Struct(name) => {
            if drop_structs.contains(name) {
                return OwnershipKind::Move;
            }
            if let Some(info) = structs.get(name) {
                for field_ty in info.fields.values() {
                    if ownership_of(field_ty, structs, drop_structs).is_move() {
                        return OwnershipKind::Move;
                    }
                }
                OwnershipKind::Copy
            } else {
                OwnershipKind::Move
            }
        }
    }
}

pub const OWNED_EXTERN_RETURNS: &[&str] = &[
    "read_file",
    "strcat",
    "i32_to_string",
    "i64_to_string",
    "array_i32_debug_string",
    "array_f64_debug_string",
    "array_f32_debug_string",
    "array_bool_debug_string",
    "array_str_debug_string",
    "substring",
    "str_to_upper",
    "str_to_lower",
    "str_trim",
    "str_replace",
    "str_replacen",
    "str_split",
    "sys_recv",
    "tcp_read",
    "http_get",
    "bridge_exec",
    "bridge_exec_arg",
    "json_get_string",
    "sha256_hex",
    "sha512_hex",
    "hmac_sha256_hex",
    "aes_cbc_encrypt_hex",
    "aes_cbc_decrypt_hex",
    "ws_recv_text",
    "clone",
    "pty_read",
    "pty_drain",
    "pty_drain_raw",
    "pty_read_wait",
    "pty_read_wait_raw",
    "stdin_read_line",
    "random_hex",
];

pub fn callee_returns_owned(callee: &str) -> bool {
    OWNED_EXTERN_RETURNS.contains(&callee)
}

/// Extern fns that return `string` views into caller-owned or global storage (must not `free`).
pub const BORROWED_EXTERN_RETURNS: &[&str] = &["os_getenv"];

pub fn callee_returns_borrowed(callee: &str) -> bool {
    BORROWED_EXTERN_RETURNS.contains(&callee)
}
