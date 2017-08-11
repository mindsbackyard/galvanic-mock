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
pub fn handle_generate_mocks() -> Vec<quote::Tokens> {
    let mockable_traits = acquire!(MOCKABLE_TRAITS);
    let mut requested_mocks = acquire!(REQUESTED_MOCKS);
    let mut given_statements = acquire!(GIVEN_STATEMENTS);
    let mut expect_statements = acquire!(EXPECT_STATEMENTS);
    let mut bindings = acquire!(BINDINGS);
    let mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);

    let all_requested_traits = mocked_trait_unifier.get_traits();
    let instantiated_traits = collect_instantiated_traits(&all_requested_traits, &mockable_traits, &mocked_trait_unifier);

    let mut tokens = implement_bindings(&bindings);
    tokens.extend(implement_given_behaviour());
    tokens.extend(implement_expect_behaviour());

    for (mock_type_name, requested_mock) in requested_mocks.iter() {
        let inst_traits = requested_mock.traits.iter().map(|trait_ty| instantiated_traits.get(trait_ty).expect("")).collect::<Vec<_>>();
        tokens.extend(handle_generate_mock(mock_type_name, &requested_mock.attributes, &inst_traits, &given_statements, &expect_statements));
    }

    bindings.clear();
    requested_mocks.clear();
    given_statements.clear();
    expect_statements.clear();

    tokens
}

fn collect_instantiated_traits(requested_traits: &[syn::Path], mockable_traits: &MockableTraits, mocked_trait_unifier: &MockedTraitUnifier)
                               -> HashMap<syn::Path, InstantiatedTrait> {

    let mut instantiated_traits = HashMap::new();

    for trait_path in requested_traits {
        // if trait_path.segments.len() != 1 {
        //     panic!("All mocked traits are supposed to be given without path by their name only.");
        // }

        let trait_info = mockable_traits
                         .get(&strip_generics(trait_path.clone()))
                         .expect(&format!("All mocked traits must be defined using 'mockable!': `{}` not found in {}",
                                          quote!(#trait_path).to_string(),
                                          mockable_traits.keys().map(|k| quote!(#k).to_string()).collect::<Vec<_>>().join(", ")));

        let mut mapper = TypeParamMapper::new();
        {
            let generics: &syn::Generics = &trait_info.generics;
            let instantiated_params = extract_parameterized_types_from_trait_use(trait_path);

            for (param, instantiated) in generics.ty_params.iter().zip(instantiated_params) {
                mapper.add_mapping(param.ident.clone(), instantiated);
            }
        }

        instantiated_traits.insert(trait_path.clone(), InstantiatedTrait {
            unique_id: mocked_trait_unifier.get_unique_id_for(trait_path).expect(""),
            trait_ty: trait_path.clone(),
            info: trait_info.clone(),
            mapper: mapper
        });
    }

    instantiated_traits
}

fn strip_generics(mut path_with_generics: syn::Path) -> syn::Path {
    let mut segment = path_with_generics.segments.pop().unwrap();
    segment.parameters = syn::PathParameters::none();
    path_with_generics.segments.push(segment);
    path_with_generics
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
fn handle_generate_mock(mock_type_name: &syn::Ident,
                        attributes: &[syn::Attribute],
                        requested_traits: &[&InstantiatedTrait],
                        given_statements: &GivenStatements,
                        expect_statements: &ExpectStatements
                       ) -> Vec<quote::Tokens> {
    let mock_implementer = MockStructImplementer::for_(mock_type_name, attributes, requested_traits);
    let mut mock = mock_implementer.implement();

    let empty_given = Vec::new();
    let empty_expect = Vec::new();
    for inst_trait in requested_traits {
        let given_statements_for_trait = given_statements.get(&inst_trait.trait_ty)
                                                         .unwrap_or(&empty_given);
        let expect_statements_for_trait = expect_statements.get(&inst_trait.trait_ty)
                                                           .unwrap_or(&empty_expect);
        mock.push(TraitImplementer::for_(mock_type_name,
                                         inst_trait,
                                         given_statements_for_trait,
                                         expect_statements_for_trait
                                    ).implement());
    }

    mock
}

fn extract_parameterized_types_from_trait_use(trait_ty: &syn::Path) -> Vec<syn::Ty> {
    match trait_ty.segments[0].parameters {
        syn::PathParameters::AngleBracketed(ref data) => data.types.clone(),
        _ => panic!("Type parameter extraction only works for angle-bracketed types.")
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


#[derive(Clone,Debug)]
pub struct InstantiatedTrait {
    trait_ty: syn::Path,
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

    pub fn expect_behaviour_field_in_mock_for(&self, method_ident: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("expect_behaviours_for_trait{}_{}", self.unique_id, method_ident))
    }

    pub fn expect_behaviour_add_method_to_mock_for(&self, method_ident: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("add_expect_behaviour_for_trait{}_{}", self.unique_id, method_ident))
    }
}
