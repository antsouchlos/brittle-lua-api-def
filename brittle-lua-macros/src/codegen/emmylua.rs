use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::ir::{ClassDef, FnDef, ModuleDef};

pub fn generate(module: &ModuleDef) -> TokenStream {
    let mut parts: Vec<String> = Vec::new();
    let type_map = &module.type_map;

    let mut sorted_classes: Vec<&ClassDef> = module.classes.iter().collect();
    sorted_classes.sort_by(|a, b| a.lua_name.cmp(&b.lua_name));

    for class in &sorted_classes {
        emit_class(&mut parts, class, type_map);
    }

    let mut sorted_functions: Vec<&FnDef> = module.functions.iter().collect();
    sorted_functions.sort_by(|a, b| a.lua_name.cmp(&b.lua_name));

    for func in &sorted_functions {
        emit_free_function(&mut parts, func, type_map);
    }

    parts.push(String::new());
    parts.push("return {}".to_string());

    let emmy_lua_str = parts.join("\n");

    quote! {
        pub static EMMY_LUA: &str = #emmy_lua_str;
    }
}

fn emit_class(parts: &mut Vec<String>, class: &ClassDef, type_map: &BTreeMap<String, String>) {
    if let Some(ref doc) = class.doc {
        for line in doc.lines() {
            parts.push(format!("--- {}", line.trim()));
        }
    }

    parts.push(format!("--- @class {}", class.lua_name));

    let mut sorted_fields: Vec<&crate::ir::FieldDef> = class.fields.iter().collect();
    sorted_fields.sort_by(|a, b| a.lua_name.cmp(&b.lua_name));

    for field in sorted_fields {
        if field.skip {
            continue;
        }

        if let Some(ref doc) = field.doc {
            for line in doc.lines() {
                parts.push(format!("--- {}", line.trim()));
            }
        }

        let optional_suffix = if field.default.is_some() && !is_option_type(&field.field_type) {
            "?"
        } else {
            ""
        };

        let emmy_type = if let Some(ref sig) = field.signature {
            sig.clone()
        } else {
            super::type_mapper::rust_type_to_emmylua(&field.field_type, type_map)
        };
        parts.push(format!(
            "--- @field {}{} {}",
            field.lua_name, optional_suffix, emmy_type
        ));
    }

    let mut sorted_methods: Vec<&crate::ir::MethodDef> = class.methods.iter().collect();
    sorted_methods.sort_by(|a, b| a.lua_name.cmp(&b.lua_name));

    for method in sorted_methods {
        emit_method(parts, method, &class.lua_name, type_map);
    }

    parts.push(String::new());
}

fn emit_method(
    parts: &mut Vec<String>,
    method: &crate::ir::MethodDef,
    class_name: &str,
    type_map: &BTreeMap<String, String>,
) {
    if let Some(ref doc) = method.doc {
        for line in doc.lines() {
            parts.push(format!("--- {}", line.trim()));
        }
    }

    let ret_type = match &method.return_type {
        syn::ReturnType::Type(_, ty) => {
            let inner = if method.is_fallible {
                unwrap_result_type(ty)
            } else {
                ty.as_ref().clone()
            };
            let resolved = resolve_self(&inner, class_name);
            let emmy = super::type_mapper::rust_type_to_emmylua(&resolved, type_map);
            format!(" {}", emmy)
        }

        syn::ReturnType::Default => String::new(),
    };

    let arg_types: Vec<String> = method
        .args
        .iter()
        .map(|(name, ty)| {
            let emmy = super::type_mapper::rust_type_to_emmylua(ty, type_map);
            format!("{}: {}", name, emmy)
        })
        .collect();

    let fun_sig = match &method.receiver {
        crate::ir::ReceiverKind::None => {
            if arg_types.is_empty() {
                format!("fun():{}", ret_type)
            } else {
                format!("fun({}):{}", arg_types.join(", "), ret_type)
            }
        }

        _ => {
            let self_param = "self";
            let all_args = std::iter::once(self_param.to_string())
                .chain(arg_types)
                .collect::<Vec<_>>()
                .join(", ");

            if all_args == "self" {
                format!("fun({}):{}", self_param, ret_type)
            } else {
                format!("fun({}):{}", all_args, ret_type)
            }
        }
    };

    parts.push(format!("--- @field {} {}", method.lua_name, fun_sig));
}

