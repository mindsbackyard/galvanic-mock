use syn;
use quote;
use std::collections::HashMap;

use super::binding_implementer::binding_name_for;
use data::*;
use super::InstantiatedTrait;

/// Generates mock structs and implementations.
pub struct MockStructImplementer<'a> {
    /// The name of the mock type
    mock_type_name: &'a syn::Ident,
    /// The traits which shall be implemented for the mock
    instantiated_traits: &'a [&'a InstantiatedTrait],
}

impl<'a> MockStructImplementer<'a> {
    /// Create a new mock struct.
    ///
    /// # Paramaters
    /// * `mock_type_name` - The name of the new struct
    /// * `requested_trait_types` - The traits which shall be implemented for the mock
    /// * `trait_infos` - A `TraitInfo` for each requested trait in the same order
    pub fn for_(mock_type_name: &'a syn::Ident, instantiated_traits: &'a [&'a InstantiatedTrait]) -> Self {
        MockStructImplementer { mock_type_name, instantiated_traits }
    }

    /// Generate the struct definition of the mock and the methods for creating/interacting with the mock.
    pub fn implement(&self) -> (ItemTokens, ImplTokens) {
        let mock_type_name = &self.mock_type_name;

        let mut behaviour_names = Vec::new();
        let mut behaviour_types = Vec::new();
        let mut add_behaviours = Vec::new();
        for inst_trait in self.instantiated_traits.iter() {
            for item in &inst_trait.info.items {
                if let syn::TraitItemKind::Method(_, _) = item.node {
                    behaviour_names.push(inst_trait.given_behaviour_field_in_mock_for(&item.ident));
                    behaviour_types.push(inst_trait.behaviour_type_for(&item.ident));
                    add_behaviours.push(inst_trait.given_behaviour_add_method_to_mock_for(&item.ident));
                }
            }
        }

        let behaviour_names = &behaviour_names;
        let behaviour_types = &behaviour_types;

        let mock_struct = quote! {
            struct #mock_type_name {
                #(#behaviour_names: std::cell::RefCell<Vec<Box<#behaviour_types>>>,)*
            }
        };

        let mock_impl = quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    #mock_type_name {
                        #(#behaviour_names: std::cell::RefCell::new(Vec::new()),)*
                    }
                }

                #(pub fn #add_behaviours(&self, behaviour: Box<#behaviour_types>) {
                    self.#behaviour_names.borrow_mut().push(behaviour);
                })*
            }
        };

        (mock_struct, mock_impl)
    }
}
