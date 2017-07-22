use syn;
use quote;

use super::*;
use data::GivenStatement;

pub fn implement_behaviour_traits(instantiated_trait: &InstantiatedTrait) -> Vec<quote::Tokens> {
    let trait_ty = &instantiated_trait.trait_ty;
    let trait_as_str = quote!(#trait_ty).to_string();

    let mut behaviour_traits = Vec::new();
    for item in &instantiated_trait.info.items {
        if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
            let method_ident = &item.ident;
            let behaviour_for_trait_method = instantiated_trait.behaviour_type_for(method_ident);
            let arg_types = &argument_types_for_method_sig(signature, &instantiated_trait.mapper).into_iter().map(|ty| unlifetime(ty)).collect::<Vec<_>>();
            let return_type = match signature.decl.output {
                syn::FunctionRetTy::Ty(ref ty) => unlifetime(instantiated_trait.mapper.instantiate_from_ty(ty)),
                _ => syn::Ty::Tup(Vec::new())
            };

            let method_as_str = quote!(#method_ident).to_string();

            behaviour_traits.push(quote! {
                #[allow(non_camel_case_types)]
                trait #behaviour_for_trait_method {
                    fn matches(&self, curried_args: &(#(#arg_types),*)) -> bool;

                    fn return_value(&self, curried_args: &(#(#arg_types),*)) -> #return_type;

                    fn is_exhausted(&self) -> bool;

                    fn associated_trait(&self) -> &str { #trait_as_str }

                    fn associated_method(&self) -> &str { #method_as_str }
                }
            });
        }
    }
    behaviour_traits
}


pub fn implement_given_behaviour(statement: &GivenStatement, instantiated_trait: &InstantiatedTrait) -> (quote::Tokens, Vec<quote::Tokens>) {
    let behaviour = syn::Ident::from(format!("GivenBehaviour{}", statement.stmt_id));
    let binding_type = binding_name_for(statement.block_id);
    let behaviour_item = quote! {
        #[allow(non_camel_case_types)]
        struct #behaviour {
            num_matches: std::cell::Cell<usize>,
            expected_matches: Option<usize>,
            bound: std::rc::Rc<#binding_type>
        }
    };

    let constructor = match statement.repeat {
        Repeat::Always => quote! {
            pub fn with(bound: std::rc::Rc<#binding_type>) -> Self {
                Self {
                    num_matches: std::cell::Cell::new(0),
                    expected_matches: None,
                    bound: bound
                }
            }
        },
        Repeat::Times(..) => quote! {
            pub fn with_times(times: usize, bound: std::rc::Rc<#binding_type>) -> Self {
                Self {
                    num_matches: std::cell::Cell::new(0),
                    expected_matches: Some(times),
                    bound: bound
                }
            }
        }
    };

    let behaviour_impl = quote! {
        impl #behaviour {
            #constructor
        }
    };

    let mut arg_types = Vec::new();
    let mut return_type = syn::Ty::Tup(Vec::new());
    for item in &instantiated_trait.info.items {
        if item.ident == statement.method {
            if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
                arg_types = argument_types_for_method_sig(signature, &instantiated_trait.mapper).into_iter().map(|ty| unlifetime(ty)).collect::<Vec<_>>();
                return_type = match signature.decl.output {
                    syn::FunctionRetTy::Ty(ref ty) => unlifetime(instantiated_trait.mapper.instantiate_from_ty(ty)),
                    _ => syn::Ty::Tup(Vec::new())
                };
            }
        }
    }

    let behaviour_for_trait_method = instantiated_trait.behaviour_type_for(&statement.method);
    let arg_types = &arg_types;
    let return_expr = match &statement.return_stmt {
        &Return::FromValue(ref expr) => quote!{ return #expr },
        &Return::FromCall(ref expr) => quote!{ return (#expr)(curried_args) },
        &Return::FromSpy => panic!("return_from_spy is not implemented yet."),
        &Return::Panic => quote!{ panic!("Panic by behaviour. Don't forget the towel.") }
    };

    let match_expr = match statement.matcher {
        BehaviourMatcher::Explicit(ref expr) => {
            quote!{ (#expr)(curried_args) }
        },
        BehaviourMatcher::PerArgument(ref exprs) => {
            let mut arg_tokens = quote::Tokens::new();
            arg_tokens.append("(");
            for idx in 0..arg_types.len() {
                if idx >= 1 {
                    arg_tokens.append("&&");
                }
                let expr = exprs.get(idx).unwrap();
                arg_tokens.append(quote!( (#expr) ));
                arg_tokens.append(format!("(&curried_args.{})", idx));
            }
            arg_tokens.append(")");
            arg_tokens
        }
    };

    let behaviour_trait_impl = quote! {
        impl #behaviour_for_trait_method for #behaviour {
            #[allow(unused_variables)]
            fn matches(&self, curried_args: &(#(#arg_types),*)) -> bool {
                let bound = &self.bound;
                use std::convert::Into;
                if (#match_expr).into() {
                    self.num_matches.set(self.num_matches.get() + 1);
                    true
                } else { false }
            }

            #[allow(unused_variables)]
            fn return_value(&self, curried_args: &(#(#arg_types),*)) -> #return_type {
                let bound = &self.bound;
                #return_expr
            }

            fn is_exhausted(&self) -> bool {
                match self.expected_matches {
                    Some(limit) => self.num_matches.get() >= limit,
                    None => false
                }
            }
        }
    };

    (behaviour_item, vec![behaviour_impl, behaviour_trait_impl])
}
