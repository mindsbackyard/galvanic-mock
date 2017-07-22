pub mod binding_implementer;
pub mod type_param_mapper;
pub mod mock_struct_implementer;
pub mod trait_implementer;
mod behaviour;

use syn;
use quote;

use std::collections::HashMap;

use ::generate::binding_implementer::*;
use ::generate::behaviour::*;
use ::generate::type_param_mapper::*;
use ::generate::mock_struct_implementer::*;
use ::generate::trait_implementer::*;
use data::*;
use util::type_name_of;

/// Generates all mock structs and implementations.
pub fn handle_generate_mocks() -> Vec<(ItemTokens, Vec<ImplTokens>)> {
    let mut requested_traits = acquire!(REQUESTED_TRAITS);
    let given_statements = acquire!(GIVEN_STATEMENTS);
    let mockable_traits = acquire!(MOCKABLE_TRAITS);
    let mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);
    let bindings = acquire!(BINDINGS);

    let all_requested_traits = mocked_trait_unifier.get_traits();
    let instantiated_traits = collect_instantiated_traits(&all_requested_traits, &mockable_traits, &mocked_trait_unifier);

    let mut tokens: Vec<(ItemTokens, Vec<ImplTokens>)> = implement_bindings(&bindings).into_iter()
                                                                                      .map(|item| (item, Vec::new()))
                                                                                      .collect::<Vec<_>>();

    for (mock_type_name, requested_traits) in requested_traits.iter() {
        let inst_traits = requested_traits.iter().map(|trait_ty| instantiated_traits.get(trait_ty).expect("")).collect::<Vec<_>>();
        tokens.push(handle_generate_mock(mock_type_name, &inst_traits));
    }

    for inst_trait in instantiated_traits.values() {
        for behaviour_trait in implement_behaviour_traits(inst_trait) {
            tokens.push((behaviour_trait, Vec::new()));
        }
    }

    for statement in given_statements.iter() {
        let instantiated_trait = instantiated_traits.get(&statement.ufc_trait).expect("");
        tokens.push(implement_given_behaviour(statement, instantiated_trait));
    }

    requested_traits.clear();

    tokens
}

fn collect_instantiated_traits(requested_traits: &[syn::Ty], mockable_traits: &MockableTraits, mocked_trait_unifier: &MockedTraitUnifier)
                               -> HashMap<syn::Ty, InstantiatedTrait> {

    let mut instantiated_traits = HashMap::new();

    for trait_ty in requested_traits {
        match trait_ty {
            &syn::Ty::Path(_, ref p) => {
                if p.segments.len() != 1 {
                    panic!("All mocked traits are supposed to be given without path by their name only.");
                }

                let trait_name = &p.segments[0].ident;
                let trait_info = mockable_traits.get(&trait_name)
                                                .expect("All mocked traits must be defined using 'mockable!'");

                let mut mapper = TypeParamMapper::new();
                {
                    let generics: &syn::Generics = &trait_info.generics;
                    let instantiated_params = extract_parameterized_types_from_trait_use(p);

                    for (param, instantiated) in generics.ty_params.iter().zip(instantiated_params) {
                        mapper.add_mapping(param.ident.clone(), instantiated);
                    }
                }

                instantiated_traits.insert(trait_ty.clone(), InstantiatedTrait {
                    unique_id: mocked_trait_unifier.get_unique_id_for(trait_ty).expect(""),
                    trait_ty: trait_ty.clone(),
                    info: trait_info.clone(),
                    mapper: mapper
                });
            },
            _ => panic!("Expected a Path as trait type got: {:?}", trait_ty)
        }
    }

    instantiated_traits
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
fn handle_generate_mock(mock_type_name: &syn::Ident, requested_traits: &[&InstantiatedTrait]) -> (ItemTokens, Vec<ImplTokens>) {
    let mock_implementer = MockStructImplementer::for_(mock_type_name, requested_traits);
    let (mock_item, mock_impl) = mock_implementer.implement();

    let mut impls = requested_traits.into_iter().map(|inst_trait| TraitImplementer::for_(mock_type_name, inst_trait).implement()).collect::<Vec<_>>();
    impls.push(mock_impl);

    (mock_item, impls)
}


fn extract_parameterized_types_from_trait_use(trait_ty: &syn::Path) -> Vec<syn::Ty> {
    match trait_ty.segments[0].parameters {
        syn::PathParameters::AngleBracketed(ref data) => data.types.clone(),
        _ => panic!("Type parameter extraction only works for angle-bracketed types.")
    }
}

#[derive(Clone,Debug)]
pub struct InstantiatedTrait {
    trait_ty: syn::Ty,
    info: TraitInfo,
    mapper: TypeParamMapper,
    unique_id: usize
}

impl InstantiatedTrait {
    pub fn given_behaviour_field_in_mock_for(&self, method_ident: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("given_behaviours_for_trait{}_{}", self.unique_id, method_ident))
    }

    pub fn given_behaviour_add_method_to_mock_for(&self, method_ident: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("add_given_behaviour_for_trait{}_{}", self.unique_id, method_ident))
    }

    pub fn behaviour_type_for(&self, method_ident: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("BehaviourTrait{}_{}", self.unique_id, method_ident))
    }
}

pub fn typed_arguments_for_method_sig(signature: &syn::MethodSig, mapper: &TypeParamMapper) -> Vec<quote::Tokens> {
    let mut arg_idx = 1;
    signature.decl.inputs.iter().map(|arg| {
        let arg_name = syn::Ident::from(format!("arg{}", arg_idx));
        match arg {
            &syn::FnArg::Captured(_, ref ty) => {
                let inst_ty = mapper.instantiate_from_ty(ty);
                arg_idx += 1;
                quote!(#arg_name: #inst_ty)
            },
            &syn::FnArg::Ignored(ref ty) => {
                let inst_ty = mapper.instantiate_from_ty(ty);
                arg_idx += 1;
                quote!(#arg_name: #inst_ty)
            }
            _ => quote!(#arg)
    }}).collect::<Vec<_>>()
}

pub fn argument_types_for_method_sig(signature: &syn::MethodSig, mapper: &TypeParamMapper) -> Vec<syn::Ty> {
    let mut arg_idx = 1;
    signature.decl.inputs.iter().filter_map(|arg| match arg {
        &syn::FnArg::Captured(_, ref ty) => Some(mapper.instantiate_from_ty(ty)),
        &syn::FnArg::Ignored(ref ty) => Some(mapper.instantiate_from_ty(ty)),
        _ => None
    }).collect::<Vec<_>>()
}

pub fn unlifetime(type_: syn::Ty) -> syn::Ty {
    match type_ {
        syn::Ty::Rptr(_, boxed_mut_ty) => {
            let ty = boxed_mut_ty.ty.clone();
            let mutability = boxed_mut_ty.mutability.clone();
            syn::Ty::Rptr(None, Box::new(syn::MutTy { ty: unlifetime(ty), mutability }))
        },
        _ => type_
    }
}
