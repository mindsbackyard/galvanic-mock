use syn;
use quote;
use std::collections::HashMap;

use super::binding_implementer::binding_name_for;
use data::*;

/// Generates mock structs and implementations.
pub struct MockStructImplementer<'a> {
    /// The name of the mock type
    mock_type_name: &'a syn::Ident,
    /// The traits which shall be implemented for the mock
    requested_trait_types: &'a [syn::Ty],
    /// `TraitInfo` objects for each requested trait. Ordered as `requested_trait_types`.
    trait_infos: &'a [&'a TraitInfo],
    given_block_infos: &'a [GivenBlockInfo]
}

impl<'a> MockStructImplementer<'a> {
    /// Create a new mock struct.
    ///
    /// # Paramaters
    /// * `mock_type_name` - The name of the new struct
    /// * `requested_trait_types` - The traits which shall be implemented for the mock
    /// * `trait_infos` - A `TraitInfo` for each requested trait in the same order
    pub fn for_(mock_type_name: &'a syn::Ident, requested_trait_types: &'a [syn::Ty],
                trait_infos: &'a [&'a TraitInfo], given_block_infos: &'a [GivenBlockInfo])
                -> MockStructImplementer<'a>  {
        MockStructImplementer {
            mock_type_name: mock_type_name,
            requested_trait_types: requested_trait_types,
            trait_infos: trait_infos,
            given_block_infos: given_block_infos
        }
    }

    /// Generate the struct definition of the mock and the methods for creating/interacting with the mock.
    pub fn implement(&self) -> (ItemTokens, ImplTokens) {
        let mock_type_name = &self.mock_type_name;

        let behaviour_names = &self.given_block_infos.iter().flat_map(|info| {
            let block_id = info.block_id;
            info.given_statements.iter().map(move |stmt| MockStructImplementer::behaviour_field_for(block_id, stmt.stmt_id))
        }).collect::<Vec<_>>();

        let bound_names = &self.given_block_infos.iter().map(|info| {
            MockStructImplementer::bound_field_for(info.block_id)
        }).collect::<Vec<_>>();
        let bound_types = &self.given_block_infos.iter().map(|info|{
            syn::Ident::from(format!("Binding{}", info.block_id))
        }).collect::<Vec<_>>();

        let given_block_flags = &self.given_block_infos.iter().map(|info|
            MockStructImplementer::given_block_activated_field_for(info.block_id)
        ).collect::<Vec<_>>();
        let activate_given_blocks = self.given_block_infos.iter().map(|info|
            self.implement_activate_given_block(info)
        ).collect::<Vec<_>>();


        let mock_struct = quote! {
            struct #mock_type_name {
                #(#behaviour_names: std::cell::Cell<(usize,Option<usize>)>,)*
                #(#bound_names: std::cell::Cell<Option<#bound_types>>,)*
                #(#given_block_flags: std::cell::Cell<bool>,)*
            }
        };

        let mock_impl = quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    #mock_type_name {
                        #(#behaviour_names: std::cell::Cell::new((0,None)),)*
                        #(#bound_names: std::cell::Cell::new(None),)*
                        #(#given_block_flags: std::cell::Cell::new(false),)*
                    }
                }

                #(#activate_given_blocks)*
            }
        };

        (mock_struct, mock_impl)
    }

    fn implement_activate_given_block(&self, block: &GivenBlockInfo) -> quote::Tokens {
        let activate_given_block = MockStructImplementer::activate_given_block_for(block.block_id);
        let given_block_flag = MockStructImplementer::given_block_activated_field_for(block.block_id);

        let behaviour_names = block.given_statements.iter().map(|stmt| {
            MockStructImplementer::behaviour_field_for(block.block_id, stmt.stmt_id)
        }).collect::<Vec<_>>();

        let times = (1..block.given_statements.len()+1)
                        .map(|idx| syn::Ident::from(format!("times_for_stmt{}", idx)))
                        .collect::<Vec<_>>();

        let bound = MockStructImplementer::bound_field_for(block.block_id);
        let binding = binding_name_for(block.block_id);
        quote! {
            pub fn #activate_given_block(&mut self, bound: #binding, #times: Option<usize>) {
                self.#given_block_flag.set(true);
                #(self.#behaviour_names.set((0,#times));)*
                self.#bound.set(Some(bound));
            }
        }
    }

    pub fn behaviour_field_for(block_id: usize, stmt_id: usize) -> syn::Ident {
        syn::Ident::from(format!("behaviour{}_of_block{}", stmt_id, block_id))
    }

    pub fn bound_field_for(block_id: usize) -> syn::Ident {
        syn::Ident::from(format!("bound{}", block_id))
    }

    pub fn activate_given_block_for(block_id: usize) -> syn::Ident {
        syn::Ident::from(format!("activate_given_block_{}", block_id))
    }

    pub fn given_block_activated_field_for(block_id: usize) -> syn::Ident {
        syn::Ident::from(format!("given_block_{}_activated", block_id))
    }
}
