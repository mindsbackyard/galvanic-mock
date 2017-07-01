use syn;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex};

pub type TypeName = syn::Ident;
pub type MockTypeName = syn::Ident;
pub type VarName = syn::Ident;
pub type TraitIdx = usize;
pub type MethodName = syn::Ident;
pub type Args = Vec<syn::Expr>;

#[derive(Debug)]
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

    pub fn get_method_by_name(&self, name: &str) -> Option<&syn::TraitItem> {
        for item in &self.items {
            if let syn::TraitItemKind::Method(_, _) = item.node {
                if item.ident == name { return Some(item); }
            }
        }
        None
    }
}

lazy_static! {
    pub static ref MockableTraits: Mutex<HashMap<TypeName, TraitInfo>> = {
        Mutex::new(HashMap::new())
    };
}


pub type TraitList = Vec<syn::Ty>;

lazy_static! {
    pub static ref RequestedTraits: Mutex<HashMap<TypeName, TraitList>> = {
        Mutex::new(HashMap::new())
    };
}

lazy_static! {
    pub static ref MockVarToType: Mutex<HashMap<VarName, TypeName>> = {
        Mutex::new(HashMap::new())
    };
}


pub enum BehaviourMatcher {
    Explicit(syn::Expr),
    PerArgument(Vec<syn::Expr>)
}

#[derive(Debug,PartialEq)]
pub enum Return {
    FromValue(syn::Expr),
    FromCall(syn::Expr),
    FromSpy,
    Panic
}

#[derive(Debug,PartialEq)]
pub enum Repeat {
    Times(syn::Expr),
    Always
}

pub struct GivenStatement {
    pub stmt_id: usize,
    pub maybe_ufc_trait: Option<syn::Ty>,
    pub method: MethodName,
    pub matcher: BehaviourMatcher,
    pub return_stmt: Return,
    pub repeat: Repeat
}

pub struct GivenBlockInfo {
    pub block_id: usize,
    pub given_statements: Vec<GivenStatement>
}

lazy_static! {
    pub static ref GivenBlocks: Mutex<HashMap<MockTypeName, Vec<GivenBlockInfo>>> = {
        Mutex::new(HashMap::new())
    };
}

lazy_static! {
    pub static ref BINDINGS: Mutex<Vec<(usize, Vec<(VarName, syn::Ty, syn::Expr)>)>> = {
        Mutex::new(Vec::new())
    };
}
