use syn;
use quote;

use std::collections::HashMap;
use quote::ToTokens;

use data::*;
use std::rc::Rc;
use std::cell::RefCell;


type ItemTokens = quote::Tokens;
type ImplTokens = quote::Tokens;

/// Generates all mock structs and implementations.
pub fn handle_generate_mocks() -> Vec<(ItemTokens, Vec<ImplTokens>)> {
    get_singleton_mut!(mock_vars of MockVars);

    let mut tokens = Vec::new();
    for &(ref mock_type_name, ref requested_traits) in mock_vars.iter() {
        tokens.push(handle_generate_mock(mock_type_name, requested_traits));
    }
    mock_vars.clear();

    tokens
}

/// Maps type parameters occuring in a generic trait definitions to types.
#[derive(Clone, Debug)]
struct TypeParamMapper {
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

fn extract_parameterized_types_from_trait_use(trait_ty: &syn::Path) -> Vec<syn::Ty> {
    match trait_ty.segments[0].parameters {
        syn::PathParameters::AngleBracketed(ref data) => data.types.clone(),
        _ => panic!("Type parameter extraction only works for angle-bracketed types.")
    }
}

/// Generates a mock implementation for a
///
/// The following elements are generated:
/// * struct definition
/// * mock interface implementation
/// * mocked trait implementations
/// for a mock of a given name, and a set of requested traits.
///
/// All mock types requested by a `new_mock!` invocations are generated with this function.
///
/// # Paramters
/// * `mock_type_name` - The name of the generated mock type
/// * `trait_tys` - The (generic) trait types which are requested for the mock
fn handle_generate_mock(mock_type_name: &syn::Ident, trait_tys: &[syn::Ty]) -> (ItemTokens, Vec<ImplTokens>) {
    get_singleton_mut!(defined_mocks of DefinedMocks);

    let mut trait_infos = Vec::new();
    let mut mappers = Vec::new();
    let mut impl_tokens = Vec::new();
    // generate impls for each requested trait
    for (trait_id, trait_) in trait_tys.iter().enumerate() {
        match trait_ {
            &syn::Ty::Path(_, ref p) => {
                if p.segments.len() != 1 {
                    panic!("All mocked traits are supposed to be given without path by their name only.");
                }

                let trait_name = &p.segments[0].ident;
                // let trait_info = defined_mocks.get_mut(&trait_name)
                //                               .expect("All mocked traits must be defined using 'define_mock!'");
                let trait_info = defined_mocks.get(&trait_name)
                                              .expect("All mocked traits must be defined using 'define_mock!'");

                let mut mapper = TypeParamMapper::new();
                {
                    let generics: &syn::Generics = &trait_info.generics;
                    let instantiated_params = extract_parameterized_types_from_trait_use(p);


                    for (param, instantiated) in generics.ty_params.iter().zip(instantiated_params) {
                        mapper.add_mapping(param.ident.clone(), instantiated);
                    }

                    let trait_implementer = TraitImplementer::for_(mock_type_name, trait_id, trait_, &trait_info, &mapper);
                    impl_tokens.push(trait_implementer.implement());
                }

                mappers.push(mapper);

                trait_infos.push(trait_info);
            },
            _ => { panic!("Expected a Path as trait type to be implemented for '{}' got: {:?}",
                          mock_type_name, trait_);
            }
        }
    }

    let mock_implementer = MockStructImplementer::for_(
        mock_type_name, trait_tys, &trait_infos, &mappers
    );

    let (mock_struct_tokens, mock_impl_tokens) = mock_implementer.implement();
    impl_tokens.push(mock_impl_tokens);
    (mock_struct_tokens, impl_tokens)
}

/// A factory for generating `quote::Tokens` related to `Behaviour`s.
///
/// A `BehaviourFactory` is always bound to a specific requested trait of a mock.
/// It provides methods for instantiating `Behaviour` types for the different methods
/// of the mock's trait and for naming `Behaviour` related fields uniquely within the mock.
struct BehaviourFactory<'a> {
    /// A number identifying a requested trait of a mock from all other requested traits of the mock.
    trait_id: usize,
    /// The type parameter to instantiated type mapping for the requrested trait
    mapper: &'a TypeParamMapper
}

