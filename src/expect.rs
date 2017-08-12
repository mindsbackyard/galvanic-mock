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
        punct!("<") >> mock_var: call!(syn::parse::ident) >> keyword!("as") >> ufc_trait: call!(syn::parse::path) >> punct!(">") >>
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

        let mut add_statements = Vec::new();
        for (idx, mut stmt) in expect_definitions.into_iter().enumerate() {
            stmt.block_id = absolute_position;
            stmt.stmt_id = absolute_position + idx;
            let stmt_id = stmt.stmt_id;

            {
                let mock_var = &stmt.mock_var;
                let ufc_trait_name = stmt.trait_name();
                let method_name = stmt.method_name();

                let stmt_repr = format!("{}", stmt);
                add_statements.push(match &stmt.repeat {
                    &ExpectRepeat::Times(ref expr) => quote!( #mock_var.add_expect_behaviour(#ufc_trait_name, #method_name, ExpectBehaviour::with_times(#expr, #stmt_id, binding.clone(), #stmt_repr)); ),
                    &ExpectRepeat::AtLeast(ref expr) => quote!( #mock_var.add_expect_behaviour(#ufc_trait_name, #method_name, ExpectBehaviour::with_at_least(#expr, #stmt_id, binding.clone(), #stmt_repr)); ),
                    &ExpectRepeat::AtMost(ref expr) => quote!( #mock_var.add_expect_behaviour(#ufc_trait_name, #method_name, ExpectBehaviour::with_at_most(#expr, #stmt_id, binding.clone(), #stmt_repr)); ),
                    &ExpectRepeat::Between(ref expr_lower, ref expr_upper) => quote!( #mock_var.add_expect_behaviour(#ufc_trait_name, #method_name, ExpectBehaviour::with_between(#expr_lower, #expr_upper, #stmt_id, binding.clone(), #stmt_repr)); ),
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
    } else { panic!("Expecting a `expect_interactions!` definition: <MOCK_VAR_NAME as MOCKED_TRAIT>::METHOD(MATCHER, ...) REPEAT; ..."); }
}
