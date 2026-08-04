/// Stores Lua callback functions in a Rust `Vec<mlua::Function>` and
/// dynamically registers new callbacks from Lua via `register_callback()`.
///
/// With mlua's `send` feature enabled, `CallbackRegistry` is `Send + Sync`
/// and can be shared across threads.
use std::sync::Arc;
use std::thread;

use mlua::{Lua, ObjectLike};

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

/// Compile-time proof that the generated struct satisfies `Send + Sync`.
fn assert_send_sync<T: Send + Sync>(_: &T) {}

fn main() -> mlua::Result<()> {
    let placeholder = callbacks::CallbackRegistry::new();
    assert_send_sync(&placeholder);

    let lua = Arc::new(Lua::new());
    callbacks::register(&lua)?;

    lua.load(
        r#"
        registry = CallbackRegistry.new()

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

    let lua2 = Arc::clone(&lua);

    let handle = thread::spawn(move || -> mlua::Result<()> {
        let globals = lua2.globals();
        let registry: mlua::AnyUserData = globals.get("registry")?;
        let count: usize = registry.call_method("count", ())?;
        println!(
            "Callback count accessed from thread: {} (struct is Send + Sync)",
            count
        );
        Ok(())
    });

    handle.join().unwrap()?;

    Ok(())
}
