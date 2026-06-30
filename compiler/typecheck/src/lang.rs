use std::collections::HashSet;

use ast::*;
use ast::expr_span;
use errors::{ErrorKind, NyraError};
use types::{enum_pattern_matches, EnumInfo, EnumVariantInfo, Type};

use crate::{TypeChecker, TypeEnv, VarInfo};

impl TypeChecker {
    pub fn register_builtins(&mut self) {
        fn unit_variants(names: &[&str]) -> Vec<EnumVariantInfo> {
            names
                .iter()
                .map(|n| EnumVariantInfo {
                    name: (*n).into(),
                    fields: vec![],
                })
                .collect()
        }
        let option = unit_variants(&["None", "Some"]);
        let result = unit_variants(&["Ok", "Err"]);
        self.enums.insert("option".into(), EnumInfo { variants: option.clone() });
        self.enums.insert("result".into(), EnumInfo { variants: result });
    }

    pub fn finalize_enum_registry(&mut self, program: &Program) {
        for e in &program.enums {
            if e.type_params.is_empty()
                && (e.name.starts_with("Option__")
                    || e.name.starts_with("Result__")
                    || e.name.starts_with("Option_")
                    || e.name.starts_with("Result_"))
            {
                self.enums.remove("Option");
                self.enums.remove("Result");
            }
        }
    }

    pub fn type_from_ann(&self, ann: &TypeAnnotation) -> Type {
        match ann {
            TypeAnnotation::Integer(k) => Type::Integer(*k),
            TypeAnnotation::F32 => Type::F32,
            TypeAnnotation::F64 => Type::F64,
            TypeAnnotation::Char => Type::Char,
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::String => Type::String,
            TypeAnnotation::VecStr => Type::VecStr,
            TypeAnnotation::Ptr => Type::Ptr,
            TypeAnnotation::RawPtr { inner } => Type::RawPtr {
                inner: Box::new(self.type_from_ann(inner)),
            },
            TypeAnnotation::Void => Type::Void,
            TypeAnnotation::Struct(n) => {
                if self.enums.contains_key(n) {
                    Type::Enum(n.clone())
                } else {
                    Type::Struct(n.clone())
                }
            }
            TypeAnnotation::Enum(n) => Type::Enum(n.clone()),
            TypeAnnotation::Array { elem, len } => Type::Array {
                elem: Box::new(self.type_from_ann(elem)),
                len: *len,
            },
            TypeAnnotation::Tuple(elems) => Type::Tuple {
                elems: elems.iter().map(|e| self.type_from_ann(e)).collect(),
            },
            TypeAnnotation::Ref {
                inner,
                mutable,
                lifetime,
            } => Type::Ref {
                inner: Box::new(self.type_from_ann(inner)),
                mutable: *mutable,
                lifetime: lifetime.clone(),
            },
            TypeAnnotation::Generic(n) if n == "_" => Type::Unknown,
            TypeAnnotation::Generic(n) => Type::Generic(n.clone()),
            TypeAnnotation::Lifetime(lt) => Type::Ref {
                inner: Box::new(Type::Unknown),
                mutable: false,
                lifetime: Some(lt.clone()),
            },
            TypeAnnotation::ForAll { lifetimes, inner } => Type::ForAll {
                lifetimes: lifetimes.clone(),
                inner: Box::new(self.type_from_ann(inner)),
            },
            TypeAnnotation::FnPtr {
                lifetime_params,
                params,
                return_type,
            } => Type::FnPtr {
                lifetime_params: lifetime_params.clone(),
                params: params.iter().map(|p| self.type_from_ann(p)).collect(),
                return_type: return_type
                    .as_ref()
                    .map(|t| Box::new(self.type_from_ann(t))),
            },
            TypeAnnotation::DynTrait { trait_name, .. } => Type::Struct(format!("Dyn_{trait_name}")),
            TypeAnnotation::Applied { .. } => {
                let t = Type::from(ann.clone());
                if let Type::Struct(n) = &t {
                    if self.enums.contains_key(n) {
                        return Type::Enum(n.clone());
                    }
                }
                t
            }
        }
    }

