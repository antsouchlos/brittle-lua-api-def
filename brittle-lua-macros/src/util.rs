use syn::{Attribute, Type};

#[derive(Debug, Default)]
pub struct FieldAttrs {
    pub default: Option<String>,
    pub rename: Option<String>,
    pub signature: Option<String>,
    pub skip: bool,
}

impl FieldAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua") {
                let meta: syn::Meta = attr.parse_args()?;
                result.apply_meta(&meta, attr)?;
            }
        }

        Ok(result)
    }

    fn apply_meta(&mut self, meta: &syn::Meta, attr: &Attribute) -> syn::Result<()> {
        match meta {
            syn::Meta::Path(p) => {
                if p.is_ident("skip") {
                    if self.skip {
                        return Err(syn::Error::new_spanned(attr, "duplicate `skip`"));
                    }

                    self.skip = true;
                } else {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!("unknown `#[lua({})]` key", quote_path(p)),
                    ));
                }
            }
            syn::Meta::NameValue(nv) => {
                let key = nv.path.get_ident().map(|i| i.to_string());

                match key.as_deref() {
                    Some("default") => {
                        if self.default.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate `default`"));
                        }

                        let val = extract_string_lit(&nv.value).ok_or_else(|| {
                            syn::Error::new_spanned(&nv.value, "expected string literal")
                        })?;

                        self.default = Some(val);
                    }
                    Some("rename") => {
                        if self.rename.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate `rename`"));
                        }

                        let val = extract_string_lit(&nv.value).ok_or_else(|| {
                            syn::Error::new_spanned(&nv.value, "expected string literal")
                        })?;

                        self.rename = Some(val);
                    }
                    Some("signature") => {
                        if self.signature.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate `signature`"));
                        }

                        let val = extract_string_lit(&nv.value).ok_or_else(|| {
                            syn::Error::new_spanned(&nv.value, "expected string literal")
                        })?;

                        self.signature = Some(val);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!("unknown `#[lua({})]` key", key.unwrap_or_default()),
                        ));
                    }
                }
            }
            syn::Meta::List(_) => {
                return Err(syn::Error::new_spanned(
                    attr,
                    "unexpected `#[lua(...)]` form",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ClassAttrs {
    pub name: Option<String>,
    pub annotation_only: bool,
}

impl ClassAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut class_attrs = Self::default();

        for attr in attrs {
            if attr.path().is_ident("luaclass") && matches!(attr.meta, syn::Meta::List(_)) {
                let nested: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> =
                    attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated)?;
                for meta in nested {
                    class_attrs.apply_meta(&meta, attr)?;
                }
            }
        }

        Ok(class_attrs)
    }

    fn apply_meta(&mut self, meta: &syn::Meta, attr: &Attribute) -> syn::Result<()> {
        match meta {
            syn::Meta::NameValue(nv) => {
                let key = nv.path.get_ident().map(|i| i.to_string());

                match key.as_deref() {
                    Some("name") => {
                        if self.name.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate `name`"));
                        }

                        let val = extract_string_lit(&nv.value).ok_or_else(|| {
                            syn::Error::new_spanned(&nv.value, "expected string literal")
                        })?;

                        self.name = Some(val);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!("unknown `#[luaclass({})]` key", key.unwrap_or_default()),
                        ));
                    }
                }
            }
            syn::Meta::Path(p) => {
                if p.is_ident("annotation") {
                    if self.annotation_only {
                        return Err(syn::Error::new_spanned(attr, "duplicate `annotation`"));
                    }

                    self.annotation_only = true;
                } else {
                    return Err(syn::Error::new_spanned(
                        p,
                        format!("unknown `#[luaclass({})]` key", quote_path(p)),
                    ));
                }
            }
            syn::Meta::List(_) => {}
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MethodAttrs {
    pub name: Option<String>,
}

impl MethodAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua") {
                let meta: syn::Meta = attr.parse_args()?;
                result.apply_meta(&meta, attr)?;
            }
        }

        Ok(result)
    }

    fn apply_meta(&mut self, meta: &syn::Meta, attr: &Attribute) -> syn::Result<()> {
        match meta {
            syn::Meta::NameValue(nv) => {
                let key = nv.path.get_ident().map(|i| i.to_string());

                match key.as_deref() {
                    Some("name") => {
                        if self.name.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate `name`"));
                        }

                        let val = extract_string_lit(&nv.value).ok_or_else(|| {
                            syn::Error::new_spanned(&nv.value, "expected string literal")
                        })?;

                        self.name = Some(val);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!("unknown `#[lua({})]` key", key.unwrap_or_default()),
                        ));
                    }
                }
            }
            syn::Meta::Path(p) => {
                return Err(syn::Error::new_spanned(
                    p,
                    format!("unknown `#[lua({})]` key", quote_path(p)),
                ));
            }
            syn::Meta::List(_) => {
                return Err(syn::Error::new_spanned(
                    attr,
                    "unexpected `#[lua(...)]` form",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NewtypeAttrs {
    pub lua_type: Option<String>,
}

impl NewtypeAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("lua_newtype") && matches!(attr.meta, syn::Meta::List(_)) {
                let meta: syn::Meta = attr.parse_args()?;
                result.apply_meta(&meta, attr)?;
            }
        }

        if result.lua_type.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[lua_newtype] requires a `to` argument, e.g. #[lua_newtype(to = \"string\")]",
            ));
        }

        Ok(result)
    }

    fn apply_meta(&mut self, meta: &syn::Meta, attr: &Attribute) -> syn::Result<()> {
        match meta {
            syn::Meta::NameValue(nv) => {
                let key = nv.path.get_ident().map(|i| i.to_string());

                match key.as_deref() {
                    Some("to") => {
                        if self.lua_type.is_some() {
                            return Err(syn::Error::new_spanned(attr, "duplicate `to`"));
                        }

                        let val = extract_string_lit(&nv.value).ok_or_else(|| {
                            syn::Error::new_spanned(&nv.value, "expected string literal")
                        })?;

                        self.lua_type = Some(val);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!("unknown `#[lua_newtype({})]` key", key.unwrap_or_default()),
                        ));
                    }
                }
            }
            syn::Meta::Path(p) => {
                return Err(syn::Error::new_spanned(
                    p,
                    format!("unknown `#[lua_newtype({})]` key", quote_path(p)),
                ));
            }
            syn::Meta::List(_) => {
                return Err(syn::Error::new_spanned(
                    attr,
                    "unexpected `#[lua_newtype(...)]` form",
                ));
            }
        }

        Ok(())
    }
}

