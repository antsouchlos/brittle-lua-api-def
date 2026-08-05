/// This example demonstrates `#[luaclass(annotation)]` and
/// `#[lua(signature = "...")]` working together.
///
/// `#[luaclass(annotation)]` generates EmmyLua `@class`/`@field` annotations
/// but skips UserData, FromLua, and register codegen.  It is useful for
/// documenting return types referenced in function signatures.
///
/// `#[lua(signature = "...")]` overrides the auto-generated EmmyLua type for
/// `mlua::Function` fields, letting you describe the function's parameter and
/// return types.
use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api {
    /// Metadata returned by a reference parser.
    #[luaclass(annotation)]
    pub struct PublicationMetadata {
        pub title: String,
        pub author: String,
        pub year: i32,
        pub journal: String,
    }

    /// A pluggable reference parser with a callback-based parser field.
    #[luaclass]
    #[derive(Clone)]
    pub struct RefParser {
        pub file_extension: String,
        #[lua(signature = "fun(src :string): PublicationMetadata")]
        pub parser: mlua::Function,
    }
}

fn main() {
    let lua = Lua::new();
    api::register(&lua).expect("register");

    println!("=== 10_annotation_class ===");
    println!("EmmyLua output:");
    println!("{}", api::EMMY_LUA);
}
