use std::collections::BTreeSet;

use proc_macro2::Span;

use crate::ir::{ModuleDef, ReceiverKind};

#[derive(Debug)]

pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

pub fn validate(module: &ModuleDef) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    let mut class_lua_names: BTreeSet<&str> = BTreeSet::new();

    for class in &module.classes {
        if !class_lua_names.insert(&class.lua_name) {
            diagnostics.push(Diagnostic {
                message: format!("duplicate Lua class name '{}'", class.lua_name),
                span: class.span,
            });
        }

        let mut method_lua_names: BTreeSet<&str> = BTreeSet::new();

        for method in &class.methods {
            if !method_lua_names.insert(&method.lua_name) {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "duplicate Lua name '{}' in class '{}'",
                        method.lua_name, class.ident
                    ),
                    span: method.span,
                });
            }
        }

        for field in &class.fields {
            if !class.annotation_only
                && field.skip
                && field.default.is_none()
                && !is_option_type(&field.field_type)
            {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "skipped field '{}' has no default: add #[lua(default = \"...\")] or make it Option<T>",
                        field.ident
                    ),
                    span: field.span,
                });
            }

            if field.skip && field.default.is_some() && is_option_type(&field.field_type) {
                // Valid: has default or Option
            }
        }

        if class.annotation_only && !class.methods.is_empty() {
            diagnostics.push(Diagnostic {
                message: format!(
                    "annotation-only class '{}' cannot have methods",
                    class.ident
                ),
                span: class.span,
            });
        }

        for method in &class.methods {
            if method.receiver == ReceiverKind::Owned
                && !method.is_fallible
                && returns_self(&method.return_type, &class.ident)
            {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "by-value method '{}' in '{}' returns Self but is not fallible",
                        method.ident, class.ident
                    ),
                    span: method.span,
                });
            }
        }
    }

    let mut func_lua_names: BTreeSet<&str> = BTreeSet::new();
    for func in &module.functions {
        if !func_lua_names.insert(&func.lua_name) {
            diagnostics.push(Diagnostic {
                message: format!("duplicate Lua function name '{}'", func.lua_name),
                span: func.span,
            });
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }

    false
}

