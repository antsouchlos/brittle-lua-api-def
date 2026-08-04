use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::ModuleDef;

pub fn generate(module: &ModuleDef, crate_path: &syn::Path) -> TokenStream {
    let class_registrations: Vec<TokenStream> = module
        .classes
        .iter()
        .map(|class| {
            let lua_name = &class.lua_name;
            let ident = &class.ident;

            quote! {
                __lua.globals().set(#lua_name, __lua.create_proxy::<#ident>()?)?;
            }
        })
        .collect();

    let function_registrations: Vec<TokenStream> = module
        .functions
        .iter()
        .map(|func| {
            let lua_name = &func.lua_name;
            let fn_ident = &func.ident;
            let arg_names: Vec<_> = func.args.iter().map(|(n, _)| n).collect();
            let arg_types: Vec<_> = func.args.iter().map(|(_, t)| t).collect();
            let arg_count = func.args.len();

            let wrap_return = if func.is_fallible {
                quote! { #fn_ident(#(#arg_names),*).map_err(#crate_path::mlua::Error::external) }
            } else {
                quote! { Ok(#fn_ident(#(#arg_names),*)) }
            };

            if arg_count == 0 {
                quote! {
                    __lua.globals().set(#lua_name, __lua.create_function(|__lua, (): ()| {
                        #wrap_return
                    })?)?;
                }
            } else {
                quote! {
                    __lua.globals().set(#lua_name, __lua.create_function(|__lua, (#(#arg_names,)*): (#(#arg_types,)*)| {
                        #wrap_return
                    })?)?;
                }
            }
        })
        .collect();

    quote! {
        pub fn register(__lua: &#crate_path::mlua::Lua) -> #crate_path::mlua::Result<()> {
            #(#class_registrations)*
            #(#function_registrations)*
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ClassDef, FnDef};
    use proc_macro2::Span;
    use syn::parse_quote;

    #[test]
    fn test_register_classes() {
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![],
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&module, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("create_proxy"));
        assert!(output.contains("MyClass"));
    }

    #[test]
    fn test_register_functions() {
        let func = FnDef {
            ident: parse_quote! { greet },
            lua_name: "greet".to_string(),
            doc: None,
            args: vec![],
            ret: syn::ReturnType::Default,
            is_fallible: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![],
            functions: vec![func],
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&module, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("create_function"));
    }
}
