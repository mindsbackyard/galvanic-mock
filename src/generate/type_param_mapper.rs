use syn;
use quote;

/// Maps type parameters occuring in a generic trait definitions to types.
#[derive(Clone, Debug)]
pub struct TypeParamMapper {
    type_param_and_type: Vec<(String, String)>
}

impl TypeParamMapper {
    pub fn new() -> TypeParamMapper {
        TypeParamMapper { type_param_and_type: Vec::new() }
    }

    pub fn add_mapping(&mut self, param: syn::Ident, ty: syn::Ty) {
        self.type_param_and_type.push((param.to_string(), quote!{ #ty }.to_string()));
    }

    /// Creates an instantiated/full type from `quote::Tokens` representing a generic type.
    ///
    /// The stored type parameter mappings a used to replace the type parameters
    /// in the generic type, e.g., `A -> i32` instantiates `MyFoo<A>` to `MyFoo<int>`.
    /// Note that the type parameters may not occur on the first level,
    /// e.g., `MyFoo<Vec<A>>` maps to `MyFoo<Vec<A>>`.
    ///
    /// Note that this uses a heuristic. The algorithm greedily replaces all
    /// whitespace separated occurances of a known type parameter with
    /// the associated type.
    ///
    /// # Panics
    /// Panics is the instantiated type cannot be parsed.`
    pub fn instantiate_from_ty_token(&self, generic_ty_tokens: &quote::Tokens) -> syn::Ty {
        syn::parse::ty(
            &generic_ty_tokens.to_string().split_whitespace()
                .map(|x| {
                    for &(ref param, ref ty) in &self.type_param_and_type {
                        if x == param { return ty.to_string(); }
                    }
                    x.to_string()
                }).collect::<String>()
        ).expect(&format!("Unable to instantiate generic type {:?} with: {:?}",
                          generic_ty_tokens, self.type_param_and_type
        ))
    }

    /// Creates an instantiated/full type from a generic type.
    pub fn instantiate_from_ty(&self, generic_ty: &syn::Ty) -> syn::Ty {
        self.instantiate_from_ty_token(&quote!{ #generic_ty })
    }
}
