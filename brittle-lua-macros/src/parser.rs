use std::collections::BTreeMap;

use syn::Ident;

use crate::ir::{ClassDef, FieldDef, FnDef, MethodDef, ModuleDef, ReceiverKind};
use crate::util::{
    detect_fallibility, extract_doc, ClassAttrs, FieldAttrs, MethodAttrs, NewtypeAttrs,
};

pub fn parse(item_mod: &syn::ItemMod) -> syn::Result<ModuleDef> {
    let mut classes: Vec<ClassDef> = Vec::new();
    let mut methods_map: BTreeMap<String, Vec<MethodDef>> = BTreeMap::new();
    let mut functions: Vec<FnDef> = Vec::new();
    let mut globals: Vec<FnDef> = Vec::new();
    let mut type_map: BTreeMap<String, String> = BTreeMap::new();

    let items = match &item_mod.content {
        Some((_, items)) => items,
        None => {
            return Ok(ModuleDef {
                classes,
                functions,
                globals,
                type_map,
            });
        }
    };

    for item in items {
        match item {
            syn::Item::Struct(item_struct) => {
                if has_attr(&item_struct.attrs, "luaclass") {
                    let class = parse_class(item_struct)?;
                    classes.push(class);
                }

                if has_attr(&item_struct.attrs, "lua_newtype") {
                    let newtype = NewtypeAttrs::parse(&item_struct.attrs)?;
                    let rust_name = item_struct.ident.to_string();
                    let lua_type = newtype.lua_type.ok_or_else(|| {
                        syn::Error::new(
                            item_struct.ident.span(),
                            "#[lua_newtype] requires a `to` argument",
                        )
                    })?;

                    type_map.insert(rust_name, lua_type);
                }
            }
            syn::Item::Impl(item_impl) => {
                if has_attr(&item_impl.attrs, "luamethods") {
                    let (struct_ident, methods) = parse_methods_impl(item_impl)?;
                    let key = struct_ident.to_string();
                    methods_map.entry(key).or_default().extend(methods);
                }
            }
            syn::Item::Fn(item_fn) => {
                if has_attr(&item_fn.attrs, "luaglobal") {
                    globals.push(parse_global(item_fn)?);
                } else if has_attr(&item_fn.attrs, "luafunction") {
                    let func = parse_function(item_fn)?;
                    functions.push(func);
                }
            }
            _ => {}
        }
    }

    for class in &mut classes {
        let key = class.ident.to_string();
        if let Some(methods) = methods_map.remove(&key) {
            class.methods = methods;
        }
    }

    classes.sort_by_key(|class| class.ident.to_string());
    functions.sort_by_key(|function| function.ident.to_string());

    globals.sort_by(|left, right| left.lua_name.cmp(&right.lua_name));

    Ok(ModuleDef {
        classes,
        functions,
        globals,
        type_map,
    })
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}

fn parse_class(item_struct: &syn::ItemStruct) -> syn::Result<ClassDef> {
    let class_attrs = ClassAttrs::parse(&item_struct.attrs)?;
    let lua_name = class_attrs
        .name
        .unwrap_or_else(|| item_struct.ident.to_string());
    let doc = extract_doc(&item_struct.attrs);

    let mut fields = Vec::new();

    for field in &item_struct.fields {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "tuple struct fields not supported"))?;

        let field_attrs = FieldAttrs::parse(&field.attrs)?;

        if field_attrs.skip && field_attrs.rename.is_some() {
            return Err(syn::Error::new(
                field_ident.span(),
                "field is both renamed and skipped",
            ));
        }

        let lua_field_name = field_attrs
            .rename
            .unwrap_or_else(|| field_ident.to_string());

        let field_doc = extract_doc(&field.attrs);

        fields.push(FieldDef {
            ident: field_ident.clone(),
            lua_name: lua_field_name,
            field_type: field.ty.clone(),
            doc: field_doc,
            default: field_attrs.default,
            signature: field_attrs.signature,
            skip: field_attrs.skip,
            span: field_ident.span(),
        });
    }

    Ok(ClassDef {
        ident: item_struct.ident.clone(),
        lua_name,
        doc,
        fields,
        methods: Vec::new(),
        annotation_only: class_attrs.annotation_only,
        span: item_struct.ident.span(),
    })
}

fn parse_methods_impl(item_impl: &syn::ItemImpl) -> syn::Result<(Ident, Vec<MethodDef>)> {
    let struct_ident = match item_impl.self_ty.as_ref() {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident.clone())
            .ok_or_else(|| syn::Error::new_spanned(item_impl, "cannot determine struct name"))?,
        _ => {
            return Err(syn::Error::new_spanned(
                item_impl,
                "impl must be on a struct type",
            ));
        }
    };

    let mut methods = Vec::new();

    for item in &item_impl.items {
        let method = match item {
            syn::ImplItem::Fn(f) => parse_method(&struct_ident, f)?,

            _ => continue,
        };

        methods.push(method);
    }

    Ok((struct_ident, methods))
}

