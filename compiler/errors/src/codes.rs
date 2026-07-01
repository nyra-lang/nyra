//! Stable diagnostic codes for `nyra explain E00x` / `P00x`.

pub const E001_IMPORT_NOT_FOUND: &str = "E001";
pub const E002_UNDEFINED_NAME: &str = "E002";
pub const E003_TYPE_MISMATCH: &str = "E003";
pub const E004_CANNOT_INFER: &str = "E004";
pub const E005_UNKNOWN_STRUCT: &str = "E005";
pub const E006_IMMUTABLE_ASSIGN: &str = "E006";
pub const E007_WRONG_ARITY: &str = "E007";
pub const E008_WRONG_ARG_TYPE: &str = "E008";
pub const E009_INVALID_ASSIGN_TARGET: &str = "E009";
pub const E010_BORROW_WHILE_ASSIGNED: &str = "E010";
pub const E011_USE_WHILE_BORROWED: &str = "E011";
pub const E012_USE_AFTER_MOVE: &str = "E012";
pub const E013_UNDEFINED_FUNCTION: &str = "E013";
pub const E014_UNKNOWN_FIELD: &str = "E014";
pub const E015_OPERATOR_MISMATCH: &str = "E015";
pub const E016_UNSAFE_REQUIRED: &str = "E016";
pub const E017_NOT_CALLABLE: &str = "E017";
pub const E018_UNKNOWN_METHOD: &str = "E018";
pub const E019_BOOL_CONDITION: &str = "E019";
pub const E020_CONTROL_FLOW: &str = "E020";
pub const E021_PLATFORM_UNSUPPORTED: &str = "E021";
pub const E022_RETURN_MISMATCH: &str = "E022";
pub const E023_MATCH: &str = "E023";
pub const E024_FOR_IN: &str = "E024";
pub const E025_DESTRUCTURE: &str = "E025";
pub const E026_BLOCK_VALUE: &str = "E026";
pub const E027_INTEGER_RANGE: &str = "E027";
pub const E028_BORROW_ACTIVE: &str = "E028";
pub const E029_MOVE_WHILE_BORROWED: &str = "E029";
pub const E030_MANUAL_FREE: &str = "E030";
pub const E031_ARRAY: &str = "E031";
pub const E032_ENUM: &str = "E032";
pub const E033_CAST: &str = "E033";
pub const E034_FFI: &str = "E034";
pub const E035_LIFETIME: &str = "E035";
pub const E036_SEND_SYNC: &str = "E036";
pub const E037_PARALLEL: &str = "E037";
pub const E038_CONST_EVAL: &str = "E038";

pub const W001_EXTENDED_TIER: &str = "W001";
pub const W002_UNUSED_IMPORT: &str = "W002";
pub const W003_UNUSED_VARIABLE: &str = "W003";

/// Parser: anonymous `{ key: value }` object literal.
pub const P001_ANON_OBJECT_LITERAL: &str = "P001";
/// Parser: standalone `{ }` used as expression.
pub const P002_STANDALONE_BLOCK_EXPR: &str = "P002";
/// Parser: missing or invalid parameter name in fn header.
pub const P003_EXPECTED_PARAM_NAME: &str = "P003";
/// Parser: missing `)` after parameter list.
pub const P004_EXPECTED_CLOSE_PAREN_PARAMS: &str = "P004";
/// Parser: missing `(` after function name.
pub const P005_EXPECTED_OPEN_PAREN_FN: &str = "P005";
/// Parser: invalid / unexpected expression (often cascade).
pub const P006_INVALID_EXPRESSION: &str = "P006";
/// Parser: `{` where an expression was expected (often cascade).
pub const P007_UNEXPECTED_LBRACE_EXPR: &str = "P007";
/// Parser: missing `}` to close block.
pub const P008_EXPECTED_CLOSE_BRACE: &str = "P008";
/// Parser: missing `{` to start block.
pub const P009_EXPECTED_OPEN_BRACE: &str = "P009";
/// Parser: item out of place at top level (often cascade).
pub const P010_EXPECTED_TOP_LEVEL_ITEM: &str = "P010";
/// Parser: missing `)` after call / argument list.
pub const P011_EXPECTED_CLOSE_PAREN_ARGS: &str = "P011";
/// Parser: missing `=>` after arrow function parameters.
pub const P012_EXPECTED_ARROW_FAT_ARROW: &str = "P012";
/// Parser: missing `)` (generic).
pub const P013_EXPECTED_CLOSE_PAREN: &str = "P013";
/// Parser: missing `]` after array / index.
pub const P014_EXPECTED_CLOSE_BRACKET: &str = "P014";
/// Parser: prefer `max` over deprecated `max_threads` / `cores` in `parallel(...)`.
pub const P015_PARALLEL_PREFER_THREADS: &str = "P015";
/// Parser: other / unclassified syntax error.
pub const P099_UNEXPECTED: &str = "P099";

/// Lexer: invalid character or token.
pub const L001_INVALID_TOKEN: &str = "L001";
/// Lexer: unclosed string, comment, or character literal.
pub const L002_UNCLOSED: &str = "L002";
/// Lexer: invalid numeric literal.
pub const L003_INVALID_NUMBER: &str = "L003";
/// Lexer: invalid attribute or macro syntax.
pub const L004_INVALID_ATTRIBUTE: &str = "L004";
