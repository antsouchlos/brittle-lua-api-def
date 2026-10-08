# brittle-lua-api-def

A Rust proc-macro library for defining Lua configuration APIs, built on
[mlua](https://github.com/mlua-rs/mlua) with Lua 5.5.

## Quick start

```rust
use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod config {
    #[luaclass]
    #[derive(Clone)]
    pub struct Server {
        pub host: String,
        pub port: u16,
    }

    #[luaglobal]
    fn default_port() -> u16 {
        8443
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    config::register(&lua)?;

    let server: config::Server = lua.load(
        r#"
        return {
            host = "0.0.0.0",
            port = default_port
        }
        "#
    ).eval()?;

    println!("host: {}, port: {}", server.host, server.port);
    Ok(())
}
```

## EmmyLua output

The generated `EMMY_LUA` constant contains EmmyLua annotation strings for use
with Lua LSP servers:

```rust
use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api {
    #[luaclass]
    pub struct Server {
        pub host: String,
        pub port: u16,
    }
}

fn main() -> mlua::Result<()> {
    std::fs::write("api.lua", api::EMMY_LUA)?;
    Ok(())
}
```

The output (`api.lua`) contains:

```
--- @class Server
--- @field host string
--- @field port integer

return {}
```

## Examples

See `brittle-lua/examples/` for runnable examples:

```
cargo run --example 01_basic_struct -p brittle-lua
```
