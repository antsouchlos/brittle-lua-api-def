use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod config {
    #[luaclass(name = "ServerConfig")]
    #[derive(Clone)]
    pub struct Server {
        /// Lua uses "server_port" as the key.
        #[lua(rename = "server_port")]
        pub port: u16,

        #[lua(default = r#""MyServer".to_string()"#)]
        pub name: String,

        /// Not exposed to Lua. Gets the default value from Rust.
        #[lua(skip)]
        #[lua(default = r#""generated-default".into()"#)]
        pub instance_id: String,
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    config::register(&lua)?;

    let server: config::Server = lua
        .load(
            r#"
            return {
                server_port = 9090,
            }
        "#,
        )
        .eval()?;

    println!("=== 05_field_attrs ===");
    println!(
        "port:        {}   (from Lua key `server_port`)",
        server.port
    );
    println!(
        "name:        {}   (not provided field, generated in Rust from default)",
        server.name
    );
    println!(
        "instance_id: {:?}   (skipped field, generated in Rust from default)",
        server.instance_id
    );

    Ok(())
}
