//! Inject `Serialize` / `Deserialize` traits and synthesize impls for auto-serde structs.

use std::collections::{HashMap, HashSet};

use ast::*;
use errors::Span;

fn self_param() -> Param {
    Param {
        name: "self".into(),
        ty: TypeAnnotation::Generic("Self".into()),
        destructure: vec![],
        no_escape: false,
        mutable: false,
    }
}

fn serialize_trait_def() -> TraitDef {
    TraitDef {
        name: "Serialize".into(),
        methods: vec![
            TraitMethodSig {
                name: "to_json".into(),
                params: vec![self_param()],
                return_type: Some(TypeAnnotation::String),
            },
            TraitMethodSig {
                name: "to_bytes".into(),
                params: vec![self_param()],
                return_type: Some(TypeAnnotation::Ptr),
            },
        ],
    }
}

fn deserialize_trait_def() -> TraitDef {
    TraitDef {
        name: "Deserialize".into(),
        methods: vec![TraitMethodSig {
            name: "from_json".into(),
            params: vec![Param {
                name: "json".into(),
                ty: TypeAnnotation::String,
                destructure: vec![],
                no_escape: false,
                mutable: false,
            }],
            return_type: Some(TypeAnnotation::Generic("Self".into())),
        }],
    }
}

pub fn ensure_serde_trait_defs(program: &mut Program) {
    if !program.traits.iter().any(|t| t.name == "Serialize") {
        program.traits.push(serialize_trait_def());
    }
    if !program.traits.iter().any(|t| t.name == "Deserialize") {
        program.traits.push(deserialize_trait_def());
    }
}

fn synthesize_serialize_to_json(type_name: &str) -> Function {
    let span = Span::default();
    let self_ty = TypeAnnotation::Struct(type_name.to_string());
    Function {
        name: format!("Serialize_{type_name}_to_json"),
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "self".into(),
            ty: self_ty.clone(),
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::String),
        body: Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(Expression::Call(CallExpr {
                    callee: format!("{type_name}_json_encode"),
                    type_args: vec![],
                    args: vec![Expression::Variable {
                        name: "self".into(),
                        span: span.clone(),
                    }],
                    span: span.clone(),
                })),
            })],
        },
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

fn synthesize_serialize_to_bytes(type_name: &str, has_bin: bool) -> Function {
    let span = Span::default();
    let self_ty = TypeAnnotation::Struct(type_name.to_string());
    let body = if has_bin {
        Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(Expression::Call(CallExpr {
                    callee: format!("{type_name}_bin_encode"),
                    type_args: vec![],
                    args: vec![Expression::Variable {
                        name: "self".into(),
                        span: span.clone(),
                    }],
                    span: span.clone(),
                })),
            })],
        }
    } else {
        Block {
            statements: vec![
                Statement::Let(LetStmt {
                    name: "json".into(),
                    mutable: false,
                    destructure: vec![],
                    span: span.clone(),
                    ty: None,
                    value: Expression::Call(CallExpr {
                        callee: format!("{type_name}_json_encode"),
                        type_args: vec![],
                        args: vec![Expression::Variable {
                            name: "self".into(),
                            span: span.clone(),
                        }],
                        span: span.clone(),
                    }),
                }),
                Statement::Let(LetStmt {
                    name: "buf".into(),
                    mutable: false,
                    destructure: vec![],
                    span: span.clone(),
                    ty: Some(TypeAnnotation::Ptr),
                    value: Expression::Call(CallExpr {
                        callee: "bin_buf_new".into(),
                        type_args: vec![],
                        args: vec![],
                        span: span.clone(),
                    }),
                }),
                Statement::Expression(Expression::Call(CallExpr {
                    callee: "bin_buf_write_string".into(),
                    type_args: vec![],
                    args: vec![
                        Expression::Variable {
                            name: "buf".into(),
                            span: span.clone(),
                        },
                        Expression::Variable {
                            name: "json".into(),
                            span: span.clone(),
                        },
                    ],
                    span: span.clone(),
                })),
                Statement::Return(ReturnStmt {
                    value: Some(Expression::Call(CallExpr {
                        callee: "bin_buf_finish".into(),
                        type_args: vec![],
                        args: vec![Expression::Variable {
                            name: "buf".into(),
                            span: span.clone(),
                        }],
                        span: span.clone(),
                    })),
                }),
            ],
        }
    };
    Function {
        name: format!("Serialize_{type_name}_to_bytes"),
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "self".into(),
            ty: self_ty,
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Ptr),
        body,
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

fn synthesize_deserialize_from_json(type_name: &str) -> Function {
    let span = Span::default();
    Function {
        name: format!("Deserialize_{type_name}_from_json"),
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "json".into(),
            ty: TypeAnnotation::String,
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Struct(type_name.to_string())),
        body: Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(Expression::Call(CallExpr {
                    callee: format!("{type_name}_json_decode"),
                    type_args: vec![],
                    args: vec![Expression::Variable {
                        name: "json".into(),
                        span: span.clone(),
                    }],
                    span: span.clone(),
                })),
            })],
        },
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

pub fn synthesize_serde_trait_impls(
    program: &mut Program,
    serde_structs: &HashSet<String>,
    bin_structs: &HashSet<String>,
) {
    ensure_serde_trait_defs(program);
    for type_name in serde_structs {
        if !program.trait_impls.iter().any(|ti| {
            ti.trait_name == "Serialize" && ti.type_name == *type_name
        }) {
            let has_bin = bin_structs.contains(type_name);
            program.trait_impls.push(TraitImpl {
                type_name: type_name.clone(),
                trait_name: "Serialize".into(),
                methods: vec![
                    synthesize_serialize_to_json(type_name),
                    synthesize_serialize_to_bytes(type_name, has_bin),
                ],
            });
        }
        if !program.trait_impls.iter().any(|ti| {
            ti.trait_name == "Deserialize" && ti.type_name == *type_name
        }) {
            program.trait_impls.push(TraitImpl {
                type_name: type_name.clone(),
                trait_name: "Deserialize".into(),
                methods: vec![synthesize_deserialize_from_json(type_name)],
            });
        }
    }
}
