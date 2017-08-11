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

named!(parse_given_func -> (syn::Ident, BehaviourMatcher, Return, GivenRepeat),
    do_parse!(
        method: call!(syn::parse::ident) >>
        args: alt!( delimited!(punct!("("), separated_list!(punct!(","), syn::parse::expr), punct!(")")) => { |es| BehaviourMatcher::PerArgument(es) }
            | call!(syn::parse::expr) => { |e| BehaviourMatcher::Explicit(e) }
        ) >>
        return_stmt: alt!( preceded!(keyword!("then_return"), syn::parse::expr) => { |e| Return::FromValue(e) }
                         | preceded!(keyword!("then_return_from"), syn::parse::expr) => { |e| Return::FromCall(e) }
                         | preceded!(keyword!("then_return_ref"), syn::parse::expr) => { |e| Return::FromValue(e) }
                         | preceded!(keyword!("then_return_ref_from"), syn::parse::expr) => { |e| Return::FromCall(e) }
                         | keyword!("then_spy_on_object") => { |e| Return::FromSpy }
                         | keyword!("then_panic") => { |e| Return::Panic }
        ) >>
        repeat: alt!( preceded!(keyword!("times"), syn::parse::expr) => { |e| GivenRepeat::Times(e) }
                    | keyword!("always") => { |e| GivenRepeat::Always }
        ) >>
        (method, args, return_stmt, repeat)
    )
);

named!(pub parse_given -> Vec<GivenStatement>,
    do_parse!(
        punct!("<") >> mock_var: call!(syn::parse::ident) >> keyword!("as") >> ufc_trait: call!(syn::parse::path) >> punct!(">") >>
        punct!("::") >>
        func: parse_given_func >>
        (vec![GivenStatement {
            block_id: 0,
            stmt_id: 0,
            mock_var,
            ufc_trait,
            method: func.0,
            matcher: func.1,
            return_stmt: func.2,
            repeat: func.3
        }])
    )
);

named!(pub parse_given_trait_block -> Vec<GivenStatement>,
    do_parse!(
        punct!("<") >> mock_var: call!(syn::parse::ident) >> keyword!("as") >> ufc_trait: call!(syn::parse::path) >> punct!(">") >>
        punct!("::") >> punct!("{") >>
        statements: terminated_list!(punct!(";"), do_parse!(
            func: parse_given_func >>
            (GivenStatement {
                block_id: 0,
                stmt_id: 0,
                mock_var: mock_var.clone(),
                ufc_trait: ufc_trait.clone(),
                method: func.0,
                matcher: func.1,
                return_stmt: func.2,
                repeat: func.3
            })
        )) >> punct!("}") >>
        (statements)
    )
);

named!(pub parse_givens -> (Vec<BindingField>, Vec<GivenStatement>),
    delimited!(tuple!(keyword!("given"), punct!("!"), punct!("{")),
               tuple!(
                   terminated_list!(punct!(";"), parse_bind),
                   map!(terminated_list!(punct!(";"), alt!(parse_given | parse_given_trait_block)),
                        |statements_list: Vec<Vec<GivenStatement>>| statements_list.into_iter().flat_map(|stmts| stmts.into_iter()).collect::<Vec<_>>()
                   )
               ),
               punct!("}")
    )
);


