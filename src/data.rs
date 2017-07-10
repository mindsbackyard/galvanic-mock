use syn;
use quote;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex};

pub type ItemTokens = quote::Tokens;
pub type ImplTokens = quote::Tokens;

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

pub type MockableTraits = HashMap<TypeName, TraitInfo>;
lazy_static! {
    pub static ref MOCKABLE_TRAITS: Mutex<MockableTraits> = {
        Mutex::new(HashMap::new())
    };
}


pub type TraitList = Vec<syn::Ty>;

pub type RequestedTraits = HashMap<TypeName, TraitList>;
lazy_static! {
    pub static ref REQUESTED_TRAITS: Mutex<RequestedTraits> = {
        Mutex::new(HashMap::new())
    };
}

pub type MockVarToType = HashMap<VarName, TypeName>;
lazy_static! {
    pub static ref MOCKVAR_TO_TYPE: Mutex<MockVarToType> = {
        Mutex::new(HashMap::new())
    };
}

#[derive(Clone)]
pub enum BehaviourMatcher {
    Explicit(syn::Expr),
    PerArgument(Vec<syn::Expr>)
}

#[derive(Debug,PartialEq,Clone)]
pub enum Return {
    FromValue(syn::Expr),
    FromCall(syn::Expr),
    FromSpy,
    Panic
}

#[derive(Debug,PartialEq,Clone)]
pub enum Repeat {
    Times(syn::Expr),
    Always
}

#[derive(Clone)]
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

pub type GivenBlocks = HashMap<MockTypeName, Vec<GivenBlockInfo>>;
lazy_static! {
    pub static ref GIVEN_BLOCKS: Mutex<GivenBlocks> = {
        Mutex::new(HashMap::new())
    };
}

pub struct Binding {
    pub block_id: usize,
    pub fields: Vec<BindingField>
}

pub struct BindingField {
    pub name: VarName,
    pub ty: syn::Ty,
    pub initializer: syn::Expr
}

pub type Bindings = Vec<Binding>;
lazy_static! {
    pub static ref BINDINGS: Mutex<Bindings> = {
        Mutex::new(Vec::new())
    };
}
