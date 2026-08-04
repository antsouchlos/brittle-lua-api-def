use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::ClassDef;

pub fn generate(class: &ClassDef, crate_path: &syn::Path) -> TokenStream {
    let ident = &class.ident;
    let lua_name = &class.lua_name;

    let field_conversions: Vec<TokenStream> = class
        .fields
        .iter()
        .map(|field| {
            let field_ident = &field.ident;
            let lua_field_name = &field.lua_name;
            let field_name_str = field_ident.to_string();

            if field.skip {
                let default_expr = if let Some(ref default) = field.default {
                    let expr: syn::Expr =
                        syn::parse_str(default).expect("invalid default expression");
                    quote! { #expr }
                } else {
                    quote! { Default::default() }
                };

                return quote! {
                    #field_ident: #default_expr,
                };
            }

            let field_name_literal = field_name_str.clone();
            let struct_name = lua_name.clone();

            quote! {
                #field_ident: {
                    if !__table.contains_key(#lua_field_name)? {
                        return Err(#crate_path::field_missing(#struct_name, #field_name_literal));
                    }
                    let __raw: #crate_path::mlua::Value = __table.get(#lua_field_name)?;
                    #crate_path::mlua::FromLua::from_lua(__raw, __lua)
                        .map_err(|e| #crate_path::field_type_mismatch(
                            #struct_name,
                            #field_name_literal,
                            e.to_string(),
                        ))?
                },
            }
        })
        .collect();

    quote! {
        impl #crate_path::mlua::FromLua for #ident {
            fn from_lua(
                __value: #crate_path::mlua::Value,
                __lua: &#crate_path::mlua::Lua,
            ) -> #crate_path::mlua::Result<Self> {
                let __table: #crate_path::mlua::Table = match __value {
                    #crate_path::mlua::Value::Table(t) => t,
                    _ => {
                        return Err(#crate_path::mlua::Error::external(format!(
                            "expected table for `{}`, got {}",
                            stringify!(#ident),
                            match __value {
                                #crate_path::mlua::Value::Nil => "nil".to_string(),
                                v => format!("{:?}", v),
                            }
                        )));
                    }
                };

                Ok(#ident {
                    #(#field_conversions)*
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::FieldDef;
    use proc_macro2::Span;
    use syn::parse_quote;

    #[test]
    fn test_simple_deserialization() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![FieldDef {
                ident: parse_quote! { x },
                lua_name: "x".to_string(),
                ty: parse_quote! { String },
                doc: None,
                default: None,
                skip: false,
                span: Span::call_site(),
            }],
            methods: vec![],
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("contains_key (\"x\")"));
        assert!(output.contains("FromLua :: from_lua"));
    }

    #[test]
    fn test_skipped_field_with_default() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![FieldDef {
                ident: parse_quote! { bar },
                lua_name: "bar".to_string(),
                ty: parse_quote! { i32 },
                doc: None,
                default: Some("42".to_string()),
                skip: true,
                span: Span::call_site(),
            }],
            methods: vec![],
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(!output.contains("contains_key"));
        assert!(output.contains("42"));
    }

    #[test]
    fn test_non_table_input_error() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![],
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("Value :: Table"));
    }
}
