use syn;
use syn::parse::*;

#[derive(Debug,PartialEq)]
pub enum Return {
    FromValue(syn::Expr),
    FromCall(syn::Expr),
    FromSpy,
    Panic
}

#[derive(Debug,PartialEq)]
pub enum Repeat {
    Times(syn::Expr),
    Always
}

pub type VarName = syn::Ident;
pub type MethodName = syn::Ident;
pub type Args = Vec<syn::Expr>;

named!(pub parse_given -> (VarName, MethodName, Args, Return, Repeat),
    tuple!(
        call!(syn::parse::ident),
        preceded!(punct!("."), syn::parse::ident),
        delimited!(punct!("("), separated_list!(punct!(","), syn::parse::expr), punct!(")")),
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

named!(pub parse_givens -> Vec<(VarName, MethodName, Args, Return, Repeat)>,
    terminated_list!(punct!(";"), parse_given)
);


pub fn handle_given(source: &str, absolute_position: usize) -> (String, String) {
    // let givens = parse_givens(&s).expect("Expecting a given! defintion: MOCK_VAR_NAME.METHOD(ARGS) THEN REPEAT; ...");
    if let IResult::Done(remainder, (var, method, args, ret, repeat)) = parse_given(source) {

    } else { panic!("Expecting a given! defintion: MOCK_VAR_NAME.METHOD(ARGS) THEN REPEAT; ..."); }

    // let mut behaviour_setters: Vec<quote::Tokens> = Vec::new();
    // for &(ref var, ref method, ref args, ref ret, ref repeat) in givens.iter() {
        // let set_return = match ret {
        //     Return::FromValue(ref val) => quote! { set_then_return_value(|| #val) },
        //     Return::FromCall(ref closure) => quote! { set_then_return_call(#closure) },
        //     Return::FromSpy => quote! { set_then_return_from_spy() },
        //     Return::Panic => quote! { set_then_panic() },
        // };
        // let set_repeat = match repeat {
        //     Repeat::Times(ref num) => quote! { set_repeat_times(#num) },
        //     Repeat::Always => quote! { set_repeat_always() }
        // };
        //
        // let add_behaviour_for = syn::Ident::from(format!("add_behaviour_for_{}", method));
        // let behaviour_setter = quote! {
        //     #var.#add_behaviour_for(#(#args),*).#set_return.#set_repeat;
        // };


    panic!("Not implemented yet");
}


#[cfg(test)]
mod tests {
    use galvanic_assert::*;
    use galvanic_assert::matchers::*;

    use super::*;

    #[test]
    fn should_parse_given_return_from_call() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo() then_return_from || { 2 } always").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args.is_empty(), otherwise "some arguments are detected");
        assert_that!(ret, eq(Return::FromCall(syn::parse::expr("|| { 2 }").expect(""))));
        assert_that!(repeat, eq(Repeat::Always));
    }

    #[test]
    fn should_parse_given_spy_on_object() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo() then_spy_on_object always").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args.is_empty(), otherwise "some arguments are detected");
        assert_that!(ret, eq(Return::FromSpy));
        assert_that!(repeat, eq(Repeat::Always));
    }

    #[test]
    fn should_parse_given_panic() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo() then_panic always").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args.is_empty(), otherwise "some arguments are detected");
        assert_that!(ret, eq(Return::Panic));
        assert_that!(repeat, eq(Repeat::Always));
    }

    #[test]
    fn should_parse_given_once() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo() then_return 1 once").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args.is_empty(), otherwise "some arguments are detected");
        assert_that!(ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
        assert_that!(repeat, eq(Repeat::Times(syn::parse::expr("1").expect(""))));
    }

    #[test]
    fn should_parse_given_times() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo() then_return 1 times 2").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args.is_empty(), otherwise "some arguments are detected");
        assert_that!(ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
        assert_that!(repeat, eq(Repeat::Times(syn::parse::expr("2").expect(""))));
    }

    #[test]
    fn should_parse_given_always() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo() then_return 1 always").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args.is_empty(), otherwise "some arguments are detected");
        assert_that!(ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
        assert_that!(repeat, eq(Repeat::Always));
    }

    #[test]
    fn should_parse_given_args() {
        let (var, method, args, ret, repeat) = parse_given("mock.foo(2, 4) then_return 1 always").expect("");

        assert_that!(var, eq(syn::Ident::from("mock")));
        assert_that!(method, eq(syn::Ident::from("foo")));
        assert_that!(args, collection::contains_in_order(vec![syn::parse::expr("2").expect(""), syn::parse::expr("4").expect("")]));
        assert_that!(ret, eq(Return::FromValue(syn::parse::expr("1").expect(""))));
        assert_that!(repeat, eq(Repeat::Always));
    }

    #[test]
    fn test_idents() {
        assert_that!(syn::Ident::from("abc"), eq(syn::Ident::from("abc")));
    }
}