fn parse_method(_struct_ident: &Ident, method: &syn::ImplItemFn) -> syn::Result<MethodDef> {
    let method_attrs = MethodAttrs::parse(&method.attrs)?;
    let lua_name = method_attrs
        .name
        .unwrap_or_else(|| method.sig.ident.to_string());
    let doc = extract_doc(&method.attrs);

    let mut receiver = ReceiverKind::None;
    let mut args: Vec<(Ident, syn::Type)> = Vec::new();

    for input in &method.sig.inputs {
        match input {
            syn::FnArg::Receiver(r) => {
                if r.reference.is_some() && r.mutability.is_some() {
                    receiver = ReceiverKind::MutRef;
                } else if r.reference.is_some() {
                    receiver = ReceiverKind::Ref;
                } else {
                    receiver = ReceiverKind::Owned;
                }
            }
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                    args.push((pat_ident.ident.clone(), pat_type.ty.as_ref().clone()));
                }
            }
        }
    }

    let return_type = method.sig.output.clone();
    let is_fallible = detect_fallibility(&return_type);

    Ok(MethodDef {
        ident: method.sig.ident.clone(),
        lua_name,
        doc,
        receiver,
        args,
        return_type,
        is_fallible,
        span: method.sig.ident.span(),
    })
}

fn parse_global(item_fn: &syn::ItemFn) -> syn::Result<FnDef> {
    for attribute in &item_fn.attrs {
        if attribute.path().is_ident("luafunction") {
            return Err(syn::Error::new_spanned(
                attribute,
                "#[luaglobal] cannot be combined with #[luafunction]",
            ));
        }

        if attribute.path().is_ident("luaglobal") && !matches!(attribute.meta, syn::Meta::Path(_)) {
            return Err(syn::Error::new_spanned(
                attribute,
                "expected bare #[luaglobal]",
            ));
        }
    }

    let signature = &item_fn.sig;
    if !signature.inputs.is_empty() {
        return Err(syn::Error::new_spanned(
            &signature.inputs,
            "global initializers must take no arguments",
        ));
    }

    if matches!(signature.output, syn::ReturnType::Default) {
        return Err(syn::Error::new_spanned(
            &signature.ident,
            "global initializers require an explicit return type",
        ));
    }

    if let Some(token) = &signature.asyncness {
        return Err(syn::Error::new_spanned(
            token,
            "global initializers cannot be async",
        ));
    }

    if let Some(token) = &signature.unsafety {
        return Err(syn::Error::new_spanned(
            token,
            "global initializers cannot be unsafe",
        ));
    }

    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.generics,
            "global initializers cannot be generic",
        ));
    }

    if let Some(variadic) = &signature.variadic {
        return Err(syn::Error::new_spanned(
            variadic,
            "global initializers cannot be variadic",
        ));
    }

    let mut global = parse_function(item_fn)?;
    global.lua_name = global.lua_name.trim_start_matches("r#").to_string();

    Ok(global)
}

fn parse_function(item_fn: &syn::ItemFn) -> syn::Result<FnDef> {
    let doc = extract_doc(&item_fn.attrs);
    let lua_name = item_fn.sig.ident.to_string();

    let mut args: Vec<(Ident, syn::Type)> = Vec::new();

    for input in &item_fn.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                args.push((pat_ident.ident.clone(), pat_type.ty.as_ref().clone()));
            }
        }
    }

    let return_type = item_fn.sig.output.clone();
    let is_fallible = detect_fallibility(&return_type);

    Ok(FnDef {
        ident: item_fn.sig.ident.clone(),
        lua_name,
        doc,
        args,
        return_type,
        is_fallible,
        span: item_fn.sig.ident.span(),
    })
}

#[cfg(test)]

mod tests {
    use syn::parse_quote;

    use crate::ir::ReceiverKind;

    use super::parse;

