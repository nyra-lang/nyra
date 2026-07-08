mod anon_structs;
mod lang;
mod ffi;
mod future_types;
mod string_builtins;
mod date_builtins;
mod array_builtins;
mod diagnostics;
mod helpers;
mod checker_setup;
mod checker_program;
mod checker_function;
mod checker_stmt;
mod checker_expr;
mod checker_io;
mod math_builtins;
mod random_builtins;
mod layout_builtins;
mod bytes_builtins;
mod param_inference;
mod send_sync;
mod trait_dispatch;

pub use diagnostics::type_pretty;
pub use array_builtins::array_method_borrows_receiver;
pub use date_builtins::{DATE_STRUCT, date_field_alias};

use std::collections::HashMap;

pub use types::{EnumInfo, StructInfo, UnionInfo, Type, integer_assignable, is_integer, unify_numeric};
pub use ownership::{ownership_of, OwnershipKind};

use errors::NyraError;
use ast::{Program, TraitMethodSig};

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    pub variables: HashMap<String, VarInfo>,
    pub functions: HashMap<String, FunctionSignature>,
}

#[derive(Debug, Clone)]
pub struct InferredBinding {
    pub name: String,
    pub span: errors::Span,
    pub ty: Type,
}

pub struct TypeChecker {
    pub env: TypeEnv,
    pub structs: HashMap<String, StructInfo>,
    pub unions: HashMap<String, UnionInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub errors: Vec<NyraError>,
    /// `let x = ...` bindings without explicit type (for IDE inlay hints).
    pub inferred_bindings: Vec<InferredBinding>,
    pub target: String,
    pub no_std: bool,
    /// Nesting depth inside `unsafe { }` blocks.
    pub unsafe_depth: u32,
    /// Nesting depth inside `while` / `for` (for `break` validation).
    pub loop_depth: u32,
    /// Top-level names for `did you mean` suggestions.
    pub global_names: Vec<String>,
    /// `trait_name` → method signatures from `trait` defs.
    pub trait_methods: HashMap<String, Vec<TraitMethodSig>>,
    /// `(concrete_type, trait_name)` pairs with `impl Trait for Type`.
    pub trait_impl_pairs: Vec<(String, String)>,
    /// When set, untyped param inference also uses call sites from this program.
    program_for_inference: Option<*const Program>,
    /// True while registering function signatures (suppresses cascading resolution errors).
    signature_inference: bool,
    /// Resolved struct names for `{ field: value }` literals (check order).
    anon_name_queue: Vec<String>,
    /// `T: Trait` bounds while checking a generic function body.
    current_type_param_bounds: HashMap<String, Vec<String>>,
    /// `field:type` shape → synthesized or matched struct name.
    anon_shape_index: std::collections::HashMap<String, String>,
    anon_counter: usize,
    synthesized_struct_defs: Vec<ast::StructDef>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
