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
    let mut type_map: BTreeMap<String, String> = BTreeMap::new();

    let items = match &item_mod.content {
        Some((_, items)) => items,
        None => {
            return Ok(ModuleDef {
                classes,
                functions,
                type_map,
            });
        }
    };

    for item in items {
        match item {
            syn::Item::Struct(s) => {
                if has_attr(&s.attrs, "luaclass") {
                    let class = parse_class(s)?;
                    classes.push(class);
                }

                if has_attr(&s.attrs, "lua_newtype") {
                    let newtype = NewtypeAttrs::parse(&s.attrs)?;
                    let rust_name = s.ident.to_string();
                    let lua_type = newtype.lua_type.expect("lua_newtype must have lua_type");

                    type_map.insert(rust_name, lua_type);
                }
            }
            syn::Item::Impl(imp) => {
                if has_attr(&imp.attrs, "luamethods") {
                    let (struct_ident, methods) = parse_methods_impl(imp)?;
                    let key = struct_ident.to_string();
                    methods_map.entry(key).or_default().extend(methods);
                }
            }
            syn::Item::Fn(f) => {
                if has_attr(&f.attrs, "luafunction") {
                    let func = parse_function(f)?;
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

    classes.sort_by(|a, b| a.ident.to_string().cmp(&b.ident.to_string()));
    functions.sort_by(|a, b| a.ident.to_string().cmp(&b.ident.to_string()));

    Ok(ModuleDef {
        classes,
        functions,
        type_map,
    })
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}

fn parse_class(s: &syn::ItemStruct) -> syn::Result<ClassDef> {
    let class_attrs = ClassAttrs::parse(&s.attrs)?;
    let lua_name = class_attrs.name.unwrap_or_else(|| s.ident.to_string());
    let doc = extract_doc(&s.attrs);

    let mut fields = Vec::new();

    for field in &s.fields {
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
            ty: field.ty.clone(),
            doc: field_doc,
            default: field_attrs.default,
            signature: field_attrs.signature,
            skip: field_attrs.skip,
            span: field_ident.span(),
        });
    }

    Ok(ClassDef {
        ident: s.ident.clone(),
        lua_name,
        doc,
        fields,
        methods: Vec::new(),
        annotation_only: class_attrs.annotation_only,
        span: s.ident.span(),
    })
}

fn parse_methods_impl(imp: &syn::ItemImpl) -> syn::Result<(Ident, Vec<MethodDef>)> {
    let struct_ident = match imp.self_ty.as_ref() {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident.clone())
            .ok_or_else(|| syn::Error::new_spanned(imp, "cannot determine struct name"))?,
        _ => {
            return Err(syn::Error::new_spanned(
                imp,
                "impl must be on a struct type",
            ));
        }
    };

    let mut methods = Vec::new();

    for item in &imp.items {
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

    let ret = method.sig.output.clone();
    let is_fallible = detect_fallibility(&ret);

    Ok(MethodDef {
        ident: method.sig.ident.clone(),
        lua_name,
        doc,
        receiver,
        args,
        ret,
        is_fallible,
        span: method.sig.ident.span(),
    })
}

fn parse_function(f: &syn::ItemFn) -> syn::Result<FnDef> {
    let doc = extract_doc(&f.attrs);
    let lua_name = f.sig.ident.to_string();

    let mut args: Vec<(Ident, syn::Type)> = Vec::new();

    for input in &f.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                args.push((pat_ident.ident.clone(), pat_type.ty.as_ref().clone()));
            }
        }
    }

    let ret = f.sig.output.clone();
    let is_fallible = detect_fallibility(&ret);

    Ok(FnDef {
        ident: f.sig.ident.clone(),
        lua_name,
        doc,
        args,
        ret,
        is_fallible,
        span: f.sig.ident.span(),
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
}
