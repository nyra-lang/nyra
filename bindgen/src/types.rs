//! Ownership-aware Rust type → C ABI / Nyra type mapping.

use syn::{PathArguments, Type, TypePath, TypeReference, TypeSlice};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyraType {
    Void,
    I32,
    I64,
    U32,
    Bool,
    String,
    Ptr,
    Handle { rust_type: String },
}

#[derive(Debug, Clone)]
pub struct MappedParam {
    pub name: String,
    pub rust_type: String,
    pub c_type: String,
    pub nyra_type: NyraType,
    pub is_self_handle: bool,
}

#[derive(Debug, Clone)]
pub struct MappedFn {
    pub symbol: String,
    pub rust_body: String,
    pub params: Vec<MappedParam>,
    pub return_type: NyraType,
    pub c_return: String,
    pub needs_handle_type: Option<String>,
}

pub struct TypeMapper {
    pub crate_name: String,
    pub owned_types: Vec<String>,
}

impl TypeMapper {
    pub fn new(crate_name: &str, owned_types: &[String]) -> Self {
        Self {
            crate_name: crate_name.replace('-', "_"),
            owned_types: owned_types.to_vec(),
        }
    }

    pub fn prefix(&self) -> String {
        self.crate_name.clone()
    }

    pub fn map_fn(
        &self,
        owner: Option<&str>,
        fn_name: &str,
        inputs: &[(String, Type)],
        output: &Type,
    ) -> Result<MappedFn, String> {
        let mut params = Vec::new();
        let mut rust_args = Vec::new();
        let mut c_param_decls = Vec::new();
        let needs_handle = owner.map(String::from);

        for (i, (name, ty)) in inputs.iter().enumerate() {
            if self.is_self_receiver(ty) {
                let handle = owner.ok_or("method receiver without owner type")?.to_string();
                params.push(MappedParam {
                    name: "self_handle".into(),
                    rust_type: handle.clone(),
                    c_type: "*mut std::ffi::c_void".into(),
                    nyra_type: NyraType::Handle {
                        rust_type: handle.clone(),
                    },
                    is_self_handle: true,
                });
                rust_args.push(format!("&*(self_handle as *const {handle})"));
                c_param_decls.push("self_handle: *mut std::ffi::c_void".into());
                continue;
            }

            let mapped = self.map_input(ty)?;
            let arg_name = if name.is_empty() {
                format!("arg{i}")
            } else {
                name.clone()
            };
            rust_args.push(mapped.rust_use.replace("ARG", &arg_name));
            c_param_decls.push(format!("{arg_name}: {}", mapped.c_type));
            params.push(MappedParam {
                name: arg_name,
                rust_type: mapped.rust_type,
                c_type: mapped.c_type,
                nyra_type: mapped.nyra_type,
                is_self_handle: false,
            });
        }

        let is_method = inputs
            .iter()
            .any(|(n, ty)| n == "self" || self.is_self_receiver(ty));

        let (return_type, c_return, _rust_return_expr) = self.map_output(output)?;
        let symbol = if let Some(owner) = owner {
            format!("{}_{owner}_{fn_name}", self.prefix())
        } else {
            format!("{}_{fn_name}", self.prefix())
        };

        let rust_body = self.build_rust_body(owner, fn_name, &rust_args, output, is_method);

        Ok(MappedFn {
            symbol,
            rust_body,
            params,
            return_type,
            c_return,
            needs_handle_type: needs_handle,
        })
    }

