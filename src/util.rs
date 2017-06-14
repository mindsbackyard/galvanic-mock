use syn;
use quote;
use syntax;

use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntax::util::small_vector::SmallVector;

use std::boxed::Box;
use std::mem;


pub fn gen_new_mock_type_name(mock_intialization_pos: usize) -> syn::Ident {
    syn::Ident::from(format!("Mock{}", mock_intialization_pos))
}

fn convert_token_to_syntax_stmt<'a>(cx: &'a mut ExtCtxt, tokens: quote::Tokens) -> Box<MacResult + 'a> {
    let maybe_stmt = syntax::parse::parse_stmt_from_source_str("".to_owned(), tokens.to_string(), cx.parse_sess()).unwrap();
    MacEager::stmts(SmallVector::one(maybe_stmt.unwrap()))
}

fn convert_token_to_syntax_stmts<'a>(cx: &'a mut ExtCtxt, tokens: Vec<quote::Tokens>) -> Box<MacResult + 'a> {
    let stmts = tokens.into_iter()
                      .map(|t| syntax::parse::parse_stmt_from_source_str("".to_owned(), t.to_string(), cx.parse_sess()).unwrap().unwrap())
                      .collect::<Vec<_>>();
    MacEager::stmts(SmallVector::many(stmts))
}

#[macro_export]
macro_rules! singleton {
    ( $type_name: ident ) => {
        impl $type_name {
            pub fn singleton() -> Self {
                use::std::mem;
                use std::sync::{Once, ONCE_INIT};
                // Initialize it to a null value
                static mut SINGLETON: *const i8 = 0 as *const i8;
                static ONCE: Once = ONCE_INIT;

                unsafe {
                    ONCE.call_once(|| {
                        let data = Self::default();
                        // Put it in the heap so it can outlive this call
                        let ptr: *const Self = mem::transmute(Box::new(data));
                        SINGLETON = mem::transmute(ptr);
                    });
                    let ptr: *const Self = mem::transmute(SINGLETON);
                    (*ptr).clone()
                }
            }
        }
    }
}

#[macro_export]
macro_rules! get_singleton {
    ( $var_name: ident of $type_name: ident ) => {
        let singleton = $type_name::singleton();
        let gate = singleton.inner.lock();
        let $var_name = gate.unwrap();
    }
}

#[macro_export]
macro_rules! get_singleton_mut {
    ( $var_name: ident of $type_name: ident ) => {
        let singleton = $type_name::singleton();
        let gate = singleton.inner.lock();
        let mut $var_name = gate.unwrap();
    }
}