fn emit_free_function(parts: &mut Vec<String>, func: &FnDef, type_map: &BTreeMap<String, String>) {
    if let Some(ref doc) = func.doc {
        for line in doc.lines() {
            parts.push(format!("--- {}", line.trim()));
        }
    }

    for (name, ty) in &func.args {
        let emmy = super::type_mapper::rust_type_to_emmylua(ty, type_map);
        parts.push(format!("--- @param {} {}", name, emmy));
    }

    let ret_type = match &func.return_type {
        syn::ReturnType::Type(_, ty) => {
            let inner = if func.is_fallible {
                unwrap_result_type(ty)
            } else {
                ty.as_ref().clone()
            };

            let emmy = super::type_mapper::rust_type_to_emmylua(&inner, type_map);
            format!("--- @return {}", emmy)
        }

        syn::ReturnType::Default => String::new(),
    };

    if !ret_type.is_empty() {
        parts.push(ret_type);
    }

    let arg_names: Vec<String> = func.args.iter().map(|(name, _)| name.to_string()).collect();

    let arg_list = arg_names.join(", ");
    let sig = if arg_list.is_empty() {
        format!("function {}() end", func.lua_name)
    } else {
        format!("function {}({}) end", func.lua_name, arg_list)
    };

    parts.push(sig);
    parts.push(String::new());
}

fn unwrap_result_type(ty: &Type) -> Type {
    if let Type::Path(typepath) = ty {
        if let Some(segment) = typepath.path.segments.last() {
            if segment.ident == "Result" || segment.ident == "LuaResult" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return inner.clone();
                    }
                }
            }
        }
    }

    ty.clone()
}

fn resolve_self(ty: &Type, class_name: &str) -> Type {
    match ty {
        Type::Path(typepath) => {
            let resolved_segments: Vec<_> = typepath
                .path
                .segments
                .iter()
                .map(|seg| {
                    if seg.ident == "Self" {
                        let mut new_seg = seg.clone();
                        new_seg.ident = syn::Ident::new(class_name, seg.ident.span());
                        new_seg
                    } else {
                        let mut new_seg = seg.clone();
                        new_seg.arguments = resolve_self_args(&seg.arguments, class_name);
                        new_seg
                    }
                })
                .collect();

            let mut new_path = typepath.path.clone();
            new_path.segments = syn::punctuated::Punctuated::from_iter(resolved_segments);

            Type::Path(syn::TypePath {
                qself: typepath.qself.clone(),
                path: new_path,
            })
        }

        Type::Reference(ref_type) => {
            let mut new_ref = ref_type.clone();
            new_ref.elem = Box::new(resolve_self(&ref_type.elem, class_name));
            Type::Reference(new_ref)
        }

        other => other.clone(),
    }
}

fn resolve_self_args(args: &syn::PathArguments, class_name: &str) -> syn::PathArguments {
    match args {
        syn::PathArguments::AngleBracketed(ab) => {
            let resolved: Vec<_> = ab
                .args
                .iter()
                .map(|arg| match arg {
                    syn::GenericArgument::Type(ty) => {
                        syn::GenericArgument::Type(resolve_self(ty, class_name))
                    }
                    other => other.clone(),
                })
                .collect();

            let mut new_ab = ab.clone();
            new_ab.args = syn::punctuated::Punctuated::from_iter(resolved);
            syn::PathArguments::AngleBracketed(new_ab)
        }

        other => other.clone(),
    }
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }

    false
}

#[cfg(test)]

mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::ir::{ClassDef, FieldDef, FnDef, MethodDef};
    use proc_macro2::Span;
    use syn::parse_quote;

    fn empty_map() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn test_class_output() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(output.contains("--- @class Foo"));
    }

    #[test]
    fn test_field_output() {
        let class = ClassDef {
            ident: parse_quote! { Server },
            lua_name: "Server".to_string(),
            doc: None,
            fields: vec![FieldDef {
                ident: parse_quote! { port },
                lua_name: "port".to_string(),
                field_type: parse_quote! { u16 },
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
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(output.contains("--- @field port integer"));
    }

    #[test]
    fn test_method_output() {
        let class = ClassDef {
            ident: parse_quote! { Server },
            lua_name: "Server".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { run },
                lua_name: "run".to_string(),
                doc: None,
                receiver: crate::ir::ReceiverKind::Ref,
                args: vec![],
                return_type: syn::ReturnType::Default,
                is_fallible: false,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(output.contains("fun(self)"));
    }

    #[test]
    fn test_footer() {
        let module = ModuleDef {
            classes: vec![],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(output.contains("return {}"));
    }

    #[test]
    fn test_fallible_method_unwraps_result() {
        let class = ClassDef {
            ident: parse_quote! { Server },
            lua_name: "Server".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { fetch },
                lua_name: "fetch".to_string(),
                doc: None,
                receiver: crate::ir::ReceiverKind::Ref,
                args: vec![],
                return_type: syn::ReturnType::Type(
                    parse_quote! { -> },
                    Box::new(parse_quote! { Result<String, mlua::Error> }),
                ),
                is_fallible: true,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("fun(self): string"),
            "expected `string` return, got: {}",
            output
        );
    }

    #[test]
    fn test_infallible_method_passes_type_through() {
        let class = ClassDef {
            ident: parse_quote! { Server },
            lua_name: "Server".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { port },
                lua_name: "port".to_string(),
                doc: None,
                receiver: crate::ir::ReceiverKind::Ref,
                args: vec![],
                return_type: syn::ReturnType::Type(
                    parse_quote! { -> },
                    Box::new(parse_quote! { u16 }),
                ),
                is_fallible: false,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("fun(self): integer"),
            "expected `integer` return, got: {}",
            output
        );
    }

    #[test]
    fn test_constructor_resolves_self() {
        let class = ClassDef {
            ident: parse_quote! { AppConfig },
            lua_name: "AppConfig".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { new },
                lua_name: "new".to_string(),
                doc: None,
                receiver: crate::ir::ReceiverKind::None,
                args: vec![],
                return_type: syn::ReturnType::Type(
                    parse_quote! { -> },
                    Box::new(parse_quote! { mlua::Result<Self> }),
                ),
                is_fallible: true,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("fun(): AppConfig"),
            "expected `AppConfig` return, got: {}",
            output
        );
    }

    #[test]
    fn test_fallible_function_unwraps_result() {
        let module = ModuleDef {
            classes: vec![],
            functions: vec![FnDef {
                ident: parse_quote! { compute },
                lua_name: "compute".to_string(),
                doc: None,
                args: vec![],
                return_type: syn::ReturnType::Type(
                    parse_quote! { -> },
                    Box::new(parse_quote! { Result<i64, mlua::Error> }),
                ),
                is_fallible: true,
                span: Span::call_site(),
            }],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("--- @return integer"),
            "expected `integer` return, got: {}",
            output
        );
    }

    #[test]
    fn test_infallible_function_passes_type_through() {
        let module = ModuleDef {
            classes: vec![],
            functions: vec![FnDef {
                ident: parse_quote! { get_value },
                lua_name: "get_value".to_string(),
                doc: None,
                args: vec![],
                return_type: syn::ReturnType::Type(
                    parse_quote! { -> },
                    Box::new(parse_quote! { String }),
                ),
                is_fallible: false,
                span: Span::call_site(),
            }],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("--- @return string"),
            "expected `string` return, got: {}",
            output
        );
    }

    #[test]
    fn test_newtype_field_uses_type_map() {
        let mut type_map = BTreeMap::new();
        type_map.insert("LuaPathBuf".to_string(), "string".to_string());

        let class = ClassDef {
            ident: parse_quote! { Config },
            lua_name: "Config".to_string(),
            doc: None,
            fields: vec![FieldDef {
                ident: parse_quote! { log_dir },
                lua_name: "log_dir".to_string(),
                field_type: parse_quote! { LuaPathBuf },
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
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map,
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("--- @field log_dir string"),
            "expected `string` annotation, got: {}",
            output
        );
        assert!(!output.contains("LuaPathBuf"));
    }

    #[test]
    fn test_newtype_return_type_uses_type_map() {
        let mut type_map = BTreeMap::new();
        type_map.insert("LuaPathBuf".to_string(), "string".to_string());

        let class = ClassDef {
            ident: parse_quote! { Config },
            lua_name: "Config".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![MethodDef {
                ident: parse_quote! { get_log_dir },
                lua_name: "get_log_dir".to_string(),
                doc: None,
                receiver: crate::ir::ReceiverKind::Ref,
                args: vec![],
                return_type: syn::ReturnType::Type(
                    parse_quote! { -> },
                    Box::new(parse_quote! { LuaPathBuf }),
                ),
                is_fallible: false,
                span: Span::call_site(),
            }],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map,
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("fun(self): string"),
            "expected `string` return, got: {}",
            output
        );
    }

    #[test]
    fn test_signature_overrides_field_type() {
        let class = ClassDef {
            ident: parse_quote! { RefParser },
            lua_name: "RefParser".to_string(),
            doc: None,
            fields: vec![FieldDef {
                ident: parse_quote! { parser },
                lua_name: "parser".to_string(),
                field_type: parse_quote! { mlua::Function },
                doc: None,
                default: None,
                signature: Some("fun(string): PublicationMetadata".to_string()),
                skip: false,
                span: Span::call_site(),
            }],
            methods: vec![],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("--- @field parser fun(string): PublicationMetadata"),
            "expected custom signature, got: {}",
            output
        );
        assert!(
            !output.contains("--- @field parser function"),
            "should NOT contain the default `function` type, got: {}",
            output
        );
    }

    #[test]
    fn test_annotation_only_class_emits_emmy_lua() {
        let class = ClassDef {
            ident: parse_quote! { PublicationMetadata },
            lua_name: "PublicationMetadata".to_string(),
            doc: None,
            fields: vec![
                FieldDef {
                    ident: parse_quote! { title },
                    lua_name: "title".to_string(),
                    field_type: parse_quote! { String },
                    doc: None,
                    default: None,
                    signature: None,
                    skip: false,
                    span: Span::call_site(),
                },
                FieldDef {
                    ident: parse_quote! { year },
                    lua_name: "year".to_string(),
                    field_type: parse_quote! { i32 },
                    doc: None,
                    default: None,
                    signature: None,
                    skip: false,
                    span: Span::call_site(),
                },
            ],
            methods: vec![],
            annotation_only: true,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(output.contains("--- @class PublicationMetadata"));
        assert!(output.contains("--- @field title string"));
        assert!(output.contains("--- @field year integer"));
    }

    #[test]
    fn test_optional_field_emits_question_mark() {
        let class = ClassDef {
            ident: parse_quote! { Config },
            lua_name: "Config".to_string(),
            doc: None,
            fields: vec![
                FieldDef {
                    ident: parse_quote! { host },
                    lua_name: "host".to_string(),
                    field_type: parse_quote! { String },
                    doc: None,
                    default: None,
                    signature: None,
                    skip: false,
                    span: Span::call_site(),
                },
                FieldDef {
                    ident: parse_quote! { port },
                    lua_name: "port".to_string(),
                    field_type: parse_quote! { u16 },
                    doc: None,
                    default: Some("3000".to_string()),
                    signature: None,
                    skip: false,
                    span: Span::call_site(),
                },
            ],
            methods: vec![],
            annotation_only: false,
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: empty_map(),
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(
            output.contains("--- @field port? integer"),
            "optional field should have `?` suffix, got: {}",
            output
        );
        assert!(
            output.contains("--- @field host string"),
            "required field should NOT have `?` suffix, got: {}",
            output
        );
        assert!(
            !output.contains("--- @field host? string"),
            "required field should NOT have `?`, got: {}",
            output
        );
    }
}