    pub fn check_match(
        &mut self,
        m: &MatchExpr,
        env: &mut TypeEnv,
    ) -> Type {
        let scrutinee_ty = self.check_expr(&m.scrutinee, env);
        let msp = expr_span(&m.scrutinee);
        let mut arm_types = Vec::new();
        let mut covered: HashSet<String> = HashSet::new();
        let mut has_wildcard = false;

        if let Type::Enum(enum_name) = &scrutinee_ty {
            let info = self.enums.get(enum_name).cloned();
            if let Some(info) = info {
                let variant_names = info.variant_names();
                for arm in &m.arms {
                    let mut arm_env = env.clone();
                    match &arm.pattern {
                        MatchPattern::Wildcard => has_wildcard = true,
                        MatchPattern::Variant(v) => {
                            if !variant_names.iter().any(|x| x == v) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    msp.clone(),
                                    format!("Unknown variant '{v}' for enum '{enum_name}'"),
                                ));
                            } else {
                                covered.insert(v.clone());
                            }
                        }
                        MatchPattern::Qualified(en, v) => {
                            if !enum_pattern_matches(en, enum_name) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    msp.clone(),
                                    format!("Pattern enum '{en}' does not match scrutinee '{enum_name}'"),
                                ));
                            } else if !variant_names.iter().any(|x| x == v) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    msp.clone(),
                                    format!("Unknown variant '{v}'"),
                                ));
                            } else {
                                covered.insert(v.clone());
                            }
                        }
                        MatchPattern::QualifiedBind(en, v, payload) => {
                            if !enum_pattern_matches(en, enum_name) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    msp.clone(),
                                    format!("Pattern enum '{en}' does not match scrutinee '{enum_name}'"),
                                ));
                            } else if !variant_names.iter().any(|x| x == v) {
                                self.errors.push(NyraError::new(
                                    ErrorKind::Type,
                                    msp.clone(),
                                    format!("Unknown variant '{v}'"),
                                ));
                            } else {
                                covered.insert(v.clone());
                                let payload_ty = info
                                    .variants
                                    .iter()
                                    .find(|variant| variant.name == *v)
                                    .and_then(|variant| variant.fields.first().cloned());
                                if let Some(pt) = payload_ty {
                                    self.check_match_payload_bindings(
                                        payload, &pt, &msp, &mut arm_env,
                                    );
                                }
                            }
                        }
                        MatchPattern::Literal(_) => {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                msp.clone(),
                                "String literal patterns require a string scrutinee",
                            ));
                        }
                        MatchPattern::Or(_) => {}
                        MatchPattern::Struct(_, _) | MatchPattern::Tuple(_) => {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                msp.clone(),
                                "struct/tuple match patterns are not supported on enum scrutinee",
                            ));
                        }
                    }
                    if let Some(g) = &arm.guard {
                        let gt = self.check_expr(g, &mut arm_env);
                        if gt != Type::Bool && gt != Type::Unknown {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                msp.clone(),
                                "match guard must be bool",
                            ));
                        }
                    }
                    arm_types.push(self.check_block_expr_value(&arm.body, &mut arm_env, &msp));
                }
                if !has_wildcard {
                    for v in &variant_names {
                        if !covered.contains(v) {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                msp.clone(),
                                format!("Non-exhaustive match: missing variant '{v}'"),
                            ));
                        }
                    }
                }
            }
        } else if scrutinee_ty == Type::String {
            for arm in &m.arms {
                let mut arm_env = env.clone();
                match &arm.pattern {
                    MatchPattern::Wildcard => has_wildcard = true,
                    MatchPattern::Literal(_) => {}
                    MatchPattern::Variant(v) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            format!("Unknown variant '{v}' for string match"),
                        ));
                    }
                    MatchPattern::Qualified(en, v) | MatchPattern::QualifiedBind(en, v, _) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            format!("Enum pattern '{en}.{v}' does not match string scrutinee"),
                        ));
                    }
                    MatchPattern::Or(_) => {}
                    MatchPattern::Struct(_, _) | MatchPattern::Tuple(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "struct/tuple patterns do not match string scrutinee",
                        ));
                    }
                }
                if let Some(g) = &arm.guard {
                    let gt = self.check_expr(g, &mut arm_env);
                    if gt != Type::Bool && gt != Type::Unknown {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "match guard must be bool",
                        ));
                    }
                }
                arm_types.push(self.check_block_expr_value(&arm.body, &mut arm_env, &msp));
            }
        } else if let Type::Struct(struct_name) = &scrutinee_ty {
            for arm in &m.arms {
                let mut arm_env = env.clone();
                match &arm.pattern {
                    MatchPattern::Wildcard => has_wildcard = true,
                    MatchPattern::Struct(pat_name, fields) => {
                        if pat_name != struct_name {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                msp.clone(),
                                format!(
                                    "Struct pattern '{pat_name}' does not match scrutinee '{struct_name}'"
                                ),
                            ));
                        } else {
                            has_wildcard = true;
                            if let Some(info) = self.structs.get(struct_name) {
                                for field_pat in fields {
                                    let bind = field_pat
                                        .bind
                                        .as_deref()
                                        .unwrap_or(field_pat.field.as_str());
                                    if bind == "_" {
                                        continue;
                                    }
                                    if let Some(ft) = info.fields.get(&field_pat.field) {
                                        arm_env.variables.insert(
                                            bind.to_string(),
                                            VarInfo {
                                                ty: ft.clone(),
                                                mutable: false,
                                            },
                                        );
                                    } else {
                                        self.errors.push(NyraError::new(
                                            ErrorKind::Type,
                                            msp.clone(),
                                            format!(
                                                "Struct '{struct_name}' has no field '{}'",
                                                field_pat.field
                                            ),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    MatchPattern::Literal(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "String literal patterns require a string scrutinee",
                        ));
                    }
                    MatchPattern::Variant(_)
                    | MatchPattern::Qualified(_, _)
                    | MatchPattern::QualifiedBind(_, _, _)
                    | MatchPattern::Or(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "enum patterns do not match struct scrutinee",
                        ));
                    }
                    MatchPattern::Tuple(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "tuple patterns do not match struct scrutinee",
                        ));
                    }
                }
                if let Some(g) = &arm.guard {
                    let gt = self.check_expr(g, &mut arm_env);
                    if gt != Type::Bool && gt != Type::Unknown {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "match guard must be bool",
                        ));
                    }
                }
                arm_types.push(self.check_block_expr_value(&arm.body, &mut arm_env, &msp));
            }
        } else if let Type::Tuple { elems } = &scrutinee_ty {
            for arm in &m.arms {
                let mut arm_env = env.clone();
                match &arm.pattern {
                    MatchPattern::Wildcard => has_wildcard = true,
                    MatchPattern::Tuple(binds) => {
                        if binds.len() != elems.len() {
                            self.errors.push(NyraError::new(
                                ErrorKind::Type,
                                msp.clone(),
                                format!(
                                    "Tuple pattern length {} does not match scrutinee length {}",
                                    binds.len(),
                                    elems.len()
                                ),
                            ));
                        } else {
                            has_wildcard = true;
                            for (bind_pat, elem_ty) in binds.iter().zip(elems.iter()) {
                                if let MatchPayloadPattern::Bind(name) = bind_pat {
                                    arm_env.variables.insert(
                                        name.clone(),
                                        VarInfo {
                                            ty: elem_ty.clone(),
                                            mutable: false,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    MatchPattern::Literal(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "String literal patterns require a string scrutinee",
                        ));
                    }
                    MatchPattern::Struct(_, _)
                    | MatchPattern::Variant(_)
                    | MatchPattern::Qualified(_, _)
                    | MatchPattern::QualifiedBind(_, _, _)
                    | MatchPattern::Or(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "pattern does not match tuple scrutinee",
                        ));
                    }
                }
                if let Some(g) = &arm.guard {
                    let gt = self.check_expr(g, &mut arm_env);
                    if gt != Type::Bool && gt != Type::Unknown {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "match guard must be bool",
                        ));
                    }
                }
                arm_types.push(self.check_block_expr_value(&arm.body, &mut arm_env, &msp));
            }
        } else {
            for arm in &m.arms {
                let mut arm_env = env.clone();
                match &arm.pattern {
                    MatchPattern::Wildcard => {}
                    MatchPattern::Variant(name) => {
                        arm_env.variables.insert(
                            name.clone(),
                            VarInfo {
                                ty: scrutinee_ty.clone(),
                                mutable: false,
                            },
                        );
                    }
                    MatchPattern::Literal(_) => {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "String literal patterns require a string scrutinee",
                        ));
                    }
                    _ => {}
                }
                if let Some(g) = &arm.guard {
                    let gt = self.check_expr(g, &mut arm_env);
                    if gt != Type::Bool && gt != Type::Unknown {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            msp.clone(),
                            "match guard must be bool",
                        ));
                    }
                }
                arm_types.push(self.check_block_expr_value(&arm.body, &mut arm_env, &msp));
            }
        }

        arm_types.first().cloned().unwrap_or(Type::Unknown)
    }

    fn check_match_payload_bindings(
        &mut self,
        payload: &MatchPayloadPattern,
        payload_ty: &Type,
        msp: &errors::Span,
        arm_env: &mut TypeEnv,
    ) {
        match payload {
            MatchPayloadPattern::Bind(name) => {
                arm_env.variables.insert(
                    name.clone(),
                    VarInfo {
                        ty: payload_ty.clone(),
                        mutable: false,
                    },
                );
            }
            MatchPayloadPattern::Wildcard => {}
            MatchPayloadPattern::Nested(pat) => {
                self.check_nested_payload_pattern(pat, payload_ty, msp, arm_env);
            }
        }
    }

    fn check_nested_payload_pattern(
        &mut self,
        pat: &MatchPattern,
        payload_ty: &Type,
        msp: &errors::Span,
        arm_env: &mut TypeEnv,
    ) {
        let Type::Enum(inner_enum) = payload_ty else {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                msp.clone(),
                "nested enum pattern requires an enum payload",
            ));
            return;
        };
        let Some(info) = self.enums.get(inner_enum).cloned() else {
            return;
        };
        let variant_names = info.variant_names();
        match pat {
            MatchPattern::Qualified(en, v) => {
                if !en.is_empty() && !enum_pattern_matches(en, inner_enum) {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        msp.clone(),
                        format!(
                            "Pattern enum '{en}' does not match payload enum '{inner_enum}'"
                        ),
                    ));
                } else if !variant_names.iter().any(|x| x == v) {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        msp.clone(),
                        format!("Unknown variant '{v}' for enum '{inner_enum}'"),
                    ));
                }
            }
            MatchPattern::QualifiedBind(en, v, inner_payload) => {
                if !en.is_empty() && !enum_pattern_matches(en, inner_enum) {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        msp.clone(),
                        format!(
                            "Pattern enum '{en}' does not match payload enum '{inner_enum}'"
                        ),
                    ));
                } else if !variant_names.iter().any(|x| x == v) {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        msp.clone(),
                        format!("Unknown variant '{v}' for enum '{inner_enum}'"),
                    ));
                } else {
                    let inner_payload_ty = info
                        .variants
                        .iter()
                        .find(|variant| variant.name == *v)
                        .and_then(|variant| variant.fields.first().cloned());
                    if let Some(pt) = inner_payload_ty {
                        self.check_match_payload_bindings(
                            inner_payload, &pt, msp, arm_env,
                        );
                    }
                }
            }
            _ => {
                self.errors.push(NyraError::new(
                    ErrorKind::Type,
                    msp.clone(),
                    "invalid nested match pattern on enum payload",
                ));
            }
        }
    }

    pub fn struct_has_clone(&self, type_name: &str) -> bool {
        let mangled = self.resolve_method_name(type_name, "clone");
        self.env.functions.contains_key(&mangled)
    }
}
