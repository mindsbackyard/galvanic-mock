use syn;

use syn::parse::IResult;

use data::*;
use util::gen_new_mock_type_name;

named!(comma_separated_types -> Vec<syn::Ty>,
    separated_nonempty_list!(punct!(","), syn::parse::ty)
);

named!(parse_new_mock -> Vec<syn::Ty>,
    terminated!(
        preceded!(tuple!(keyword!("new_mock"), punct!("!")),
                  delimited!(punct!("("), call!(comma_separated_types), punct!(")"))
        ), punct!(";")
    )
);

pub fn handle_new_mock(source: &str, absolute_position: usize) -> (String, String) {
    //println!("Handling new_mock!: {}", source);
    if let IResult::Done(remainder, req_traits) = parse_new_mock(source) {
        let mut requested_traits = acquire!(REQUESTED_TRAITS);
        let mut mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);

        let mock_type_name = gen_new_mock_type_name(absolute_position);
        requested_traits.insert(mock_type_name.clone(), req_traits.clone());

        for trait_ty in req_traits {
            mocked_trait_unifier.register_trait(trait_ty);
        }

        let assignment_stmt = quote! { #mock_type_name::new(); };
        return (assignment_stmt.to_string(), remainder.to_string());
    }

    panic!("Expecting a new_mock defintion: let MOCK_VAR_NAME : TRAIT, TRAIT, ...");
}
