//! Built-in I/O (`print`, buffered writes) argument checking.

use ast::*;
use errors::Span;

use super::{TypeChecker, TypeEnv};
use super::diagnostics;
use types::{self, Type};

impl TypeChecker {
    pub(super) fn check_io_arg(
        &mut self,
        arg: &Expression,
        env: &mut TypeEnv,
        sp: Span,
        callee: &str,
    ) {
        match arg {
            Expression::TemplateLiteral(t) => {
                for part in &t.parts {
                    if let TemplatePart::Interpolation(expr) = part {
                        let ty = self.check_expr(expr, env);
                        if !types::is_print_arg(&ty) {
                            diagnostics::print_template_interpolation_invalid(self, &ty, t.span.clone());
                        }
                    }
                }
            }
            other => {
                let ty = self.check_expr(other, env);
                if !types::is_print_arg(&ty) {
                    if callee == "print" {
                        diagnostics::invalid_print_arg(self, &ty, sp);
                    } else {
                        diagnostics::io_arg_not_printable(self, callee, &ty, sp);
                    }
                }
            }
        }
    }

    pub(super) fn check_print_color(&mut self, color: &Expression, env: &mut TypeEnv, sp: Span) {
        match color {
            Expression::Literal(Literal::String(_)) => {}
            Expression::Variable { name, .. } if !env.variables.contains_key(name) => {}
            other => {
                let ty = self.check_expr(other, env);
                if ty != Type::String && ty != Type::Unknown {
                    diagnostics::print_color_must_be_string(self, &ty, sp);
                }
            }
        }
    }
}
