use syn;
use quote;

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


macro_rules! acquire {
    ( $global_var: ident ) => { $global_var.lock().unwrap() }
}
