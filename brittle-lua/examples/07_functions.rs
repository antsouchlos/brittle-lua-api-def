use mlua::Lua;

use crate::api::Config;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api {
    #[luaclass]
    #[derive(Clone)]
    pub struct Config {
        pub callbacks: Vec<mlua::Function>,
    }

    #[luamethods]
    impl Config {
        pub fn new() -> Self {
            Config {
                callbacks: Vec::new(),
            }
        }
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    api::register(&lua)?;

    let config: Config = lua
        .load(
            r#"
            return {
                callbacks = {
                    function()
                        print("Func 1")
                    end,
                    function(args, args2)
                        print("Func ", args, args2)
                    end
                }
            }
            "#,
        )
        .eval()?;

    for func in &config.callbacks {
        func.call::<()>(("sdf", "asdf"))?;
    }

    Ok(())
}
