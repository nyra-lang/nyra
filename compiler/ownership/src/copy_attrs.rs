//! Validate explicit `#[derive(Copy)]` / `struct S Copy { }` against field types.

use ast::Program;
use errors::{NyraError, Span};
use types::Type;

use crate::context::OwnershipCtx;
use crate::diag;

pub fn check_copy_attrs(program: &Program, ctx: &OwnershipCtx, errors: &mut Vec<NyraError>) {
    for s in &program.structs {
        if !s.attrs.copy {
            continue;
        }
        let ty = Type::Struct(s.name.clone());
        if ctx.kind_of(&ty).is_move() {
            errors.push(diag::struct_cannot_be_copy(&s.name, Span::default()));
        }
    }
}
