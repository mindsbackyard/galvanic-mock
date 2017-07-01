use syn;
use quote;

use std::collections::HashMap;
use quote::ToTokens;

use data::*;
use util::type_name_of;
use std::rc::Rc;


type ItemTokens = quote::Tokens;
type ImplTokens = quote::Tokens;

/// Generates all mock structs and implementations.
pub fn handle_generate_mocks() -> Vec<(ItemTokens, Vec<ImplTokens>)> {
    let requested_traits = acquire!(RequestedTraits);
    let given_blocks_per_type = acquire!(GivenBlocks);

    let mut tokens = Vec::new();
    for (mock_type_name, requested_traits) in requested_traits.iter() {
        let given_blocks: &[GivenBlockInfo]= given_blocks_per_type.get(mock_type_name).unwrap();
        tokens.push(handle_generate_mock(mock_type_name, requested_traits, given_blocks));
    }
    requested_traits.clear();

    tokens
}

/// Maps type parameters occuring in a generic trait definitions to types.
#[derive(Clone, Debug)]
struct TypeParamMapper {
    type_param_and_type: Vec<(String, String)>
}

impl TypeParamMapper {
    pub fn new() -> TypeParamMapper {
        TypeParamMapper { type_param_and_type: Vec::new() }
    }

    pub fn add_mapping(&mut self, param: syn::Ident, ty: syn::Ty) {
        self.type_param_and_type.push((param.to_string(), quote!{ #ty }.to_string()));
    }

    /// Creates an instantiated/full type from `quote::Tokens` representing a generic type.
    ///
    /// The stored type parameter mappings a used to replace the type parameters
    /// in the generic type, e.g., `A -> i32` instantiates `MyFoo<A>` to `MyFoo<int>`.
    /// Note that the type parameters may not occur on the first level,
    /// e.g., `MyFoo<Vec<A>>` maps to `MyFoo<Vec<A>>`.
    ///
    /// Note that this uses a heuristic. The algorithm greedily replaces all
    /// whitespace separated occurances of a known type parameter with
    /// the associated type.
    ///
    /// # Panics
    /// Panics is the instantiated type cannot be parsed.`
    pub fn instantiate_from_ty_token(&self, generic_ty_tokens: &quote::Tokens) -> syn::Ty {
        syn::parse::ty(
            &generic_ty_tokens.to_string().split_whitespace()
                .map(|x| {
                    for &(ref param, ref ty) in &self.type_param_and_type {
                        if x == param { return ty.to_string(); }
                    }
                    x.to_string()
                }).collect::<String>()
        ).expect(&format!("Unable to instantiate generic type {:?} with: {:?}",
                          generic_ty_tokens, self.type_param_and_type
        ))
    }

    /// Creates an instantiated/full type from a generic type.
    pub fn instantiate_from_ty(&self, generic_ty: &syn::Ty) -> syn::Ty {
        self.instantiate_from_ty_token(&quote!{ #generic_ty })
    }
}

fn extract_parameterized_types_from_trait_use(trait_ty: &syn::Path) -> Vec<syn::Ty> {
    match trait_ty.segments[0].parameters {
        syn::PathParameters::AngleBracketed(ref data) => data.types.clone(),
        _ => panic!("Type parameter extraction only works for angle-bracketed types.")
    }
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
fn handle_generate_mock(mock_type_name: &syn::Ident, trait_tys: &[syn::Ty], given_block_infos_for_mock: &[GivenBlockInfo])
                        -> (ItemTokens, Vec<ImplTokens>) {
    let mockable_traits = acquire!(MockableTraits);
    let mockable_traits = acquire!(MockableTraits);

    for block in given_block_infos_for_mock {
        let mut stmts_per_trait = HashMap::<usize, Vec<GivenStatement>>::new();
        for stmt in block.given_statements {
            let maybe_trait_idx = match stmt.maybe_ufc_trait {
                None => determine_requested_trait_idx_by_method(mock_type_name, &stmt.method),
                Some(ufc_trait_ty) => determine_requested_trait_idx_by_ufc(mock_type_name, &ufc_trait_ty, &stmt.method)
            };
        }
    }

    let mut trait_infos = Vec::new();
    let mut mappers = Vec::new();
    let mut impl_tokens = Vec::new();
    // generate impls for each requested trait
    for (trait_id, trait_) in trait_tys.iter().enumerate() {
        match trait_ {
            &syn::Ty::Path(_, ref p) => {
                if p.segments.len() != 1 {
                    panic!("All mocked traits are supposed to be given without path by their name only.");
                }

                let trait_name = &p.segments[0].ident;
                let trait_info = mockable_traits.get(&trait_name)
                                                .expect("All mocked traits must be defined using 'define_mock!'");

                let mut mapper = TypeParamMapper::new();
                {
                    let generics: &syn::Generics = &trait_info.generics;
                    let instantiated_params = extract_parameterized_types_from_trait_use(p);

                    for (param, instantiated) in generics.ty_params.iter().zip(instantiated_params) {
                        mapper.add_mapping(param.ident.clone(), instantiated);
                    }

                    let trait_implementer = TraitImplementer::for_(mock_type_name, trait_id, trait_, &trait_info, &mapper);
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
        mock_type_name, trait_tys, &trait_infos, given_block_infos
    );

    let (mock_struct_tokens, mock_impl_tokens) = mock_implementer.implement();
    impl_tokens.push(mock_impl_tokens);
    (mock_struct_tokens, impl_tokens)
}

pub fn determine_requested_trait_idx_by_method(mock_ty_name: &syn::Ident, method_ident: &syn::Ident) -> Result<usize, String> {
    let mut maybe_trait_idx: Option<usize> = None;

    get_singleton!(requested_traits of RequestedTraits);
    let trait_tys = try!(requested_traits.get(&mock_ty_name).ok_or_else(
        || format!("No mock type `{}` is known.", mock_ty_name)
    ));

    let method_name = method_ident.to_string();
    get_singleton!(mockable_traits of MockableTraits);
    for (idx, trait_ty) in trait_tys.into_iter().enumerate() {
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

pub fn determine_requested_trait_idx_by_ufc(mock_ty_name: &syn::Ident, ufc_trait_ty: &syn::Ty, method_ident: &syn::Ident) -> Result<usize, String> {
    let method_name = method_ident.to_string();

    get_singleton!(requested_traits of RequestedTraits);
    let trait_tys = try!(requested_traits.get(&mock_ty_name).ok_or_else(
        || format!("No mock type `{}` is known.", mock_ty_name)
    ));

    let ufc_trait_idx = try!(trait_tys.iter().position(|ty| ty == ufc_trait_ty).ok_or_else(
        || format!("No requested trait for mock type `{}` matches the given UFC trait `{:?}`.", mock_ty_name, ufc_trait_ty)
    ));

    let trait_name = try!(type_name_of(&ufc_trait_ty).ok_or_else(
        || format!("Unable to extract trait name from `{:?}`", ufc_trait_ty)
    ));

    get_singleton!(mockable_traits of MockableTraits);
    let trait_info = try!(mockable_traits.get(&trait_name).ok_or_else(
        || format!("`{:?}` is not a mockable trait. Has it been defined with `define_mock!`?", ufc_trait_ty)
    ));

    try!(trait_info.get_method_by_name(&method_name).ok_or_else(
        || format!("Unable to process `given!` statement. No method `{}` found for requested trait `{:?}`",
                   method_name, ufc_trait_ty)
    ));

    println!("{:?} {:?}", ufc_trait_idx, requested_traits.iter().collect::<Vec<_>>());
    // println!("{:?}", mockable_traits);
    Ok(ufc_trait_idx)
}


fn behaviour_field_for(block_id: usize, stmt_id: usize) -> syn::Ident {
    syn::Ident::from(format!("behaviour{}_of_block{}", stmt_id, block_id))
}

fn bound_field_for(block_id: usize) -> syn::Ident {
    syn::Ident::from(format!("bound{}", block_id))
}

fn activate_given_block_for(block_id: usize) -> syn::Ident {
    syn::Ident::from(format!("activate_given_block_{}", block_id))
}

fn given_block_activated_field_for(block_id: usize) -> syn::Ident {
    syn::Ident::from(format!("given_block_{}_activated", block_id))
}

/// Generates mock structs and implementations.
struct MockStructImplementer<'a> {
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
    fn implement(&self) -> (ItemTokens, ImplTokens) {
        let mock_type_name = &self.mock_type_name;

        let behaviour_names = self.given_block_infos.iter().flat_map(|info| info.given_statements.iter().map(|stmt| {
            behaviour_field_for(info.block_id, stmt.stmt_id)
        })).collect::<Vec<_>>();
        let bound_names = self.given_block_infos.iter().map(|info|{
            bound_field_for(info.block_id)
        }).collect::<Vec<_>>();

        let bound_types = self.given_block_infos.iter().map(|info|{
            syn::Ident::from(format!("Binding{}", info.block_id))
        }).collect::<Vec<_>>();
        let set_bound_values_for_given_blocks = self.given_block_infos.iter().map(|info|
            syn::Ident::from(format!("set_bound_value_for_block_{}", info.block_id))
        ).collect::<Vec<_>>();

        let given_block_flags = self.given_block_infos.iter().map(|info|
            given_block_activated_field_for(info.block_id)
        ).collect::<Vec<_>>();
        let activate_given_blocks = self.given_block_infos.iter().map(|info|
            self.implement_activate_given_block(info)
        ).collect::<Vec<_>>();

        (quote! {
            struct #mock_type_name {
                #(#behaviour_names: usize),*,
                #(#bound_names: Option<#bound_types>),*
            }
         }, quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    #mock_type_name {
                        #(#behaviour_names: 0),*,
                        #(#bound_names: None),*,
                        #(#given_block_flags: false),*
                    }
                }

                #(pub fn #set_bound_values_for_given_blocks(&mut self, bound: #bound_types) {
                    self.#bound_names = Some(bound);
                })*

                #(#activate_given_blocks)*
            }
        })
    }

    fn implement_activate_given_block(&self, block: &GivenBlockInfo) -> quote::Tokens {
        let activate_given_block = activate_given_block_for(block.block_id);
        let given_block_flag = given_block_activated_field_for(block.block_id);

        let behaviour_names = block.given_statements.iter().map(|stmt| {
            behaviour_field_for(block.block_id, stmt.stmt_id)
        }).collect::<Vec<_>>();

        quote! {
            pub fn #activate_given_block(&mut self) {
                self.#given_block_flag = true;
                (self.#behaviour_names = 0);*
            }
        }
    }
}


struct TraitImplementer<'a> {
    mock_type_name: &'a syn::Ident,
    requested_trait_type: &'a syn::Ty,
    trait_info: &'a TraitInfo,
    mapper: &'a TypeParamMapper,
    given_blocks: &'a [GivenBlockInfo]
}

impl<'a> TraitImplementer<'a> {
    pub fn for_(mock_type_name: &'a syn::Ident, trait_id: usize, requested_trait_type: &'a syn::Ty,
                trait_info: &'a TraitInfo, mapper: &'a TypeParamMapper, given_blocks_for_trait: &'a [GivenBlockInfo])
                -> TraitImplementer<'a>  {
        TraitImplementer {
            mock_type_name: mock_type_name,
            requested_trait_type: requested_trait_type,
            trait_info: trait_info,
            mapper: mapper,
            given_blocks: given_blocks_for_trait
        }
    }

    fn implement(&self) -> quote::Tokens {
        let methods: Vec<_> = self.trait_info.items.iter().flat_map(|item|
                                  self.implement_mocked_method(item).into_iter()
                              ).collect();

        let struct_lifetime = syn::Lifetime::new("'a");
        //let lifetimes = vec![struct_lifetime.clone()];
        let lifetimes: Vec<syn::Lifetime> = Vec::new();

        let mock_type_name = self.mock_type_name.clone();
        let trait_ty = self.requested_trait_type.clone();
        // all generic type parameters need to be bound so only lifetimes must be provided
        //TODO add #lifetimes and bind lifetimes into trait_ty, maybe provide lifetime for mock_type_name
        quote! {
            //impl<#(#lifetimes),*> #trait_ty for #mock_type_name<#struct_lifetime> {
            impl<#(#lifetimes),*> #trait_ty for #mock_type_name{
                #(#methods)*
            }
        }
    }

    fn implement_mocked_method(&self, item: &syn::TraitItem) -> Option<quote::Tokens> {
        let mut tokens = quote::Tokens::new();
        if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
            if !signature.generics.ty_params.is_empty() {
                panic!("Generic methods are not supported yet.")
            }

            let func_name = &item.ident;

            // generate fn signature/header
            signature.constness.to_tokens(&mut tokens);
            signature.unsafety.to_tokens(&mut tokens);
            signature.abi.to_tokens(&mut tokens);
            tokens.append("fn");
            func_name.to_tokens(&mut tokens);
            signature.generics.to_tokens(&mut tokens);
            tokens.append("(");

            // rewrite argument patterns to be unit and instantiate generic argument types
            let mut arg_idx = 1;
            let args = signature.decl.inputs.iter().map(|arg| {
                let arg_name = quote!(arg#arg_idx);
                match arg {
                    &syn::FnArg::Captured(_, ref ty) => {
                        let inst_ty = self.mapper.instantiate_from_ty(ty);
                        arg_idx += 1;
                        quote!(#arg_name: #inst_ty)
                    },
                    &syn::FnArg::Ignored(ref ty) => {
                        let inst_ty = self.mapper.instantiate_from_ty(ty);
                        arg_idx += 1;
                        quote!(#arg_name: #inst_ty)
                    }
                    _ => quote!(#arg)
            }}).collect::<Vec<_>>();

            tokens.append_separated(&args, ",");
            tokens.append(")");
            if let syn::FunctionRetTy::Ty(ref ty) = signature.decl.output {
                tokens.append("->");
                ty.to_tokens(&mut tokens);
            }
            signature.generics.where_clause.to_tokens(&mut tokens);
            tokens.append("{");

            if let syn::FunctionRetTy::Ty(ref return_ty) = signature.decl.output {
                let args = self.generate_argument_names(&signature.decl.inputs);

                // self.given_blocks.iter().map(|stmt| stmt)
                //
                // tokens.append(&quote! {
                //     let curried_args = (#(#args),*);
                //     (if  {
                //
                //     })*
                // }.to_string());
            }

            tokens.append("}");
            return Some(tokens);
        }
        None
    }

    fn generate_argument_names(&self, func_inputs: &[syn::FnArg]) -> Vec<syn::Ident> {
        let mut arg_names = Vec::new();
        let mut arg_idx = 1;
        for arg in func_inputs {
            match arg {
                &syn::FnArg::Captured(..) | &syn::FnArg::Ignored(..) => {
                    arg_names.push(syn::Ident::from(format!("arg{}", arg_idx)));
                    arg_idx += 1;
                },
                _ => {}
            }
        }
        arg_names
    }

    fn implement_given_block(&self, block: &GivenBlockInfo, func_args: &[syn::Ident]) -> quote::Tokens {
        let bound_field = bound_field_for(block.block_id);
        let activated_field = given_block_activated_field_for(block.block_id);

        let behaviours = Vec::new();
        for stmt in &block.given_statements {
            let behaviour_field = behaviour_field_for(block.block_id, stmt.stmt_id);
            let match_expr = match stmt.matcher {
                BehaviourMatcher::Explicit(ref expr) => {
                    quote!{ (#expr)(#(&#func_args),*) }
                },
                BehaviourMatcher::PerArgument(ref exprs) => {
                    let mut arg_tokens = quote::Tokens::new();
                    for idx in 0..func_args.len() {
                        if idx >= 1 {
                            arg_tokens.append("&&");
                        }
                        let expr = exprs.get(idx).unwrap();
                        arg_tokens.append(quote!((#expr)));
                        let arg = func_args.get(idx).unwrap();
                        arg_tokens.append(quote!((&#arg)));
                    }
                    arg_tokens
                }
            };

            let return_expr = match stmt.return_stmt {
                Return::FromValue(expr) => quote!{ return #expr },
                Return::FromCall(expr) => quote!{ return (#expr)(#(&#func_args),*) },
                Return::FromSpy => panic!("return_from_spy is not implemented yet."),
                Return::Panic => quote!{ panic!("Don't forget the towel.") }
            };

            let behaviour = match stmt.repeat {
                Repeat::Always => quote! {
                    if #match_expr.into() {
                        #return_expr;
                    }
                },
                Repeat::Times(..) => quote! {
                    let (num_matches, bound) = self.#behaviour_field.get();
                    if num_matches < bound.unwrap() && #match_expr.into() {
                        self.#behaviour_field.set((num_matches+1, bound));
                        #return_expr;
                    }
                }
            };
            behaviours.push(behaviour);
        }

        quote! {
            if #activated_field {
                let bound = self.#bound_field;
                #(#behaviours)*
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use galvanic_assert::*;
    use galvanic_assert::matchers::*;
    use galvanic_assert::matchers::variant::*;

    fn define_mockable_trait(trait_source: &str) {
        let trait_item = syn::parse_item(trait_source).unwrap();
        match trait_item.node {
            syn::ItemKind::Trait(safety, generics, bounds, items) => {
                get_singleton_mut!(mockable_traits of MockableTraits);
                mockable_traits.insert(trait_item.ident.clone(), TraitInfo::new(safety, generics, bounds, items));
            },
            _ => panic!("Expecting a trait definition")
        }
    }

    fn reset_globals() {
        {
            get_singleton_mut!(requested_traits of RequestedTraits);
            requested_traits.clear();
        }
        {
            get_singleton_mut!(mockable_traits of MockableTraits);
            mockable_traits.clear();
        }
    }

    mod trait_idx_by_ufc {
        use super::*;
        use super::super::*;
        use super::super::super::*;

        #[test]
        fn should_get_trait_idx_by_ufc() {
            reset_globals();
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty1 = syn::parse_type("MyTrait<i32>").unwrap();
            let trait_ty2 = syn::parse_type("MyTrait<f32>").unwrap();
            let method_ident = syn::Ident::from("foo");
            {
                get_singleton_mut!(requested_traits of RequestedTraits);
                requested_traits.insert(mock_ty_name.clone(), vec![trait_ty1.clone(), trait_ty2.clone()]);
                define_mockable_trait("trait MyTrait<T> { fn foo(); }");
            }
            //when
            let maybe_idx1 = determine_requested_trait_idx_by_ufc(&mock_ty_name, &trait_ty1, &method_ident);
            let maybe_idx2 = determine_requested_trait_idx_by_ufc(&mock_ty_name, &trait_ty2, &method_ident);
            //then
            assert_that!(&maybe_idx1, maybe_ok(eq(0)));
            assert_that!(&maybe_idx2, maybe_ok(eq(1)));
        }

        #[test]
        fn should_fail_get_trait_idx_by_ufc_unmockable_trait() {
            reset_globals();
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty = syn::parse_type("MyTrait<i32>").unwrap();
            let method_ident = syn::Ident::from("foo");
            {
                get_singleton_mut!(requested_traits of RequestedTraits);
                requested_traits.insert(mock_ty_name.clone(), vec![trait_ty.clone()]);
            }
            // when
            let maybe_idx = determine_requested_trait_idx_by_ufc(&mock_ty_name, &trait_ty, &method_ident);
            // then
            assert_that!(maybe_idx.is_err(), otherwise "trait should not be found");
        }

        #[test]
        fn should_fail_get_trait_idx_by_wrong_ufc() {
            reset_globals();
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty = syn::parse_type("MyTrait<i32>").unwrap();
            let method_ident = syn::Ident::from("foo");
            {
                get_singleton_mut!(requested_traits of RequestedTraits);
                requested_traits.insert(mock_ty_name.clone(), vec![]);
                define_mockable_trait("trait MyTrait<T> { fn foo(); }");
            }
            // when
            let maybe_idx = determine_requested_trait_idx_by_ufc(&mock_ty_name, &trait_ty, &method_ident);
            // then
            assert_that!(maybe_idx.is_err(), otherwise "trait should not be found");
        }

        #[test]
        fn should_fail_get_trait_idx_by_ufc_wrong_method() {
            reset_globals();
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty = syn::parse_type("MyTrait<i32>").unwrap();
            let method_ident = syn::Ident::from("bar");
            {
                get_singleton_mut!(requested_traits of RequestedTraits);
                requested_traits.insert(mock_ty_name.clone(), vec![trait_ty.clone()]);
                define_mockable_trait("trait MyTrait<T> { fn foo(); }");
            }
            // when
            let maybe_idx = determine_requested_trait_idx_by_ufc(&mock_ty_name, &trait_ty, &method_ident);
            // then
            assert_that!(maybe_idx.is_err(), otherwise "trait should not be found");
        }
    }

    mod trait_idx_by_method {
        use super::*;
        use super::super::*;
        use super::super::super::*;

        #[test]
        fn should_get_trait_idx_by_method() {
            reset_globals();
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty1 = syn::parse_type("MyTrait1").unwrap();
            let trait_ty2 = syn::parse_type("MyTrait2").unwrap();
            let method_ident1 = syn::Ident::from("foo");
            let method_ident2 = syn::Ident::from("bar");
            {
                get_singleton_mut!(requested_traits of RequestedTraits);
                requested_traits.insert(mock_ty_name.clone(), vec![trait_ty1.clone(), trait_ty2.clone()]);
                define_mockable_trait("trait MyTrait1 { fn foo(); }");
                define_mockable_trait("trait MyTrait2 { fn bar(); }");
            }
            //when
            let maybe_idx1 = determine_requested_trait_idx_by_method(&mock_ty_name, &method_ident1);
            let maybe_idx2 = determine_requested_trait_idx_by_method(&mock_ty_name, &method_ident2);
            //then
            assert_that!(&maybe_idx1, maybe_ok(eq(0)));
            assert_that!(&maybe_idx2, maybe_ok(eq(1)));
        }

        #[test]
        fn should_get_trait_idx_by_method_ambiguous_method() {
            reset_globals();
            // given
            let mock_ty_name = syn::Ident::from("MyMock");
            let trait_ty1 = syn::parse_type("MyTrait1").unwrap();
            let trait_ty2 = syn::parse_type("MyTrait2").unwrap();
            let method_ident = syn::Ident::from("foo");
            {
                get_singleton_mut!(requested_traits of RequestedTraits);
                requested_traits.insert(mock_ty_name.clone(), vec![trait_ty1.clone(), trait_ty2.clone()]);
                define_mockable_trait("trait MyTrait1 { fn foo(); }");
                define_mockable_trait("trait MyTrait2 { fn foo(); }");
            }
            //when
            let maybe_idx = determine_requested_trait_idx_by_method(&mock_ty_name, &method_ident);
            //then
            assert_that!(maybe_idx.is_err(), otherwise "ambiguous method name should not map to an index");
        }
    }
}