impl<'a> BehaviourFactory<'a> {
    pub fn new(trait_id: usize, mapper: &'a TypeParamMapper) -> BehaviourFactory<'a> {
        BehaviourFactory {
            trait_id: trait_id,
            mapper: mapper
        }
    }

    /// Returns a instantiated `Behaviour`type as `quote::Tokens` for a method of the mocked trait.
    ///
    /// # parameters
    /// * `args` - The arguments of the method
    /// * `return_ty` - The retun type of the method
    /// * `lifetime_to_bind_type` - The name of the lifetime to which the `Behviour` is bound
    pub fn behaviour_ty_for(&self, args: &[syn::FnArg], return_ty: &syn::Ty, lifetime_to_bind_type: &syn::Lifetime) -> (quote::Tokens, quote::Tokens) {
        //TODO bind lifetime
        let arg_tys = args.iter()
                          .filter_map(|arg| match arg {
                              &syn::FnArg::Captured(_, ref ty) => {
                                  Some(self.mapper.instantiate_from_ty(ty))
                              },
                              _ => None
                          }).collect::<Vec<_>>();

        let curried_args = quote!{ (#(#arg_tys),*) };
        let return_ty = self.mapper.instantiate_from_ty(return_ty);
        //quote!{ galvanic_mock_lib::Behaviour<#lifetime_to_bind_type, #curried_args, #return_ty> }
        (quote!{ galvanic_mock_lib::GivenBehaviours<#curried_args, #return_ty> }, quote!{ galvanic_mock_lib::Behaviour<#curried_args, #return_ty> })
    }

    /// Returns a field name for storing 'given' behaviours for a mocked methods.
    ///
    /// The generated name is unique within the mock.
    pub fn behaviour_collection_name(&self, method_name: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("given_behaviours_for_trait{}_{}",
                                 self.trait_id,
                                 method_name
        ))
    }
}


/// Generates mock structs and implementations.
struct MockStructImplementer<'a> {
    /// The name of the mock type
    mock_type_name: &'a syn::Ident,
    /// The traits which shall be implemented for the mock
    requested_trait_types: &'a [syn::Ty],
    /// `TraitInfo` objects for each requested trait. Ordered as `requested_trait_types`.
    trait_infos: &'a [&'a TraitInfo],
    /// `BehaviourFactory`s for each requested trait. Ordered as `requested_trait_types`.
    behaviour_factories: Vec<BehaviourFactory<'a>>
}

impl<'a> MockStructImplementer<'a> {
    /// Create a new mock struct.
    ///
    /// # Paramaters
    /// * `mock_type_name` - The name of the new struct
    /// * `requested_trait_types` - The traits which shall be implemented for the mock
    /// * `trait_infos` - A `TraitInfo` for each requested trait in the same order
    /// * `mappers` - `TypeParamMapper`s for each requested trait in the same order
    pub fn for_(mock_type_name: &'a syn::Ident, requested_trait_types: &'a [syn::Ty], trait_infos: &'a [&'a TraitInfo], mappers: &'a [TypeParamMapper]) -> MockStructImplementer<'a>  {
        MockStructImplementer {
            mock_type_name: mock_type_name,
            requested_trait_types: requested_trait_types,
            trait_infos: trait_infos,
            behaviour_factories: mappers.into_iter().enumerate()
                                        .map(|(idx, m)| BehaviourFactory::new(idx, m))
                                        .collect()
        }
    }

    /// Generate the code implementing the mock.
    pub fn implement(&self) -> (ItemTokens, ImplTokens) {
        self.implement_struct()
    }

    /// Generate the struct definition of the mock and the methods for creating/interacting with the mock.
    fn implement_struct(&self) -> (ItemTokens, ImplTokens) {
        let mock_type_name = &self.mock_type_name;
        let mock_struct_lifetime = syn::Lifetime::new("'a");

        let fields = self.generate_given_behaviours_fields(&mock_struct_lifetime);
        let field_names = &fields.iter().map(|&(ref name, _, _)| name).collect::<Vec<_>>();
        let givenbehaviours_tys = &fields.iter().map(|&(_, ref ty, _)| ty).collect::<Vec<_>>();
        let behaviour_tys = &fields.iter().map(|&(_, _, ref ty)| ty).collect::<Vec<_>>();

        let add_given_behaviours = field_names.iter().map(|ref name|
            syn::Ident::from(format!("add_{}", name))
        ).collect::<Vec<_>>();

        (quote! {
            struct #mock_type_name {
                #(#field_names: #givenbehaviours_tys),*
            }
         }, quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    #mock_type_name {
                        #(#field_names: galvanic_mock_lib::GivenBehaviours::new()),*
                    }
                }

                #(pub fn #add_given_behaviours(&mut self, behaviour: #behaviour_tys) {
                    self.#field_names.add_behaviour(behaviour);
                })*
            }
        })
    }

    fn generate_given_behaviours_fields(&self, lifetime_to_bind_field: &syn::Lifetime) -> Vec<(syn::Ident, quote::Tokens, quote::Tokens)> {
        let mut fields = Vec::new();
        for (behaviour_factory, trait_info) in self.behaviour_factories.iter().zip(self.trait_infos.iter()) {
            for item in &trait_info.items {
                if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
                    match self.generate_given_behaviours_field_for(behaviour_factory, &item.ident, signature, lifetime_to_bind_field) {
                        Some(field_data) => fields.push(field_data),
                        None => {}
                    };
                }
            }
        }
        fields
    }

    fn generate_given_behaviours_field_for(&self,
                                            behaviour_factory: &BehaviourFactory,
                                            method_name: &syn::Ident,
                                            method_signature: &syn::MethodSig,
                                            lifetime_to_bind_field: &syn::Lifetime)
                                            -> Option<(syn::Ident, quote::Tokens, quote::Tokens)> {
        if !method_signature.generics.ty_params.is_empty() {
            // TODO try to handle generic methods; how to deal with monomorphization?
            panic!("Generic methods are not supported yet.")
        }

        if let syn::FunctionRetTy::Ty(ref return_ty) = method_signature.decl.output {
            let behaviours_field = behaviour_factory.behaviour_collection_name(method_name);
            let (givenbehaviours_type, behaviour_type) = behaviour_factory.behaviour_ty_for(
                &method_signature.decl.inputs, return_ty, lifetime_to_bind_field
            );
            Some((behaviours_field, givenbehaviours_type, behaviour_type))

        } else { None }
    }
}


