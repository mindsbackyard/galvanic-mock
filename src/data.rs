use syn;

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex};

use util::Singleton;


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

impl DefinedMocks {
    pub fn singleton() -> Self {
        use::std::mem;
        use std::sync::{Once, ONCE_INIT};
        // Initialize it to a null value
        static mut SINGLETON: *const i8 = 0 as *const i8;
        static ONCE: Once = ONCE_INIT;

        unsafe {
            ONCE.call_once(|| {
                let data = Self::default();
                // Put it in the heap so it can outlive this call
                let ptr: *const Self = mem::transmute(Box::new(data));
                SINGLETON = mem::transmute(ptr);
            });
            let ptr: *const Self = mem::transmute(SINGLETON);
            (*ptr).clone()
        }
    }
}


pub struct PositionedIdentStore<V> {
    idents: HashMap<syn::Ident, BTreeMap<usize, V>>
}

impl<V> Default for PositionedIdentStore<V> {
    fn default() -> Self { PositionedIdentStore { idents: HashMap::new() } }
}

impl<V> PositionedIdentStore<V> {
    pub fn insert(&mut self, ident: syn::Ident, pos: usize, value: V) {
        let mut pos_map = self.idents.entry(ident).or_insert(BTreeMap::new());
        pos_map.insert(pos, value);
    }
}

pub type TraitList = Vec<syn::Ty>;

#[derive(Clone)]
pub struct MockVars {
    pub inner: Arc<Mutex<Vec<(syn::Ident, TraitList)>>>
}

impl Default for MockVars {
    fn default() -> Self { MockVars { inner: Arc::new(Mutex::new(Vec::new())) } }
}

impl MockVars {
    pub fn singleton() -> Self {
        use::std::mem;
        use std::sync::{Once, ONCE_INIT};
        // Initialize it to a null value
        static mut SINGLETON: *const i8 = 0 as *const i8;
        static ONCE: Once = ONCE_INIT;

        unsafe {
            ONCE.call_once(|| {
                let data = Self::default();
                // Put it in the heap so it can outlive this call
                let ptr: *const Self = mem::transmute(Box::new(data));
                SINGLETON = mem::transmute(ptr);
            });
            let ptr: *const Self = mem::transmute(SINGLETON);
            (*ptr).clone()
        }
    }
}
