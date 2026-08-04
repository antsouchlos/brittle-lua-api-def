/// This example demonstrates `#[lua_newtype]` for wrapping types that don't
/// have a native Lua representation (e.g. `PathBuf`).
///
/// Use `#[lua_newtype(to = "string")]` to tell the codegen what EmmyLua
/// annotation to emit.  The runtime conversion is handled by your own
/// `FromLua` and `IntoLua` impls on the wrapper.
use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api {
    use std::path::PathBuf;

    /// Wraps PathBuf so it deserializes from a Lua string.
    #[lua_newtype(to = "string")]
    #[derive(Debug, Clone)]
    pub struct LuaPath(pub PathBuf);

    impl mlua::FromLua for LuaPath {
        fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
            let s: String = mlua::FromLua::from_lua(value, lua)?;
            Ok(LuaPath(PathBuf::from(s)))
        }
    }

    impl mlua::IntoLua for LuaPath {
        fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
            self.0.to_string_lossy().into_owned().into_lua(lua)
        }
    }

    /// Application configuration with filesystem paths.
    #[luaclass]
    #[derive(Clone)]
    pub struct App {
        pub data_dir: LuaPath,
        pub log_file: LuaPath,
        pub port: u16,
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    api::register(&lua)?;

    let app: api::App = lua
        .load(
            r#"
            return {
                data_dir = "/var/lib/myapp",
                log_file = "/var/log/myapp/debug.log",
                port     = 8080,
            }
        "#,
        )
        .eval()?;

    println!("=== 10_newtype ===");

    println!(
        "App {{ data_dir: {:?}, log_file: {:?}, port: {} }}",
        app.data_dir.0, app.log_file.0, app.port
    );

    println!();
    println!("EmmyLua for the App class:");
    println!("{}", api::EMMY_LUA);

    Ok(())
}
