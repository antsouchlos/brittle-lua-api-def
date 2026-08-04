use syn::{Attribute, Type};

#[derive(Debug, Default)]
pub struct FieldAttrs {
    pub default: Option<String>,
    pub rename: Option<String>,
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
}

impl ClassAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut class_attrs = Self::default();

        for attr in attrs {
            if attr.path().is_ident("luaclass") && matches!(attr.meta, syn::Meta::List(_)) {
                let meta: syn::Meta = attr.parse_args()?;
                class_attrs.apply_meta(&meta, attr)?;
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
                return Err(syn::Error::new_spanned(
                    p,
                    format!("unknown `#[luaclass({})]` key", quote_path(p)),
                ));
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