fn returns_self(return_type: &syn::ReturnType, self_ident: &syn::Ident) -> bool {
    match return_type {
        syn::ReturnType::Type(_, ty) => {
            if let syn::Type::Path(tp) = ty.as_ref() {
                if let Some(seg) = tp.path.segments.last() {
                    return seg.ident == *self_ident;
                }
            }
            false
        }

        syn::ReturnType::Default => false,
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::ir::{ClassDef, FieldDef, FnDef, MethodDef, ModuleDef, ReceiverKind};
    use proc_macro2::Span;
    use std::collections::BTreeMap;
    use syn::parse_quote;

    fn call_site() -> Span {
        Span::call_site()
    }

    #[test]
    fn test_duplicate_lua_class_names() {
        let class_a = ClassDef {
            ident: parse_quote! { ClassA },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };
        let class_b = ClassDef {
            ident: parse_quote! { ClassB },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class_a, class_b],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0]
            .message
            .contains("duplicate Lua class name 'MyClass'"));
    }

    #[test]
    fn test_duplicate_method_names_in_class() {
        let method_a = MethodDef {
            ident: parse_quote! { method_a },
            lua_name: "do_thing".into(),
            doc: None,
            receiver: ReceiverKind::None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let method_b = MethodDef {
            ident: parse_quote! { method_b },
            lua_name: "do_thing".into(),
            doc: None,
            receiver: ReceiverKind::None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![],
            methods: vec![method_a, method_b],
            annotation_only: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0]
            .message
            .contains("duplicate Lua name 'do_thing' in class 'MyClass'"));
    }

    #[test]
    fn test_duplicate_free_function_names() {
        let func_a = FnDef {
            ident: parse_quote! { func_a },
            lua_name: "my_func".into(),
            doc: None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let func_b = FnDef {
            ident: parse_quote! { func_b },
            lua_name: "my_func".into(),
            doc: None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![],
            functions: vec![func_a, func_b],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0]
            .message
            .contains("duplicate Lua function name 'my_func'"));
    }

    #[test]
    fn test_skipped_field_without_default_or_option() {
        let field = FieldDef {
            ident: parse_quote! { my_field },
            lua_name: "my_field".into(),
            field_type: parse_quote! { i32 },
            doc: None,
            default: None,
            signature: None,
            skip: true,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![field],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0]
            .message
            .contains("skipped field 'my_field' has no default"));
    }

    #[test]
    fn test_skipped_option_field_valid() {
        let field = FieldDef {
            ident: parse_quote! { my_field },
            lua_name: "my_field".into(),
            field_type: parse_quote! { Option<String> },
            doc: None,
            default: None,
            signature: None,
            skip: true,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![field],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_skipped_field_with_default_valid() {
        let field = FieldDef {
            ident: parse_quote! { my_field },
            lua_name: "my_field".into(),
            field_type: parse_quote! { i32 },
            doc: None,
            default: Some("0".into()),
            signature: None,
            skip: true,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![field],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_owned_method_returning_self_not_fallible() {
        let method = MethodDef {
            ident: parse_quote! { into_inner },
            lua_name: "into_inner".into(),
            doc: None,
            receiver: ReceiverKind::Owned,
            args: vec![],
            return_type: parse_quote! { -> MyClass },
            is_fallible: false,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![],
            methods: vec![method],
            annotation_only: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains(
            "by-value method 'into_inner' in 'MyClass' returns Self but is not fallible"
        ));
    }

    #[test]
    fn test_valid_module() {
        let field = FieldDef {
            ident: parse_quote! { name },
            lua_name: "name".into(),
            field_type: parse_quote! { String },
            doc: None,
            default: None,
            signature: None,
            skip: false,
            span: call_site(),
        };
        let method = MethodDef {
            ident: parse_quote! { greet },
            lua_name: "greet".into(),
            doc: None,
            receiver: ReceiverKind::Ref,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { MyClass },
            lua_name: "MyClass".into(),
            doc: None,
            fields: vec![field],
            methods: vec![method],
            annotation_only: false,
            span: call_site(),
        };
        let func = FnDef {
            ident: parse_quote! { my_func },
            lua_name: "my_func".into(),
            doc: None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![func],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_err_count() {
        let class_a = ClassDef {
            ident: parse_quote! { ClassA },
            lua_name: "Duplicate".into(),
            doc: None,
            fields: vec![],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };
        let class_b = ClassDef {
            ident: parse_quote! { ClassB },
            lua_name: "Duplicate".into(),
            doc: None,
            fields: vec![],
            methods: vec![],
            annotation_only: false,
            span: call_site(),
        };

        let method_a = MethodDef {
            ident: parse_quote! { m_a },
            lua_name: "dup_method".into(),
            doc: None,
            receiver: ReceiverKind::None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let method_b = MethodDef {
            ident: parse_quote! { m_b },
            lua_name: "dup_method".into(),
            doc: None,
            receiver: ReceiverKind::None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };

        let skipped_field = FieldDef {
            ident: parse_quote! { bad_field },
            lua_name: "bad_field".into(),
            field_type: parse_quote! { i32 },
            doc: None,
            default: None,
            signature: None,
            skip: true,
            span: call_site(),
        };

        let class_c = ClassDef {
            ident: parse_quote! { ClassC },
            lua_name: "ClassC".into(),
            doc: None,
            fields: vec![skipped_field],
            methods: vec![method_a, method_b],
            annotation_only: false,
            span: call_site(),
        };

        let module = ModuleDef {
            classes: vec![class_a, class_b, class_c],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn test_annotation_only_class_cannot_have_methods() {
        let method = MethodDef {
            ident: parse_quote! { do_thing },
            lua_name: "do_thing".into(),
            doc: None,
            receiver: ReceiverKind::None,
            args: vec![],
            return_type: syn::ReturnType::Default,
            is_fallible: false,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { Meta },
            lua_name: "Meta".into(),
            doc: None,
            fields: vec![],
            methods: vec![method],
            annotation_only: true,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("cannot have methods"));
    }

    #[test]
    fn test_annotation_only_skips_skipped_field_validation() {
        let field = FieldDef {
            ident: parse_quote! { secret },
            lua_name: "secret".into(),
            field_type: parse_quote! { i32 },
            doc: None,
            default: None,
            signature: None,
            skip: true,
            span: call_site(),
        };
        let class = ClassDef {
            ident: parse_quote! { Meta },
            lua_name: "Meta".into(),
            doc: None,
            fields: vec![field],
            methods: vec![],
            annotation_only: true,
            span: call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
            type_map: BTreeMap::new(),
        };
        let result = validate(&module);
        assert!(result.is_ok());
    }
}
