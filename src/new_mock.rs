use syn;

use syn::parse::IResult;

use data::*;
use util::gen_new_mock_type_name;

named!(comma_separated_types -> Vec<syn::Path>,
    separated_nonempty_list!(punct!(","), syn::parse::path)
);

named!(outer_attr -> syn::Attribute,
    do_parse!(
        punct!("#") >> punct!("[") >>
        content: take_until!("]") >>
        punct!("]") >>
        (syn::parse_outer_attr(&format!("#[{}]", content)).unwrap())
    )
);

named!(parse_new_mock -> RequestedMock,
    terminated!(
        do_parse!(keyword!("new_mock") >> punct!("!") >> punct!("(") >>
                  traits: call!(comma_separated_types) >>
                  attributes: many0!(outer_attr) >>
                  punct!(")") >>
                  (RequestedMock { traits, attributes })
        ), punct!(";")
    )
);

pub fn handle_new_mock(source: &str, absolute_position: usize) -> (String, String) {
    if let IResult::Done(remainder, requested_mock) = parse_new_mock(source) {
        let mut requested_mocks = acquire!(REQUESTED_MOCKS);
        let mut mocked_trait_unifier = acquire!(MOCKED_TRAIT_UNIFIER);

        for trait_ty in requested_mock.traits.iter() {
            mocked_trait_unifier.register_trait(trait_ty);
        }

        let mock_type_name = gen_new_mock_type_name(absolute_position);
        requested_mocks.insert(mock_type_name.clone(), requested_mock);

        let assignment_stmt = quote! { #mock_type_name::new(); };
        return (assignment_stmt.to_string(), remainder.to_string());
    }

    panic!(format!(concat!("Expecting a new_mock defintion of the form: new_mock!(paths::to::Traits, ... #[optional_attributes]...);\n",
                           "\tGot: {}"), source));
}