fn extract_string_lit(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(lit) = expr {
        if let syn::Lit::Str(s) = &lit.lit {
            return Some(s.value());
        }
    }

    None
}

fn quote_path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

pub fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Str(s) = &lit.lit {
                            return Some(s.value().trim().to_string());
                        }
                    }
                }
            }

            None
        })
        .collect();

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

pub fn detect_fallibility(ret: &syn::ReturnType) -> bool {
    let ty = match ret {
        syn::ReturnType::Type(_, ty) => ty.as_ref(),
        syn::ReturnType::Default => return false,
    };

    is_result_type(ty) || is_lua_result_type(ty)
}

fn is_result_type(ty: &Type) -> bool {
    if let Type::Path(typepath) = ty {
        if let Some(segment) = typepath.path.segments.last() {
            return segment.ident == "Result";
        }
    }

    false
}

fn is_lua_result_type(ty: &Type) -> bool {
    if let Type::Path(typepath) = ty {
        if let Some(segment) = typepath.path.segments.last() {
            return segment.ident == "LuaResult";
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_newtype_to_string() {
        let s: syn::ItemStruct = parse_quote! {
            #[lua_newtype(to = "string")]
            struct Foo;
        };
        let attrs = NewtypeAttrs::parse(&s.attrs).unwrap();
        assert_eq!(attrs.lua_type.as_deref(), Some("string"));
    }

    #[test]
    fn test_newtype_to_number() {
        let s: syn::ItemStruct = parse_quote! {
            #[lua_newtype(to = "number")]
            struct Bar;
        };
        let attrs = NewtypeAttrs::parse(&s.attrs).unwrap();
        assert_eq!(attrs.lua_type.as_deref(), Some("number"));
    }

    #[test]
    fn test_newtype_missing_to_is_error() {
        let s: syn::ItemStruct = parse_quote! {
            #[lua_newtype]
            struct Baz;
        };
        let result = NewtypeAttrs::parse(&s.attrs);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires a `to` argument"));
    }

    #[test]
    fn test_newtype_without_attribute_is_error() {
        let s: syn::ItemStruct = parse_quote! {
            struct Plain;
        };
        let result = NewtypeAttrs::parse(&s.attrs);
        assert!(result.is_err());
    }

    #[test]
    fn test_field_signature_parsing() {
        let s: syn::ItemStruct = parse_quote! {
            struct Foo {
                #[lua(signature = "fun(string): Bar")]
                pub parser: Function,
            }
        };
        let field = s.fields.iter().next().unwrap();
        let attrs = FieldAttrs::parse(&field.attrs).unwrap();
        assert_eq!(attrs.signature.as_deref(), Some("fun(string): Bar"));
    }

    #[test]
    fn test_field_signature_duplicate_is_error() {
        let s: syn::ItemStruct = parse_quote! {
            struct Foo {
                #[lua(signature = "a")]
                #[lua(signature = "b")]
                pub parser: Function,
            }
        };
        let field = s.fields.iter().next().unwrap();
        let result = FieldAttrs::parse(&field.attrs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn test_luaclass_annotation_parsing() {
        let s: syn::ItemStruct = parse_quote! {
            #[luaclass(annotation)]
            struct Metadata {
                pub title: String,
            }
        };
        let attrs = ClassAttrs::parse(&s.attrs).unwrap();
        assert!(attrs.annotation_only);
    }

    #[test]
    fn test_luaclass_annotation_duplicate_is_error() {
        let s: syn::ItemStruct = parse_quote! {
            #[luaclass(annotation, annotation)]
            struct Metadata;
        };
        let result = ClassAttrs::parse(&s.attrs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn test_luaclass_annotation_with_name() {
        let s: syn::ItemStruct = parse_quote! {
            #[luaclass(annotation, name = "Meta")]
            struct Metadata;
        };
        let attrs = ClassAttrs::parse(&s.attrs).unwrap();
        assert!(attrs.annotation_only);
        assert_eq!(attrs.name.as_deref(), Some("Meta"));
    }
}
