use syn;
use quote;

use std::collections::HashMap;
use quote::ToTokens;

use data::*;
use util::Singleton;
use std::rc::Rc;
use std::cell::RefCell;


type ItemTokens = quote::Tokens;
type ImplTokens = quote::Tokens;
pub fn handle_generate_mocks() -> Vec<(ItemTokens, ImplTokens)> {
    let singleton = MockVars::singleton();
    let gate = singleton.inner.lock();
    let mut mock_vars = gate.unwrap();

    let mut tokens = Vec::new();
    for &(ref mock_type_name, ref traits) in mock_vars.iter() {
        tokens.push(handle_generate_mock(mock_type_name, traits));
    }
    mock_vars.clear();

    tokens
}

#[derive(Clone)]
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

    pub fn instantiate_from_tokens(&self, generic_ty_tokens: &quote::Tokens) -> syn::Ty {
        println!("{:?}", generic_ty_tokens);
        syn::parse::ty(
            &generic_ty_tokens.to_string().split_whitespace()
                .map(|x| {
                    for &(ref param, ref ty) in &self.type_param_and_type {
                        if x == param { println!("Ty: {:?}", ty); return ty.to_string(); }
                    }
                    x.to_string()
                }).collect::<String>()
        ).expect(&format!("Unable to instantiate generic type {:?} with: {:?}",
                          generic_ty_tokens, self.type_param_and_type
        ))
    }

    pub fn instantiate_from_ty(&self, generic_ty: &syn::Ty) -> syn::Ty {
        self.instantiate_from_tokens(&quote!{ #generic_ty })
    }
}

fn extract_parameterized_types_from_trait_use(trait_ty: &syn::Path) -> Vec<syn::Ty> {
    match trait_ty.segments[0].parameters {
        syn::PathParameters::AngleBracketed(ref data) => data.types.clone(),
        _ => panic!("Type parameter extraction only works for angle-bracketed types.")
    }
}

fn handle_generate_mock(mock_type_name: &syn::Ident, trait_tys: &[syn::Ty]) -> (ItemTokens, ImplTokens) {
    let singleton = DefinedMocks::singleton();
    let gate = singleton.inner.lock();
    let mut defined_mocks = gate.unwrap();

    let mut trait_infos = Vec::new();
    let mut mappers = Vec::new();
    for trait_ in trait_tys {
        match trait_ {
            &syn::Ty::Path(_, ref p) => {
                if p.segments.len() != 1 {
                    panic!("All mocked traits are supposed to be given without path by their name only.");
                }

                let trait_name = &p.segments[0].ident;
                let trait_info = defined_mocks.get(&trait_name)
                                              .expect("All mocked traits must be defined using 'define_mock!'");

                let generics: &syn::Generics = &trait_info.generics;
                let instantiated_params = extract_parameterized_types_from_trait_use(p);

                let mut mapper = TypeParamMapper::new();
                for (param, instantiated) in generics.ty_params.iter().zip(instantiated_params) {
                    mapper.add_mapping(param.ident.clone(), instantiated);
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
        mock_type_name.clone(), trait_tys, &trait_infos, &mappers
    );
    mock_implementer.implement()
}

struct BehaviourFactory<'a> {
    trait_id: usize,
    mapper: &'a TypeParamMapper
}

impl<'a> BehaviourFactory<'a> {
    pub fn new(trait_id: usize, mapper: &'a TypeParamMapper) -> BehaviourFactory<'a> {
        BehaviourFactory {
            trait_id: trait_id,
            mapper: mapper
        }
    }

    pub fn behaviour_ty_for(&self, args: &[syn::FnArg], return_ty: &syn::Ty) -> quote::Tokens {
        let arg_tys = args.iter()
                          .filter_map(|arg| match arg {
                              &syn::FnArg::Captured(_, ref ty) => {
                                  Some(self.mapper.instantiate_from_ty(ty))
                              },
                              _ => None
                          }).collect::<Vec<_>>();

        let curried_args = quote!{ (#(#arg_tys),*) };
        let return_ty = self.mapper.instantiate_from_ty(return_ty);
        quote!{ galvanic_mock_lib::Behaviour<#curried_args, #return_ty> }
    }

    pub fn behaviour_collection_name(&self, method_name: &syn::Ident) -> syn::Ident {
        syn::Ident::from(format!("given_behaviours_for_trait{}_{}",
                                 self.trait_id,
                                 method_name
        ))
    }
}


struct MockStructImplementer<'a> {
    mock_type_name: syn::Ident,
    instantiated_trait_types: &'a [syn::Ty],
    trait_infos: &'a [&'a TraitInfo],
    behaviour_factories: Vec<BehaviourFactory<'a>>
}

impl<'a> MockStructImplementer<'a> {
    pub fn for_(mock_type_name: syn::Ident, instantiated_trait_types: &'a [syn::Ty], trait_infos: &'a [&'a TraitInfo], mappers: &'a [TypeParamMapper]) -> MockStructImplementer<'a>  {
        MockStructImplementer {
            mock_type_name: mock_type_name,
            instantiated_trait_types: instantiated_trait_types,
            trait_infos: trait_infos,
            behaviour_factories: mappers.into_iter().enumerate()
                                        .map(|(idx, m)| BehaviourFactory::new(idx, m))
                                        .collect()
        }
    }

    pub fn implement(&self) -> (ItemTokens, ImplTokens) {
        self.implement_struct()
    }

    fn implement_struct(&self) -> (ItemTokens, ImplTokens) {
        let mock_type_name = &self.mock_type_name;
        let fields = self.implement_given_behaviours_fields();
        let field_names = &fields.iter().map(|&(ref name, _)| name).collect::<Vec<_>>();
        let field_tys = &fields.iter().map(|&(_, ref ty)| ty).collect::<Vec<_>>();

        let add_given_behaviours = fields.iter().map(|&(ref name, _)|
            syn::Ident::from(format!("add_{}", name))
        ).collect::<Vec<_>>();

        (quote! {
            struct #mock_type_name {
                #(#field_names: Vec<#field_tys>),*
            }
         }, quote! {
            impl #mock_type_name {
                pub fn new() -> Self {
                    #mock_type_name {
                        #(#field_names: Vec::new()),*
                    }
                }

                #(pub fn #add_given_behaviours(&mut self, behaviour: #field_tys) {
                    self.#field_names.push(behaviour);
                })*
            }
        })
    }

    fn implement_given_behaviours_fields(&self) -> Vec<(syn::Ident, quote::Tokens)> {
        let mut fields = Vec::new();
        for (behaviour_factory, trait_info) in self.behaviour_factories.iter().zip(self.trait_infos.iter()) {
            for item in &trait_info.items {
                if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
                    match self.implement_given_behaviours_field_for(behaviour_factory, &item.ident, signature) {
                        Some(field_data) => fields.push(field_data),
                        None => {}
                    };
                }
            }
        }
        fields
    }

    fn implement_given_behaviours_field_for(&self,
                                            behaviour_factory: &BehaviourFactory,
                                            method_name: &syn::Ident,
                                            method_signature: &syn::MethodSig)
                                            -> Option<(syn::Ident, quote::Tokens)> {
        if !method_signature.generics.ty_params.is_empty() {
            panic!("Generic methods are not supported yet.")
        }

        if let syn::FunctionRetTy::Ty(ref return_ty) = method_signature.decl.output {
            let behaviours_field = behaviour_factory.behaviour_collection_name(method_name);
            let behaviour_type = behaviour_factory.behaviour_ty_for(
                &method_signature.decl.inputs, return_ty
            );
            Some((behaviours_field, behaviour_type))

        } else { None }
    }
}


