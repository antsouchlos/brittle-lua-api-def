use std::collections::BTreeMap;

use proc_macro2::Span;
use syn::{Ident, ReturnType, Type};

#[derive(Clone)]
pub struct ModuleDef {
    pub classes: Vec<ClassDef>,
    pub functions: Vec<FnDef>,
    pub type_map: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct ClassDef {
    pub ident: Ident,
    pub lua_name: String,
    pub doc: Option<String>,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDef>,
    pub annotation_only: bool,
    pub span: Span,
}

#[derive(Clone)]
pub struct FieldDef {
    pub ident: Ident,
    pub lua_name: String,
    pub field_type: Type,
    pub doc: Option<String>,
    pub default: Option<String>,
    pub signature: Option<String>,
    pub skip: bool,
    pub span: Span,
}

#[derive(Clone)]
pub struct MethodDef {
    pub ident: Ident,
    pub lua_name: String,
    pub doc: Option<String>,
    pub receiver: ReceiverKind,
    pub args: Vec<(Ident, Type)>,
    pub return_type: ReturnType,
    pub is_fallible: bool,
    pub span: Span,
}

#[derive(Clone)]
pub struct FnDef {
    pub ident: Ident,
    pub lua_name: String,
    pub doc: Option<String>,
    pub args: Vec<(Ident, Type)>,
    pub return_type: ReturnType,
    pub is_fallible: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReceiverKind {
    None,
    Ref,
    MutRef,
    Owned,
}
