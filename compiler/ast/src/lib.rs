mod float;
mod integer;
mod span;
mod expr_walk;

pub use float::FloatKind;
pub use integer::IntKind;
pub use span::{binding_name, expr_span, is_explicit_move, stmt_span, variable_name};
pub use expr_walk::{for_each_expr_in_block, for_each_expr_in_block_mut};

use errors::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StructAttrs {
    pub send: bool,
    pub sync: bool,
    /// `repr(C)` — C-compatible field layout at FFI boundaries.
    pub repr_c: bool,
    /// Explicit `#[derive(Copy)]` / `struct S Copy { }` — validates all fields are Copy.
    pub copy: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: String,
    /// `import "path.ny" as alias` — symbols merged as `alias__name` / `alias::name`.
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub module: Option<String>,
    /// `# no_std` / `no_std` directive — no automatic `nyra_rt` linking; freestanding subset.
    pub no_std: bool,
    /// `comptime` directive — entire file is compile-time only (first line of the unit).
    pub comptime: bool,
    /// `allow_extended` directive — suppress Extended-tier stability warnings (W001) for this unit.
    pub allow_extended: bool,
    pub imports: Vec<ImportDecl>,
    pub consts: Vec<ConstDef>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub traits: Vec<TraitDef>,
    pub trait_impls: Vec<TraitImpl>,
    pub macros: Vec<MacroDef>,
    pub impls: Vec<ImplDef>,
    pub externs: Vec<ExternFn>,
    pub functions: Vec<Function>,
    /// Explicit monomorph exports for generic `export fn` (e.g. `export inst id<i32>`).
    pub export_instances: Vec<ExportInstance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportInstance {
    pub fn_name: String,
    pub type_args: Vec<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethodSig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitImpl {
    pub type_name: String,
    pub trait_name: String,
    pub methods: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub name: String,
    pub ty: Option<TypeAnnotation>,
    pub value: Expression,
    /// Visible to importers (`priv const` hides from other files).
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    pub type_name: String,
    pub methods: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFn {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub attrs: StructAttrs,
    pub fields: Vec<StructField>,
    pub doc: Option<String>,
    /// Visible to importers (`priv struct` hides from other files).
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: TypeAnnotation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantDef {
    pub name: String,
    pub fields: Vec<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariantDef>,
    /// Visible to importers (`priv enum` hides from other files).
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub is_test: bool,
    pub ignore_test: bool,
    pub should_fail_test: bool,
    pub is_async: bool,
    pub exported: bool,
    /// Visible to importers (`priv fn` hides from other files; separate from `export fn` FFI).
    pub public: bool,
    pub span: Span,
    pub type_params: Vec<String>,
    /// Trait bounds per type parameter (`T: Greet` → `T` → `["Greet"]`).
    pub type_param_bounds: HashMap<String, Vec<String>>,
    pub lifetime_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    /// `#[inline]` — emit LLVM `alwaysinline`.
    pub inline: bool,
    /// `#[hot]` — emit LLVM `inlinehint`.
    pub hot: bool,
    /// `#[cold]` — emit LLVM `cold`.
    pub cold: bool,
    /// `#[comptime]` — evaluate at compile time when called with known arguments; stripped from runtime binary.
    pub comptime: bool,
    /// Leading `///` doc comment lines joined with newlines.
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeAnnotation,
    /// Tuple-destructure names for `((a, b)) =>` arrow params.
    pub destructure: Vec<String>,
    /// `#[no_escape]` — borrow must not escape via return, spawn, or channel.
    pub no_escape: bool,
    /// `mut x` in `fn f(mut x: i32)` — parameter may be reassigned in the body.
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt {
    pub args: Vec<Expression>,
    pub color: Option<Expression>,
}

impl PrintStmt {
    pub fn map_expressions<F>(self, mut f: F) -> Self
    where
        F: FnMut(Expression) -> Expression,
    {
        Self {
            args: self.args.into_iter().map(&mut f).collect(),
            color: self.color.map(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let(LetStmt),
    Const(LetStmt),
    Assign(AssignStmt),
    Return(ReturnStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Break { span: Span },
    Continue { span: Span },
    Expression(Expression),
    Print(PrintStmt),
    Defer(Expression),
    Spawn(Block),
    Benchmark(Block),
    Unsafe(Block),
    /// `asm "template"` — LLVM inline assembly; requires enclosing `unsafe`.
    Asm { template: String, span: Span },
    Import(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForKind {
    /// `for i in start..end`
    Range {
        start: Expression,
        end: Expression,
    },
    /// `for x in arr` / `for c in str`
    Iterable {
        iterable: Expression,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParallelMode {
    #[default]
    Auto,
    Balanced,
    MaxPerformance,
    Background,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParallelThreads {
    /// Runtime picks worker count from mode and CPU topology.
    Auto,
    /// Cap workers (`max_threads = N`); may use fewer when iteration count is small.
    Max(Expression),
    /// Exact worker count (`threads = N`).
    Exact(Expression),
    /// Fraction of logical CPUs (`cpu = 80%`).
    CpuPercent(Expression),
}

impl Default for ParallelThreads {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParallelConfig {
    pub mode: ParallelMode,
    pub threads: ParallelThreads,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            mode: ParallelMode::Auto,
            threads: ParallelThreads::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgressConfig {
    /// Optional status label (`progress(label = "parser tests") for ...`).
    pub label: Option<Expression>,
}

impl ProgressConfig {
    pub fn map_exprs_mut<F: FnMut(&mut Expression)>(&mut self, mut f: F) {
        if let Some(label) = &mut self.label {
            f(label);
        }
    }

    pub fn for_each_expr<F: FnMut(&Expression)>(&self, mut f: F) {
        if let Some(label) = &self.label {
            f(label);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub var: String,
    pub kind: ForKind,
    pub body: Block,
    /// `parallel for` / `parallel(...) for` — iterations run on a worker pool.
    pub parallel: Option<ParallelConfig>,
    /// `progress for` — automatic progress bar and status line each iteration.
    pub progress: Option<ProgressConfig>,
}

impl ForStmt {
    pub fn map_exprs_mut<F: FnMut(&mut Expression)>(&mut self, mut f: F) {
        match &mut self.kind {
            ForKind::Range { start, end } => {
                f(start);
                f(end);
            }
            ForKind::Iterable { iterable } => f(iterable),
        }
        if let Some(cfg) = &mut self.parallel {
            cfg.map_exprs_mut(&mut f);
        }
        if let Some(cfg) = &mut self.progress {
            cfg.map_exprs_mut(&mut f);
        }
    }

    pub fn for_each_expr<F: FnMut(&Expression)>(&self, mut f: F) {
        match &self.kind {
            ForKind::Range { start, end } => {
                f(start);
                f(end);
            }
            ForKind::Iterable { iterable } => f(iterable),
        }
        if let Some(cfg) = &self.parallel {
            cfg.for_each_expr(&mut f);
        }
        if let Some(cfg) = &self.progress {
            cfg.for_each_expr(&mut f);
        }
    }
}

impl ParallelConfig {
    pub fn map_exprs_mut<F: FnMut(&mut Expression)>(&mut self, f: &mut F) {
        self.threads.map_expr_mut(f);
    }

    pub fn for_each_expr<F: FnMut(&Expression)>(&self, mut f: F) {
        self.threads.for_each_expr(&mut f);
    }
}

impl ParallelThreads {
    pub fn map_expr_mut<F: FnMut(&mut Expression)>(&mut self, f: &mut F) {
        match self {
            ParallelThreads::Auto => {}
            ParallelThreads::Max(e) | ParallelThreads::Exact(e) | ParallelThreads::CpuPercent(e) => {
                f(e);
            }
        }
    }

    pub fn for_each_expr<F: FnMut(&Expression)>(&self, mut f: F) {
        match self {
            ParallelThreads::Auto => {}
            ParallelThreads::Max(e) | ParallelThreads::Exact(e) | ParallelThreads::CpuPercent(e) => {
                f(e);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    /// L-value: variable, `*ptr`, field access, or index.
    pub target: Expression,
    pub span: Span,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub mutable: bool,
    pub name: String,
    pub destructure: Vec<String>,
    pub span: Span,
    pub ty: Option<TypeAnnotation>,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expression,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Variable { name: String, span: Span },
    Binary(Box<BinaryExpr>),
    Unary(Box<UnaryExpr>),
    Call(CallExpr),
    MethodCall(Box<MethodCallExpr>),
    FieldAccess(Box<FieldAccessExpr>),
    StructLiteral(StructLiteralExpr),
    EnumVariant(EnumVariantExpr),
    Match(Box<MatchExpr>),
    If(Box<IfExpr>),
    Index(Box<IndexExpr>),
    ArrayLiteral(ArrayLiteralExpr),
    /// `[expr; N]` fixed-size array repeat
    ArrayRepeat {
        element: Box<Expression>,
        count: usize,
        /// Module/function `const` name when `N` was written as an identifier (resolved in const_eval).
        count_from: Option<String>,
        /// Compile-time count expression (`COLS * ROWS`) resolved in const_eval.
        count_expr: Option<Box<Expression>>,
        span: Span,
    },
    TupleLiteral(Vec<Expression>),
    Grouped(Box<Expression>),
    Await(Box<Expression>),
    TemplateLiteral(TemplateLiteralExpr),
    /// `expr as Type` — raw casts and coercions (raw-pointer casts require `unsafe`).
    Cast(Box<CastExpr>),
    /// ES6-style arrow function `(x: T) => expr` or `(x: T) => { ... }` (hoisted before typecheck).
    ArrowFn(Box<ArrowFnExpr>),
    /// `comptime { ... }` — compile-time block expression (folded when evaluable).
    ComptimeBlock { body: Block, span: Span },
    Invalid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrowFnExpr {
    pub params: Vec<Param>,
    pub body: ArrowBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    Expr(Expression),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastExpr {
    pub expr: Expression,
    pub target_type: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateLiteralExpr {
    pub parts: Vec<TemplatePart>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Static(String),
    Interpolation(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr {
    pub object: Expression,
    pub method: String,
    pub span: Span,
    pub args: Vec<Expression>,
    /// `?.method()` optional chaining — desugared to `Option` match in expand.
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantExpr {
    pub enum_name: Option<String>,
    pub variant: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    pub object: Expression,
    pub index: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub condition: Expression,
    pub then_block: Block,
    pub else_block: Block,
    pub span: Span,
}

/// Wrap a single expression as a one-statement block (if branches, ternary desugar).
pub fn block_from_expr(expr: Expression) -> Block {
    Block {
        statements: vec![Statement::Expression(expr)],
    }
}

/// Last value-producing statement in a block expression (`expr` or `return expr`).
pub fn block_trailing_expression(block: &Block) -> Option<Expression> {
    for stmt in block.statements.iter().rev() {
        match stmt {
            Statement::Expression(e) => return Some(e.clone()),
            Statement::Return(r) => return r.value.clone(),
            _ => {}
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpr {
    pub scrutinee: Box<Expression>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<Expression>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    Wildcard,
    /// String literal arm — `match s { "GET" => … }`
    Literal(String),
    Variant(String),
    Qualified(String, String),
    /// `Type.Variant(payload)` — payload bind, wildcard, or nested enum pattern
    QualifiedBind(String, String, MatchPayloadPattern),
    /// `A | B | C` — desugared to multiple arms before codegen
    Or(Vec<MatchPattern>),
    /// `Point { x, y }` / `Point { x: a }`
    Struct(String, Vec<StructMatchField>),
    /// `(a, b, _)` on tuple scrutinee
    Tuple(Vec<MatchPayloadPattern>),
}

/// Field pattern inside `MatchPattern::Struct`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructMatchField {
    pub field: String,
    /// `None` → bind field name; `Some("_")` → ignore; `Some(name)` → `field: name`.
    pub bind: Option<String>,
}

/// Payload sub-pattern inside `QualifiedBind` — `x`, `_`, or nested `Some(x)` / `Option.Some(x)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPayloadPattern {
    Bind(String),
    Wildcard,
    Nested(Box<MatchPattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldAccessExpr {
    pub object: Expression,
    pub field: String,
    pub optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayLiteralExpr {
    /// `...expr` — copy elements from an array value.
    pub spreads: Vec<Expression>,
    pub elems: Vec<Expression>,
    pub span: Span,
}

impl ArrayLiteralExpr {
    pub fn from_elems(elems: Vec<Expression>) -> Self {
        let span = elems.first().map(expr_span).unwrap_or_default();
        Self {
            spreads: Vec::new(),
            elems,
            span,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.spreads.is_empty() && self.elems.is_empty()
    }

    pub fn all_exprs(&self) -> impl Iterator<Item = &Expression> {
        self.spreads.iter().chain(self.elems.iter())
    }

    pub fn all_exprs_mut(&mut self) -> impl Iterator<Item = &mut Expression> {
        self.spreads.iter_mut().chain(self.elems.iter_mut())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralExpr {
    pub name: String,
    /// `..spread` — copy fields from existing struct values (same or other struct types).
    pub spreads: Vec<Expression>,
    pub fields: Vec<(String, Expression)>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub left: Expression,
    pub op: BinaryOp,
    pub right: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub callee: String,
    pub type_args: Vec<TypeAnnotation>,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    /// Integer with explicit suffix (`255u8`, `42i32`).
    IntKind(i64, IntKind),
    /// IEEE-754 float; second field is literal width (`f32` suffix or default `f64`).
    Float(f64, FloatKind),
    /// Unicode scalar value (valid `char` range).
    Char(u32),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    /// `<<` bitwise left shift
    Shl,
    /// `>>` bitwise right shift (arithmetic for signed integers)
    Shr,
    /// `&` bitwise AND (distinct from reference `&x` in unary position)
    BitAnd,
    /// `|` bitwise OR
    BitOr,
    /// `^` bitwise XOR
    BitXor,
    /// `??` — nullish coalescing (desugared to `match` on `Option`).
    NullishCoalesce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Ref,
    RefMut,
    Deref,
    /// Explicit ownership transfer at a call site (`save(move user)`).
    Move,
    /// Explicit clone before use (`save(clone user)` → `user.clone()`).
    Clone,
    /// `expr?` — propagate `Result::Err` from the enclosing function (desugared).
    Try,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnotation {
    /// Fixed-width integer: `i8`…`i128`, `u8`…`u128`, `isize`, `usize`.
    Integer(IntKind),
    F32,
    F64,
    Char,
    Bool,
    String,
    /// Split result: `let parts: VecStr = s.split(",")`
    VecStr,
    Ptr,
    /// Typed raw pointer `*T` (memory address with pointee type).
    RawPtr {
        inner: Box<TypeAnnotation>,
    },
    Void,
    Struct(String),
    /// `Vec<i32>` — generic type application.
    Applied {
        base: String,
        args: Vec<TypeAnnotation>,
    },
    Enum(String),
    Array {
        elem: Box<TypeAnnotation>,
        len: Option<usize>,
    },
    Tuple(Vec<TypeAnnotation>),
    Ref {
        inner: Box<TypeAnnotation>,
        mutable: bool,
        lifetime: Option<String>,
    },
    Generic(String),
    Lifetime(String),
    /// `for<'a, 'b> T` — higher-ranked lifetime binder (HRTB).
    ForAll {
        lifetimes: Vec<String>,
        inner: Box<TypeAnnotation>,
    },
    /// Function pointer type for HRTB callbacks.
    FnPtr {
        lifetime_params: Vec<String>,
        params: Vec<TypeAnnotation>,
        return_type: Option<Box<TypeAnnotation>>,
    },
    /// Trait object type: `dyn TraitName` or `dyn Trait + Send + Sync`.
    DynTrait {
        trait_name: String,
        bounds: Vec<String>,
    },
}