// struct TraitImplementer<'a> {
//     mock_type_name: syn::Ident,
//     instantiated_trait_type: syn::Ty,
//     trait_info: TraitInfo,
//     mapper: &'a TypeParamMapper,
//     behaviour_factory: Rc<RefCell<BehaviourFactory<'a>>>
// }
//
// impl<'a> TraitImplementer<'a> {
//     pub fn for_(mock_type_name: syn::Ident, instantiated_trait_type: syn::Ty, trait_info: TraitInfo, mapper: &'a TypeParamMapper) -> TraitImplementer<'a>  {
//         TraitImplementer {
//             mock_type_name: mock_type_name,
//             instantiated_trait_type: instantiated_trait_type,
//             trait_info: trait_info,
//             mapper: mapper,
//             behaviour_factory: Rc::new(RefCell::new(BehaviourFactory::new(mapper)))
//         }
//     }
//
//     fn implement(&mut self) {
//         // quote! {
//         //     impl #lifetimes #trait_ty for #mock_type_name {
//         //         #(mocked_methods)*
//         //     }
//         // }
//     }
//
//     fn implement_mocked_method(&mut self, item: syn::TraitItem) {
//         if let &syn::TraitItemKind::Method(ref signature, _) = &item.node {
//             if !signature.generics.ty_params.is_empty() {
//                 panic!("Generic methods are not supported yet.")
//             }
//
//             let func_name = &item.ident;
//             let tokens = &mut quote::Tokens::new();
//
//             signature.constness.to_tokens(tokens);
//             signature.unsafety.to_tokens(tokens);
//             signature.abi.to_tokens(tokens);
//             tokens.append("fn");
//             func_name.to_tokens(tokens);
//             signature.generics.to_tokens(tokens);
//             tokens.append("(");
//             tokens.append_separated(&signature.decl.inputs, ",");
//             tokens.append(")");
//             if let syn::FunctionRetTy::Ty(ref ty) = signature.decl.output {
//                 tokens.append("->");
//                 ty.to_tokens(tokens);
//             }
//             signature.generics.where_clause.to_tokens(tokens);
//             tokens.append("{");
//
//             if let syn::FunctionRetTy::Ty(ref return_ty) = signature.decl.output {
//                 let given_behaviours = self.behaviour_factory.borrow_mut()
//                                            .behaviour_collection_name(&self.instantiated_trait_type, func_name);
//                 let args = signature.decl.inputs.iter()
//                               .filter_map(|arg| match arg {
//                                   &syn::FnArg::Captured(ref pat, _) => Some(pat.clone()),
//                                   _ => None
//                               }).collect::<Vec<_>>();
//
//                 tokens.append(&quote! {
//                     let curried_args = (#(args),*);
//                     let mut maybe_exhausted_idx: Option<usize> = None;
//                     let mut maybe_return_value: Option<#return_ty> = None
//                     for (idx, behaviour) in self.#given_behaviours.iter().enumerate() {
//                         maybe_return_value = behaviour.try_match(curried_args);
//                         if maybe_return_value.is_some()  {
//                             if behaviour.is_exhausted() {
//                                 maybe_exhausted_idx = Some(idx);
//                             }
//                             break;
//                         }
//                     }
//
//                     if let Some(return_value) = maybe_return_value {
//                         if let Some(exhausted_idx) = maybe_exhausted_idx {
//                             self.#given_behaviours.remove(exhausted_idx);
//                         }
//                         return return_value;
//                     }
//                     panic!("No 'given' behaviour satisfied for call {}", stringify!(#func_name));
//                 }.to_string());
//             }
//
//             tokens.append("}");
//         }
//     }
// }
