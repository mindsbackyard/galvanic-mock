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
use syn;
use quote;
use quote::ToTokens;

use std;

use super::InstantiatedTrait;
use super::typed_arguments_for_method_sig;
use super::behaviour::*;
use data::*;

pub struct TraitImplementer<'a> {
    mock_type_name: &'a syn::Ident,
    instantiated_trait: &'a InstantiatedTrait,
    given_statements: &'a [GivenStatement],
    expect_statements: &'a [ExpectStatement]
}

impl<'a> TraitImplementer<'a> {
    pub fn for_(mock_type_name: &'a syn::Ident,
                instantiated_trait: &'a InstantiatedTrait,
                given_statements_for_trait: &'a [GivenStatement],
                expect_statements_for_trait: &'a [ExpectStatement]
               ) -> TraitImplementer<'a>  {
        TraitImplementer {
            mock_type_name: mock_type_name,
            instantiated_trait: instantiated_trait,
            given_statements: given_statements_for_trait,
            expect_statements: expect_statements_for_trait
        }
    }

    pub fn implement(&self) -> quote::Tokens {
        let methods: Vec<_> = self.instantiated_trait.info.items.iter().flat_map(|item|
                                  self.implement_mocked_method(item).into_iter()
                              ).collect();

        let lifetime_defs = &self.instantiated_trait.info.generics.lifetimes;
        let lifetimes  = lifetime_defs.into_iter().map(|def| def.lifetime.clone()).collect::<Vec<_>>();

        let mock_type_name = self.mock_type_name.clone();
        let mut trait_ty = self.instantiated_trait.trait_ty.clone();

        let bindings = TraitImplementer::extract_associated_types(&mut trait_ty, lifetimes);
        let assoc_types = bindings.into_iter().map(|syn::TypeBinding{ref ident, ref ty}| quote!(#ident = #ty)).collect::<Vec<_>>();

        quote! {
            impl<#(#lifetime_defs),*> #trait_ty for #mock_type_name{
                #(type #assoc_types;)*
                #(#methods)*
            }
        }
    }

    fn extract_associated_types(trait_ty: &mut syn::Path, lifetimes: Vec<syn::Lifetime>) -> Vec<syn::TypeBinding> {
        let ty = trait_ty.segments.last_mut().expect("A type path without segment is not valid.");
        if let &mut syn::PathParameters::AngleBracketed(ref mut params) = &mut ty.parameters {
            params.lifetimes = lifetimes;
            std::mem::replace(&mut params.bindings, Vec::new())
        } else { Vec::new() }
    }

    fn implement_mocked_method(&self, item: &syn::TraitItem) -> Option<quote::Tokens> {
        let mut tokens = quote::Tokens::new();
        if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
            signature.decl.inputs.iter().find(|arg| match arg {
                &&syn::FnArg::SelfValue(..) => true,
                &&syn::FnArg::SelfRef(..) => true,
                _ => false
            }).expect("Static methods are not supported yet.");

            let func_name = &item.ident;

            // generate fn signature/header
            signature.constness.to_tokens(&mut tokens);
            signature.unsafety.to_tokens(&mut tokens);
            signature.abi.to_tokens(&mut tokens);
            tokens.append("fn");
            func_name.to_tokens(&mut tokens);
            signature.generics.to_tokens(&mut tokens);
            tokens.append("(");

            let args = typed_arguments_for_method_sig(signature, &self.instantiated_trait.mapper);
            tokens.append_separated(&args, ",");

            tokens.append(")");
            if let syn::FunctionRetTy::Ty(ref ty) = signature.decl.output {
                tokens.append("->");
                self.instantiated_trait.mapper.instantiate_from_ty(ty).to_tokens(&mut tokens);
            }
            signature.generics.where_clause.to_tokens(&mut tokens);
            tokens.append("{");

            if let syn::FunctionRetTy::Ty(_) = signature.decl.output {
                let args = self.generate_argument_names(&signature.decl.inputs);

                let given_behaviour_impls = self.given_statements.iter()
                                                .filter(|stmt| stmt.method == item.ident)
                                                .map(|stmt| implement_given_behaviour_matcher(stmt))
                                                .collect::<Vec<_>>();
                let expect_behaviour_impls = self.expect_statements.iter()
                                                .filter(|stmt| stmt.method == item.ident)
                                                .map(|stmt| implement_expect_behaviour_matcher(stmt))
                                                .collect::<Vec<_>>();

                let trait_ty = &self.instantiated_trait.trait_ty;
                let trait_name = quote!(#trait_ty).to_string();
                let method_name = func_name.to_string();

                tokens.append(quote!{
                    let curried_args = (#(#args,)*);
                    for behaviour in self.expect_behaviours.borrow_mut().entry((#trait_name, #method_name)).or_insert_with(|| Vec::new()).iter() {
                        #(
                            #expect_behaviour_impls
                        )*
                    }

                    let mut maybe_remove_idx = None;
                    let mut return_value = None;
                    let mut all_given_behaviours_ref = self.given_behaviours.borrow_mut();
                    let given_behaviours = all_given_behaviours_ref.entry((#trait_name, #method_name)).or_insert_with(|| Vec::new());
                    for (idx, behaviour) in given_behaviours.iter().enumerate() {
                        #(
                            #given_behaviour_impls
                        )*
                    }

                    if let Some(idx) = maybe_remove_idx {
                        if (&given_behaviours[idx] as &GivenBehaviour).is_saturated() {
                            given_behaviours.remove(idx);
                        }
                    }

                    if let Some(value) = return_value {
                        return value;
                    }
                    panic!("No matching given! statement found among the remaining ones: {}",
                        given_behaviours.iter().map(|behaviour| format!("\n\t{}", behaviour.describe())).collect::<String>()
                    )
                });
            }

            tokens.append("}");
            return Some(tokens);
        }
        None
    }

    fn generate_argument_names(&self, func_inputs: &[syn::FnArg]) -> Vec<syn::Ident> {
        let mut arg_names = Vec::new();
        let mut arg_idx = 1;
        for arg in func_inputs {
            match arg {
                &syn::FnArg::Captured(..) | &syn::FnArg::Ignored(..) => {
                    arg_names.push(syn::Ident::from(format!("arg{}", arg_idx)));
                    arg_idx += 1;
                },
                _ => {}
            }
        }
        arg_names
    }
}
