use syn;
use quote;

use super::*;
use data::GivenStatement;

pub fn implement_given_behaviour() -> Vec<quote::Tokens> {
    let behaviour_item = quote! {
        struct GivenBehaviour {
            stmt_id: usize,
            num_matches: std::cell::Cell<usize>,
            expected_matches: Option<usize>,
            bound: std::rc::Rc<std::any::Any>,
            stmt_repr: String
        }
    };

    let behaviour_impl = quote! {
        #[allow(dead_code)]
        impl GivenBehaviour {
            pub fn with(stmt_id: usize, bound: std::rc::Rc<std::any::Any>, stmt_repr: &str) -> Self {
                Self {
                    stmt_id: stmt_id,
                    num_matches: std::cell::Cell::new(0),
                    expected_matches: None,
                    bound: bound,
                    stmt_repr: stmt_repr.to_string()
                }
            }

            pub fn with_times(times: usize, stmt_id: usize, bound: std::rc::Rc<std::any::Any>, stmt_repr: &str) -> Self {
                Self {
                    stmt_id: stmt_id,
                    num_matches: std::cell::Cell::new(0),
                    expected_matches: Some(times),
                    bound: bound,
                    stmt_repr: stmt_repr.to_string()
                }
            }

            pub fn matched(&self) {
                self.num_matches.set(self.num_matches.get() + 1);
            }

            pub fn is_saturated(&self) -> bool {
                match self.expected_matches {
                    Some(limit) => self.num_matches.get() >= limit,
                    None => false
                }
            }

            pub fn describe(&self) -> &str {
                &self.stmt_repr
            }
        }
    };

    vec![behaviour_item, behaviour_impl]
}

pub fn implement_given_behaviour_matcher(statement: &GivenStatement) -> quote::Tokens {
    let return_expr = match &statement.return_stmt {
        &Return::FromValue(ref expr) => quote!{ #expr },
        &Return::FromCall(ref expr) => quote!{ (#expr)(&curried_args) },
        &Return::FromSpy => panic!("return_from_spy is not implemented yet."),
        &Return::Panic => quote!{ panic!("Panic by behaviour. Don't forget the towel.") }
    };

    let match_expr = match statement.matcher {
        BehaviourMatcher::Explicit(ref expr) => quote!{ (#expr)(&curried_args) },
        BehaviourMatcher::PerArgument(ref exprs) => {
            let mut arg_tokens = quote::Tokens::new();
            arg_tokens.append("(");
            for idx in 0..exprs.len() {
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

    let stmt_id = statement.stmt_id;
    let return_value = syn::Ident::from("return_value");
    let behaviour_idx = syn::Ident::from("idx");
    let maybe_remove_idx = syn::Ident::from("maybe_remove_idx");
    let binding_type = binding_name_for(statement.block_id);
    quote! {
        if behaviour.stmt_id == #stmt_id {
            let bound = behaviour.bound.downcast_ref::<#binding_type>()
                                       .expect("galvanic_mock internal error: unable to downcast binding type");
            use std::convert::Into;
            if (#match_expr).into() {
                behaviour.matched();
                #return_value = Some(#return_expr);
                if behaviour.is_saturated() {
                    #maybe_remove_idx = Some(#behaviour_idx);
                }
                break;
            }
        }
    }
}


pub fn implement_expect_behaviour() -> Vec<quote::Tokens> {
    let behaviour_item = quote! {
        struct ExpectBehaviour {
            stmt_id: usize,
            num_matches: std::cell::Cell<usize>,
            expected_min_matches: Option<usize>,
            expected_max_matches: Option<usize>,
            in_order: Option<bool>,
            bound: std::rc::Rc<std::any::Any>,
            stmt_repr: String
        }
    };

    let behaviour_impl = quote! {
        #[allow(dead_code)]
        impl ExpectBehaviour {
            pub fn with_times(times: usize, stmt_id: usize, bound: std::rc::Rc<std::any::Any>, stmt_repr: &str) -> Self {
                Self {
                    stmt_id: stmt_id,
                    num_matches: std::cell::Cell::new(0),
                    expected_min_matches: Some(times),
                    expected_max_matches: Some(times),
                    in_order: None,
                    bound: bound,
                    stmt_repr: stmt_repr.to_string()
                }
            }

            pub fn with_at_least(at_least_times: usize, stmt_id: usize, bound: std::rc::Rc<std::any::Any>, stmt_repr: &str) -> Self {
                Self {
                    stmt_id: stmt_id,
                    num_matches: std::cell::Cell::new(0),
                    expected_min_matches: Some(at_least_times),
                    expected_max_matches: None,
                    in_order: None,
                    bound: bound,
                    stmt_repr: stmt_repr.to_string()
                }
            }

            pub fn with_at_most(at_most_times: usize, stmt_id: usize, bound: std::rc::Rc<std::any::Any>, stmt_repr: &str) -> Self {
                Self {
                    stmt_id: stmt_id,
                    num_matches: std::cell::Cell::new(0),
                    expected_min_matches: None,
                    expected_max_matches: Some(at_most_times),
                    in_order: None,
                    bound: bound,
                    stmt_repr: stmt_repr.to_string()
                }
            }

            pub fn with_between(at_least_times: usize, at_most_times: usize, stmt_id: usize, bound: std::rc::Rc<std::any::Any>, stmt_repr: &str) -> Self {
                Self {
                    stmt_id: stmt_id,
                    num_matches: std::cell::Cell::new(0),
                    expected_min_matches: Some(at_least_times),
                    expected_max_matches: Some(at_most_times),
                    in_order: None,
                    bound: bound,
                    stmt_repr: stmt_repr.to_string()
                }
            }

            pub fn matched(&self) {
                self.num_matches.set(self.num_matches.get() + 1);
            }

            pub fn is_saturated(&self) -> bool {
                self.expected_min_matches.unwrap_or(0) <= self.num_matches.get()
                    && self.num_matches.get() <= self.expected_max_matches.unwrap_or(std::usize::MAX)
            }

            pub fn describe(&self) -> &str {
                &self.stmt_repr
            }
        }
    };

    vec![behaviour_item, behaviour_impl]
}

pub fn implement_expect_behaviour_matcher(statement: &ExpectStatement) -> quote::Tokens {
    let match_expr = match statement.matcher {
        BehaviourMatcher::Explicit(ref expr) => quote!{ (#expr)(&curried_args) },
        BehaviourMatcher::PerArgument(ref exprs) => {
            let mut arg_tokens = quote::Tokens::new();
            arg_tokens.append("(");
            for idx in 0..exprs.len() {
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

    let stmt_id = statement.stmt_id;
    let binding_type = binding_name_for(statement.block_id);
    quote! {
        if behaviour.stmt_id == #stmt_id {
            let bound = behaviour.bound.downcast_ref::<#binding_type>()
                                       .expect("galvanic_mock internal error: unable to downcast binding type");
            use std::convert::Into;
            if (#match_expr).into() {
                behaviour.matched();
                break;
            }
        }
    }
}