struct TraitImplementer<'a> {
    mock_type_name: &'a syn::Ident,
    requested_trait_type: &'a syn::Ty,
    trait_info: &'a TraitInfo,
    mapper: &'a TypeParamMapper,
    behaviour_factory: BehaviourFactory<'a>
}

impl<'a> TraitImplementer<'a> {
    pub fn for_(mock_type_name: &'a syn::Ident, trait_id: usize, requested_trait_type: &'a syn::Ty, trait_info: &'a TraitInfo, mapper: &'a TypeParamMapper) -> TraitImplementer<'a>  {
        TraitImplementer {
            mock_type_name: mock_type_name,
            requested_trait_type: requested_trait_type,
            trait_info: trait_info,
            mapper: mapper,
            behaviour_factory: BehaviourFactory::new(trait_id, mapper)
        }
    }

    fn implement(&self) -> quote::Tokens {
        let methods: Vec<_> = self.trait_info.items.iter().flat_map(|item|
                                  self.implement_mocked_method(item).into_iter()
                              ).collect();

        let struct_lifetime = syn::Lifetime::new("'a");
        //let lifetimes = vec![struct_lifetime.clone()];
        let lifetimes: Vec<syn::Lifetime> = Vec::new();

        let mock_type_name = self.mock_type_name.clone();
        let trait_ty = self.requested_trait_type.clone();
        // all generic type parameters need to be bound so only lifetimes must be provided
        //TODO add #lifetimes and bind lifetimes into trait_ty, maybe provide lifetime for mock_type_name
        quote! {
            //impl<#(#lifetimes),*> #trait_ty for #mock_type_name<#struct_lifetime> {
            impl<#(#lifetimes),*> #trait_ty for #mock_type_name{
                #(#methods)*
            }
        }
    }

    fn implement_mocked_method(&self, item: &syn::TraitItem) -> Option<quote::Tokens> {
        let mut tokens = quote::Tokens::new();
        if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
            if !signature.generics.ty_params.is_empty() {
                panic!("Generic methods are not supported yet.")
            }

            let func_name = &item.ident;

            // generate fn signature/header
            signature.constness.to_tokens(&mut tokens);
            signature.unsafety.to_tokens(&mut tokens);
            signature.abi.to_tokens(&mut tokens);
            tokens.append("fn");
            func_name.to_tokens(&mut tokens);
            signature.generics.to_tokens(&mut tokens);
            tokens.append("(");

            let args = signature.decl.inputs.iter().map(|arg| match arg {
                &syn::FnArg::Captured(ref pat, ref ty) => {
                    syn::FnArg::Captured(pat.clone(), self.mapper.instantiate_from_ty(ty))
                },
                _ => arg.clone()
            }).collect::<Vec<_>>();

            tokens.append_separated(&args, ",");
            tokens.append(")");
            if let syn::FunctionRetTy::Ty(ref ty) = signature.decl.output {
                tokens.append("->");
                ty.to_tokens(&mut tokens);
            }
            signature.generics.where_clause.to_tokens(&mut tokens);
            tokens.append("{");

            if let syn::FunctionRetTy::Ty(ref return_ty) = signature.decl.output {
                let given_behaviours = self.behaviour_factory
                                           .behaviour_collection_name(func_name);
                let args = signature.decl.inputs.iter()
                              .filter_map(|arg| match arg {
                                  &syn::FnArg::Captured(ref pat, _) => Some(pat.clone()),
                                  _ => None
                              }).collect::<Vec<_>>();

                // generate body
                // tokens.append(&quote! {
                //     let curried_args = (#(#args),*);
                //     let mut maybe_exhausted_idx: Option<usize> = None;
                //     let mut maybe_return_value: Option<#return_ty> = None;
                //     for (idx, behaviour) in self.#given_behaviours.borrow_mut().iter_mut().enumerate() {
                //         maybe_return_value = behaviour.try_match(&curried_args);
                //         if maybe_return_value.is_some()  {
                //             if behaviour.is_exhausted() {
                //                 maybe_exhausted_idx = Some(idx);
                //             }
                //             break;
                //         }
                //     }
                //
                //     if let Some(return_value) = maybe_return_value {
                //         let mut given_behaviours = self.#given_behaviours.borrow_mut();
                //         if let Some(exhausted_idx) = maybe_exhausted_idx {
                //             given_behaviours.remove(exhausted_idx);
                //         }
                //         return return_value;
                //     }
                //     panic!("No 'given' behaviour satisfied for call {}", stringify!(#func_name));
                // }.to_string());
                tokens.append(&quote! {
                    let curried_args = (#(#args),*);
                    self.#given_behaviours.match_behaviour_or_fail(curried_args)
                }.to_string());
            }

            tokens.append("}");
            return Some(tokens);
        }
        None
    }
}
