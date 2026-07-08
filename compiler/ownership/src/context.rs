use std::collections::{HashMap, HashSet};

use ast::*;
use crate::kind::{ownership_of, OwnershipKind};
use crate::send_sync::{thread_safety_of_struct, ThreadSafety};
use types::{StructInfo, Type};

#[derive(Debug, Clone)]
pub struct StructMeta {
    pub info: StructInfo,
    pub explicit_send: bool,
    pub explicit_sync: bool,
}

#[derive(Debug, Clone)]
pub struct OwnershipCtx {
    pub structs: HashMap<String, StructMeta>,
    pub drop_structs: HashSet<String>,
    /// `extern fn` symbols declared to return heap-owned `string`.
    pub owned_extern_returns: HashSet<String>,
    /// Enum name → payload type when payload needs heap cleanup (e.g. `string`).
    pub enum_heap_payload: HashMap<String, Type>,
}

impl OwnershipCtx {
    pub fn from_program(program: &Program) -> Self {
        let mut structs = HashMap::new();
        for s in &program.structs {
            let mut fields = HashMap::new();
            let mut field_order = Vec::new();
            for f in &s.fields {
                fields.insert(f.name.clone(), Type::from(f.ty.clone()));
                field_order.push(f.name.clone());
            }
            let mut field_anns = HashMap::new();
            for f in &s.fields {
                field_anns.insert(f.name.clone(), f.ty.clone());
            }
            structs.insert(
                s.name.clone(),
                StructMeta {
                    info: StructInfo {
                        fields,
                        field_anns,
                        field_order,
                        repr_c: s.attrs.repr_c,
                        align: s.attrs.align,
                        packed: s.attrs.packed,
                    },
                    explicit_send: s.attrs.send,
                    explicit_sync: s.attrs.sync,
                },
            );
        }
        let mut drop_structs = HashSet::new();
        for ti in &program.trait_impls {
            if ti.trait_name == "Drop" {
                drop_structs.insert(ti.type_name.clone());
            }
        }
        let mut owned_extern_returns = HashSet::new();
        for ext in &program.externs {
            if ext
                .return_type
                .as_ref()
                .is_some_and(|t| matches!(Type::from(t.clone()), Type::String))
                && !crate::kind::callee_returns_borrowed(&ext.name)
            {
                owned_extern_returns.insert(ext.name.clone());
            }
        }
        Self {
            structs,
            drop_structs,
            owned_extern_returns,
            enum_heap_payload: Self::collect_enum_heap_payload(program),
        }
    }

    fn collect_enum_heap_payload(program: &Program) -> HashMap<String, Type> {
        let mut map = HashMap::new();
        for e in &program.enums {
            if !e.type_params.is_empty() {
                continue;
            }
            if let Some(payload_ann) = e.variants.iter().find_map(|v| v.fields.first()) {
                let payload = Type::from(payload_ann.clone());
                if matches!(payload, Type::String) {
                    map.insert(e.name.clone(), payload);
                }
            }
        }
        map
    }

    pub fn enum_needs_payload_drop(&self, name: &str) -> bool {
        self.enum_heap_payload.contains_key(name)
    }

    pub fn struct_field_type(&self, struct_name: &str, field: &str) -> Option<Type> {
        self.structs
            .get(struct_name)
            .and_then(|m| m.info.fields.get(field).cloned())
    }

    /// Whether a callee returns a heap-owned value the caller must drop.
    pub fn callee_returns_owned(&self, callee: &str) -> bool {
        (crate::kind::OWNED_EXTERN_RETURNS.contains(&callee)
            || self.owned_extern_returns.contains(callee))
            && !crate::kind::callee_returns_borrowed(callee)
    }

    /// Struct has heap-owned fields and no custom `impl Drop` — needs composite field cleanup.
    pub fn struct_needs_composite_drop(&self, name: &str) -> bool {
        if self.drop_structs.contains(name) {
            return false;
        }
        self.struct_has_heap_owned_fields(name)
    }

