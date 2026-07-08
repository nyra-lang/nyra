//! TypeChecker construction, target flags, and the default prelude env.
use std::collections::HashMap;

use ast::*;

use super::{FunctionSignature, TypeChecker, TypeEnv};
use types::{self, unify_numeric, Type};

impl TypeChecker {
    pub fn new() -> Self {
        Self::with_target("")
    }

    pub fn set_target(&mut self, target: &str) {
        self.target = target.to_string();
    }

    pub(super) fn with_target(target: &str) -> Self {
        Self {
            env: Self::default_env(),
            structs: HashMap::new(),
            unions: HashMap::new(),
            enums: HashMap::new(),
            errors: vec![],
            inferred_bindings: vec![],
            target: target.to_string(),
            no_std: false,
            unsafe_depth: 0,
            loop_depth: 0,
            global_names: vec![],
            trait_methods: HashMap::new(),
            trait_impl_pairs: vec![],
            program_for_inference: None,
            signature_inference: false,
            anon_name_queue: Vec::new(),
            anon_shape_index: std::collections::HashMap::new(),
            anon_counter: 0,
            synthesized_struct_defs: Vec::new(),
            current_type_param_bounds: HashMap::new(),
        }
    }

    pub(super) fn target_is_wasm(&self) -> bool {
        self.target.contains("wasm")
    }

    pub(super) fn default_env() -> TypeEnv {
        let mut env = TypeEnv::default();
        env.functions.insert(
            "print".into(),
            FunctionSignature {
                params: vec![Type::Unknown],
                return_type: Type::Void,
            },
        );
        for name in ["write", "println"] {
            env.functions.insert(
                name.into(),
                FunctionSignature {
                    params: vec![Type::Unknown],
                    return_type: Type::Void,
                },
            );
        }
        env.functions.insert(
            "flush".into(),
            FunctionSignature {
                params: vec![],
                return_type: Type::Void,
            },
        );
        env.functions.insert(
            "input".into(),
            FunctionSignature {
                params: vec![],
                return_type: Type::String,
            },
        );
        for name in ["time_start", "time_end", "mem_start", "mem_end"] {
            env.functions.insert(
                name.into(),
                FunctionSignature {
                    params: vec![Type::String],
                    return_type: Type::Void,
                },
            );
        }
        TypeChecker::register_date_env(&mut env);
        env.functions.insert(
            "cpu_count".into(),
            FunctionSignature {
                params: vec![],
                return_type: Type::Integer(ast::IntKind::I32),
            },
        );
        for name in ["random", "random_f64"] {
            env.functions.insert(
                name.into(),
                FunctionSignature {
                    params: vec![],
                    return_type: if name == "random" {
                        Type::Integer(ast::IntKind::I32)
                    } else {
                        Type::F64
                    },
                },
            );
        }
        for name in [
            "spawn",
            "async_await",
            "async_run",
            "async_poll",
            "runtime_run",
            "io_register",
            "io_wait_once",
            "tls_available",
        ] {
            let (params, ret) = match name {
                "async_await" | "async_poll" => (vec![Type::Handle], Type::Integer(ast::IntKind::I32)),
                "async_run" => (vec![Type::Integer(ast::IntKind::I32)], Type::Handle),
                "async_promise_complete" => (vec![Type::Handle, Type::Integer(ast::IntKind::I32)], Type::Void),
                "io_register" => (vec![Type::Integer(ast::IntKind::I32), Type::Integer(ast::IntKind::I32)], Type::Integer(ast::IntKind::I32)),
                "io_wait_once" => (vec![Type::Integer(ast::IntKind::I32)], Type::Integer(ast::IntKind::I32)),
                _ => (vec![], Type::Integer(ast::IntKind::I32)),
            };
            if name == "async_promise_complete" {
                env.functions.insert(
                    name.into(),
                    FunctionSignature {
                        params,
                        return_type: Type::Void,
                    },
                );
                continue;
            }
            if name == "runtime_run" {
                env.functions.insert(
                    name.into(),
                    FunctionSignature {
                        params: vec![],
                        return_type: Type::Void,
                    },
                );
                continue;
            }
            env.functions.insert(
                name.into(),
                FunctionSignature {
                    params,
                    return_type: ret,
                },
            );
        }
        env.functions.insert(
            "async_promise_new".into(),
            FunctionSignature {
                params: vec![],
                return_type: Type::Handle,
            },
        );
        env.functions.insert(
            "async_promise_complete".into(),
            FunctionSignature {
                params: vec![Type::Handle, Type::Integer(ast::IntKind::I32)],
                return_type: Type::Void,
            },
        );
        for (name, params, ret) in [
            (
                "async_promise_complete_bool",
                vec![Type::Handle, Type::Integer(ast::IntKind::I32)],
                Type::Void,
            ),
            (
                "async_promise_complete_ptr",
                vec![Type::Handle, Type::String],
                Type::Void,
            ),
            (
                "async_await_bool",
                vec![Type::Handle],
                Type::Integer(ast::IntKind::I32),
            ),
            ("async_await_ptr", vec![Type::Handle], Type::String),
            (
                "async_poll_bool",
                vec![Type::Handle],
                Type::Integer(ast::IntKind::I32),
            ),
            (
                "async_future_done",
                vec![Type::Handle],
                Type::Integer(ast::IntKind::I32),
            ),
            ("async_future_ptr_value", vec![Type::Handle], Type::String),
        ] {
            env.functions.insert(
                name.into(),
                FunctionSignature {
                    params,
                    return_type: ret,
                },
            );
        }
        env.functions.insert(
            "channel_new".into(),
            FunctionSignature {
                params: vec![],
                return_type: Type::Handle,
            },
        );
        env.functions.insert(
            "channel_recv".into(),
            FunctionSignature {
                params: vec![Type::Handle],
                return_type: Type::Integer(ast::IntKind::I32),
            },
        );
        env.functions.insert(
            "channel_send".into(),
            FunctionSignature {
                params: vec![Type::Handle, Type::Integer(ast::IntKind::I32)],
                return_type: Type::Void,
            },
        );
        env
    }

