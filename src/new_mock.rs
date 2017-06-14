use syn;

use syn::parse::IResult;

use data::*;
use util::gen_new_mock_type_name;

named!(comma_separated_types -> Vec<syn::Ty>,
    separated_nonempty_list!(punct!(","), syn::parse::ty)
);

named!(parse_new_mock_content -> (syn::Ident, Vec<syn::Ty>),
    preceded!(keyword!("let"), tuple!(
        terminated!(syn::parse::ident, punct!(":")),
        comma_separated_types
    ))
);

named!(parse_new_mock -> (syn::Ident, Vec<syn::Ty>),
    terminated!(
        preceded!(tuple!(keyword!("new_mock"), punct!("!")),
                  delimited!(punct!("("), call!(parse_new_mock_content), punct!(")"))
        ), punct!(";")
    )
);

pub fn handle_new_mock(source: &str, absolute_position: usize) -> (String, String) {
    println!("Handling new_mock!: {}", source);
    if let IResult::Done(remainder, result) = parse_new_mock(source) {
        let (mock_var, req_traits) = result;

        get_singleton_mut!(mock_vars of MockVars);
        let mock_type_name = gen_new_mock_type_name(absolute_position);
        mock_vars.push((mock_type_name.clone(), req_traits));

        let assignment_stmt = quote! { let #mock_var = #mock_type_name::new(); };
        return (assignment_stmt.to_string(), remainder.to_string());
    }

    panic!("Expecting a new_mock defintion: let MOCK_VAR_NAME : TRAIT, TRAIT, ...");
}
