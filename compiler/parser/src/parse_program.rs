//! Top-level `parse()` driver and module item dispatch.
use ast::*;
use errors::NyraError;
use lexer::{Token, TokenKind};
use super::recovery::{check, is_at_end, merge_spans, skip_newlines, synchronize};

use super::Parser;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            errors: vec![],
            parsed_enum_names: vec![
                "option".into(),
                "Option".into(),
                "result".into(),
                "Result".into(),
            ],
            parsed_struct_names: vec![],
            pending_struct_attrs: StructAttrs::default(),
            pending_fn_attrs: super::FnAttrs::default(),
        }
    }

    pub fn parse(mut self) -> (Program, Vec<NyraError>) {
        skip_newlines(&self.tokens, &mut self.position);
        let mut comptime = false;
        if let TokenKind::Identifier(ref name) = self.current_kind() {
            if name == "comptime" {
                comptime = true;
                self.advance();
                skip_newlines(&self.tokens, &mut self.position);
            }
        }
        let mut module = None;
        if check(&self.tokens, self.position, &TokenKind::Module) {
            module = self.parse_module_decl();
        }
        let mut imports = Vec::new();
        let mut consts = Vec::new();
        let mut structs = Vec::new();
        let mut unions = Vec::new();
        let mut enums = Vec::new();
        let mut traits = Vec::new();
        let mut trait_impls = Vec::new();
        let mut macros = Vec::new();
        let mut impls = Vec::new();
        let mut externs = Vec::new();
        let mut functions = Vec::new();
        let mut export_instances = Vec::new();

        let mut no_std = false;
        let mut allow_extended = false;
        while !is_at_end(&self.tokens, self.position) {
            let loop_start = self.position;
            skip_newlines(&self.tokens, &mut self.position);
            if is_at_end(&self.tokens, self.position) {
                break;
            }
            if let TokenKind::Identifier(ref name) = self.current_kind() {
                match name.as_str() {
                    "no_std" if !no_std => {
                        no_std = true;
                        self.advance();
                        continue;
                    }
                    "allow_extended" if !allow_extended => {
                        allow_extended = true;
                        self.advance();
                        continue;
                    }
                    "comptime" => {
                        self.parse_error_here(
                            "`comptime` must appear once at the top of the file (before imports and declarations)",
                        );
                        self.advance();
                        continue;
                    }
                    _ => {}
                }
            }
            match self.current_kind().clone() {
                TokenKind::Import => {
                    let import_kw_span = self.current_span();
                    self.advance();
                    if let TokenKind::StringLit(path) = self.current_kind().clone() {
                        let path_span = self.current_span();
                        self.advance();
                        let mut alias = None;
                        if check(&self.tokens, self.position, &TokenKind::As) {
                            self.advance();
                            if let TokenKind::Identifier(a) = self.current_kind().clone() {
                                self.advance();
                                alias = Some(a);
                            } else {
                                self.parse_error_here("Expected alias name after 'as'");
                            }
                        }
                        let span = merge_spans(&import_kw_span, &path_span);
                        imports.push(ImportDecl { path, alias, span });
                    } else {
                        self.parse_error_here(                            "Expected string path after import",
                        );
                    }
                }
                TokenKind::Const => {
                    if let Some(c) = self.parse_const_def() {
                        consts.push(c);
                    }
                }
                TokenKind::Pub | TokenKind::Priv => match self.tokens.get(self.position + 1) {
                    Some(Token { kind: TokenKind::Fn | TokenKind::Test | TokenKind::Async, .. }) => {
                        if let Some(f) = self.parse_function() {
                            functions.push(f);
                        }
                    }
                    Some(Token { kind: TokenKind::Struct, .. }) => {
                        if let Some(s) = self.parse_struct() {
                            self.parsed_struct_names.push(s.name.clone());
                            structs.push(s);
                        }
                    }
                    Some(Token { kind: TokenKind::Union, .. }) => {
                        if let Some(u) = self.parse_union() {
                            self.parsed_struct_names.push(u.name.clone());
                            unions.push(u);
                        }
                    }
                    Some(Token { kind: TokenKind::Enum, .. }) => {
                        if let Some(e) = self.parse_enum() {
                            self.parsed_enum_names.push(e.name.clone());
                            enums.push(e);
                        }
                    }
                    Some(Token { kind: TokenKind::Const, .. }) => {
                        if let Some(c) = self.parse_const_def() {
                            consts.push(c);
                        }
                    }
                    _ => {
                        self.parse_error_here(
                            "Expected `fn`, `struct`, `union`, `enum`, or `const` after `pub`/`priv`",
                        );
                        synchronize(&self.tokens, &mut self.position);
                    }
                },
                TokenKind::AttrDerive(derives) => {
                    for d in derives {
                        match d.as_str() {
                            "Copy" => self.pending_struct_attrs.copy = true,
                            other => self.parse_error_here(                                format!("Unknown derive trait '{other}' (supported: Copy)"),
                            ),
                        }
                    }
                    self.advance();
                }
                TokenKind::AttrNoEscape => {
                    self.parse_error_here(
                        "#[no_escape] is only valid on function parameters",
                    );
                    self.advance();
                }
                TokenKind::AttrInline => {
                    self.pending_fn_attrs.inline = true;
                    self.advance();
                }
                TokenKind::AttrHot => {
                    self.pending_fn_attrs.hot = true;
                    self.advance();
                }
                TokenKind::AttrCold => {
                    self.pending_fn_attrs.cold = true;
                    self.advance();
                }
                TokenKind::AttrComptime => {
                    self.pending_fn_attrs.comptime = true;
                    self.advance();
                }
                TokenKind::Struct => {
                    if let Some(s) = self.parse_struct() {
                        self.parsed_struct_names.push(s.name.clone());
                        structs.push(s);
                    }
                }
                TokenKind::Union => {
                    if let Some(u) = self.parse_union() {
                        self.parsed_struct_names.push(u.name.clone());
                        unions.push(u);
                    }
                }
                TokenKind::Enum => {
                    if let Some(e) = self.parse_enum() {
                        self.parsed_enum_names.push(e.name.clone());
                        enums.push(e);
                    }
                }
                TokenKind::Trait => {
                    if let Some(t) = self.parse_trait() {
                        traits.push(t);
                    }
                }
                TokenKind::Macro => {
                    if let Some(m) = self.parse_macro_def() {
                        macros.push(m);
                    }
                }
                TokenKind::Impl => {
                    if let Some(ti) = self.parse_trait_impl() {
                        for m in &ti.methods {
                            functions.push(m.clone());
                        }
                        trait_impls.push(ti);
                    } else if let Some(i) = self.parse_impl() {
                        for m in &i.methods {
                            functions.push(m.clone());
                        }
                        impls.push(i);
                    }
                }
                TokenKind::Extern => {
                    if let Some(e) = self.parse_extern() {
                        externs.push(e);
                    }
                }
                TokenKind::Export => {
                    if check(&self.tokens, self.position + 1, &TokenKind::Inst) {
                        if let Some(inst) = self.parse_export_instance() {
                            export_instances.push(inst);
                        }
                    } else if let Some(f) = self.parse_function() {
                        functions.push(f);
                    }
                }
                TokenKind::DocComment(_) => {
                    if let Some(s) = self.parse_struct() {
                        self.parsed_struct_names.push(s.name.clone());
                        structs.push(s);
                    } else if let Some(f) = self.parse_function() {
                        functions.push(f);
                    } else {
                        self.parse_error_here(
                            "Doc comment must be followed by `fn` or `struct`",
                        );
                        synchronize(&self.tokens, &mut self.position);
                    }
                }
                TokenKind::Fn | TokenKind::Test | TokenKind::Async => {
                    let before = self.position;
                    if let Some(f) = self.parse_function() {
                        functions.push(f);
                    } else if self.position == before {
                        synchronize(&self.tokens, &mut self.position);
                    }
                }
                TokenKind::Let => {
                    self.parse_error_here(
                        "Top-level `let` is not allowed; use `const` or declare bindings inside a function",
                    );
                    synchronize(&self.tokens, &mut self.position);
                }
                _ => {
                    synchronize(&self.tokens, &mut self.position);
                }
            }
            skip_newlines(&self.tokens, &mut self.position);
            if self.position == loop_start {
                synchronize(&self.tokens, &mut self.position);
            }
        }

        let errors = std::mem::take(&mut self.errors);
        (
            Program {
                module,
                no_std,
                comptime,
                allow_extended,
                imports,
                consts,
                structs,
                unions,
                enums,
                traits,
                trait_impls,
                macros,
                impls,
                externs,
                functions,
                export_instances,
            },
            errors,
        )
    }
}

