pub mod binding_implementer;
pub mod type_param_mapper;
pub mod mock_struct_implementer;
pub mod trait_implementer;

use syn;
use quote;

use std::collections::HashMap;

use ::generate::binding_implementer::*;
use ::generate::type_param_mapper::*;
use ::generate::mock_struct_implementer::*;
use ::generate::trait_implementer::*;
use data::*;
use util::type_name_of;

/// Generates all mock structs and implementations.
pub fn handle_generate_mocks() -> Vec<(ItemTokens, Vec<ImplTokens>)> {
    let mut requested_traits = acquire!(REQUESTED_TRAITS);
    let given_blocks_per_type = acquire!(GIVEN_BLOCKS);
    let mockable_traits = acquire!(MOCKABLE_TRAITS);
    let bindings = acquire!(BINDINGS);

    let mut tokens: Vec<(ItemTokens, Vec<ImplTokens>)> = implement_bindings(&bindings).into_iter()
                                                                                      .map(|item| (item, Vec::new()))
                                                                                      .collect::<Vec<_>>();

    let empty = &Vec::<GivenBlockInfo>::new();
    for (mock_type_name, requested_traits) in requested_traits.iter() {
        let given_blocks = given_blocks_per_type.get(mock_type_name).unwrap_or(empty);
        tokens.push(handle_generate_mock(mock_type_name, requested_traits, given_blocks, &mockable_traits));
    }
    requested_traits.clear();

    tokens
}

/// Generates a mock implementation for a
///
/// The following elements are generated:
/// * struct definition
/// * mock interface implementation
/// * mocked trait implementations
/// for a mock of a given name, and a set of requested traits.
///
/// All mock types requested by a `new_mock!` invocations are generated with this function.
///
/// # Paramters
/// * `mock_type_name` - The name of the generated mock type
/// * `trait_tys` - The (generic) trait types which are requested for the mock
fn handle_generate_mock(mock_type_name: &syn::Ident, trait_tys: &[syn::Ty], given_block_infos_for_mock: &[GivenBlockInfo],
                        mockable_traits: &MockableTraits)
                        -> (ItemTokens, Vec<ImplTokens>) {

    let mut blocks_per_trait = HashMap::<usize, Vec<GivenBlockInfo>>::new();
    for block in given_block_infos_for_mock {
        let mut stmts_per_trait = HashMap::<usize, Vec<GivenStatement>>::new();
        for stmt in &block.given_statements {
            let maybe_trait_idx = match &stmt.maybe_ufc_trait {
                &None => determine_requested_trait_idx_by_method(&stmt.method, trait_tys, mockable_traits),
                &Some(ref ufc_trait_ty) => determine_requested_trait_idx_by_ufc(ufc_trait_ty, &stmt.method, trait_tys, mockable_traits)
            };
            match maybe_trait_idx {
                Err(reason) => panic!(reason),
                Ok(idx) => stmts_per_trait.entry(idx).or_insert_with(|| Vec::new()).push(stmt.clone())
            };
        }

        for (trait_idx, statements) in stmts_per_trait {
            blocks_per_trait.entry(trait_idx)
                            .or_insert_with(|| Vec::new())
                            .push(GivenBlockInfo {
                                    block_id: block.block_id,
                                    given_statements: statements
                            });
        }
    }

    let mut trait_infos = Vec::new();
    let mut mappers = Vec::new();
    let mut impl_tokens = Vec::new();
    // generate impls for each requested trait
    for (trait_idx, trait_) in trait_tys.iter().enumerate() {
        match trait_ {
            &syn::Ty::Path(_, ref p) => {
                if p.segments.len() != 1 {
                    panic!("All mocked traits are supposed to be given without path by their name only.");
                }

                let trait_name = &p.segments[0].ident;
                let trait_info = mockable_traits.get(&trait_name)
                                                .expect("All mocked traits must be defined using 'define_mock!'");

                let empty = &Vec::<GivenBlockInfo>::new();
                let mut mapper = TypeParamMapper::new();
                {
                    let generics: &syn::Generics = &trait_info.generics;
                    let instantiated_params = extract_parameterized_types_from_trait_use(p);

                    for (param, instantiated) in generics.ty_params.iter().zip(instantiated_params) {
                        mapper.add_mapping(param.ident.clone(), instantiated);
                    }

                    let trait_implementer = TraitImplementer::for_(
                        mock_type_name, trait_idx, trait_,
                        &trait_info, &mapper,
                        blocks_per_trait.get(&trait_idx).unwrap_or(empty)
                    );
                    impl_tokens.push(trait_implementer.implement());
                }

                mappers.push(mapper);

                trait_infos.push(trait_info);
            },
            _ => { panic!("Expected a Path as trait type to be implemented for '{}' got: {:?}",
                          mock_type_name, trait_);
            }
        }
    }

    let mock_implementer = MockStructImplementer::for_(
        mock_type_name, trait_tys, &trait_infos, given_block_infos_for_mock
    );

    let (mock_struct_tokens, mock_impl_tokens) = mock_implementer.implement();
    impl_tokens.push(mock_impl_tokens);
    (mock_struct_tokens, impl_tokens)
}

