#![feature(plugin_registrar, rustc_private, quote)]
#![allow(warnings)]
#![recursion_limit = "128"]

#[macro_use] mod util;
mod mock_definition;
mod new_mock;
mod given;
mod generate_mocks;
mod data;

#[cfg(test)]#[macro_use]
extern crate galvanic_assert;

extern crate syntax;
extern crate rustc;
extern crate rustc_plugin;
extern crate rustc_errors;

extern crate syn;
#[macro_use] extern crate synom;
#[macro_use] extern crate quote;


use rustc_plugin::Registry;

use syntax::codemap::{BytePos, Pos};
use syntax::ext::base::{SyntaxExtension, ExtCtxt, MacResult, DummyResult, MacEager};
use syntax::parse::{token, parser};
use syntax::ast;
use syntax::tokenstream::TokenTree;
use syntax::ptr::P;
use syntax::ext::quote::rt::Span;
use syntax::util::small_vector::SmallVector;
use syntax::print::pprust;
use syntax::visit;

use syn::parse::IResult;

use mock_definition::handle_define_mock;
use new_mock::handle_new_mock;
use given::handle_given;
use generate_mocks::handle_generate_mocks;
use data::*;


#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("define_mock", handle_define_mock);
    reg.register_macro("use_mocks", handle_use_mocks);
}



fn handle_expect_invocations(source: &str, absolute_position: usize) -> (String, String) {
    panic!("Not implemented yet");
}

fn handle_then_verify_interactions(source: &str, absolute_position: usize) -> (String, String) {
    panic!("Not implemented yet");
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
pub enum MacroInvocationPos {
    NewMock(usize),
    Given(usize),
    ExpectInteractions(usize),
    ThenVerifyInteractions(usize)
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
    let macro_names = ["new_mock !", "given !", "expect_interactions !", "then_verify_interactions !"];
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
                   "then_verify_interactions !" => ThenVerifyInteractions(pos),
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

fn handle_use_mocks<'a>(cx: &'a mut ExtCtxt, sp: Span, token_tree: &[TokenTree]) -> Box<MacResult + 'a> {
    use MacroInvocationPos::*;

    // to parse the macros related to mock ussage the function is converted to string form
    let func_def: String = syntax::print::pprust::tts_to_string(token_tree);
    let mut reassembled = String::new();
    let mut remainder = func_def;
    let mut left: String;

    // parse one macro a time then search for the next macro in the remaining string
    let BytePos(mut absolute_pos) = sp.lo;
    let absolute_pos = absolute_pos as usize;
    while !remainder.is_empty() {

        match find_next_mock_macro_invocation(&remainder) {
            None => {
                reassembled.push_str(&remainder);
                remainder = String::new();
            },
            Some(invocation) => {
                let (left, absolute_pos, right) = match invocation {
                    NewMock(pos) => handle_macro(&remainder, pos, absolute_pos, handle_new_mock),
                    Given(pos) => handle_macro(&remainder, pos, absolute_pos, handle_given),
                    ExpectInteractions(pos) => handle_macro(&remainder, pos, absolute_pos, handle_expect_invocations),
                    ThenVerifyInteractions(pos) => handle_macro(&remainder, pos, absolute_pos, handle_then_verify_interactions),
                };

                reassembled.push_str(&left);
                remainder = right;
            }
        }
    }

    println!("{}", reassembled);
    // once all macro invocations have been removed from the string (and replaced with the actual mock code) it can be parsed back into a function item
    let maybe_fn = syntax::parse::parse_item_from_source_str("".to_owned(), reassembled, cx.parse_sess()).unwrap();
    let fn_: P<ast::Item> = maybe_fn.unwrap();

    let token_pairs = handle_generate_mocks();
    for &(ref tok, ref toks) in token_pairs.iter() {
        println!("{}", tok);
        for t in toks {
            println!("{}", t);
        }
    }
    let mut items = token_pairs.iter()
                               .map(|&(ref t, _)| {
                                   syntax::parse::parse_item_from_source_str(
                                       "".to_owned(), t.to_string(), cx.parse_sess()
                                   ).unwrap().unwrap()
                               }).collect::<Vec<_>>();
    items.push(fn_);

    let mut impls = token_pairs.iter()
                               .flat_map(|&(_, ref t)| t.into_iter())
                               .map(|t| {
                                   syntax::parse::parse_item_from_source_str(
                                       "".to_owned(), t.to_string(), cx.parse_sess()
                                   ).unwrap().unwrap()
                               }).collect::<Vec<_>>();
    items.extend(impls);

    MacEager::items(SmallVector::many(items))
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
