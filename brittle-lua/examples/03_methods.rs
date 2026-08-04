use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod calculator {
    #[luaclass]
    #[derive(Clone)]
    pub struct Calc {
        pub value: i64,
    }

    #[luamethods]
    impl Calc {
        pub fn new(initial: i64) -> mlua::Result<Self> {
            Ok(Calc { value: initial })
        }

        pub fn get(&self) -> i64 {
            self.value
        }

        pub fn add(&mut self, n: i64) {
            self.value += n;
        }

        pub fn multiply(&mut self, n: i64) {
            self.value *= n;
        }

        pub fn consume(self) -> i64 {
            self.value
        }
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    calculator::register(&lua)?;

    lua.load(
        r#"
        local c1 = Calc.new(10)
        print("initial:", c1:get())       -- 10

        c1:add(5)
        print("after add(5):", c1:get())  -- 15

        c1:multiply(3)
        print("after multiply(3):", c1:get()) -- 45

        local final_value = c1:consume()
        print("consumed:", final_value)    -- 45
        "#,
    )
    .exec()?;

    Ok(())
}
