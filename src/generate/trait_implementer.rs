use syn;
use quote;
use quote::ToTokens;

use std;

use super::InstantiatedTrait;
use super::typed_arguments_for_method_sig;
use super::type_param_mapper::*;
use super::mock_struct_implementer::*;
use data::*;

pub struct TraitImplementer<'a> {
    mock_type_name: &'a syn::Ident,
    instantiated_trait: &'a InstantiatedTrait
}

impl<'a> TraitImplementer<'a> {
    pub fn for_(mock_type_name: &'a syn::Ident, instantiated_trait: &'a InstantiatedTrait)
                -> TraitImplementer<'a>  {
        TraitImplementer {
            mock_type_name: mock_type_name,
            instantiated_trait: instantiated_trait
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

        // all generic type parameters need to be bound so only lifetimes must be provided
        //TODO add #lifetimes and bind lifetimes into trait_ty, maybe provide lifetime for mock_type_name
        quote! {
            impl<#(#lifetime_defs),*> #trait_ty for #mock_type_name{
                #(type #assoc_types;)*
                #(#methods)*
            }
        }
    }

    fn extract_associated_types(trait_ty: &mut syn::Ty, lifetimes: Vec<syn::Lifetime>) -> Vec<syn::TypeBinding> {
        if let &mut syn::Ty::Path(_, ref mut path) = trait_ty {
            let ty = path.segments.last_mut().expect("A type path without segment is not valid.");
            if let &mut syn::PathParameters::AngleBracketed(ref mut params) = &mut ty.parameters {
                params.lifetimes = lifetimes;
                std::mem::replace(&mut params.bindings, Vec::new())
            } else { Vec::new() }
        } else { Vec::new() }
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

            let args = typed_arguments_for_method_sig(signature, &self.instantiated_trait.mapper);
            tokens.append_separated(&args, ",");

            tokens.append(")");
            if let syn::FunctionRetTy::Ty(ref ty) = signature.decl.output {
                tokens.append("->");
                self.instantiated_trait.mapper.instantiate_from_ty(ty).to_tokens(&mut tokens);
            }
            signature.generics.where_clause.to_tokens(&mut tokens);
            tokens.append("{");

            let given_behaviours = self.instantiated_trait.given_behaviour_field_in_mock_for(&func_name);
            if let syn::FunctionRetTy::Ty(ref return_ty) = signature.decl.output {
                let args = self.generate_argument_names(&signature.decl.inputs);

                tokens.append(quote!{
                    let curried_args = (#(#args),*);
                    let mut matched_idx = None;
                    for (idx, behaviour) in self.#given_behaviours.borrow().iter().enumerate() {
                        if behaviour.matches(&curried_args) {
                            matched_idx = Some(idx);
                            break;
                        }
                    }

                    if let Some(idx) = matched_idx {
                        let result = self.#given_behaviours.borrow()[idx].return_value(&curried_args);
                        if self.#given_behaviours.borrow()[idx].is_saturated() {
                            self.#given_behaviours.borrow_mut().remove(idx);
                        }
                        return result;
                    }
                    panic!("No matching given! statement found.");
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

    // fn implement_given_block(&self, block: &GivenBlockInfo, func_args: &[syn::Ident]) -> quote::Tokens {
    //     let bound_field = MockStructImplementer::bound_field_for(block.block_id);
    //     let activated_field = MockStructImplementer::given_block_activated_field_for(block.block_id);
    //
    //     let mut behaviours = Vec::new();
    //     for stmt in &block.given_statements {
    //         let behaviour_field = MockStructImplementer::behaviour_field_for(block.block_id, stmt.stmt_id);
    //         let match_expr = match stmt.matcher {
    //             BehaviourMatcher::Explicit(ref expr) => {
    //                 quote!{ (#expr)(#(&#func_args),*) }
    //             },
    //             BehaviourMatcher::PerArgument(ref exprs) => {
    //                 let mut arg_tokens = quote::Tokens::new();
    //                 arg_tokens.append("(");
    //                 for idx in 0..func_args.len() {
    //                     if idx >= 1 {
    //                         arg_tokens.append("&&");
    //                     }
    //                     let expr = exprs.get(idx).unwrap();
    //                     arg_tokens.append(quote!((#expr)));
    //                     let arg = func_args.get(idx).unwrap();
    //                     arg_tokens.append(quote!((&#arg)));
    //                 }
    //                 arg_tokens.append(")");
    //                 arg_tokens
    //             }
    //         };
    //
    //         let return_expr = match &stmt.return_stmt {
    //             &Return::FromValue(ref expr) => quote!{ return #expr },
    //             &Return::FromCall(ref expr) => quote!{ return (#expr)(#(&#func_args),*) },
    //             &Return::FromSpy => panic!("return_from_spy is not implemented yet."),
    //             &Return::Panic => quote!{ panic!("Don't forget the towel.") }
    //         };
    //
    //         let behaviour = match stmt.repeat {
    //             Repeat::Always => quote! {
    //                 if #match_expr.into() {
    //                     #return_expr;
    //                 }
    //             },
    //             Repeat::Times(..) => {
    //                 let err_msg = "Number of matches for `given` matches has been limited but limit has not been set. This is most likely an error in the library.";
    //                 quote! {
    //                     let (num_matches, maybe_match_limit) = self.#behaviour_field.get();
    //                     let match_limit = maybe_match_limit.expect(#err_msg);
    //                     if num_matches < match_limit && #match_expr.into() {
    //                         self.#behaviour_field.set((num_matches+1, bound));
    //                         #return_expr;
    //                     }
    //                 }
    //             }
    //         };
    //         behaviours.push(behaviour);
    //     }
    //
    //     let blocked_behaviours = quote! {
    //         if self.#activated_field.get() {
    //             let bound = &self.#bound_field;
    //             #(#behaviours)*
    //         }
    //     };
    //
    //     blocked_behaviours
    // }
}
