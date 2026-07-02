//! Scan crate public API with `syn`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use syn::{
    FnArg, ImplItem, Item, ItemFn, ItemImpl, ItemStruct, Pat, PatType, ReturnType, Type,
    Visibility,
};

use crate::types::{MappedFn, TypeMapper};

#[derive(Debug, Clone)]
pub struct BindItem {
    pub owner: Option<String>,
    pub fn_name: String,
    pub mapped: MappedFn,
}

#[derive(Debug, Clone)]
pub struct BindSpec {
    pub crate_name: String,
    pub owned_types: Vec<String>,
    pub items: Vec<BindItem>,
}

pub fn scan_crate_api(
    crate_root: &Path,
    crate_name: &str,
    export_filter: Option<&[String]>,
) -> Result<BindSpec, String> {
    let src_dir = crate_root.join("src");
    let mut source_files = Vec::new();
    collect_rs_files(&src_dir, &mut source_files)?;
    if source_files.is_empty() {
        return Err(format!("no source files under {}", src_dir.display()));
    }

    let mut collector = ApiCollector {
        crate_root: crate_root.to_path_buf(),
        owned_types: Vec::new(),
        items: Vec::new(),
        export_filter: export_filter.map(|s| s.iter().cloned().collect()),
        seen_fns: HashSet::new(),
    };

    for path in &source_files {
        let path_norm = path.to_string_lossy().replace('\\', "/");
        if path_norm.ends_with("/bytes.rs") || path_norm.contains("/bytes/") {
            continue;
        }
        let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let file = syn::parse_file(&src).map_err(|e| format!("parse {}: {e}", path.display()))?;
        for item in &file.items {
            collector.collect_item(item);
        }
    }

    let mapper = TypeMapper::new(crate_name, &collector.owned_types);
    let mut bound = Vec::new();

    for raw in collector.items {
        if !raw.is_public || raw.has_generics || raw.is_async {
            continue;
        }
        if let Some(ref filter) = collector.export_filter {
            let key = if let Some(ref o) = raw.owner {
                format!("{o}::{}", raw.fn_name)
            } else {
                raw.fn_name.clone()
            };
            if !filter.iter().any(|f| {
                f == &raw.fn_name
                    || f == &key
                    || raw
                        .owner
                        .as_ref()
                        .is_some_and(|o| f == &format!("{o}::{}", raw.fn_name))
            }) {
                continue;
            }
        }

        match mapper.map_fn(
            raw.owner.as_deref(),
            &raw.fn_name,
            &raw.inputs,
            &raw.output,
        ) {
            Ok(mapped) => bound.push(BindItem {
                owner: raw.owner,
                fn_name: raw.fn_name,
                mapped,
            }),
            Err(e) => {
                eprintln!("bindgen: skip {}: {e}", raw.fn_name);
            }
        }
    }

    Ok(BindSpec {
        crate_name: crate_name.to_string(),
        owned_types: collector.owned_types,
        items: bound,
    })
}

struct RawFn {
    owner: Option<String>,
    fn_name: String,
    inputs: Vec<(String, Type)>,
    output: Type,
    is_public: bool,
    has_generics: bool,
    is_async: bool,
}

struct ApiCollector {
    crate_root: PathBuf,
    owned_types: Vec<String>,
    items: Vec<RawFn>,
    export_filter: Option<HashSet<String>>,
    seen_fns: HashSet<String>,
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

impl ApiCollector {
    fn collect_item(&mut self, item: &Item) {
        match item {
            Item::Struct(ItemStruct { ident, vis, .. }) if is_pub(vis) => {
                let name = ident.to_string();
                if !self.owned_types.contains(&name) {
                    self.owned_types.push(name);
                }
            }
            Item::Fn(f) => self.collect_free_fn(f),
            Item::Impl(imp) => self.collect_impl(imp),
            Item::Mod(m) => {
                if let Some((_, items)) = &m.content {
                    for i in items {
                        self.collect_item(i);
                    }
                } else if is_pub(&m.vis) {
                    let path = self.crate_root.join("src").join(format!("{}.rs", m.ident));
                    if path.is_file() {
                        if let Ok(src) = std::fs::read_to_string(&path) {
                            if let Ok(file) = syn::parse_file(&src) {
                                for i in &file.items {
                                    self.collect_item(i);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_free_fn(&mut self, f: &ItemFn) {
        if !is_pub(&f.vis) || !f.sig.generics.params.is_empty() {
            return;
        }
        if let Some(raw) =
            fn_to_raw(None, &f.sig.ident.to_string(), &f.sig, f.sig.asyncness.is_some())
        {
            self.items.push(raw);
        }
    }

    fn collect_impl(&mut self, imp: &ItemImpl) {
        if imp.trait_.is_some() || !imp.generics.params.is_empty() {
            return;
        }
        let Type::Path(ref tp) = *imp.self_ty else {
            return;
        };
        let owner = tp
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if !self.owned_types.contains(&owner) {
            self.owned_types.push(owner.clone());
        }

        for item in &imp.items {
            if let ImplItem::Fn(f) = item {
                if !is_pub(&f.vis) || !f.sig.generics.params.is_empty() {
                    continue;
                }
                if let Some(raw) = fn_to_raw(
                    Some(owner.clone()),
                    &f.sig.ident.to_string(),
                    &f.sig,
                    f.sig.asyncness.is_some(),
                ) {
                    let key = format!("{owner}::{}", f.sig.ident);
                    if self.seen_fns.insert(key) {
                        self.items.push(raw);
                    }
                }
            }
        }
    }
}

fn is_pub(v: &Visibility) -> bool {
    matches!(v, Visibility::Public(_))
}

fn fn_to_raw(
    owner: Option<String>,
    name: &str,
    sig: &syn::Signature,
    is_async: bool,
) -> Option<RawFn> {
    let mut inputs = Vec::new();
    for arg in &sig.inputs {
        match arg {
            FnArg::Receiver(recv) => {
                let ty = if recv.mutability.is_some() {
                    syn::parse_str("&mut Self").ok()?
                } else {
                    syn::parse_str("&Self").ok()?
                };
                inputs.push(("self".into(), ty));
            }
            FnArg::Typed(PatType { pat, ty, .. }) => {
                inputs.push((pat_to_name(pat), ty.as_ref().clone()));
            }
        }
    }
    let output = match &sig.output {
        ReturnType::Default => syn::parse_str("()").ok()?,
        ReturnType::Type(_, ty) => ty.as_ref().clone(),
    };
    Some(RawFn {
        owner,
        fn_name: name.to_string(),
        inputs,
        output,
        is_public: true,
        has_generics: false,
        is_async,
    })
}

fn pat_to_name(pat: &Pat) -> String {
    match pat {
        Pat::Ident(i) => i.ident.to_string(),
        _ => String::new(),
    }
}
