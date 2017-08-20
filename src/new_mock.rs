/* Copyright 2017 Christopher Bacher
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use syn;
use syn::parse::IResult;
use data::*;

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
                  maybe_type_name: option!(preceded!(keyword!("for"), syn::parse::ident)) >>
                  punct!(")") >>
                  (RequestedMock { traits, attributes, maybe_type_name })
        ), punct!(";")
    )
);

pub fn handle_new_mock(source: &str, absolute_position: usize) -> (String, String) {
    if let IResult::Done(remainder, mut requested_mock) = parse_new_mock(source) {
        let mut requested_mocks = acquire!(REQUESTED_MOCKS);

        if requested_mock.maybe_type_name.is_none() {
            requested_mock.maybe_type_name = Some(syn::Ident::from(format!("Mock{}", absolute_position)));
        }
        let mock_type_name = requested_mock.maybe_type_name.clone().unwrap();
        requested_mocks.push(requested_mock);

        let assignment_stmt = quote! { mock::#mock_type_name::new(); };
        return (assignment_stmt.to_string(), remainder.to_string());
    }

    panic!(format!(concat!("Expecting a new_mock defintion of the form: new_mock!(paths::to::Traits, ... #[optional_attributes]...);\n",
                           "\tGot: {}"), source));
}
