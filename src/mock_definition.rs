use syn;
use syntax;

use syntax::ext::base::{ExtCtxt, MacResult, DummyResult};
use syntax::tokenstream::TokenTree;
use syntax::ext::quote::rt::Span;

use data::*;
use util::Singleton;

pub fn handle_define_mock<'a>(cx: &'a mut ExtCtxt, sp: Span, token_tree: &[TokenTree]) -> Box<MacResult + 'a> {
    let s = syntax::print::pprust::tts_to_string(token_tree);
    let trait_item = syn::parse_item(&s).expect("Expecting a trait definition.");

    match trait_item.node {
        syn::ItemKind::Trait(safety, generics, bounds, items) => {
            let singleton = DefinedMocks::singleton();
            let gate = singleton.inner.lock();
            let mut defined_mocks = gate.unwrap();
            defined_mocks.insert(trait_item.ident.clone(), TraitInfo::new(safety, generics, bounds, items));
        },
        _ => cx.span_err(sp, "Expecting a trait definition")
    }

    DummyResult::any(sp)
}
