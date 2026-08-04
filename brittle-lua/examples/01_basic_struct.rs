use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod config {
    #[luaclass]
    #[derive(Clone)]
    pub struct Server {
        pub host: String,
        pub port: u16,
        pub debug: bool,
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
                port = 8443,
                debug = true,
            }
        "#,
        )
        .eval()?;

    println!("=== 01_basic_struct ===");
    println!(
        "Server {{ host: {:?}, port: {}, debug: {} }}",
        server.host, server.port, server.debug
    );

    Ok(())
}