pub fn determine_requested_trait_idx_by_method(method_ident: &syn::Ident,
                                               requested_traits_for_mock: &[syn::Ty], mockable_traits: &MockableTraits)
                                               -> Result<usize, String> {
    let mut maybe_trait_idx: Option<usize> = None;
    let method_name = method_ident.to_string();

    for (idx, trait_ty) in requested_traits_for_mock.into_iter().enumerate() {
        let trait_name = type_name_of(trait_ty).expect("");
        let trait_info = try!(mockable_traits.get(&trait_name).ok_or_else(
            || format!("`{}` is not a mockable trait. Has it been defined with `define_mock!`?", trait_name)
        ));
        if trait_info.get_method_by_name(&method_name).is_some() {
            if maybe_trait_idx.is_some() {
                return Err(format!("Multiple requested traits have a method `{}`. Use Universal Function Call syntax to specify the correct trait.", method_name));
            }
            maybe_trait_idx = Some(idx);
        }
    }

    maybe_trait_idx.ok_or_else(|| format!("No requested trait with a method named `{}` found.", method_name))
}

pub fn determine_requested_trait_idx_by_ufc(ufc_trait_ty: &syn::Ty, method_ident: &syn::Ident,
                                            requested_traits_for_mock: &[syn::Ty], mockable_traits: &MockableTraits)
                                            -> Result<usize, String> {
    let method_name = method_ident.to_string();

    let ufc_trait_idx = try!(requested_traits_for_mock.iter().position(|ty| ty == ufc_trait_ty).ok_or_else(
        || format!("No requested trait matches the given UFC trait `{:?}`.", ufc_trait_ty)
    ));

    let trait_name = try!(type_name_of(&ufc_trait_ty).ok_or_else(
        || format!("Unable to extract trait name from `{:?}`", ufc_trait_ty)
    ));

    let trait_info = try!(mockable_traits.get(&trait_name).ok_or_else(
        || format!("`{:?}` is not a mockable trait. Has it been defined with `define_mock!`?", ufc_trait_ty)
    ));

    try!(trait_info.get_method_by_name(&method_name).ok_or_else(
        || format!("Unable to process `given!` statement. No method `{}` found for requested trait `{:?}`",
                   method_name, ufc_trait_ty)
    ));

    Ok(ufc_trait_idx)
}

