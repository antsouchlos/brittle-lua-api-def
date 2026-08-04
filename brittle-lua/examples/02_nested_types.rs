use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod config {
    #[luaclass]
    #[derive(Clone)]
    pub struct Server {
        pub host: String,
        pub logging: LogConfig,
    }

    #[luaclass]
    #[derive(Clone)]
    pub struct LogConfig {
        pub level: String,
        pub file: Option<String>,
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    config::register(&lua)?;

    let server: config::Server = lua
        .load(
            r#"
            return {
                host = "0.0.0.0",
                logging = {
                    level = "debug",
                    file = "/var/log/server.log",
                },
            }
        "#,
        )
        .eval()?;

    println!("=== 02_nested_types ===");
    println!("Server {{ host: {:?} }}", server.host);
    println!(
        "LogConfig {{ level: {:?}, file: {:?} }}",
        server.logging.level, server.logging.file
    );

    Ok(())
}
