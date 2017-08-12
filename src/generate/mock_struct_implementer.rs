use syn;
use quote;

/// Generates mock structs and implementations.
pub struct MockStructImplementer<'a> {
    /// The name of the mock type
    mock_type_name: &'a syn::Ident,
    /// The attributes which should be applied to the generated mock
    attributes: &'a [syn::Attribute],
}

impl<'a> MockStructImplementer<'a> {
    /// Create a new mock struct.
    ///
    /// # Paramaters
    /// * `mock_type_name` - The name of the new struct
    /// * `requested_trait_types` - The traits which shall be implemented for the mock
    /// * `trait_infos` - A `TraitInfo` for each requested trait in the same order
    pub fn for_(mock_type_name: &'a syn::Ident, attributes: &'a [syn::Attribute]) -> Self {
        MockStructImplementer { mock_type_name, attributes }
    }

    /// Generate the struct definition of the mock and the methods for creating/interacting with the mock.
    pub fn implement(&self) -> Vec<quote::Tokens> {
        let mock_type_name = &self.mock_type_name;
        let attributes = self.attributes;

        let mock_struct = quote! {
            #(#attributes)*
            struct #mock_type_name {
                given_behaviours: std::cell::RefCell<std::collections::HashMap<(&'static str, &'static str), Vec<GivenBehaviour>>>,
                expect_behaviours: std::cell::RefCell<std::collections::HashMap<(&'static str, &'static str), Vec<ExpectBehaviour>>>,
                verify_on_drop: bool,
            }
        };

        let mock_impl = quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    Self {
                        given_behaviours: std::cell::RefCell::new(std::collections::HashMap::new()),
                        expect_behaviours: std::cell::RefCell::new(std::collections::HashMap::new()),
                        verify_on_drop: true,
                    }
                }

                pub fn should_verify_on_drop(&mut self, flag: bool) { self.verify_on_drop = flag; }

                #[allow(dead_code)]
                pub fn add_given_behaviour(&self, requested_trait: &'static str, method: &'static str, behaviour: GivenBehaviour) {
                    self.given_behaviours.borrow_mut()
                        .entry((requested_trait, method))
                        .or_insert_with(|| Vec::new())
                        .push(behaviour);
                }

                #[allow(dead_code)]
                pub fn reset_given_behaviours(&mut self) {
                    self.given_behaviours.borrow_mut().clear();
                }

                #[allow(dead_code)]
                pub fn add_expect_behaviour(&self, requested_trait: &'static str, method: &'static str, behaviour: ExpectBehaviour) {
                    self.expect_behaviours.borrow_mut()
                        .entry((requested_trait, method))
                        .or_insert_with(|| Vec::new())
                        .push(behaviour);
                }

                #[allow(dead_code)]
                pub fn reset_expected_behaviours(&mut self) {
                    self.expect_behaviours.borrow_mut().clear();
                }

                #[allow(dead_code)]
                pub fn are_expected_behaviours_satisfied(&self) -> bool {
                    let mut unsatisfied_messages: Vec<String> = Vec::new();
                    for behaviour in self.expect_behaviours.borrow().values().flat_map(|vs| vs) {
                        if !behaviour.is_saturated() {
                            unsatisfied_messages.push(format!("Behaviour unsatisfied: {}", behaviour.describe()));
                        }
                    }

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

        let mock_drop_impl = quote! {
            impl std::ops::Drop for #mock_type_name {
                fn drop(&mut self) {
                    if self.verify_on_drop {
                        self.verify();
                    }
                }
            }
        };

        vec![mock_struct, mock_impl, mock_drop_impl]
    }
}
