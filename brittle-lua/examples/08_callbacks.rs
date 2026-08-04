/// Stores Lua callback functions in a Rust `Vec<mlua::Function>` and
/// dynamically registers new callbacks from Lua via `register_callback()`.
use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod callbacks {
    #[luaclass]
    #[derive(Clone)]
    pub struct CallbackRegistry {
        #[lua(skip)]
        #[lua(default = "Vec::new()")]
        pub callbacks: Vec<mlua::Function>,
    }

    #[luamethods]
    impl CallbackRegistry {
        pub fn new() -> Self {
            CallbackRegistry {
                callbacks: Vec::new(),
            }
        }

        pub fn register_callback(&mut self, callback: mlua::Function) {
            self.callbacks.push(callback);
        }

        pub fn count(&self) -> usize {
            self.callbacks.len()
        }

        pub fn fire(&self, message: String) -> mlua::Result<()> {
            for cb in &self.callbacks {
                cb.call::<()>(message.as_str())?;
            }
            Ok(())
        }
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    callbacks::register(&lua)?;

    lua.load(
        r#"
        local registry = CallbackRegistry.new()

        registry:register_callback(function(msg)
            print("[LOG]    " .. msg)
        end)

        registry:register_callback(function(msg)
            print("[UPPER]  " .. string.upper(msg))
        end)

        registry:register_callback(function(msg)
            print("[LENGTH] " .. #msg .. " chars")
        end)

        print("Registered callbacks: " .. registry:count())

        registry:fire("hello from lua")
        "#,
    )
    .exec()?;

    Ok(())
}