    fn build_rust_body(
        &self,
        owner: Option<&str>,
        fn_name: &str,
        rust_args: &[String],
        output: &Type,
        is_method: bool,
    ) -> String {
        let call = if is_method {
            let owner = owner.expect("method without owner");
            let method_args: Vec<String> = rust_args.iter().skip(1).cloned().collect();
            format!(
                "unsafe {{ (&*(self_handle as *const {owner})).{fn_name}({}) }}",
                method_args.join(", ")
            )
        } else if let Some(owner) = owner {
            format!("{owner}::{fn_name}({})", rust_args.join(", "))
        } else {
            let crate_ident = self.crate_name.replace('-', "_");
            format!("{crate_ident}::{fn_name}({})", rust_args.join(", "))
        };

        if self.as_result_ok(output).is_some() {
            if let Some(ok_ty) = self.as_result_ok(output) {
                if self.is_owned_type(ok_ty) {
                    return format!(
                        "match {call} {{ Ok(v) => Box::into_raw(Box::new(v)) as *mut std::ffi::c_void, Err(_) => std::ptr::null_mut(), }}"
                    );
                }
                if self.is_str_like(ok_ty) {
                    return format!(
                        "match {call} {{ Ok(s) => match std::ffi::CString::new(s) {{ Ok(c) => c.into_raw(), Err(_) => std::ptr::null_mut() }}, Err(_) => std::ptr::null_mut(), }}"
                    );
                }
                if self.is_bool(ok_ty) {
                    return format!("match {call} {{ Ok(v) => if v {{ 1 }} else {{ 0 }}, Err(_) => 0 }}");
                }
                if self.is_i32(ok_ty) {
                    return format!("match {call} {{ Ok(v) => v, Err(_) => 0 }}");
                }
            }
            return format!("match {call} {{ Ok(v) => v, Err(_) => Default::default() }}");
        }

        if self.is_str_like(output) {
            return format!(
                "match std::ffi::CString::new({call}) {{ Ok(c) => c.into_raw(), Err(_) => std::ptr::null_mut() }}"
            );
        }

        if self.is_bool(output) {
            return format!("if {call} {{ 1 }} else {{ 0 }}");
        }

        if self.is_owned_type(output) {
            return format!("Box::into_raw(Box::new({call})) as *mut std::ffi::c_void");
        }

        if self.is_unit(output) {
            return format!("{call};");
        }

        call
    }

    fn map_output(&self, ty: &Type) -> Result<(NyraType, String, String), String> {
        if self.is_unit(ty) {
            return Ok((NyraType::Void, "void".into(), "()".into()));
        }
        if let Some(inner) = self.as_result_ok(ty) {
            if self.is_owned_type(inner) {
                return Ok((
                    NyraType::Ptr,
                    "*mut std::ffi::c_void".into(),
                    "handle".into(),
                ));
            }
            if self.is_str_like(inner) {
                return Ok((
                    NyraType::String,
                    "*mut std::ffi::c_char".into(),
                    "s".into(),
                ));
            }
            if self.is_bool(inner) {
                return Ok((NyraType::Bool, "i32".into(), "b".into()));
            }
            if self.is_i32(inner) {
                return Ok((NyraType::I32, "i32".into(), "v".into()));
            }
        }
        if self.is_str_like(ty) {
            return Ok((
                NyraType::String,
                "*mut std::ffi::c_char".into(),
                "s".into(),
            ));
        }
        if self.is_bool(ty) {
            return Ok((NyraType::Bool, "i32".into(), "b".into()));
        }
        if self.is_i32(ty) {
            return Ok((NyraType::I32, "i32".into(), "v".into()));
        }
        if self.is_i64(ty) {
            return Ok((NyraType::I64, "i64".into(), "v".into()));
        }
        if self.is_u32(ty) {
            return Ok((NyraType::U32, "u32".into(), "v".into()));
        }
        if self.is_owned_type(ty) {
            let name = self
                .type_name(ty)
                .ok_or_else(|| format!("unknown owned type: {}", quote_type(ty)))?;
            return Ok((
                NyraType::Handle {
                    rust_type: name,
                },
                "*mut std::ffi::c_void".into(),
                "handle".into(),
            ));
        }
        Err(format!("unsupported return type: {}", quote_type(ty)))
    }

