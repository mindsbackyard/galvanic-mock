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

named!(pub parse_given -> GivenStatement,
    do_parse!(
        punct!("<") >> mock_var: call!(syn::parse::ident) >> keyword!("as") >> ufc_trait: call!(syn::parse::ty) >> punct!(">") >>
        punct!("::") >> method: call!(syn::parse::ident) >>
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
        repeat: alt!( keyword!("once") => { |_| GivenRepeat::Times(syn::parse::expr("1").expect("")) }
                    | preceded!(keyword!("times"), syn::parse::expr) => { |e| GivenRepeat::Times(e) }
                    | keyword!("always") => { |e| GivenRepeat::Always }
        ) >>
        (GivenStatement {
            block_id: 0,
            stmt_id: 0,
            mock_var,
            ufc_trait,
            method,
            matcher: args,
            return_stmt,
            repeat
        })
    )
);

named!(pub parse_givens -> (Vec<BindingField>, Vec<GivenStatement>),
    delimited!(tuple!(keyword!("given"), punct!("!"), punct!("{")),
               tuple!(
                   terminated_list!(punct!(";"), parse_bind),
                   terminated_list!(punct!(";"), parse_given)
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
            stmt.stmt_id = idx;

            {
                let mock_var = &stmt.mock_var;
                let unique_id = mocked_trait_unifier.get_unique_id_for(&stmt.ufc_trait).expect("");
                let add_method = syn::Ident::from(format!("add_given_behaviour_for_trait{}_{}", unique_id, stmt.method));
                let stmt_repr = format!("{}", stmt);
                add_statements.push(match &stmt.repeat {
                    &GivenRepeat::Always => quote!( #mock_var.#add_method(GivenBehaviour::with(#idx, binding.clone(), #stmt_repr)); ),
                    &GivenRepeat::Times(ref expr) => quote!( #mock_var.#add_method(GivenBehaviour::with_times(#expr, #idx, binding.clone(), #stmt_repr)); ),
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
    } else { panic!("Expecting a `given!` defintion: MOCK_VAR_NAME.METHOD(ARGS) THEN REPEAT; ..."); }
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
            let stmt = parse_given("<mock as MyTrait>::foo() then_return_from || { 2 } always").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_with_universal_function_call_syntax() {
            let stmt = parse_given("<mock as MyTrait>::foo() then_return_from || { 2 } always").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.ufc_trait, eq(syn::parse::ty("MyTrait").expect("Could not parse expected type")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_spy_on_object() {
            let stmt = parse_given("<mock as MyTrait>::foo() then_spy_on_object always").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromSpy));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_panic() {
            let stmt = parse_given("<mock as MyTrait>::foo() then_panic always").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::Panic));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_once() {
            let stmt = parse_given("<mock as MyTrait>::foo() then_return 1 once").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Times(syn::parse::expr("1").expect(""))));
        }

        #[test]
        fn should_parse_given_times() {
            let stmt = parse_given("<mock as MyTrait>::foo() then_return 1 times 2").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Times(syn::parse::expr("2").expect(""))));
        }

        #[test]
        fn should_parse_given_always() {
            let stmt = parse_given("<mock as MyTrait>::foo() then_return 1 always").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_args() {
            let stmt = parse_given("<mock as MyTrait>::foo(2, 4) then_return 1 always").expect("");

            assert_that!(&stmt.mock_var, eq(syn::Ident::from("mock")));
            assert_that!(&stmt.method, eq(syn::Ident::from("foo")));
            // assert_that!(&args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
            assert_that!(&stmt.return_stmt, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&stmt.repeat, eq(GivenRepeat::Always));
        }

        #[test]
        fn should_parse_given_matcher() {
            let stmt = parse_given("<mock as MyTrait>::foo |a,b| true then_return 1 always").expect("");

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
        fn should_parse_givens_with_bind() {
            let (binds, givens) = parse_givens("given! { bind x: i32 = 1; bind x: f32 = 2.0; <mock as MyTrait>::foo() then_return 1 always; }").expect("");
            parse_givens("given ! { < x as TestTrait < i32 , f64 >>::func ( eq ( 2 ) , eq ( 2.2 ) ) then_return 12 always ; }").expect("");

            assert_that!(&binds.len(), eq(2));
            assert_that!(&givens.len(), eq(1));
        }
    }
}
