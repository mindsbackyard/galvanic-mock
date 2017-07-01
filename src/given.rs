use syn;
use syn::parse::*;
use data::*;
use std::collections::HashMap;


named!(pub parse_bind -> (VarName, syn::Ty, syn::Expr),
    tuple!(
        preceded!(punct!("bind"), syn::parse::ident),
        preceded!(punct!(":"), syn::parse::ty),
        preceded!(punct!("="), syn::parse::expr)
    )
);

named!(pub parse_given -> ((VarName, Option<syn::Ty>), MethodName, BehaviourMatcher, Return, Repeat),
    tuple!(
        alt!( tuple!( call!(syn::parse::ident), terminated!(value!(None), punct!(".")) )
            | tuple!( preceded!(punct!("<"), syn::parse::ident), delimited!(keyword!("as"), map!(syn::parse::ty, |t| Some(t)), punct!(">::")) )
        ),
        call!(syn::parse::ident),
        alt!( call!(syn::parse::expr) => { |e| BehaviourMatcher::Explicit(e) }
            | delimited!(punct!("("), separated_list!(punct!(","), syn::parse::expr), punct!(")")) => { |es| BehaviourMatcher::PerArgument(es) }
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

named!(pub parse_givens -> (Vec<(VarName, syn::Ty, syn::Expr)>, Vec<((VarName, Option<syn::Ty>), MethodName, BehaviourMatcher, Return, Repeat)>),
    tuple!(
        terminated_list!(punct!(";"), parse_bind),
        terminated_list!(punct!(";"), parse_given)
    )
);


pub fn handle_given(source: &str, absolute_position: usize) -> (String, String) {
    if let IResult::Done(remainder, (bindings, given_definitions)) = parse_givens(source) {
        let block_id = absolute_position;
        let mut given_statements_per_type: HashMap<MockTypeName, Vec<GivenStatement>> = HashMap::new();

        let given_blocks = acquire!(GivenBlocks);
        let var_to_type = acquire!(MockVarToType);
        let mut idx = 0;
        for ((mock_var, maybe_trait), method_ident, args, return_, repeat) in given_definitions {
            let mock_ty_ident = var_to_type.get(&mock_var).expect(
                &format!("No variable `{}` is known to be a mock named.", mock_var.to_string())
            );

            let stmt = GivenStatement {
                stmt_id: idx,
                maybe_ufc_trait: maybe_trait,
                method: method_ident,
                matcher: args,
                return_stmt: return_,
                repeat: repeat
            };
            given_statements_per_type.entry(mock_ty_ident.clone())
                                     .or_insert_with(|| Vec::new())
                                     .push(stmt);
            idx += 1;
        }

        BINDINGS.lock().unwrap().push((block_id, bindings));

        for (mock_ty_ident, given_statements) in given_statements_per_type.into_iter() {
            let info = GivenBlockInfo {
                block_id: block_id,
                given_statements: given_statements
            };
            given_blocks.entry(mock_ty_ident).or_insert_with(|| Vec::new()).push(info);
        }


    } else { panic!("Expecting a given! defintion: MOCK_VAR_NAME.METHOD(ARGS) THEN REPEAT; ..."); }

    panic!("Not implemented yet");
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
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo() then_return_from || { 2 } always").expect("");

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
            assert_that!(&trait_ty, maybe_some(eq(syn::parse::ty("MyTrait").expect("Could not parse expected type"))));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_spy_on_object() {
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo() then_spy_on_object always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromSpy));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_panic() {
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo() then_panic always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::Panic));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_once() {
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo() then_return 1 once").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Times(syn::parse::expr("1").expect(""))));
        }

        #[test]
        fn should_parse_given_times() {
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo() then_return 1 times 2").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Times(syn::parse::expr("2").expect(""))));
        }

        #[test]
        fn should_parse_given_always() {
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo() then_return 1 always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(args.is_empty(), otherwise "some arguments are detected");
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_given_args() {
            let ((var, _), method, args, ret, repeat) = parse_given("mock.foo(2, 4) then_return 1 always").expect("");

            assert_that!(&var, eq(syn::Ident::from("mock")));
            assert_that!(&method, eq(syn::Ident::from("foo")));
            // assert_that!(&args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
            assert_that!(&ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
            assert_that!(&repeat, eq(Repeat::Always));
        }

        #[test]
        fn should_parse_givens() {
            let (binds, givens) = parse_givens("bind x: i32 = 1; bind x: f32 = 2.0; mock.foo() then_return 1 always;").expect("");

            assert_that!(&binds.len(), eq(2));
            assert_that!(&givens.len(), eq(1));
        }
    }
}
