use std::collections::BTreeMap;

use syn::Type;

pub fn rust_type_to_emmylua(ty: &Type, type_map: &BTreeMap<String, String>) -> String {
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
                            return format!("{}?", rust_type_to_emmylua(inner, type_map));
                        }
                    }
                    "any?".to_string()
                }
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return format!("{}[]", rust_type_to_emmylua(inner, type_map));
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
                                    rust_type_to_emmylua(k, type_map),
                                    rust_type_to_emmylua(v, type_map),
                                );
                            }
                        }
                    }
                    "table<any, any>".to_string()
                }
                _ => {
                    if let Some(mapped) = type_map.get(&name) {
                        return mapped.clone();
                    }

                    name
                }
            }
        }

        Type::Reference(ref_type) => rust_type_to_emmylua(ref_type.elem.as_ref(), type_map),

        Type::Tuple(tuple) => {
            if tuple.elems.is_empty() {
                return "nil".to_string();
            }

            let types: Vec<String> = tuple
                .elems
                .iter()
                .map(|t| rust_type_to_emmylua(t, type_map))
                .collect();
            format!("({})", types.join(", "))
        }

        _ => "any".to_string(),
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use syn::parse_quote;

    fn empty_map() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn test_primitives() {
        let map = empty_map();
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { String }, &map),
            "string"
        );
        assert_eq!(rust_type_to_emmylua(&parse_quote! { u32 }, &map), "integer");
        assert_eq!(rust_type_to_emmylua(&parse_quote! { f64 }, &map), "number");
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { bool }, &map),
            "boolean"
        );
    }

    #[test]
    fn test_option() {
        let map = empty_map();
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Option<String> }, &map),
            "string?"
        );
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Option<i32> }, &map),
            "integer?"
        );
    }

    #[test]
    fn test_vec() {
        let map = empty_map();
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Vec<String> }, &map),
            "string[]"
        );
    }

    #[test]
    fn test_hashmap() {
        let map = empty_map();
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { HashMap<String, u32> }, &map),
            "table<string, integer>"
        );
    }

    #[test]
    fn test_custom_type() {
        let map = empty_map();
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { LogConfig }, &map),
            "LogConfig"
        );
    }

    #[test]
    fn test_function() {
        let map = empty_map();
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { mlua::Function }, &map),
            "function"
        );
    }

    #[test]
    fn test_custom_type_with_map() {
        let mut map = BTreeMap::new();
        map.insert("LuaPathBuf".to_string(), "string".to_string());
        map.insert("LuaDuration".to_string(), "number".to_string());

        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { LuaPathBuf }, &map),
            "string"
        );
        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { LuaDuration }, &map),
            "number"
        );
    }

    #[test]
    fn test_option_with_mapped_type() {
        let mut map = BTreeMap::new();
        map.insert("LuaPathBuf".to_string(), "string".to_string());

        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Option<LuaPathBuf> }, &map),
            "string?"
        );
    }

    #[test]
    fn test_vec_with_mapped_type() {
        let mut map = BTreeMap::new();
        map.insert("LuaPathBuf".to_string(), "string".to_string());

        assert_eq!(
            rust_type_to_emmylua(&parse_quote! { Vec<LuaPathBuf> }, &map),
            "string[]"
        );
    }
}
