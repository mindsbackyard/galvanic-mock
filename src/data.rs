use syn;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex};

pub struct TraitInfo {
    pub safety: syn::Unsafety,
    pub generics: syn::Generics,
    pub generic_bounds: Vec<syn::TyParamBound>,
    pub items: Vec<syn::TraitItem>
}

impl TraitInfo {
    pub fn new(safety: syn::Unsafety,
               generics: syn::Generics,
               generic_bounds: Vec<syn::TyParamBound>,
               items: Vec<syn::TraitItem>) -> TraitInfo {
        TraitInfo {
            safety: safety,
            generics: generics,
            generic_bounds: generic_bounds,
            items: items
        }
    }
}

#[derive(Clone)]
pub struct DefinedMocks {
    pub inner: Arc<Mutex<HashMap<syn::Ident, TraitInfo>>>
}

impl Default for DefinedMocks {
    fn default() -> Self { DefinedMocks { inner: Arc::new(Mutex::new(HashMap::new())) } }
}

singleton!(DefinedMocks);


pub type TraitList = Vec<syn::Ty>;

#[derive(Clone)]
pub struct MockVars {
    pub inner: Arc<Mutex<Vec<(syn::Ident, TraitList)>>>
}

impl Default for MockVars {
    fn default() -> Self { MockVars { inner: Arc::new(Mutex::new(Vec::new())) } }
}

singleton!(MockVars);
