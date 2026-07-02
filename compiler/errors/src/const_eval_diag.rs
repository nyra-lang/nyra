use crate::{ErrorKind, NyraError, Span, E038_CONST_EVAL};

/// Build a coded compile-time evaluation diagnostic.
pub fn coded_comptime_error(span: Span, message: impl Into<String>) -> NyraError {
    NyraError::coded(E038_CONST_EVAL, ErrorKind::ConstEval, span, message)
}
