use syn;
use quote;
use syntax;

use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntax::util::small_vector::SmallVector;

use std::boxed::Box;
use std::mem;

pub fn type_name_of(ty: &syn::Ty) -> Option<syn::Ident> {
    match ty {
        &syn::Ty::Path(_, ref path) => path.segments.last()
                                      .map(|seg| seg.ident.clone()),
        _ => None
    }
}

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
macro_rules! acquire {
    ( $global_var: ident ) => { $global_var.lock().unwrap() }
}
