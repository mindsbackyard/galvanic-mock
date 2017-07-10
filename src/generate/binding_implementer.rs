use syn;
use quote;
use data::Binding;

pub fn binding_name_for(block_id: usize) -> syn::Ident {
    syn::Ident::from(format!("Binding{}", block_id))
}

pub fn implement_bindings(bindings: &[Binding]) -> Vec<quote::Tokens> {
    bindings.into_iter().map(|binding| implement_binding(binding)).collect::<Vec<_>>()
}

fn implement_binding(binding: &Binding) -> quote::Tokens {
    let binding_name = binding_name_for(binding.block_id);
    let fields = binding.fields.iter().map(|field| {
        let name = &field.name;
        let ty = &field.ty;
        quote!(#name: #ty)
    }).collect::<Vec<_>>();

    quote!{
        #[derive(Clone)]
        struct #binding_name {
            #(#fields),*
        }
    }
}

pub fn implement_initialize_binding(binding: &Binding) -> quote::Tokens {
    let binding_name = binding_name_for(binding.block_id);
    let field_initializers = binding.fields.iter().map(|field| {
        let name = &field.name;
        let initializer = &field.initializer;
        quote!(#name: #initializer)
    }).collect::<Vec<_>>();

    quote!{
        #binding_name {
            #(#field_initializers),*
        }
    }
}
