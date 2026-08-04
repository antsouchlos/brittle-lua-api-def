# brittle-lua

A Rust proc-macro library for defining Lua configuration APIs, built on
[mlua](https://github.com/mlua-rs/mlua) with Lua 5.5 (Luau).

The `#[lua_module]` attribute processes a `mod` block and generates:

- `UserData + FromLua` impls for `#[luaclass]` structs
- A `register(&Lua)` function that wires everything into the Lua runtime
- A `static EMMY_LUA: &str` with EmmyLua annotations for LSP autocompletion

## Installation

```toml
[dependencies]
brittle-lua = "0.1.0"
mlua = "0.10"
```

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
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    config::register(&lua)?;

    let server: config::Server = lua.load(r#"
        return { host = "0.0.0.0", port = 8443 }
    "#).eval()?;

    println!("host: {}, port: {}", server.host, server.port);
    Ok(())
}
```

## Attributes

### `#[luaclass]` — structs

A struct marked `#[luaclass]` gets `UserData + FromLua` impls.

| Option | Example | Effect |
|---|---|---|
| `name = "..."` | `#[luaclass(name = "ServerConfig")]` | Override the Lua type name |

**Field attributes** (`#[lua(...)]` on struct fields):

| Option | Example | Effect |
|---|---|---|
| `rename = "..."` | `#[lua(rename = "server_port")]` | Change the Lua key name |
| `skip` | `#[lua(skip)]` | Exclude from Lua serialization |
| `default = "expr"` | `#[lua(default = "42")]` | Rust expression used when field is absent |

A skipped field without a `default` or `Option<T>` type produces a compile error.

### `#[luamethods]` — impl blocks

Methods in a `#[luamethods] impl` block become Lua methods. All four receiver kinds
are supported:

| Rust receiver | Lua registration |
|---|---|
| `&self` | `add_method` |
| `&mut self` | `add_method_mut` |
| `self` (by value) | `add_method` with clone |
| no self (constructor) | `add_function` |

Methods returning `Result<T, E>` (where `E: Into<mlua::Error>`) are detected as
fallible and errors are propagated to Lua. Plain return types are wrapped in `Ok(...)`.

| Method option | Example | Effect |
|---|---|---|
| `name = "..."` | `#[lua(name = "get_port")]` | Override the Lua method name |

### `#[luafunction]` — free functions

Standalone functions marked `#[luafunction]` are registered as Lua globals.

## Generated register function

The macro emits `pub fn register(lua: &Lua) -> mlua::Result<()>`. Call it once
to make all classes and functions available in the Lua runtime. It is safe to
call multiple times — mlua's `create_proxy` caches by `TypeId`.

## EmmyLua output

The generated `EMMY_LUA` constant contains EmmyLua annotation strings for use
with Lua LSP servers:

```
--- A server configuration.
--- @class ServerConfig
--- The bind address.
--- @field host string
--- The port number.
--- @field port integer
--- @field new fun(port: integer): ServerConfig
--- @field listen_addr fun(self): string

return {}
```

## Crate path

The `crate` argument tells the macro where to find mlua types:

```rust
#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api { ... }
```

Generated code uses this path as a prefix for all mlua type references
(e.g. `brittle_lua::mlua::UserData`).

## Examples

See `brittle-lua/examples/` for runnable examples:

```
cargo run --example 01_basic_struct  -p brittle-lua
cargo run --example 02_nested_types  -p brittle-lua
cargo run --example 03_methods       -p brittle-lua
cargo run --example 04_free_functions -p brittle-lua
cargo run --example 05_field_attrs   -p brittle-lua
cargo run --example 06_emmylua       -p brittle-lua
cargo run --example 07_config_file   -p brittle-lua
```

## Error messages

When `FromLua` deserialization fails, errors include the struct name and field
that caused the problem:

```
failed to deserialize `ServerConfig`: field `port`: invalid type: string "abc",
expected u16
```

Missing required fields produce:

```
failed to deserialize `ServerConfig`: missing field `port`
```

## License

MIT
