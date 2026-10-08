use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api {
    #[luaglobal]
    fn default_port() -> u16 {
        8080
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    api::register(&lua)?;

    let original_port: u16 = lua
        .load(
            r#"
            return default_port
            "#,
        )
        .eval()?;

    println!("=== 11_global_variables ===");
    println!("default_port: {}", original_port);

    println!();
    println!("EmmyLua output:");
    println!("{}", api::EMMY_LUA);

    Ok(())
}
