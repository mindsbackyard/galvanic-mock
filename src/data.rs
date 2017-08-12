use syn;
use std::collections::HashMap;
use std::sync::Mutex;

pub type TypeName = syn::Ident;
pub type VarName = syn::Ident;
pub type MethodName = syn::Ident;

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
}

pub type MockableTraits = HashMap<syn::Path, TraitInfo>;
lazy_static! {
    pub static ref MOCKABLE_TRAITS: Mutex<MockableTraits> = {
        Mutex::new(HashMap::new())
    };
}


pub struct RequestedMock {
    pub traits: Vec<syn::Path>,
    pub attributes: Vec<syn::Attribute>
}
lazy_static! {
    pub static ref REQUESTED_MOCKS: Mutex<HashMap<TypeName, RequestedMock>> = {
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
    pub ufc_trait: syn::Path,
    pub method: MethodName,
    pub matcher: BehaviourMatcher,
    pub return_stmt: Return,
    pub repeat: GivenRepeat,
}

impl GivenStatement {
    pub fn trait_name(&self) -> String {
        let ufc_trait = &self.ufc_trait;
        quote!(#ufc_trait).to_string()
    }

    pub fn method_name(&self) -> String {
        let method = &self.method;
        quote!(#method).to_string()
    }
}

impl ::std::fmt::Display for GivenStatement {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let match_expr = match &self.matcher {
            &BehaviourMatcher::Explicit(ref expr) => format!(" {} ", quote!(#expr)),
            &BehaviourMatcher::PerArgument(ref exprs) => format!("({})", exprs.iter().map(|e| quote!(#e).to_string()).collect::<Vec<_>>().join(", "))
        };
        let return_expr = match &self.return_stmt {
            &Return::FromValue(ref expr) => format!("then_return {}", quote!(#expr)),
            &Return::FromCall(ref expr) => format!("then_return_from {}", quote!(#expr)),
            &Return::FromSpy => panic!("Return::FromSpy is not supported yet"),
            &Return::Panic => String::from("then_panic")
        };
        let repeat_expr = match &self.repeat {
            &GivenRepeat::Times(ref expr) => format!("times {}", quote!(#expr)),
            &GivenRepeat::Always => String::from("always")
        };

        let ufc_trait = &self.ufc_trait;
        write!(f, "{}::{}{} {} {}",
               quote!(#ufc_trait),
               self.method,
               match_expr,
               return_expr,
               repeat_expr
        )
    }
}

pub type GivenStatements = HashMap<syn::Path, Vec<GivenStatement>>;
lazy_static! {
    pub static ref GIVEN_STATEMENTS: Mutex<GivenStatements> = {
        Mutex::new(HashMap::new())
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
    pub ufc_trait: syn::Path,
    pub method: MethodName,
    pub matcher: BehaviourMatcher,
    pub repeat: ExpectRepeat
}

impl ExpectStatement {
    pub fn trait_name(&self) -> String {
        let ufc_trait = &self.ufc_trait;
        quote!(#ufc_trait).to_string()
    }

    pub fn method_name(&self) -> String {
        let method = &self.method;
        quote!(#method).to_string()
    }
}

impl ::std::fmt::Display for ExpectStatement {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let match_expr = match &self.matcher {
            &BehaviourMatcher::Explicit(ref expr) => format!(" {} ", quote!(#expr)),
            &BehaviourMatcher::PerArgument(ref exprs) => format!("({})", exprs.iter().map(|e| quote!(#e).to_string()).collect::<Vec<_>>().join(", "))
        };
        let repeat_expr = match &self.repeat {
            &ExpectRepeat::Times(ref expr) => format!("times {}", quote!(#expr)),
            &ExpectRepeat::AtLeast(ref expr) => format!("at_least {}", quote!(#expr)),
            &ExpectRepeat::AtMost(ref expr) => format!("at_most {}", quote!(#expr)),
            &ExpectRepeat::Between(ref lb, ref ub) => format!("at_least {}, {}", quote!(#lb), quote!(#ub)),
        };

        let ufc_trait = &self.ufc_trait;
        write!(f, "{}::{}{} {}",
               quote!(#ufc_trait),
               self.method,
               match_expr,
               repeat_expr
        )
    }
}

pub type ExpectStatements = HashMap<syn::Path, Vec<ExpectStatement>>;
lazy_static! {
    pub static ref EXPECT_STATEMENTS: Mutex<ExpectStatements> = {
        Mutex::new(HashMap::new())
    };
}
