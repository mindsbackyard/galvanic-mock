use syn;
use quote;
use quote::ToTokens;

use std;

use super::type_param_mapper::*;
use super::mock_struct_implementer::*;
use data::*;

pub struct TraitImplementer<'a> {
    mock_type_name: &'a syn::Ident,
    requested_trait_type: &'a syn::Ty,
    trait_info: &'a TraitInfo,
    mapper: &'a TypeParamMapper,
    given_blocks: &'a [GivenBlockInfo]
}

impl<'a> TraitImplementer<'a> {
    pub fn for_(mock_type_name: &'a syn::Ident, trait_id: usize, requested_trait_type: &'a syn::Ty,
                trait_info: &'a TraitInfo, mapper: &'a TypeParamMapper, given_blocks_for_trait: &'a [GivenBlockInfo])
                -> TraitImplementer<'a>  {
        TraitImplementer {
            mock_type_name: mock_type_name,
            requested_trait_type: requested_trait_type,
            trait_info: trait_info,
            mapper: mapper,
            given_blocks: given_blocks_for_trait
        }
    }

    pub fn implement(&self) -> quote::Tokens {
        let methods: Vec<_> = self.trait_info.items.iter().flat_map(|item|
                                  self.implement_mocked_method(item).into_iter()
                              ).collect();

        let struct_lifetime = syn::Lifetime::new("'a");
        //let lifetimes = vec![struct_lifetime.clone()];
        let lifetimes: Vec<syn::Lifetime> = Vec::new();

        let mock_type_name = self.mock_type_name.clone();
        let mut trait_ty = self.requested_trait_type.clone();

        let bindings = TraitImplementer::extract_associated_types(&mut trait_ty);

        let assoc_types = bindings.iter().map(|&syn::TypeBinding{ref ident, ref ty}| quote!(#ident = #ty)).collect::<Vec<_>>();

        // all generic type parameters need to be bound so only lifetimes must be provided
        //TODO add #lifetimes and bind lifetimes into trait_ty, maybe provide lifetime for mock_type_name
        quote! {
            impl<#(#lifetimes),*> #trait_ty for #mock_type_name{
                #(type #assoc_types;)*
                #(#methods)*
            }
        }
    }

    fn extract_associated_types(trait_ty: &mut syn::Ty) -> Vec<syn::TypeBinding> {
        if let &mut syn::Ty::Path(_, ref mut path) = trait_ty {
            let ty = path.segments.last_mut().expect("A type path without segment is not valid.");
            if let &mut syn::PathParameters::AngleBracketed(ref mut params) = &mut ty.parameters {
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

            // rewrite argument patterns to be unit patterns and instantiate generic argument types
            let mut arg_idx = 1;
            let args = signature.decl.inputs.iter().map(|arg| {
                let arg_name = syn::Ident::from(format!("arg{}", arg_idx));
                match arg {
                    &syn::FnArg::Captured(_, ref ty) => {
                        let inst_ty = self.mapper.instantiate_from_ty(ty);
                        arg_idx += 1;
                        quote!(#arg_name: #inst_ty)
                    },
                    &syn::FnArg::Ignored(ref ty) => {
                        let inst_ty = self.mapper.instantiate_from_ty(ty);
                        arg_idx += 1;
                        quote!(#arg_name: #inst_ty)
                    }
                    _ => quote!(#arg)
            }}).collect::<Vec<_>>();

            tokens.append_separated(&args, ",");
            tokens.append(")");
            if let syn::FunctionRetTy::Ty(ref ty) = signature.decl.output {
                tokens.append("->");
                ty.to_tokens(&mut tokens);
            }
            signature.generics.where_clause.to_tokens(&mut tokens);
            tokens.append("{");

            if let syn::FunctionRetTy::Ty(ref return_ty) = signature.decl.output {
                let args = self.generate_argument_names(&signature.decl.inputs);

                for block in self.given_blocks.iter() {
                    tokens.append(self.implement_given_block(block, &args));
                }

                tokens.append(quote!(panic!("No matching given! statement found.");));
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

    fn implement_given_block(&self, block: &GivenBlockInfo, func_args: &[syn::Ident]) -> quote::Tokens {
        let bound_field = MockStructImplementer::bound_field_for(block.block_id);
        let activated_field = MockStructImplementer::given_block_activated_field_for(block.block_id);

        let mut behaviours = Vec::new();
        for stmt in &block.given_statements {
            let behaviour_field = MockStructImplementer::behaviour_field_for(block.block_id, stmt.stmt_id);
            let match_expr = match stmt.matcher {
                BehaviourMatcher::Explicit(ref expr) => {
                    quote!{ (#expr)(#(&#func_args),*) }
                },
                BehaviourMatcher::PerArgument(ref exprs) => {
                    let mut arg_tokens = quote::Tokens::new();
                    arg_tokens.append("(");
                    for idx in 0..func_args.len() {
                        if idx >= 1 {
                            arg_tokens.append("&&");
                        }
                        let expr = exprs.get(idx).unwrap();
                        arg_tokens.append(quote!((#expr)));
                        let arg = func_args.get(idx).unwrap();
                        arg_tokens.append(quote!((&#arg)));
                    }
                    arg_tokens.append(")");
                    arg_tokens
                }
            };

            let return_expr = match &stmt.return_stmt {
                &Return::FromValue(ref expr) => quote!{ return #expr },
                &Return::FromCall(ref expr) => quote!{ return (#expr)(#(&#func_args),*) },
                &Return::FromSpy => panic!("return_from_spy is not implemented yet."),
                &Return::Panic => quote!{ panic!("Don't forget the towel.") }
            };

            let behaviour = match stmt.repeat {
                Repeat::Always => quote! {
                    if #match_expr.into() {
                        #return_expr;
                    }
                },
                Repeat::Times(..) => {
                    let err_msg = "Number of matches for `given` matches has been limited but limit has not been set. This is most likely an error in the library.";
                    quote! {
                        let (num_matches, maybe_match_limit) = self.#behaviour_field.get();
                        let match_limit = maybe_match_limit.expect(#err_msg);
                        if num_matches < match_limit && #match_expr.into() {
                            self.#behaviour_field.set((num_matches+1, bound));
                            #return_expr;
                        }
                    }
                }
            };
            behaviours.push(behaviour);
        }

        let blocked_behaviours = quote! {
            if self.#activated_field.get() {
                let bound = &self.#bound_field;
                #(#behaviours)*
            }
        };

        blocked_behaviours
    }
}