fn extract_parameterized_types_from_trait_use(trait_ty: &syn::Path) -> Vec<syn::Ty> {
    match trait_ty.segments[0].parameters {
        syn::PathParameters::AngleBracketed(ref data) => data.types.clone(),
        _ => panic!("Type parameter extraction only works for angle-bracketed types.")
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use galvanic_assert::*;
    use galvanic_assert::matchers::*;
    use galvanic_assert::matchers::variant::*;

    fn define_mockable_trait(trait_source: &str) -> (syn::Ident, TraitInfo) {
        let trait_item = syn::parse_item(trait_source).unwrap();
        match trait_item.node {
            syn::ItemKind::Trait(safety, generics, bounds, items) => {
                return (trait_item.ident.clone(), TraitInfo::new(safety, generics, bounds, items));
            },
            _ => panic!("Expecting a trait definition")
        }
    }

    fn define_mockable_traits(trait_sources: &[&str]) -> HashMap<syn::Ident, TraitInfo> {
        trait_sources.into_iter()
                     .map(|source| define_mockable_trait(source))
                     .collect::<HashMap<_,_>>()
    }

    mod trait_idx_by_ufc {
        use super::*;
        use super::super::*;

        #[test]
        fn should_get_trait_idx_by_ufc() {
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty1 = syn::parse_type("MyTrait<i32>").unwrap();
            let trait_ty2 = syn::parse_type("MyTrait<f32>").unwrap();
            let method_ident = syn::Ident::from("foo");

            let requested_traits_for_mock = vec![trait_ty1.clone(), trait_ty2.clone()];
            let mockable_traits = define_mockable_traits(&["trait MyTrait<T> { fn foo(); }"]);

            //when
            let maybe_idx1 = determine_requested_trait_idx_by_ufc(&trait_ty1, &method_ident, &requested_traits_for_mock, &mockable_traits);
            let maybe_idx2 = determine_requested_trait_idx_by_ufc(&trait_ty2, &method_ident, &requested_traits_for_mock, &mockable_traits);
            //then
            assert_that!(&maybe_idx1, maybe_ok(eq(0)));
            assert_that!(&maybe_idx2, maybe_ok(eq(1)));
        }

        #[test]
        fn should_fail_get_trait_idx_by_ufc_unmockable_trait() {
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty = syn::parse_type("MyTrait<i32>").unwrap();
            let method_ident = syn::Ident::from("foo");

            let requested_traits_for_mock = vec![trait_ty.clone()];
            let mockable_traits = HashMap::new();

            // when
            let maybe_idx = determine_requested_trait_idx_by_ufc(&trait_ty, &method_ident, &requested_traits_for_mock, &mockable_traits);
            // then
            assert_that!(maybe_idx.is_err(), otherwise "trait should not be found");
        }

        #[test]
        fn should_fail_get_trait_idx_by_wrong_ufc() {
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty = syn::parse_type("MyTrait<i32>").unwrap();
            let method_ident = syn::Ident::from("foo");

            let requested_traits_for_mock = vec![];
            let mockable_traits = define_mockable_traits(&["trait MyTrait<T> { fn foo(); }"]);

            // when
            let maybe_idx = determine_requested_trait_idx_by_ufc(&trait_ty, &method_ident, &requested_traits_for_mock, &mockable_traits);
            // then
            assert_that!(maybe_idx.is_err(), otherwise "trait should not be found");
        }

        #[test]
        fn should_fail_get_trait_idx_by_ufc_wrong_method() {
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty = syn::parse_type("MyTrait<i32>").unwrap();
            let method_ident = syn::Ident::from("bar");

            let requested_traits_for_mock = vec![trait_ty.clone()];
            let mockable_traits = define_mockable_traits(&["trait MyTrait<T> { fn foo(); }"]);

            // when
            let maybe_idx = determine_requested_trait_idx_by_ufc(&trait_ty, &method_ident, &requested_traits_for_mock, &mockable_traits);
            // then
            assert_that!(maybe_idx.is_err(), otherwise "trait should not be found");
        }
    }

    mod trait_idx_by_method {
        use super::*;
        use super::super::*;

        #[test]
        fn should_get_trait_idx_by_method() {
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty1 = syn::parse_type("MyTrait1").unwrap();
            let trait_ty2 = syn::parse_type("MyTrait2").unwrap();
            let method_ident1 = syn::Ident::from("foo");
            let method_ident2 = syn::Ident::from("bar");

            let requested_traits_for_mock = vec![trait_ty1.clone(), trait_ty2.clone()];
            let mockable_traits = define_mockable_traits(&[
                "trait MyTrait1 { fn foo(); }",
                "trait MyTrait2 { fn bar(); }"
            ]);

            //when
            let maybe_idx1 = determine_requested_trait_idx_by_method(&method_ident1, &requested_traits_for_mock, &mockable_traits);
            let maybe_idx2 = determine_requested_trait_idx_by_method(&method_ident2, &requested_traits_for_mock, &mockable_traits);
            //then
            assert_that!(&maybe_idx1, maybe_ok(eq(0)));
            assert_that!(&maybe_idx2, maybe_ok(eq(1)));
        }

        #[test]
        fn should_get_trait_idx_by_method_ambiguous_method() {
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty1 = syn::parse_type("MyTrait1").unwrap();
            let trait_ty2 = syn::parse_type("MyTrait2").unwrap();
            let method_ident = syn::Ident::from("foo");

            let requested_traits_for_mock = vec![trait_ty1.clone(), trait_ty2.clone()];
            let mockable_traits = define_mockable_traits(&[
                "trait MyTrait1 { fn foo(); }",
                "trait MyTrait2 { fn foo(); }"
            ]);

            //when
            let maybe_idx = determine_requested_trait_idx_by_method(&method_ident, &requested_traits_for_mock, &mockable_traits);
            //then
            assert_that!(maybe_idx.is_err(), otherwise "ambiguous method name should not map to an index");
        }
    }
}
