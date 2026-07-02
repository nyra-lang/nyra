use std::collections::{HashMap, HashSet};

use ast::*;
use crate::context::OwnershipCtx;
use types::Type;

/// Tracks which bindings need cleanup at scope exit.
#[derive(Debug, Clone, Default)]
pub struct DropPlan {
    /// Per-function: variables declared as owned heap (string / owned call result).
    pub owned_bindings: HashMap<String, HashSet<String>>,
    /// Per-function: struct locals with a custom `Drop` impl.
    pub custom_struct_bindings: HashMap<String, HashSet<String>>,
    /// Per-function: struct locals needing automatic field-wise heap cleanup (no custom Drop).
    pub composite_struct_bindings: HashMap<String, HashSet<String>>,
    /// Per-function: enum locals with heap payload (e.g. Option<string>).
    pub enum_payload_bindings: HashMap<String, HashSet<String>>,
    /// Per-function: `JoinHandle` locals needing `spawn_handle_drop` if not joined.
    pub join_handle_bindings: HashMap<String, HashSet<String>>,
    /// Per-function: task vs OS thread for each join handle binding.
    pub join_handle_kinds: HashMap<String, HashMap<String, SpawnKind>>,
    /// Bindings explicitly freed via `defer free(x)` or `free(x)` call.
    pub manually_freed: HashMap<String, HashSet<String>>,
    /// Struct type name → `Drop_Type_drop` LLVM symbol.
    pub custom_drop_fns: HashMap<String, String>,
}

