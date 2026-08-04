use syn::Type;

pub fn rust_type_to_emmylua(ty: &Type) -> String {
    match ty {
        Type::Path(typepath) => {
            let last_seg = match typepath.path.segments.last() {
                Some(seg) => seg,
                None => return "any".to_string(),
            };

            let name = last_seg.ident.to_string();

            match name.as_str() {
                "String" | "str" => "string".to_string(),
                "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
                    "integer".to_string()
                }
                "f32" | "f64" => "number".to_string(),
                "bool" => "boolean".to_string(),
                "Function" => "function".to_string(),
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return format!("{}?", rust_type_to_emmylua(inner));
                        }
                    }
                    "any?".to_string()
                }
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return format!("{}[]", rust_type_to_emmylua(inner));
                        }
                    }
                    "any[]".to_string()
                }
                "HashMap" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        let inner_args: Vec<_> = args.args.iter().collect();
                        if inner_args.len() == 2 {
                            if let (syn::GenericArgument::Type(k), syn::GenericArgument::Type(v)) =
                                (&inner_args[0], &inner_args[1])
                            {
                                return format!(
                                    "table<{}, {}>",
                                    rust_type_to_emmylua(k),
                                    rust_type_to_emmylua(v)
                                );
                            }
                        }
                    }
                    "table<any, any>".to_string()
                }
                _ => name,
            }
        }
        Type::Reference(ref_type) => rust_type_to_emmylua(ref_type.elem.as_ref()),
        Type::Tuple(tuple) => {
            if tuple.elems.is_empty() {
                return "nil".to_string();
            }

            let types: Vec<String> = tuple.elems.iter().map(rust_type_to_emmylua).collect();
            format!("({})", types.join(", "))
        }
        _ => "any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_primitives() {
        assert_eq!(rust_type_to_emmylua(&parse_quote! { String }), "string");
        assert_eq!(rust_type_to_emmylua(&parse_quote! { u32 }), "integer");
        assert_eq!(rust_type_to_emmylua(&parse_quote! { f64 }), "number");
        assert_eq!(rust_type_to_emmylua(&parse_quote! { bool }), "boolean");
    }

    #[test]
    fn test_option() {
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Option<String> }),
            "string?"
        );
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Option<i32> }),
            "integer?"
        );
    }

    #[test]
    fn test_vec() {
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Vec<String> }),
            "string[]"
        );
    }

    #[test]
    fn test_hashmap() {
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { HashMap<String, u32> }),
            "table<string, integer>"
        );
    }

    #[test]
    fn test_custom_type() {
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { LogConfig }),
            "LogConfig"
        );
    }

    #[test]
    fn test_function() {
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { mlua::Function }),
            "function"
        );
    }
}
