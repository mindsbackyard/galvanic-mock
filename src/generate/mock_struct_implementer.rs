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
    pub fn implement(&self) -> Vec<quote::Tokens> {
        let mock_type_name = &self.mock_type_name;

        let mut given_behaviour_names = Vec::new();
        let mut add_given_behaviours = Vec::new();
        let mut expect_behaviour_names = Vec::new();
        let mut add_expect_behaviours = Vec::new();
        let mut behaviour_types = Vec::new();
        for inst_trait in self.instantiated_traits.iter() {
            for item in &inst_trait.info.items {
                if let syn::TraitItemKind::Method(_, _) = item.node {
                    given_behaviour_names.push(inst_trait.given_behaviour_field_in_mock_for(&item.ident));
                    add_given_behaviours.push(inst_trait.given_behaviour_add_method_to_mock_for(&item.ident));
                    expect_behaviour_names.push(inst_trait.expect_behaviour_field_in_mock_for(&item.ident));
                    add_expect_behaviours.push(inst_trait.expect_behaviour_add_method_to_mock_for(&item.ident));
                    behaviour_types.push(inst_trait.behaviour_type_for(&item.ident));
                }
            }
        }

        let given_behaviour_names = &given_behaviour_names;
        let expect_behaviour_names = &expect_behaviour_names;
        let behaviour_types = &behaviour_types;

        let mock_struct = quote! {
            struct #mock_type_name {
                #(#given_behaviour_names: std::cell::RefCell<Vec<GivenBehaviour>>,)*
                #(#expect_behaviour_names: std::cell::RefCell<Vec<ExpectBehaviour>>,)*
                verify_on_drop: bool,
            }
        };

        let mock_impl = quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    #mock_type_name {
                        #(#given_behaviour_names: std::cell::RefCell::new(Vec::new()),)*
                        #(#expect_behaviour_names: std::cell::RefCell::new(Vec::new()),)*
                        verify_on_drop: true,
                    }
                }

                pub fn should_verify_on_drop(&mut self, flag: bool) { self.verify_on_drop = flag; }

                #(#[allow(dead_code)] pub fn #add_given_behaviours(&self, behaviour: GivenBehaviour) {
                    self.#given_behaviour_names.borrow_mut().push(behaviour);
                })*

                #[allow(dead_code)]
                pub fn reset_given_behaviours(&mut self) {
                    #(self.#given_behaviour_names.borrow_mut().clear();)*
                }

                #(#[allow(dead_code)] pub fn #add_expect_behaviours(&self, behaviour: ExpectBehaviour) {
                    self.#expect_behaviour_names.borrow_mut().push(behaviour);
                })*

                #[allow(dead_code)]
                pub fn reset_expected_behaviours(&mut self) {
                    #(self.#expect_behaviour_names.borrow_mut().clear();)*
                }

                #[allow(dead_code)]
                pub fn are_expected_behaviours_satisfied(&self) -> bool {
                    let mut unsatisfied_messages: Vec<String> = Vec::new();
                    #(for behaviour in self.#expect_behaviour_names.borrow().iter() {
                        if !behaviour.is_saturated() {
                            unsatisfied_messages.push(format!("Behaviour unsatisfied: {}", behaviour.describe()));
                        }
                    })*

                    if !unsatisfied_messages.is_empty() {
                        for message in unsatisfied_messages {
                            println!("{}", message);
                        }
                        false
                    } else { true }
                }

                #[allow(dead_code)]
                pub fn verify(&self) {
                    if !self.are_expected_behaviours_satisfied() && !std::thread::panicking() {
                        panic!("There are unsatisfied expected behaviours for mocked traits.");
                    }
                }
            }
        };

        vec![mock_struct, mock_impl]
    }
}
