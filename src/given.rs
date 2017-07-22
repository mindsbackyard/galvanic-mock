use syn;
use syn::parse::*;
use data::*;
use std::collections::{HashMap, HashSet};

use generate::mock_struct_implementer::MockStructImplementer;
use generate::binding_implementer::{binding_name_for, implement_bindings, implement_initialize_binding};

named!(pub parse_bind -> (VarName, syn::Ty, syn::Expr),
    tuple!(
        preceded!(punct!("bind"), syn::parse::ident),
        preceded!(punct!(":"), syn::parse::ty),
        preceded!(punct!("="), syn::parse::expr)
    )
);

named!(pub parse_given -> ((VarName, syn::Ty), MethodName, BehaviourMatcher, Return, Repeat),
    tuple!(
        tuple!( preceded!(punct!("<"), syn::parse::ident), delimited!(keyword!("as"), call!(syn::parse::ty), punct!(">")) ),
        preceded!(punct!("::"), call!(syn::parse::ident)),
        alt!( delimited!(punct!("("), separated_list!(punct!(","), syn::parse::expr), punct!(")")) => { |es| BehaviourMatcher::PerArgument(es) }
            | call!(syn::parse::expr) => { |e| BehaviourMatcher::Explicit(e) }
        ),
        alt!( preceded!(keyword!("then_return"), syn::parse::expr) => { |e| Return::FromValue(e) }
            | preceded!(keyword!("then_return_from"), syn::parse::expr) => { |e| Return::FromCall(e) }
            | keyword!("then_spy_on_object") => { |e| Return::FromSpy }
            | keyword!("then_panic") => { |e| Return::Panic }
        ),
        alt!( keyword!("once") => { |_| Repeat::Times(syn::parse::expr("1").expect("")) }
            | preceded!(keyword!("times"), syn::parse::expr) => { |e| Repeat::Times(e) }
            | keyword!("always") => { |e| Repeat::Always }
        )
    )
);

named!(pub parse_givens -> (Vec<(VarName, syn::Ty, syn::Expr)>, Vec<((VarName, syn::Ty), MethodName, BehaviourMatcher, Return, Repeat)>),
    delimited!(tuple!(keyword!("given"), punct!("!"), punct!("{")),
               tuple!(
                   terminated_list!(punct!(";"), parse_bind),
                   terminated_list!(punct!(";"), parse_given)
               ),
               punct!("}")
    )
);


pub fn handle_given(source: &str, absolute_position: usize) -> (String, String) {
    if let IResult::Done(remainder, (bindings, given_definitions)) = parse_givens(source) {
        let mut statements = acquire!(GIVEN_STATEMENTS);
        let mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);

        let mut add_statements = Vec::new();
        for (idx, ((mock_var, mocked_trait_ty), method_ident, args, return_, repeat)) in given_definitions.into_iter().enumerate() {
            let stmt = GivenStatement {
                block_id: absolute_position,
                stmt_id: idx,
                ufc_trait: mocked_trait_ty.clone(),
                method: method_ident.clone(),
                matcher: args,
                return_stmt: return_,
                repeat: repeat.clone()
            };
            statements.push(stmt);

            let unique_id = mocked_trait_unifier.get_unique_id_for(&mocked_trait_ty).expect("");
            let add_method = syn::Ident::from(format!("add_given_behaviour_for_trait{}_{}", unique_id, method_ident));
            let behaviour = syn::Ident::from(format!("GivenBehaviour{}", idx));
            add_statements.push(match repeat {
                Repeat::Always => quote!( #mock_var.#add_method(Box::new(#behaviour::with(binding.clone()))); ),
                Repeat::Times(expr) => quote!( #mock_var.#add_method(Box::new(#behaviour::with_times(#expr, binding.clone()))); ),
            });
        }

        let binding_fields = bindings.into_iter().map(|field| BindingField {
            name: field.0.clone(),
            ty: field.1.clone(),
            initializer: field.2.clone()
        }).collect::<Vec<_>>();

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
            let (var, ty, expr) = parse_bind("bind x: i32 = 1 + 2").expect("");

            assert_that!(&var, eq(syn::Ident::from("x")));
            assert_that!(&ty, is_variant!(syn::Ty::Path));
            assert_that!(&expr.node, is_variant!(syn::ExprKind::Binary));
        }

        #[test]
        fn should_parse_given_return_from_call() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_return_from || { 2 } always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_with_universal_function_call_syntax() {
            let ((var, trait_ty), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_return_from || { 2 } always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&trait_ty, eq(syn::parse::ty("MyTrait").expect("Could not parse expected type")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_spy_on_object() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_spy_on_object always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromSpy));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_panic() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_panic always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::Panic));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_once() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_return 1 once").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Times(syn::parse::expr("1").expect(""))));
        }

        #[test]
        fn should_parse_given_times() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_return 1 times 2").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Times(syn::parse::expr("2").expect(""))));
        }

        #[test]
        fn should_parse_given_always() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo() then_return 1 always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_args() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo(2, 4) then_return 1 always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(&args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_matcher() {
            let ((var, _), method, args, ret, repeat) = parse_given("<mock as MyTrait>::foo |a,b| true then_return 1 always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(&args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
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
