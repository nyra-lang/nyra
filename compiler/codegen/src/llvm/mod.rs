//! LLVM IR codegen for Nyra.
//!
//! The backend is split into focused submodules (each with one `impl Codegen` block).
//! Internal methods use `pub(super)` so sibling modules can call each other within `llvm`.
#![allow(
    clippy::if_same_then_else,
    clippy::only_used_in_recursion,
    clippy::single_char_add_str,
    clippy::needless_borrow,
    clippy::needless_as_bytes
)]

mod bindings;
mod block;
mod channel;
mod closure;
mod collections;
mod control;
mod core;
mod drop;
mod expr;
mod ffi;
mod const_mod;
mod intrinsics;
mod literals;
mod ops;
mod print;
mod parallel;
mod program;
mod progress;
mod spawn;
mod simd;
mod stmt;
mod store;
mod strings;
mod trait_objects;
mod types_map;
pub(crate) mod util;

use std::collections::{BTreeSet, HashMap, HashSet};

use ast::*;
use ownership::{DropPlan, EscapePlan};

const LOCAL_CHANNEL_TYPE: &str = "%NyraLocalChannel_i32";
const LOCAL_CHANNEL_CAP: i32 = 16;

pub struct Codegen {
    module_name: String,
    strings: Vec<String>,
    string_intern: HashMap<String, usize>,
    lines: Vec<String>,
    temp_counter: usize,
    label_counter: usize,
    struct_fields: HashMap<String, Vec<(String, TypeAnnotation)>>,
    tuple_fields: HashMap<String, Vec<TypeAnnotation>>,
    functions: HashMap<String, Function>,
    extern_functions: HashMap<String, ExternFn>,
    call_returns: HashMap<String, String>,
    enum_variants: HashMap<String, Vec<String>>,
    enum_has_payload: HashMap<String, bool>,
    enum_payload_llvm: HashMap<String, String>,
    /// Variable name → enum type name (for match tag checks).
    enum_locals: HashMap<String, String>,
    /// Top-level `const` bindings (including from imports).
    module_consts: HashMap<String, ExprValue>,
    /// Enum type names from the merged program (for resolving `Struct` annotations).
    enum_names: HashSet<String>,
    target: String,
    drop_plan: DropPlan,
    escape_plan: EscapePlan,
    /// Composite struct locals proven stack-only (NoEscape + static string fields): skip field free.
    no_escape_stack_safe: HashSet<String>,
    used_runtime: BTreeSet<String>,
    needs_malloc_decl: bool,
    /// Emitted at least one `call @puts` (static string print fast path).
    uses_puts: bool,
    skip_runtime_decls: HashSet<String>,
    /// Module-level IR emitted from inside functions (spawn capture types + helper fns).
    module_level: Vec<String>,
    /// When set, `emit` appends to this buffer (spawn helper function bodies).
    emit_buf: Option<Vec<String>>,
    current_async_fn: bool,
    /// Active drop state while compiling a function body (closure capture moves).
    compiling_drop: Option<*mut DropState>,
    /// Function-pointer parameters/locals in the current function (name → signature).
    current_fn_ptrs: HashMap<String, FnPtrSig>,
    /// Capturing closure metadata from the most recent `compile_arrow_fn` (inline arg sites).
    pending_closure_meta: Option<ClosureMeta>,
    /// When true, the next capturing arrow fn uses heap env promotion.
    closure_force_heap: bool,
    /// Name of the function currently being codegen'd (for owned-string heuristics).
    current_func: String,
    local_channel_type_emitted: bool,
    /// `let mut` scalars promoted to SSA (name → update binding on assign, no alloca).
    mut_ssa_locals: HashSet<String>,
    /// Locals whose current `ptr` value came from heap allocation (safe to `free` on reassignment).
    heap_string_bindings: HashSet<String>,
    /// Locals proven to stay >= 0 for i32 (loop counters, positive `%` chains).
    non_negative_vars: HashSet<String>,
    /// `let mut` SSA scalars initialized from a non-negative literal (e.g. `mut acc = 0`).
    zero_init_ssa_vars: HashSet<String>,
    /// Active loop phi contexts (while/for) for loop-carried SSA.
    loop_stack: Vec<LoopPhiContext>,
    /// LLVM block label for the most recently emitted basic block.
    current_block: String,
    /// Monotonic parallel helper index per function (expression-site `parallel any/find/all`).
    func_par_idx: usize,
    /// Monotonic spawn-body index per function. Drives the emitted `__spawn_<fn>_<n>`
    /// symbol so statement spawns and expression spawns (which use independent
    /// `DropState`s) can never collide on the same LLVM function name.
    func_spawn_idx: usize,
    /// LLVM attribute groups for `#[inline]` / `#[hot]` / `#[cold]` functions.
    fn_attr_sets: Vec<String>,
    /// Struct types declared with `repr(C)` (C ABI at FFI boundaries).
    repr_c_structs: HashSet<String>,
    /// Union field layouts by name.
    union_fields: HashMap<String, Vec<(String, TypeAnnotation)>>,
    /// Struct layout metadata for `size_of` / `align_of` intrinsics.
    struct_layout_infos: HashMap<String, types::StructInfo>,
    union_layout_infos: HashMap<String, types::UnionInfo>,
    repr_c_unions: HashSet<String>,
    /// Per-enum variant payload LLVM types (heterogeneous enums).
    enum_variant_payload_llvm: HashMap<String, HashMap<String, String>>,
    /// Names of `extern fn` symbols (calls into native C libraries).
    extern_fn_names: HashSet<String>,
    /// Nyra `extern fn` name → linked C runtime symbol (e.g. `strlen` → `strlen`).
    extern_c_symbols: HashMap<String, String>,
    /// C symbols already emitted as `declare` (avoid duplicate LLVM declarations).
    declared_c_syms: HashSet<String>,
    /// Integer variable → Nyra `IntKind` (for generic `random` dispatch).
    local_int_kinds: HashMap<String, IntKind>,
    /// LLVM intrinsic `declare` lines (math builtins).
    intrinsic_decl_lines: Vec<String>,
    intrinsic_decls: HashSet<String>,
    /// Static trait dispatch: `(struct_name, method) → mangled fn` (e.g. `Add_Counter_add`).
    trait_method_callees: HashMap<(String, String), String>,
}

