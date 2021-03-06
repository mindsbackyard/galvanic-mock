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
        quote!(pub #name: #ty)
    }).collect::<Vec<_>>();

    quote!{
        #[derive(Clone)]
        pub(crate) struct #binding_name {
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
        mock::#binding_name {
            #(#field_initializers),*
        }
    }
}
