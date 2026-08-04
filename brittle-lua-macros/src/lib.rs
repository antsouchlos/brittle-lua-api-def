mod codegen;
mod ir;
mod parser;
mod util;
mod validator;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn lua_module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_mod = match syn::parse::<syn::ItemMod>(item.clone()) {
        Ok(m) => m,
        Err(e) => return e.to_compile_error().into(),
    };

    let module_def = match parser::parse(&item_mod) {
        Ok(def) => def,
        Err(e) => return e.to_compile_error().into(),
    };

    match validator::validate(&module_def) {
        Ok(()) => {}
        Err(diagnostics) => {
            let errors: Vec<_> = diagnostics
                .into_iter()
                .map(|d| d.into_compile_error())
                .collect();
            return quote! { #(#errors)* }.into();
        }
    }

    let crate_path = parse_crate_arg(_attr);

    codegen::generate(&module_def, &item_mod, &crate_path).into()
}

fn parse_crate_arg(attr: TokenStream) -> String {
    if attr.is_empty() {
        return "::brittle_lua".to_string();
    }

    let parsed = syn::parse::<CrateArg>(attr);
    match parsed {
        Ok(arg) => arg.path,
        Err(_) => "::brittle_lua".to_string(),
    }
}

struct CrateArg {
    path: String,
}

impl syn::parse::Parse for CrateArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident != "crate" {
            return Err(syn::Error::new(ident.span(), "expected `crate`"));
        }

        input.parse::<syn::Token![=]>()?;

        let lit: syn::LitStr = input.parse()?;
        Ok(CrateArg { path: lit.value() })
    }
}

impl validator::Diagnostic {
    fn into_compile_error(self) -> proc_macro2::TokenStream {
        syn::Error::new(self.span, self.message).to_compile_error()
    }
}
