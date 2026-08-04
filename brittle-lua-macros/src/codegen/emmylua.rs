use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::{ClassDef, FnDef, ModuleDef};

pub fn generate(module: &ModuleDef) -> TokenStream {
    let mut parts: Vec<String> = Vec::new();

    for class in &module.classes {
        emit_class(&mut parts, class);
    }

    for func in &module.functions {
        emit_free_function(&mut parts, func);
    }

    parts.push(String::new());
    parts.push("return {}".to_string());

    let emmy_lua_str = parts.join("\n");

    quote! {
        pub static EMMY_LUA: &str = #emmy_lua_str;
    }
}

fn emit_class(parts: &mut Vec<String>, class: &ClassDef) {
    if let Some(ref doc) = class.doc {
        for line in doc.lines() {
            parts.push(format!("--- {}", line.trim()));
        }
    }

    parts.push(format!("--- @class {}", class.lua_name));

    for field in &class.fields {
        if field.skip {
            continue;
        }

        if let Some(ref doc) = field.doc {
            for line in doc.lines() {
                parts.push(format!("--- {}", line.trim()));
            }
        }

        let emmy_type = super::type_mapper::rust_type_to_emmylua(&field.ty);
        parts.push(format!("--- @field {} {}", field.lua_name, emmy_type));
    }

    for method in &class.methods {
        emit_method(parts, method, &class.lua_name);
    }

    parts.push(String::new());
}

fn emit_method(parts: &mut Vec<String>, method: &crate::ir::MethodDef, _class_name: &str) {
    if let Some(ref doc) = method.doc {
        for line in doc.lines() {
            parts.push(format!("--- {}", line.trim()));
        }
    }

    let ret_type = match &method.ret {
        syn::ReturnType::Type(_, ty) => {
            let emmy = super::type_mapper::rust_type_to_emmylua(ty);
            format!(" {}", emmy)
        }
        syn::ReturnType::Default => String::new(),
    };

    let arg_types: Vec<String> = method
        .args
        .iter()
        .map(|(name, ty)| {
            let emmy = super::type_mapper::rust_type_to_emmylua(ty);
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

fn emit_free_function(parts: &mut Vec<String>, func: &FnDef) {
    if let Some(ref doc) = func.doc {
        for line in doc.lines() {
            parts.push(format!("--- {}", line.trim()));
        }
    }

    for (name, ty) in &func.args {
        let emmy = super::type_mapper::rust_type_to_emmylua(ty);
        parts.push(format!("--- @param {} {}", name, emmy));
    }

    let ret_type = match &func.ret {
        syn::ReturnType::Type(_, ty) => {
            let emmy = super::type_mapper::rust_type_to_emmylua(ty);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ClassDef, FieldDef, MethodDef};
    use proc_macro2::Span;
    use syn::parse_quote;

    #[test]
    fn test_class_output() {
        let class = ClassDef {
            ident: parse_quote! { Foo },
            lua_name: "Foo".to_string(),
            doc: None,
            fields: vec![],
            methods: vec![],
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
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
                ty: parse_quote! { u16 },
                doc: None,
                default: None,
                skip: false,
                span: Span::call_site(),
            }],
            methods: vec![],
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
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
                ret: syn::ReturnType::Default,
                is_fallible: false,
                span: Span::call_site(),
            }],
            span: Span::call_site(),
        };
        let module = ModuleDef {
            classes: vec![class],
            functions: vec![],
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
        };
        let tokens = generate(&module);
        let output = tokens.to_string();

        assert!(output.contains("return {}"));
    }
}