impl DropPlan {
    pub fn is_owned_in(&self, func: &str, name: &str) -> bool {
        self.owned_bindings
            .get(func)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn is_custom_struct_in(&self, func: &str, name: &str) -> bool {
        self.custom_struct_bindings
            .get(func)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn is_composite_struct_in(&self, func: &str, name: &str) -> bool {
        self.composite_struct_bindings
            .get(func)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn is_enum_payload_in(&self, func: &str, name: &str) -> bool {
        self.enum_payload_bindings
            .get(func)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn needs_struct_drop_in(&self, func: &str, name: &str) -> bool {
        self.is_custom_struct_in(func, name) || self.is_composite_struct_in(func, name)
    }

    pub fn is_manually_freed_in(&self, func: &str, name: &str) -> bool {
        self.manually_freed
            .get(func)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn is_join_handle_in(&self, func: &str, name: &str) -> bool {
        self.join_handle_bindings
            .get(func)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn join_handle_kind(&self, func: &str, name: &str) -> SpawnKind {
        self.join_handle_kinds
            .get(func)
            .and_then(|m| m.get(name).copied())
            .unwrap_or(SpawnKind::Task)
    }

    pub fn custom_drop_fn_for_struct(&self, struct_name: &str) -> Option<&str> {
        self.custom_drop_fns.get(struct_name).map(|s| s.as_str())
    }
}

pub fn plan_drops(program: &Program, ctx: &OwnershipCtx) -> DropPlan {
    let mut plan = DropPlan::default();
    for ti in &program.trait_impls {
        if ti.trait_name == "Drop" {
            let fn_name = ti
                .methods
                .iter()
                .find(|m| m.name.ends_with("_drop"))
                .map(|m| m.name.clone())
                .unwrap_or_else(|| format!("Drop_{}_drop", ti.type_name));
            plan.custom_drop_fns
                .insert(ti.type_name.clone(), fn_name);
        }
    }
    for sdef in &program.structs {
        if let Some(trait_name) = sdef.name.strip_prefix("Dyn_") {
            plan.custom_drop_fns.insert(
                sdef.name.clone(),
                format!("__dyn_{trait_name}_drop"),
            );
        }
    }
    let custom_drop_fns = plan.custom_drop_fns.clone();
    for func in &program.functions {
        if !func.type_params.is_empty() {
            continue;
        }
        scan_function(&func.body, &func.name, ctx, &custom_drop_fns, &mut plan);
    }
    for imp in &program.impls {
        for method in &imp.methods {
            scan_function(&method.body, &method.name, ctx, &custom_drop_fns, &mut plan);
        }
    }
    for ti in &program.trait_impls {
        for method in &ti.methods {
            scan_function(&method.body, &method.name, ctx, &custom_drop_fns, &mut plan);
        }
    }
    plan
}

fn scan_function(
    body: &Block,
    func: &str,
    ctx: &OwnershipCtx,
    custom_drop_fns: &HashMap<String, String>,
    plan: &mut DropPlan,
) {
    let mut owned = HashSet::new();
    let mut custom_struct = HashSet::new();
    let mut composite_struct = HashSet::new();
    let mut manual = HashSet::new();
    let mut types: HashMap<String, Type> = HashMap::new();
    let mut spawn_id = 0usize;
    scan_block(
        body,
        func,
        ctx,
        custom_drop_fns,
        plan,
        &mut owned,
        &mut custom_struct,
        &mut composite_struct,
        &mut manual,
        &mut types,
        &mut spawn_id,
    );
    if !owned.is_empty() {
        plan.owned_bindings.insert(func.to_string(), owned);
    }
    if !custom_struct.is_empty() {
        plan.custom_struct_bindings
            .insert(func.to_string(), custom_struct);
    }
    if !composite_struct.is_empty() {
        plan.composite_struct_bindings
            .insert(func.to_string(), composite_struct);
    }
    if !manual.is_empty() {
        plan.manually_freed.insert(func.to_string(), manual);
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_block(
    block: &Block,
    func: &str,
    ctx: &OwnershipCtx,
    custom_drop_fns: &HashMap<String, String>,
    plan: &mut DropPlan,
    owned: &mut HashSet<String>,
    custom_struct: &mut HashSet<String>,
    composite_struct: &mut HashSet<String>,
    manual: &mut HashSet<String>,
    types: &mut HashMap<String, Type>,
    spawn_id: &mut usize,
) {
    for stmt in &block.statements {
        scan_statement(
            stmt,
            func,
            ctx,
            custom_drop_fns,
            plan,
            owned,
            custom_struct,
            composite_struct,
            manual,
            types,
            spawn_id,
        );
    }
}

/// Bindings declared only inside `if` / `while` / `for` bodies must not be
/// auto-dropped at function exit (they are not live on all paths).
#[allow(clippy::too_many_arguments)]
fn scan_ephemeral_block(
    block: &Block,
    func: &str,
    ctx: &OwnershipCtx,
    custom_drop_fns: &HashMap<String, String>,
    plan: &mut DropPlan,
    _owned: &mut HashSet<String>,
    _custom_struct: &mut HashSet<String>,
    _composite_struct: &mut HashSet<String>,
    manual: &mut HashSet<String>,
    types: &mut HashMap<String, Type>,
    spawn_id: &mut usize,
) {
    let mut ephemeral_owned = HashSet::new();
    let mut ephemeral_custom = HashSet::new();
    let mut ephemeral_composite = HashSet::new();
    scan_block(
        block,
        func,
        ctx,
        custom_drop_fns,
        plan,
        &mut ephemeral_owned,
        &mut ephemeral_custom,
        &mut ephemeral_composite,
        manual,
        types,
        spawn_id,
    );
}

#[allow(clippy::too_many_arguments)]
fn scan_statement(
    stmt: &Statement,
    func: &str,
    ctx: &OwnershipCtx,
    custom_drop_fns: &HashMap<String, String>,
    plan: &mut DropPlan,
    owned: &mut HashSet<String>,
    custom_struct: &mut HashSet<String>,
    composite_struct: &mut HashSet<String>,
    manual: &mut HashSet<String>,
    types: &mut HashMap<String, Type>,
    spawn_id: &mut usize,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let ty = l
                .ty
                .clone()
                .map(Type::from)
                .unwrap_or_else(|| ctx.infer_expr_type(&l.value));
            types.insert(l.name.clone(), ty.clone());
            if ty == Type::JoinHandle || matches!(&l.value, Expression::Spawn { .. }) {
                plan.join_handle_bindings
                    .entry(func.to_string())
                    .or_default()
                    .insert(l.name.clone());
                if let Expression::Spawn { kind, .. } = &l.value {
                    plan.join_handle_kinds
                        .entry(func.to_string())
                        .or_default()
                        .insert(l.name.clone(), *kind);
                }
            }
            if needs_heap_drop_binding(&l.value, &ty, ctx) {
                owned.insert(l.name.clone());
            }
            if let Type::Struct(name) = &ty {
                if custom_drop_fns.contains_key(name) {
                    custom_struct.insert(l.name.clone());
                } else if ctx.struct_needs_composite_drop(name) {
                    composite_struct.insert(l.name.clone());
                }
            }
            if let Type::Enum(name) = &ty {
                if ctx.enum_needs_payload_drop(name) {
                    plan.enum_payload_bindings
                        .entry(func.to_string())
                        .or_default()
                        .insert(l.name.clone());
                }
            }
            if let Expression::Variable { name, .. } = &l.value {
                if ctx.kind_of(types.get(name).unwrap_or(&Type::Unknown)).is_move() {
                    owned.remove(name);
                    custom_struct.remove(name);
                    composite_struct.remove(name);
                    if let Some(set) = plan.enum_payload_bindings.get_mut(func) {
                        set.remove(name);
                    }
                }
            }
        }
        Statement::Expression(expr) | Statement::Defer(expr) => {
            register_manual_free(expr, manual);
            register_move_from_call(expr, ctx, types, owned, custom_struct, composite_struct);
        }
        Statement::Print(p) => {
            for arg in &p.args {
                register_move_from_call(arg, ctx, types, owned, custom_struct, composite_struct);
            }
            if let Some(c) = &p.color {
                register_move_from_call(c, ctx, types, owned, custom_struct, composite_struct);
            }
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                register_move_from_expr(v, ctx, types, owned, custom_struct, composite_struct);
            }
        }
        Statement::If(i) => {
            scan_ephemeral_block(
                &i.then_block,
                func,
                ctx,
                custom_drop_fns,
                plan,
                owned,
                custom_struct,
                composite_struct,
                manual,
                types,
                spawn_id,
            );
            if let Some(e) = &i.else_block {
                scan_ephemeral_block(
                    e,
                    func,
                    ctx,
                    custom_drop_fns,
                    plan,
                    owned,
                    custom_struct,
                    composite_struct,
                    manual,
                    types,
                    spawn_id,
                );
            }
        }
        Statement::While(w) => scan_ephemeral_block(
            &w.body,
            func,
            ctx,
            custom_drop_fns,
            plan,
            owned,
            custom_struct,
            composite_struct,
            manual,
            types,
            spawn_id,
        ),
        Statement::For(f) => scan_ephemeral_block(
            &f.body,
            func,
            ctx,
            custom_drop_fns,
            plan,
            owned,
            custom_struct,
            composite_struct,
            manual,
            types,
            spawn_id,
        ),
        Statement::Asm { .. } => {}
        Statement::Unsafe(body) => scan_block(
            body,
            func,
            ctx,
            custom_drop_fns,
            plan,
            owned,
            custom_struct,
            composite_struct,
            manual,
            types,
            spawn_id,
        ),
        Statement::Benchmark(body) => scan_block(
            body,
            func,
            ctx,
            custom_drop_fns,
            plan,
            owned,
            custom_struct,
            composite_struct,
            manual,
            types,
            spawn_id,
        ),
        Statement::Spawn(sp) => {
            let spawn_fn = format!("{func}__spawn_{spawn_id}");
            *spawn_id += 1;
            let mut s_owned = HashSet::new();
            let mut s_custom = HashSet::new();
            let mut s_composite = HashSet::new();
            let mut s_manual = HashSet::new();
            let mut s_types = HashMap::new();
            let mut inner_spawn = 0usize;
            scan_block(
                &sp.body,
                &spawn_fn,
                ctx,
                custom_drop_fns,
                plan,
                &mut s_owned,
                &mut s_custom,
                &mut s_composite,
                &mut s_manual,
                &mut s_types,
                &mut inner_spawn,
            );
            if !s_owned.is_empty() {
                plan.owned_bindings.insert(spawn_fn.clone(), s_owned);
            }
            if !s_custom.is_empty() {
                plan.custom_struct_bindings
                    .insert(spawn_fn.clone(), s_custom);
            }
            if !s_composite.is_empty() {
                plan.composite_struct_bindings
                    .insert(spawn_fn.clone(), s_composite);
            }
            if !s_manual.is_empty() {
                plan.manually_freed.insert(spawn_fn.clone(), s_manual);
            }
        }
        _ => {}
    }
}

fn needs_heap_drop_binding(value: &Expression, ty: &Type, ctx: &OwnershipCtx) -> bool {
    match ty {
        Type::String if ctx.kind_of(ty).is_move() => match value {
            Expression::Call(c) if ctx.callee_returns_owned(&c.callee) => true,
            Expression::Variable { .. } => true,
            Expression::StructLiteral(_) | Expression::TemplateLiteral(_) => true,
            _ => false,
        },
        Type::VecStr => match value {
            Expression::MethodCall(mc) if mc.method == "split" => true,
            Expression::Call(c) if c.callee == "str_split" => true,
            Expression::Variable { .. } => true,
            _ => false,
        },
        Type::Struct(name) if ctx.struct_needs_composite_drop(name) => matches!(
            value,
            Expression::StructLiteral(_)
                | Expression::Variable { .. }
                | Expression::Call(_)
        ),
        _ => false,
    }
}

fn register_manual_free(expr: &Expression, manual: &mut HashSet<String>) {
    if let Expression::Call(c) = expr {
        if c.callee == "free" {
            if let Some(Expression::Variable { name, .. }) = c.args.first() {
                manual.insert(name.clone());
            }
        }
    }
}

fn register_move_from_call(
    expr: &Expression,
    ctx: &OwnershipCtx,
    types: &HashMap<String, Type>,
    owned: &mut HashSet<String>,
    custom_struct: &mut HashSet<String>,
    composite_struct: &mut HashSet<String>,
) {
    let Expression::Call(c) = expr else {
        return;
    };
    if c.callee == "free" {
        return;
    }
    for arg in &c.args {
        register_move_from_expr(arg, ctx, types, owned, custom_struct, composite_struct);
    }
}

fn register_move_from_expr(
    expr: &Expression,
    ctx: &OwnershipCtx,
    types: &HashMap<String, Type>,
    owned: &mut HashSet<String>,
    custom_struct: &mut HashSet<String>,
    composite_struct: &mut HashSet<String>,
) {
    let Expression::Variable { name, .. } = expr else {
        return;
    };
    if ctx.kind_of(types.get(name).unwrap_or(&Type::Unknown)).is_move() {
        owned.remove(name);
        custom_struct.remove(name);
        composite_struct.remove(name);
    }
}