#[derive(Clone)]
struct LoopPhiContext {
    /// Loop-carried binding → canonical latch reg (phi back-edge operand).
    latch_regs: HashMap<String, String>,
    latch_label: String,
    /// Block before `latch_label` where loop-carried values are synced (continue targets here).
    latch_sync_label: String,
    body_label: String,
    end_label: String,
    cond_label: String,
    /// Block that emits the branch to `end_label` when the loop condition is false.
    exit_pred: String,
    carried: Vec<String>,
    header_phi_regs: HashMap<String, (String, String)>,
    break_edges: Vec<(String, HashMap<String, (String, String)>)>,
}

struct NestedFnCodegenScope {
    current_block: String,
    loop_stack: Vec<LoopPhiContext>,
    mut_ssa_locals: HashSet<String>,
}

#[derive(Clone)]
struct FnPtrSig {
    reg: String,
    _param_tys: Vec<String>,
    ret_ty: String,
    /// When set, load env ptr into this slot before indirect calls (capturing closures).
    invoke_slot: Option<String>,
    env_alloca: Option<String>,
}

#[derive(Clone)]
enum EnvKind {
    Stack { alloca: String },
    Heap { global: String },
}

#[derive(Clone)]
struct ClosureMeta {
    body_symbol: String,
    wrap_symbol: String,
    invoke_slot: String,
    env_kind: EnvKind,
    heap_owned: bool,
    param_tys: Vec<String>,
    ret_ty: String,
}

#[derive(Clone)]
enum Binding {
    /// Function parameter (%0, %1, …).
    Param { index: usize, ty: String },
    /// SSA virtual register name without leading `%` (e.g. add.3, str.0).
    Reg { reg: String, ty: String },
    /// Stack slot for `let mut` and owned values needing drop.
    Stack { slot: String, ty: String },
    /// NoEscape struct decomposed into per-field SSA registers (SROA).
    PromotedStruct {
        struct_name: String,
        value_ty: String,
        fields: HashMap<String, (String, String)>,
    },
    /// Stack-allocated capturing closure `{ fn, env }`.
    Closure(ClosureMeta),
    /// Single-thread stack ring buffer channel (NoEscape); no mutex / heap handle.
    LocalChannel { slot: String },
}

type Env = HashMap<String, Binding>;

#[derive(Debug, Default)]
struct DropState {
    func: String,
    moved: HashSet<String>,
    spawn_id: usize,
    par_id: usize,
    closure_id: usize,
}

impl DropState {
    fn new(func: &str) -> Self {
        Self {
            func: func.to_string(),
            moved: HashSet::new(),
            spawn_id: 0,
            par_id: 0,
            closure_id: 0,
        }
    }

    fn mark_moved(&mut self, name: &str) {
        self.moved.insert(name.to_string());
    }

    fn next_spawn_key(&mut self) -> String {
        let key = format!("{}__spawn_{}", self.func, self.spawn_id);
        self.spawn_id += 1;
        key
    }

    fn next_par_idx(&mut self) -> usize {
        let id = self.par_id;
        self.par_id += 1;
        id
    }

    fn next_closure_idx(&mut self) -> usize {
        let id = self.closure_id;
        self.closure_id += 1;
        id
    }
}

#[derive(Clone)]
struct ExprValue {
    reg: String,
    ty: String,
}
