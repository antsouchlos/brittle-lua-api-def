use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::{ClassDef, ReceiverKind};

pub fn generate(class: &ClassDef, crate_path: &syn::Path) -> TokenStream {
    let ident = &class.ident;

    let field_getters_setters: Vec<TokenStream> = class
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|field| {
            let lua_name = &field.lua_name;
            let field_ident = &field.ident;

            quote! {
                __fields.add_field_method_get(#lua_name, |__lua, this| {
                    Ok(this.#field_ident.clone())
                });
                __fields.add_field_method_set(#lua_name, |__lua, this, __v| {
                    this.#field_ident = __v;
                    Ok(())
                });
            }
        })
        .collect();

    let method_registrations: Vec<TokenStream> = class
        .methods
        .iter()
        .map(|method| {
            let lua_name = &method.lua_name;
            let method_ident = &method.ident;
            let arg_names: Vec<_> = method.args.iter().map(|(n, _)| n).collect();
            let arg_types: Vec<_> = method.args.iter().map(|(_, t)| t).collect();
            let arg_count = method.args.len();

            match &method.receiver {
                ReceiverKind::None => {
                    let wrap_return = if method.is_fallible {
                        quote! { Self::#method_ident(#(#arg_names),*).map_err(#crate_path::mlua::Error::external) }
                    } else {
                        quote! { Ok(Self::#method_ident(#(#arg_names),*)) }
                    };

                    if arg_count == 0 {
                        quote! {
                            __methods.add_function(#lua_name, |__lua, (): ()| {
                                #wrap_return
                            });
                        }
                    } else {
                        quote! {
                            __methods.add_function(#lua_name, |__lua, (#(#arg_names,)*): (#(#arg_types,)*)| {
                                #wrap_return
                            });
                        }
                    }
                }
                ReceiverKind::Ref => {
                    let wrap_return = if method.is_fallible {
                        quote! { Self::#method_ident(this, #(#arg_names),*).map_err(#crate_path::mlua::Error::external) }
                    } else {
                        quote! { Ok(Self::#method_ident(this, #(#arg_names),*)) }
                    };

                    if arg_count == 0 {
                        quote! {
                            __methods.add_method(#lua_name, |__, this, (): ()| {
                                #wrap_return
                            });
                        }
                    } else {
                        quote! {
                            __methods.add_method(#lua_name, |__, this, (#(#arg_names,)*): (#(#arg_types,)*)| {
                                #wrap_return
                            });
                        }
                    }
                }
                ReceiverKind::MutRef => {
                    let wrap_return = if method.is_fallible {
                        quote! { Self::#method_ident(this, #(#arg_names),*).map_err(#crate_path::mlua::Error::external) }
                    } else {
                        quote! { Ok(Self::#method_ident(this, #(#arg_names),*)) }
                    };

                    if arg_count == 0 {
                        quote! {
                            __methods.add_method_mut(#lua_name, |__, this, (): ()| {
                                #wrap_return
                            });
                        }
                    } else {
                        quote! {
                            __methods.add_method_mut(#lua_name, |__, this, (#(#arg_names,)*): (#(#arg_types,)*)| {
                                #wrap_return
                            });
                        }
                    }
                }
                ReceiverKind::Owned => {
                    let wrap_return = if method.is_fallible {
                        quote! {
                            let __clone = this.clone();
                            Self::#method_ident(__clone, #(#arg_names),*).map_err(#crate_path::mlua::Error::external)
                        }
                    } else {
                        quote! {
                            let __clone = this.clone();
                            Ok(Self::#method_ident(__clone, #(#arg_names),*))
                        }
                    };

                    if arg_count == 0 {
                        quote! {
                            __methods.add_method(#lua_name, |__, this, (): ()| {
                                #wrap_return
                            });
                        }
                    } else {
                        quote! {
                            __methods.add_method(#lua_name, |__, this, (#(#arg_names,)*): (#(#arg_types,)*)| {
                                #wrap_return
                            });
                        }
                    }
                }
            }
        })
        .collect();

    let add_methods_block = if method_registrations.is_empty() {
        quote! {}
    } else {
        quote! {
            fn add_methods<__M: #crate_path::mlua::UserDataMethods<Self>>(__methods: &mut __M) {
                #(#method_registrations)*
            }
        }
    };

    quote! {
        impl #crate_path::mlua::UserData for #ident {
            fn add_fields<__F: #crate_path::mlua::UserDataFields<Self>>(
                __fields: &mut __F,
            ) {
                #(#field_getters_setters)*
            }

            #add_methods_block
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FieldDef, MethodDef};
    use proc_macro2::Span;
    use syn::parse_quote;

    #[test]
    fn test_fields_output() {
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
                signature: None,
                skip: false,
                span: Span::call_site(),
            }],
            methods: vec![],
            annotation_only: false,
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("add_field_method_get (\"x\""));
        assert!(output.contains("add_field_method_set (\"x\""));
    }

    #[test]
    fn test_method_ref_output() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { foo },
                lua_name: "foo".to_string(),
                doc: None,
                receiver: ReceiverKind::Ref,
                args: vec![],
                ret: syn::ReturnType::Default,
                is_fallible: false,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("add_method (\"foo\""));
    }

    #[test]
    fn test_skipped_field_not_in_output() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![FieldDef {
                ident: parse_quote! { secret },
                lua_name: "secret".to_string(),
                ty: parse_quote! { String },
                doc: None,
                default: None,
                signature: None,
                skip: true,
                span: Span::call_site(),
            }],
            methods: vec![],
            annotation_only: false,
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(!output.contains("add_field_method"));
    }

    #[test]
    fn test_constructor_output() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { new },
                lua_name: "new".to_string(),
                doc: None,
                receiver: ReceiverKind::None,
                args: vec![(parse_quote! { port }, parse_quote! { u16 })],
                ret: parse_quote! { -> Self },
                is_fallible: true,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let crate_path: syn::Path = parse_quote! { test_crate };
        let tokens = generate(&class, &crate_path);
        let output = tokens.to_string();

        assert!(output.contains("add_function (\"new\""));
    }
}