    #[test]
    fn test_flat_struct_single_field() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[luaclass]
                struct Foo {
                    x: String,
                }
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        let class = &result.classes[0];
        assert_eq!(class.ident.to_string(), "Foo");
        assert_eq!(class.lua_name, "Foo");
        assert_eq!(class.fields.len(), 1);
        let field = &class.fields[0];
        assert_eq!(field.ident.to_string(), "x");
        assert_eq!(field.lua_name, "x");
    }

    #[test]
    fn test_struct_with_rename_and_skip() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[luaclass]
                struct Bar {
                    #[lua(rename = "y")]
                    pub a: u32,
                    #[lua(skip)]
                    #[lua(default = "7")]
                    pub b: u64,
                }
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        let class = &result.classes[0];
        assert_eq!(class.fields.len(), 2);

        let field0 = &class.fields[0];
        assert_eq!(field0.ident.to_string(), "a");
        assert_eq!(field0.lua_name, "y");
        assert!(!field0.skip);

        let field1 = &class.fields[1];
        assert_eq!(field1.ident.to_string(), "b");
        assert!(field1.skip);
        assert_eq!(field1.default.as_deref(), Some("7"));
    }

    #[test]
    fn test_class_with_custom_lua_name() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[luaclass(name = "MyServer")]
                struct ServerConfig {}
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].lua_name, "MyServer");
    }

    #[test]
    fn test_methods_impl() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[luaclass]
                struct Foo;

                #[luamethods]
                impl Foo {
                    fn bar(&self) -> String {
                        "hello".into()
                    }

                    fn baz(&mut self, x: u16) {}
                }
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        let class = &result.classes[0];
        assert_eq!(class.methods.len(), 2);

        let bar = class.methods.iter().find(|m| m.ident == "bar").unwrap();
        assert_eq!(bar.receiver, ReceiverKind::Ref);

        let baz = class.methods.iter().find(|m| m.ident == "baz").unwrap();
        assert_eq!(baz.receiver, ReceiverKind::MutRef);
        assert_eq!(baz.args.len(), 1);
    }

    #[test]
    fn test_free_function() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[luafunction]
                fn env(name: String) -> mlua::Result<String> {
                    Ok("hello".into())
                }
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.functions.len(), 1);
        let func = &result.functions[0];
        assert_eq!(func.ident.to_string(), "env");
        assert_eq!(func.args.len(), 1);
        assert!(func.is_fallible);
    }

    #[test]
    fn test_doc_extraction() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                /// Hello
                #[luaclass]
                struct Doc {}
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        let doc = result.classes[0].doc.as_deref().unwrap();
        assert!(doc.contains("Hello"));
    }

    #[test]
    fn test_merge_multiple_impl_blocks() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[luaclass]
                struct Foo;

                #[luamethods]
                impl Foo {
                    fn first(&self) {}
                }

                #[luamethods]
                impl Foo {
                    fn second(&self) {}
                }
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        let class = &result.classes[0];
        assert_eq!(class.methods.len(), 2);
        assert!(class.methods.iter().any(|m| m.ident == "first"));
        assert!(class.methods.iter().any(|m| m.ident == "second"));
    }

    #[test]
    fn test_non_lua_items_preserved() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                fn regular_fn() {}

                struct NotLuaClass;
            }
        };

        let result = parse(&item_mod).unwrap();

        assert!(result.classes.is_empty());
        assert!(result.functions.is_empty());
    }

    #[test]
    fn test_lua_newtype_collects_type_map() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[lua_newtype(to = "string")]
                struct LuaPathBuf;

                #[lua_newtype(to = "number")]
                struct LuaDuration;
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.type_map.len(), 2);
        assert_eq!(
            result.type_map.get("LuaPathBuf"),
            Some(&"string".to_string())
        );
        assert_eq!(
            result.type_map.get("LuaDuration"),
            Some(&"number".to_string())
        );
    }

    #[test]
    fn test_lua_newtype_alongside_luaclass() {
        let item_mod: syn::ItemMod = parse_quote! {
            mod test_mod {
                #[lua_newtype(to = "string")]
                struct LuaPathBuf;

                #[luaclass]
                struct Config {
                    log_dir: LuaPathBuf,
                }
            }
        };

        let result = parse(&item_mod).unwrap();

        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.type_map.len(), 1);
        assert_eq!(
            result.type_map.get("LuaPathBuf"),
            Some(&"string".to_string())
        );
    }

    #[test]

    fn global_initializer_signatures_are_validated() {
        let cases: Vec<(syn::ItemFn, &str)> = vec![
            (
                parse_quote! { fn value(argument: u16) -> u16 { argument } },
                "global initializers must take no arguments",
            ),
            (
                parse_quote! { fn value() {} },
                "global initializers require an explicit return type",
            ),
            (
                parse_quote! { async fn value() -> u16 { 1 } },
                "global initializers cannot be async",
            ),
            (
                parse_quote! { unsafe fn value() -> u16 { 1 } },
                "global initializers cannot be unsafe",
            ),
            (
                parse_quote! { fn value<T>() -> u16 { 1 } },
                "global initializers cannot be generic",
            ),
            (
                parse_quote! { #[luafunction] fn value() -> u16 { 1 } },
                "#[luaglobal] cannot be combined with #[luafunction]",
            ),
            (
                parse_quote! { #[luaglobal(name = "value")] fn value() -> u16 { 1 } },
                "expected bare #[luaglobal]",
            ),
        ];

        for (initializer, expected) in cases {
            let error = super::parse_global(&initializer).err().unwrap();
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]

    fn globals_are_sorted_and_separate_from_functions() {
        let module = parse(&parse_quote! {
            mod api {
                #[luaglobal]
                fn zebra() -> u16 { 1 }
                #[luaglobal]
                fn apple() -> u16 { 2 }
                #[luafunction]
                fn callable() {}
            }
        })
        .unwrap();

        assert_eq!(
            module
                .globals
                .iter()
                .map(|global| global.lua_name.as_str())
                .collect::<Vec<_>>(),
            vec!["apple", "zebra"]
        );
        assert_eq!(module.functions.len(), 1);
    }
}