    pub(super) fn in_unsafe(&self) -> bool {
        self.unsafe_depth > 0
    }

    pub(super) fn is_raw_pointer_type(ty: &Type) -> bool {
        matches!(ty, Type::RawPtr { .. } | Type::Ptr)
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub(super) fn is_numeric_type(ty: &Type) -> bool {
        types::is_numeric(ty)
    }

    pub(super) fn unify_numeric_type(left: Type, right: Type) -> Type {
        unify_numeric(left, right)
    }

    pub(super) fn type_to_ann(ty: &Type) -> TypeAnnotation {
        match ty {
            Type::Integer(k) => TypeAnnotation::Integer(*k),
            Type::F32 => TypeAnnotation::F32,
            Type::F64 => TypeAnnotation::F64,
            Type::Char => TypeAnnotation::Char,
            Type::Bool => TypeAnnotation::Bool,
            Type::String => TypeAnnotation::String,
            Type::VecStr => TypeAnnotation::VecStr,
            Type::Tuple { elems } => {
                TypeAnnotation::Tuple(elems.iter().map(Self::type_to_ann).collect())
            }
            Type::Array { elem, len } => TypeAnnotation::Array {
                elem: Box::new(Self::type_to_ann(elem)),
                len: *len,
            },
            Type::Struct(n) => TypeAnnotation::Struct(n.clone()),
            Type::Enum(n) => TypeAnnotation::Enum(n.clone()),
            Type::Void => TypeAnnotation::Void,
            Type::Ptr => TypeAnnotation::Ptr,
            Type::RawPtr { inner } => TypeAnnotation::RawPtr {
                inner: Box::new(Self::type_to_ann(inner)),
            },
            Type::Ref {
                inner,
                mutable,
                lifetime,
            } => TypeAnnotation::Ref {
                inner: Box::new(Self::type_to_ann(inner)),
                mutable: *mutable,
                lifetime: lifetime.clone(),
            },
            Type::Unknown => TypeAnnotation::Generic("_".into()),
            _ => TypeAnnotation::Generic("_".into()),
        }
    }
}

