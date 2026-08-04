use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::ModuleDef;

mod emmylua;
mod from_lua;
mod register;
mod type_mapper;
mod userdata;

pub fn generate(module: &ModuleDef, item_mod: &syn::ItemMod, crate_path_str: &str) -> TokenStream {
    let crate_path: syn::Path =
        syn::parse_str(crate_path_str).unwrap_or_else(|_| syn::parse_str("::brittle_lua").unwrap());

    let attrs = &item_mod.attrs;
    let vis = &item_mod.vis;
    let ident = &item_mod.ident;

    let inside_items = make_inside_items(module, item_mod, &crate_path);

    quote! {
        #(#attrs)* #vis mod #ident {
            #(#inside_items)*
        }
    }
}

fn make_inside_items(
    module: &ModuleDef,
    item_mod: &syn::ItemMod,
    crate_path: &syn::Path,
) -> Vec<TokenStream> {
    let mut inside = Vec::new();

    let original_items = get_original_items(item_mod);
    inside.extend(original_items);

    for class in &module.classes {
        inside.push(self::userdata::generate(class, crate_path));
        inside.push(self::from_lua::generate(class, crate_path));
    }

    inside.push(self::register::generate(module, crate_path));
    inside.push(self::emmylua::generate(module));

    inside
}

fn get_original_items(item_mod: &syn::ItemMod) -> Vec<TokenStream> {
    let items = match &item_mod.content {
        Some((_, items)) => items,
        None => return vec![],
    };

    items
        .iter()
        .map(|item| {
            let is_lua_item = match item {
                syn::Item::Struct(s) => s.attrs.iter().any(|a| a.path().is_ident("luaclass")),
                syn::Item::Impl(i) => i.attrs.iter().any(|a| a.path().is_ident("luamethods")),
                syn::Item::Fn(f) => f.attrs.iter().any(|a| a.path().is_ident("luafunction")),
                _ => false,
            };

            if is_lua_item {
                strip_lua_attrs(item)
            } else {
                quote! { #item }
            }
        })
        .collect()
}

fn strip_lua_attrs(item: &syn::Item) -> TokenStream {
    match item {
        syn::Item::Struct(s) => {
            let attrs: Vec<_> = s
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("luaclass"))
                .collect();
            let vis = &s.vis;
            let ident = &s.ident;
            let generics = &s.generics;
            let fields: Vec<_> = s
                .fields
                .iter()
                .map(|f| {
                    let f_attrs: Vec<_> = f
                        .attrs
                        .iter()
                        .filter(|a| !a.path().is_ident("lua"))
                        .collect();
                    let ident = &f.ident;
                    let ty = &f.ty;
                    quote! { #(#f_attrs)* #vis #ident: #ty }
                })
                .collect();

            quote! {
                #(#attrs)*
                #vis struct #ident #generics {
                    #(#fields,)*
                }
            }
        }
        syn::Item::Impl(i) => {
            let attrs: Vec<_> = i
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("luamethods"))
                .collect();
            let self_ty = &i.self_ty;
            let items: Vec<_> = i
                .items
                .iter()
                .map(|item| {
                    if let syn::ImplItem::Fn(f) = item {
                        let f_attrs: Vec<_> = f
                            .attrs
                            .iter()
                            .filter(|a| !a.path().is_ident("lua"))
                            .collect();
                        let vis = &f.vis;
                        let sig = &f.sig;
                        let block = &f.block;
                        quote! { #(#f_attrs)* #vis #sig #block }
                    } else {
                        quote! { #item }
                    }
                })
                .collect();

            quote! {
                #(#attrs)*
                impl #self_ty {
                    #(#items)*
                }
            }
        }
        syn::Item::Fn(f) => {
            let attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("luafunction"))
                .collect();
            let vis = &f.vis;
            let sig = &f.sig;
            let block = &f.block;

            quote! {
                #(#attrs)*
                #vis #sig #block
            }
        }
        _ => quote! { #item },
    }
}
