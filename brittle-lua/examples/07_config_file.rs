/// Practical example: parse a Lua config file string into a Rust struct,
/// validate the values, and run Lua code that uses the type directly.
use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod config {
    #[luaclass(name = "ServerConfig")]
    #[derive(Clone)]
    pub struct ServerConfig {
        pub host: String,
        pub port: u16,
        pub workers: u16,
        pub tls_enabled: bool,
    }

    #[luamethods]
    impl ServerConfig {
        pub fn listen_addr(&self) -> String {
            format!(
                "{}://{}:{}",
                if self.tls_enabled { "https" } else { "http" },
                self.host,
                self.port
            )
        }
    }
}

fn main() -> mlua::Result<()> {
    let lua_file_content = r#"
        return {
            host = "0.0.0.0",
            port = 8443,
            workers = 4,
            tls_enabled = true,
        }
    "#;

    let lua = Lua::new();
    config::register(&lua)?;

    let server_config: config::ServerConfig = lua.load(lua_file_content).eval()?;

    println!("=== 07_config_file ===");
    println!("Parsed from Lua config file:");
    println!("  host:        {}", server_config.host);
    println!("  port:        {}", server_config.port);
    println!("  workers:     {}", server_config.workers);
    println!("  tls_enabled: {}", server_config.tls_enabled);
    println!();

    // Call the generated method from Rust
    let addr = server_config.listen_addr();
    println!("listen_addr() -> {:?}", addr);

    Ok(())
}
