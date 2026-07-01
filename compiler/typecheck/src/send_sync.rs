//! Send / Sync checking for trait object auto-trait bounds.

use types::{StructInfo, Type};

use crate::TypeChecker;

impl TypeChecker {
    pub(super) fn type_is_send(&self, ty: &Type) -> bool {
        self.thread_safety_of(ty).0
    }

    pub(super) fn type_is_sync(&self, ty: &Type) -> bool {
        self.thread_safety_of(ty).1
    }

    fn thread_safety_of(&self, ty: &Type) -> (bool, bool) {
        match ty {
            Type::Integer(_)
            | Type::F32
            | Type::F64
            | Type::Char
            | Type::Bool
            | Type::Void
            | Type::Enum(_)
            | Type::Handle
            | Type::VecStr
            | Type::Ptr
            | Type::String
            | Type::Bytes
            | Type::Simd { .. }
            | Type::Union(_) => (true, true),
            Type::RawPtr { .. } => (false, false),
            Type::Struct(name) => self.struct_thread_safety(name),
            Type::Array { elem, .. } => self.thread_safety_of(elem),
            Type::Tuple { elems } => {
                let mut send = true;
                let mut sync = true;
                for e in elems {
                    let (s, y) = self.thread_safety_of(e);
                    send &= s;
                    sync &= y;
                }
                (send, sync)
            }
            Type::Ref {
                inner,
                mutable: false,
                ..
            } => {
                let (_, sync) = self.thread_safety_of(inner);
                (sync, true)
            }
            Type::Ref {
                inner,
                mutable: true,
                ..
            } => {
                let (send, _) = self.thread_safety_of(inner);
                (send, send)
            }
            Type::ForAll { inner, .. } => self.thread_safety_of(inner),
            Type::FnPtr { .. } => (true, true),
            Type::Generic(_) | Type::Unknown => (false, false),
        }
    }

    fn struct_thread_safety(&self, name: &str) -> (bool, bool) {
        if name.starts_with("Arc_") || name == "Arc" {
            return (true, true);
        }
        let Some(StructInfo { fields, .. }) = self.structs.get(name) else {
            return (false, false);
        };
        let mut send = true;
        let mut sync = true;
        for ty in fields.values() {
            let (s, y) = self.thread_safety_of(ty);
            send &= s;
            sync &= y;
        }
        (send, sync)
    }
}
