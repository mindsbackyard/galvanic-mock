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
#![feature(proc_macro)]
#![recursion_limit = "128"]

#[macro_use] mod acquire;
mod new_mock;
mod given;
mod expect;
mod generate;
mod data;

extern crate proc_macro;
#[macro_use] extern crate lazy_static;

extern crate syn;
#[macro_use] extern crate synom;
#[macro_use] extern crate quote;

#[cfg(test)]#[macro_use]
extern crate galvanic_assert;

use proc_macro::TokenStream;

use new_mock::handle_new_mock;
use given::handle_given;
use expect::handle_expect_interactions;
use generate::handle_generate_mocks;
use data::*;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;


enum MockedTraitLocation {
    TraitDef(syn::Path),
    Referred(syn::Path)
}

named!(parse_trait_path -> MockedTraitLocation,
    delimited!(
        punct!("("),
        do_parse!(
            external: option!(alt!(keyword!("intern") | keyword!("extern"))) >> path: call!(syn::parse::path) >>
            (match external {
                Some(..) => MockedTraitLocation::Referred(path),
                None => MockedTraitLocation::TraitDef(path)
            })
        ),
        punct!(")")
    )
);

#[proc_macro_attribute]
pub fn mockable(args: TokenStream, input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let trait_item = syn::parse_item(&s).expect("Expecting a trait definition.");

    let args_str = &args.to_string();

    match trait_item.node {
        syn::ItemKind::Trait(safety, generics, bounds, items) => {
            let mut mockable_traits = acquire!(MOCKABLE_TRAITS);

            if args_str.is_empty() {
                mockable_traits.insert(trait_item.ident.clone().into(), TraitInfo::new(safety, generics, bounds, items));
                return input;
            }

            let trait_location = parse_trait_path(args_str)
                                 .expect(concat!("#[mockable(..)] requires the absolute path of the trait's module.",
                                                 "It must be preceded with `extern`/`intern` if the trait is defined in another crate/module"));
            match trait_location {
                MockedTraitLocation::TraitDef(mut trait_path) => {
                    trait_path.segments.push(trait_item.ident.clone().into());
                    mockable_traits.insert(trait_path, TraitInfo::new(safety, generics, bounds, items));
                    input
                },
                MockedTraitLocation::Referred(mut trait_path) => {
                    trait_path.segments.push(trait_item.ident.clone().into());
                    mockable_traits.insert(trait_path, TraitInfo::new(safety, generics, bounds, items));
                    "".parse().unwrap()
                }
            }
        },
        _ => panic!("Expecting a trait definition.")
    }
}


#[proc_macro_attribute]
pub fn use_mocks(_: TokenStream, input: TokenStream) -> TokenStream {
    use MacroInvocationPos::*;

    // to parse the macros related to mock ussage the function is converted to string form
    let mut reassembled = String::new();
    let parsed = syn::parse_item(&input.to_string()).unwrap();
    let mut remainder = quote!(#parsed).to_string();

    // parse one macro a time then search for the next macro in the remaining string
    let mut absolute_pos = 0;
    while !remainder.is_empty() {

        match find_next_mock_macro_invocation(&remainder) {
            None => {
                reassembled.push_str(&remainder);
                remainder = String::new();
            },
            Some(invocation) => {
                let (left, new_absolute_pos, right) = match invocation {
                    NewMock(pos) => handle_macro(&remainder, pos, absolute_pos, handle_new_mock),
                    Given(pos) => handle_macro(&remainder, pos, absolute_pos, handle_given),
                    ExpectInteractions(pos) => handle_macro(&remainder, pos, absolute_pos, handle_expect_interactions),
                };

                absolute_pos = new_absolute_pos;
                reassembled.push_str(&left);
                remainder = right;
            }
        }
    }

    // once all macro invocations have been removed from the string (and replaced with the actual mock code) it can be parsed back into a function item
    let mut mock_using_item = syn::parse_item(&reassembled).expect("Reassembled function whi");
    mock_using_item.vis = syn::Visibility::Public;

    let item_ident = &mock_using_item.ident;
    let item_vis = &mock_using_item.vis;
    let mod_fn = syn::Ident::from(format!("mod_{}", item_ident));


    if let syn::ItemKind::Mod(Some(ref mut mod_items)) = mock_using_item.node {
        insert_use_generated_mocks_into_modules(mod_items);
    }

    let mocks = handle_generate_mocks();

    let generated_mock = (quote! {
        #[allow(unused_imports)]
        #item_vis use self::#mod_fn::#item_ident;
        mod #mod_fn {
            #![allow(dead_code)]
            #![allow(unused_imports)]
            #![allow(unused_variables)]
            use super::*;

            #mock_using_item

            pub(in self) mod mock {
                use std;
                use super::*;

                #(#mocks)*
            }
        }
    }).to_string();

    debug(&item_ident, &generated_mock);
    generated_mock.parse().unwrap()
}

