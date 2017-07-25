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

#[derive(Debug, Clone)]
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


pub struct MockedTraitUnifier {
    next_id: usize,
    trait_to_unique_id: HashMap<syn::Ty, usize>
}

impl MockedTraitUnifier {
    pub fn new() -> Self {
        Self { next_id: 1, trait_to_unique_id: HashMap::new() }
    }

    pub fn register_trait(&mut self, new_trait_ty: syn::Ty) {
        if self.trait_to_unique_id.get(&new_trait_ty).is_none() {
            self.trait_to_unique_id.insert(new_trait_ty, self.next_id);
            self.next_id += 1;
        }
    }

    pub fn get_unique_id_for(&self, trait_ty: &syn::Ty) -> Option<usize> {
        self.trait_to_unique_id.get(trait_ty).cloned()
    }

    pub fn get_traits(&self) -> Vec<syn::Ty> {
        self.trait_to_unique_id.keys().cloned().collect()
    }
}

lazy_static! {
    pub static ref MOCKED_TRAIT_UNIFIER: Mutex<MockedTraitUnifier> = {
        Mutex::new(MockedTraitUnifier::new())
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


#[derive(Debug,Clone)]
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
pub enum GivenRepeat {
    Times(syn::Expr),
    Always
}

#[derive(Debug,Clone)]
pub struct GivenStatement {
    pub block_id: usize,
    pub stmt_id: usize,
    pub mock_var: syn::Ident,
    pub ufc_trait: syn::Ty,
    pub method: MethodName,
    pub matcher: BehaviourMatcher,
    pub return_stmt: Return,
    pub repeat: GivenRepeat,
}

pub type GivenStatements = Vec<GivenStatement>;
lazy_static! {
    pub static ref GIVEN_STATEMENTS: Mutex<GivenStatements> = {
        Mutex::new(Vec::new())
    };
}


#[derive(Debug,PartialEq,Clone)]
pub enum ExpectRepeat {
    Times(syn::Expr),
    AtLeast(syn::Expr),
    AtMost(syn::Expr),
    Between(syn::Expr, syn::Expr),
}

#[derive(Debug,Clone)]
pub struct ExpectStatement {
    pub block_id: usize,
    pub stmt_id: usize,
    pub mock_var: syn::Ident,
    pub ufc_trait: syn::Ty,
    pub method: MethodName,
    pub matcher: BehaviourMatcher,
    pub repeat: ExpectRepeat
}

pub type ExpectStatements = Vec<ExpectStatement>;
lazy_static! {
    pub static ref EXPECT_STATEMENTS: Mutex<ExpectStatements> = {
        Mutex::new(Vec::new())
    };
}
