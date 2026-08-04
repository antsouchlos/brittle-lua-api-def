use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod helpers {
    /// Greet someone by name.
    #[luafunction]
    fn greet(name: String) -> mlua::Result<String> {
        Ok(format!("hello, {}!", name))
    }

    /// Add two integers.
    #[luafunction]
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    helpers::register(&lua)?;

    let greeting: String = lua.load(r#"return greet("world")"#).eval()?;

    let sum: i64 = lua.load(r#"return add(3, 4)"#).eval()?;

    println!("=== 04_free_functions ===");
    println!("greet(\"world\") -> {:?}", greeting);
    println!("add(3, 4)      -> {}", sum);

    Ok(())
}