fn insert_use_generated_mocks_into_modules(mod_items: &mut Vec<syn::Item>) {
    for item in mod_items.iter_mut() {
        if let syn::ItemKind::Mod(Some(ref mut sub_mod_items)) = item.node {
            insert_use_generated_mocks_into_modules(sub_mod_items);
        }
    }
    mod_items.push(syn::parse_item(quote!(pub use super::*;).as_str()).unwrap());
}

fn debug(item_ident: &syn::Ident, generated_mock: &str) {
    if let Some((_, path)) = env::vars().find(|&(ref key, _)| key == "GA_WRITE_MOCK") {
        if path.is_empty() {
            println!("{}", generated_mock);
        } else {
            let success = File::create(Path::new(&path).join(&(item_ident.to_string())))
                               .and_then(|mut f| f.write_all(generated_mock.as_bytes()));
            if let Err(err) = success {
                eprintln!("Unable to write generated mock to file '{}' because: {}", path, err);
            }
        }
    }
}

fn has_balanced_quotes(source: &str)  -> bool {
    let mut count = 0;
    let mut skip = false;
    for c in source.chars() {
        if skip {
            skip = false;
            continue;
        }

        if c == '\\' {
            skip = true;
        } else if c == '\"' {
            count += 1;
        }
        //TODO handle raw strings
    }
    count % 2 == 0
}

/// Stores position of a macro invocation with the variant naming the macro.
enum MacroInvocationPos {
    NewMock(usize),
    Given(usize),
    ExpectInteractions(usize),
}

/// Find the next galvanic-mock macro invocation in the source string.
///
/// Looks for `new_mock!``, `given!`, `expect_interactions!`, and `then_verify_interactions!`.
/// The `source` string must have been reassembled from a `TokenTree`.
/// The `source` string is expected to start in a code context, i.e., not inside
/// a string.
fn find_next_mock_macro_invocation(source: &str) -> Option<MacroInvocationPos> {
    use MacroInvocationPos::*;
    // there must be a space between the macro name and the ! as the ! is a separate token in the tree
    let macro_names = ["new_mock !", "given !", "expect_interactions !"];
    // not efficient but does the job
    macro_names.into_iter()
               .filter_map(|&mac| {
                            source.find(mac).and_then(|pos| {
                                if has_balanced_quotes(&source[.. pos]) {
                                    Some((pos, mac))
                                } else { None }
                            })
               })
               .min_by_key(|&(pos, _)| pos)
               .and_then(|(pos, mac)| Some(match mac {
                   "new_mock !" => NewMock(pos),
                   "given !" => Given(pos),
                   "expect_interactions !" => ExpectInteractions(pos),
                   _ => panic!("Unreachable. No variant for macro name: {}", mac)
                }))
}

fn handle_macro<F>(source: &str, mac_pos_relative_to_source: usize, absolute_pos_of_source: usize, handler: F) -> (String, usize, String)
where F: Fn(&str, usize) -> (String, String) {
    let absolute_pos_of_mac = absolute_pos_of_source + mac_pos_relative_to_source;

    let (left_of_mac, right_with_mac) = source.split_at(mac_pos_relative_to_source);
    let (mut generated_source, unhandled_source) = handler(right_with_mac, absolute_pos_of_mac);
    generated_source.push_str(&unhandled_source);

    (left_of_mac.to_string(), absolute_pos_of_mac, generated_source)
}


#[cfg(test)]
mod test_has_balanced_quotes {
    use super::*;

    #[test]
    fn should_have_balanced_quotes_if_none_exist() {
        let x = "df df df";
        assert!(has_balanced_quotes(x));
    }

    #[test]
    fn should_have_balanced_quotes_if_single_pair() {
        let x = "df \"df\" df";
        assert!(has_balanced_quotes(x));
    }

    #[test]
    fn should_have_balanced_quotes_if_single_pair_with_escapes() {
        let x = "df \"d\\\"f\" df";
        assert!(has_balanced_quotes(x));
    }

    #[test]
    fn should_have_balanced_quotes_if_multiple_pairs() {
        let x = "df \"df\" \"df\" df";
        assert!(has_balanced_quotes(x));
    }

    #[test]
    fn should_not_have_balanced_quotes_if_single() {
        let x = "df \"df df";
        assert!(!has_balanced_quotes(x));
    }

    #[test]
    fn should_not_have_balanced_quotes_if_escaped_pair() {
        let x = "df \"d\\\" df";
        assert!(!has_balanced_quotes(x));
    }
}