    fn map_input(&self, ty: &Type) -> Result<MappedInput, String> {
        if self.is_str_like(ty) {
            return Ok(MappedInput {
                rust_type: "&str".into(),
                c_type: "*const std::ffi::c_char".into(),
                nyra_type: NyraType::String,
                rust_use: "unsafe { std::ffi::CStr::from_ptr(ARG) }.to_str().unwrap_or(\"\")".into(),
            });
        }
        if self.is_byte_slice(ty) {
            return Ok(MappedInput {
                rust_type: "&[u8]".into(),
                c_type: "*const std::ffi::c_char".into(),
                nyra_type: NyraType::String,
                rust_use: "unsafe { std::ffi::CStr::from_ptr(ARG) }.to_bytes()".into(),
            });
        }
        if self.is_bool(ty) {
            return Ok(MappedInput {
                rust_type: "bool".into(),
                c_type: "i32".into(),
                nyra_type: NyraType::Bool,
                rust_use: "ARG != 0".into(),
            });
        }
        if self.is_i32(ty) {
            return Ok(MappedInput {
                rust_type: "i32".into(),
                c_type: "i32".into(),
                nyra_type: NyraType::I32,
                rust_use: "ARG".into(),
            });
        }
        if self.is_i64(ty) {
            return Ok(MappedInput {
                rust_type: "i64".into(),
                c_type: "i64".into(),
                nyra_type: NyraType::I64,
                rust_use: "ARG".into(),
            });
        }
        if self.is_u32(ty) {
            return Ok(MappedInput {
                rust_type: "u32".into(),
                c_type: "u32".into(),
                nyra_type: NyraType::U32,
                rust_use: "ARG".into(),
            });
        }
        Err(format!("unsupported parameter type: {}", quote_type(ty)))
    }

    fn is_self_receiver(&self, ty: &Type) -> bool {
        if let Type::Reference(TypeReference { elem, .. }) = ty {
            if let Type::Path(p) = &**elem {
                return p.path.segments.last().is_some_and(|s| s.ident == "Self");
            }
        }
        false
    }

    fn is_unit(&self, ty: &Type) -> bool {
        matches!(ty, Type::Tuple(t) if t.elems.is_empty())
    }

    fn is_bool(&self, ty: &Type) -> bool {
        type_last_ident(ty).is_some_and(|i| i == "bool")
    }

    fn is_i32(&self, ty: &Type) -> bool {
        type_last_ident(ty).is_some_and(|i| i == "i32")
    }

    fn is_i64(&self, ty: &Type) -> bool {
        type_last_ident(ty).is_some_and(|i| i == "i64")
    }

    fn is_u32(&self, ty: &Type) -> bool {
        type_last_ident(ty).is_some_and(|i| i == "u32")
    }

    fn is_str_like(&self, ty: &Type) -> bool {
        if let Type::Reference(r) = ty {
            return type_last_ident(&r.elem).is_some_and(|i| i == "str");
        }
        type_last_ident(ty).is_some_and(|i| i == "String")
    }

    fn is_byte_slice(&self, ty: &Type) -> bool {
        if let Type::Reference(r) = ty {
            if let Type::Slice(TypeSlice { elem, .. }) = &*r.elem {
                return type_last_ident(elem).is_some_and(|i| i == "u8");
            }
        }
        false
    }

    fn is_owned_type(&self, ty: &Type) -> bool {
        self.type_name(ty)
            .map(|n| self.owned_types.iter().any(|t| t == &n))
            .unwrap_or(false)
    }

    fn as_result_ok<'a>(&self, ty: &'a Type) -> Option<&'a Type> {
        let Type::Path(TypePath { path, .. }) = ty else {
            return None;
        };
        if path.segments.last()?.ident != "Result" {
            return None;
        }
        let PathArguments::AngleBracketed(args) = &path.segments.last()?.arguments else {
            return None;
        };
        args.args.first().and_then(|a| match a {
            syn::GenericArgument::Type(t) => Some(t),
            _ => None,
        })
    }

    fn type_name(&self, ty: &Type) -> Option<String> {
        let Type::Path(TypePath { path, .. }) = ty else {
            return None;
        };
        path.segments.last().map(|s| s.ident.to_string())
    }
}

struct MappedInput {
    rust_type: String,
    c_type: String,
    nyra_type: NyraType,
    rust_use: String,
}

fn type_last_ident(ty: &Type) -> Option<String> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };
    path.segments.last().map(|s| s.ident.to_string())
}

fn quote_type(ty: &Type) -> String {
    quote::quote!(#ty).to_string()
}

impl NyraType {
    pub fn nyra_ann(&self) -> &'static str {
        match self {
            NyraType::Void => "void",
            NyraType::I32 => "i32",
            NyraType::I64 => "i64",
            NyraType::U32 => "u32",
            NyraType::Bool => "i32",
            NyraType::String => "string",
            NyraType::Ptr | NyraType::Handle { .. } => "ptr",
        }
    }
}