    fn struct_has_heap_owned_fields(&self, name: &str) -> bool {
        let Some(meta) = self.structs.get(name) else {
            return false;
        };
        for field_ty in meta.info.fields.values() {
            if self.type_needs_heap_drop(field_ty) {
                return true;
            }
        }
        false
    }

    fn type_needs_heap_drop(&self, ty: &Type) -> bool {
        match ty {
            Type::String => true,
            Type::Struct(name) => self.struct_needs_composite_drop(name),
            Type::Array { elem, .. } => self.type_needs_heap_drop(elem),
            Type::Tuple { elems } => elems.iter().any(|e| self.type_needs_heap_drop(e)),
            _ => false,
        }
    }

    pub fn struct_info(&self, name: &str) -> Option<&StructInfo> {
        self.structs.get(name).map(|m| &m.info)
    }

    pub fn kind_of(&self, ty: &Type) -> OwnershipKind {
        ownership_of(ty, &self.struct_fields_map(), &self.drop_structs)
    }

    fn struct_fields_map(&self) -> HashMap<String, StructInfo> {
        self.structs
            .iter()
            .map(|(k, v)| (k.clone(), v.info.clone()))
            .collect()
    }

    pub fn thread_safety_of_struct(&self, name: &str) -> ThreadSafety {
        thread_safety_of_struct(name, self)
    }

    pub fn derived_thread_safety_of_struct(&self, name: &str) -> ThreadSafety {
        crate::send_sync::derived_thread_safety_of_struct(name, self)
    }

    pub fn expr_kind(&self, expr: &Expression) -> OwnershipKind {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Int(_) | Literal::IntKind(_, _) | Literal::Float(_, _) | Literal::Char(_) | Literal::Bool(_) => OwnershipKind::Copy,
                Literal::String(_) => OwnershipKind::Move,
            },
            Expression::Variable { .. } => OwnershipKind::Move,
            Expression::Call(c) if self.callee_returns_owned(&c.callee) => OwnershipKind::Move,
            Expression::TemplateLiteral(_) => OwnershipKind::Move,
            Expression::StructLiteral(_) => OwnershipKind::Move,
            Expression::EnumVariant(_) => OwnershipKind::Copy,
            _ => OwnershipKind::Copy,
        }
    }

    pub fn infer_expr_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(lit) => types::literal_type(lit),
            Expression::Variable { .. } => Type::Unknown,
            Expression::Call(c) if c.callee == "channel_new" || c.callee == "async_promise_new" || c.callee == "async_run" => {
                Type::Handle
            }
            Expression::Call(c) if c.callee == "rt_tcp_hub_new" => Type::Ptr,
            Expression::Call(c) if c.callee == "channel_recv" => Type::Integer(ast::IntKind::I32),
            Expression::Call(c) => {
                if c.callee.ends_with("_new") {
                    let prefix = c.callee.strip_suffix("_new").unwrap_or("");
                    if self.structs.contains_key(prefix) {
                        return Type::Struct(prefix.to_string());
                    }
                }
                if self.callee_returns_owned(&c.callee) {
                    Type::String
                } else {
                    Type::Unknown
                }
            }
            Expression::MethodCall(mc) if mc.method == "split" => Type::VecStr,
            Expression::TemplateLiteral(_) => Type::String,
            Expression::StructLiteral(sl) => Type::Struct(sl.name.clone()),
            Expression::EnumVariant(ev) => ev
                .enum_name
                .clone()
                .map(Type::Enum)
                .unwrap_or(Type::Unknown),
            Expression::Spawn { .. } => Type::JoinHandle,
            Expression::ParallelSearch(ps) => match ps.config.op {
                ParallelOp::Find => Type::Integer(ast::IntKind::I32),
                ParallelOp::Any | ParallelOp::All | ParallelOp::Iterate => Type::Bool,
            },
            _ => Type::Unknown,
        }
    }
}