pub fn handle_given(source: &str, absolute_position: usize) -> (String, String) {
    if let IResult::Done(remainder, (binding_fields, given_definitions)) = parse_givens(source) {
        let mut statements = acquire!(GIVEN_STATEMENTS);
        let mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);

        let mut add_statements = Vec::new();
        for (idx, mut stmt) in given_definitions.into_iter().enumerate() {
            stmt.block_id = absolute_position;
            stmt.stmt_id = absolute_position + idx;
            let stmt_id = stmt.stmt_id;

            {
                let mock_var = &stmt.mock_var;
                let ufc_trait = &stmt.ufc_trait;
                let unique_id = mocked_trait_unifier
                                .get_unique_id_for(ufc_trait)
                                .expect(&format!(concat!("The trait `{}` used in the given statement has not been requested for any mock. ",
                                                         "Did you specify all generic and associated types (and in the same order)?"), quote!(#ufc_trait)));

                let add_method = syn::Ident::from(format!("add_given_behaviour_for_trait{}_{}", unique_id, stmt.method));
                let stmt_repr = format!("{}", stmt);
                add_statements.push(match &stmt.repeat {
                    &GivenRepeat::Always => quote!( #mock_var.#add_method(GivenBehaviour::with(#stmt_id, binding.clone(), #stmt_repr)); ),
                    &GivenRepeat::Times(ref expr) => quote!( #mock_var.#add_method(GivenBehaviour::with_times(#expr, #stmt_id, binding.clone(), #stmt_repr)); ),
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
    } else { panic!("Expecting a `given!` definition: <MOCK_VAR_NAME as MOCKED_TRAIT>::METHOD(MATCHER, ...) THEN REPEAT; ..."); }
}


#[cfg(test)]
mod test {
    use galvanic_assert::*;
    use galvanic_assert::matchers::*;
    use galvanic_assert::matchers::variant::*;

    mod parsers {
        use super::*;
        use super::super::*;

        #[test]
        fn should_parse_bind() {
            let field = parse_bind("bind x: i32 = 1 + 2").expect("");

            assert_that!(&field.name, eq(syn::Ident::from("x")));
            assert_that!(&field.ty, is_variant!(syn::Ty::Path));
            assert_that!(&field.initializer.node, is_variant!(syn::ExprKind::Binary));
        }

        #[test]
        fn should_parse_given_return_from_call() {
            let stmt = &parse_given("<mock as MyTrait>::foo() then_return_from || { 2 } always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_with_universal_function_call_syntax() {
            let stmt = &parse_given("<mock as MyTrait>::foo() then_return_from || { 2 } always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.ufc_trait, eq(syn::parse::path("MyTrait").expect("Could not parse expected type")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_spy_on_object() {
            let stmt = &parse_given("<mock as MyTrait>::foo() then_spy_on_object always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromSpy));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_panic() {
            let stmt = &parse_given("<mock as MyTrait>::foo() then_panic always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::Panic));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_times() {
            let stmt = &parse_given("<mock as MyTrait>::foo() then_return 1 times 2").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Times(syn::parse::expr("2").expect(""))));
        }

        #[test]
        fn should_parse_given_always() {
            let stmt = &parse_given("<mock as MyTrait>::foo() then_return 1 always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_args() {
            let stmt = &parse_given("<mock as MyTrait>::foo(2, 4) then_return 1 always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(&args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_matcher() {
            let stmt = &parse_given("<mock as MyTrait>::foo |a,b| true then_return 1 always").expect("")[0];

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(&args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_givens() {
            let (binds, givens) = parse_givens("given! { <mock as MyTrait>::foo() then_return 1 always; }").expect("");

            assert_that!(&binds.len(), eq(0));
            assert_that!(&givens.len(), eq(1));
        }

        #[test]
        fn should_parse_givens_with_block() {
            let (binds, givens) = parse_givens("given! { <mock as MyTrait>::foo() then_return 1 always; <mock as MyTrait>::{ foo() then_return 1 always; foo() then_return 1 always; }; }").expect("");

            assert_that!(&binds.len(), eq(0));
            assert_that!(&givens.len(), eq(3));
        }

        #[test]
        fn should_parse_givens_with_bind() {
            let (binds, givens) = parse_givens("given! { bind x: i32 = 1; bind x: f32 = 2.0; <mock as MyTrait>::foo() then_return 1 always; }").expect("");
            parse_givens("given ! { < x as TestTrait < i32 , f64 >>::func ( eq ( 2 ) , eq ( 2.2 ) ) then_return 12 always ; }").expect("");

            assert_that!(&binds.len(), eq(2));
            assert_that!(&givens.len(), eq(1));
        }
    }
}
