/* Copyright 2017 Christopher Bacher
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
pub mod binding_implementer;
mod type_param_mapper;
mod mock_struct_implementer;
mod trait_implementer;
mod behaviour;

use syn;
use quote;

use ::generate::binding_implementer::*;
use ::generate::behaviour::*;
use ::generate::type_param_mapper::*;
use ::generate::mock_struct_implementer::*;
use ::generate::trait_implementer::*;
use data::*;

/// Generates all mock structs and implementations.
pub fn handle_generate_mocks() -> Vec<quote::Tokens> {
    let mockable_traits = acquire!(MOCKABLE_TRAITS);
    let mut requested_mocks = acquire!(REQUESTED_MOCKS);
    let mut given_statements = acquire!(GIVEN_STATEMENTS);
    let mut expect_statements = acquire!(EXPECT_STATEMENTS);
    let mut bindings = acquire!(BINDINGS);

    let mut tokens = implement_bindings(&bindings);
    tokens.extend(implement_given_behaviour());
    tokens.extend(implement_expect_behaviour());

    for (mock_type_name, requested_mock) in requested_mocks.iter() {
        let inst_traits = requested_mock.traits.iter().map(|trait_ty| create_instantiated_traits(trait_ty, &mockable_traits)).collect::<Vec<_>>();
        tokens.extend(handle_generate_mock(mock_type_name, &requested_mock.attributes, &inst_traits, &given_statements, &expect_statements));
    }

    bindings.clear();
    requested_mocks.clear();
    given_statements.clear();
    expect_statements.clear();

    tokens
}

fn create_instantiated_traits(trait_path: &syn::Path, mockable_traits: &MockableTraits)
                               -> InstantiatedTrait {
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

    InstantiatedTrait {
        trait_ty: trait_path.clone(),
        info: trait_info.clone(),
        mapper: mapper
    }
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
                        requested_traits: &[InstantiatedTrait],
                        given_statements: &GivenStatements,
                        expect_statements: &ExpectStatements
                       ) -> Vec<quote::Tokens> {
    let mock_implementer = MockStructImplementer::for_(mock_type_name, attributes);
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
}
