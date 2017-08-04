use syn;
use syn::parse::*;
use data::*;
use std::collections::{HashMap, HashSet};

use generate::mock_struct_implementer::MockStructImplementer;
use generate::binding_implementer::{binding_name_for, implement_bindings, implement_initialize_binding};

named!(pub parse_bind -> BindingField,
    do_parse!(
        punct!("bind") >> name: call!(syn::parse::ident) >>
        punct!(":") >> ty: call!(syn::parse::ty) >>
        punct!("=") >> initializer: call!(syn::parse::expr) >>
        (BindingField { name, ty, initializer })
    )
);

named!(pub parse_expect_interaction -> ExpectStatement,
    do_parse!(
        punct!("<") >> mock_var: call!(syn::parse::ident) >> keyword!("as") >> ufc_trait: call!(syn::parse::ty) >> punct!(">") >>
        punct!("::") >> method: call!(syn::parse::ident) >>
        args: alt!( delimited!(punct!("("), separated_list!(punct!(","), syn::parse::expr), punct!(")")) => { |es| BehaviourMatcher::PerArgument(es) }
            | call!(syn::parse::expr) => { |e| BehaviourMatcher::Explicit(e) }
        ) >>
        repeat: alt!( preceded!(keyword!("times"), syn::parse::expr) => { |e| ExpectRepeat::Times(e) }
                    | preceded!(keyword!("at_least"), syn::parse::expr) => { |e| ExpectRepeat::AtLeast(e) }
                    | preceded!(keyword!("at_most"), syn::parse::expr) => { |e| ExpectRepeat::AtMost(e) }
                    | preceded!(keyword!("between"), tuple!( call!(syn::parse::expr), preceded!(punct!(","), syn::parse::expr) )) => { |(e1, e2)| ExpectRepeat::Between(e1, e2) }
                    | keyword!("never") => { |e| ExpectRepeat::Times(syn::parse::expr("0").expect("")) }
        ) >>
        (ExpectStatement {
            block_id: 0,
            stmt_id: 0,
            mock_var,
            ufc_trait,
            method,
            matcher: args,
            repeat
        })
    )
);

named!(pub parse_expect_interactions -> (Vec<BindingField>, Vec<ExpectStatement>),
    delimited!(tuple!(keyword!("expect_interactions"), punct!("!"), punct!("{")),
               tuple!(
                   terminated_list!(punct!(";"), parse_bind),
                   terminated_list!(punct!(";"), parse_expect_interaction)
               ),
               punct!("}")
    )
);


pub fn handle_expect_interactions(source: &str, absolute_position: usize) -> (String, String) {
    if let IResult::Done(remainder, (binding_fields, expect_definitions)) = parse_expect_interactions(source) {
        let mut statements = acquire!(EXPECT_STATEMENTS);
        let mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);

        let mut add_statements = Vec::new();
        for (idx, mut stmt) in expect_definitions.into_iter().enumerate() {
            stmt.block_id = absolute_position;
            stmt.stmt_id = idx;

            {
                let mock_var = &stmt.mock_var;
                let unique_id = mocked_trait_unifier.get_unique_id_for(&stmt.ufc_trait).expect("");
                let add_method = syn::Ident::from(format!("add_expect_behaviour_for_trait{}_{}", unique_id, stmt.method));
                let stmt_repr = format!("{}", stmt);
                add_statements.push(match &stmt.repeat {
                    &ExpectRepeat::Times(ref expr) => quote!( #mock_var.#add_method(ExpectBehaviour::with_times(#expr, #idx, binding.clone(), #stmt_repr)); ),
                    &ExpectRepeat::AtLeast(ref expr) => quote!( #mock_var.#add_method(ExpectBehaviour::with_at_least(#expr, #idx, binding.clone(), #stmt_repr)); ),
                    &ExpectRepeat::AtMost(ref expr) => quote!( #mock_var.#add_method(ExpectBehaviour::with_at_most(#expr, #idx, binding.clone(), #stmt_repr)); ),
                    &ExpectRepeat::Between(ref expr_lower, ref expr_upper) => quote!( #mock_var.#add_method(ExpectBehaviour::with_between(#expr_lower, #expr_upper, #idx, binding.clone(), #stmt_repr)); ),
                });
            }
            statements.entry(stmt.ufc_trait.clone())
                      .or_insert_with(|| Vec::new())
                      .push(stmt);
        }

        let binding = Binding {
            block_id: absolute_position,
            fields: binding_fields
        };
        let binding_initialization = implement_initialize_binding(&binding);
        acquire!(BINDINGS).push(binding);

        let given_block = quote! {
            let binding = std::rc::Rc::new(#binding_initialization);
            #(#add_statements)*
        };

        return (given_block.to_string(), remainder.to_owned());
    } else { panic!("Expecting a `expect_interactions!` defintion"); }
}
