//! Conformance test suite entry (CONF-* rule IDs).

mod common;

#[path = "conformance/ownership.rs"]
mod ownership;

#[path = "conformance/borrowck.rs"]
mod borrowck;

#[path = "conformance/ffi.rs"]
mod ffi;

#[path = "conformance/generics.rs"]
mod generics;

#[path = "conformance/adt.rs"]
mod adt;

#[path = "conformance/arc.rs"]
mod arc;

#[path = "conformance/async_v1.rs"]
mod async_v1;

#[path = "conformance/workspace.rs"]
mod workspace;

#[path = "conformance/inference.rs"]
mod inference;

#[path = "conformance/language_gaps.rs"]
mod language_gaps;

#[path = "conformance/stdlib_gaps.rs"]
mod stdlib_gaps;

#[path = "conformance/games_gaps.rs"]
mod games_gaps;

#[path = "conformance/vec_reloc.rs"]
mod vec_reloc;

#[path = "conformance/coercion.rs"]
mod coercion;

#[path = "conformance/diagnostics.rs"]
mod diagnostics;

#[path = "conformance/copy.rs"]
mod copy;

#[path = "conformance/f64.rs"]
mod f64;

#[path = "conformance/f32.rs"]
mod f32;

#[path = "conformance/char.rs"]
mod char;

#[path = "conformance/comments.rs"]
mod comments;

#[path = "conformance/trait.rs"]
mod trait_dispatch;

#[path = "conformance/comptime.rs"]
mod comptime;

#[path = "conformance/trait_bounds.rs"]
mod trait_bounds;

#[path = "conformance/struct_serde.rs"]
mod struct_serde;

#[path = "conformance/windows.rs"]
mod windows;

#[path = "conformance/tls.rs"]
mod tls;
