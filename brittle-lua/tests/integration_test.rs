use mlua::Lua;

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api {
    /// A server configuration.
    #[luaclass(name = "ServerConfig")]
    #[derive(Clone)]

    pub struct ServerConfig {
        /// The bind address.
        pub bind: String,
        /// The port number.
        pub port: u16,
        /// Optional logging configuration.
        #[lua(default = "None")]
        pub logging: Option<LogConfig>,
        #[lua(skip)]
        #[lua(default = "42")]
        pub internal_id: u64,
    }

    #[luaclass]
    #[derive(Clone)]

    pub struct LogConfig {
        pub level: String,
        pub file: Option<String>,
    }

    #[luamethods]
    impl ServerConfig {
        #[lua(name = "get_bind")]
        pub fn bind_address(&self) -> String {
            self.bind.clone()
        }

        pub fn set_port(&mut self, port: u16) {
            self.port = port;
        }

        pub fn new(port: u16) -> mlua::Result<Self> {
            Ok(ServerConfig {
                bind: "127.0.0.1".into(),
                port,
                logging: None,
                internal_id: 0,
            })
        }
    }

    #[luafunction]
    fn env(name: String) -> mlua::Result<String> {
        std::env::var(&name).map_err(|e| mlua::Error::external(e))
    }
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_empty {
    #[luaclass]
    pub struct Empty {}
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_multi {
    #[luaclass]
    #[derive(Clone)]

    pub struct MultiMethod {
        pub value: i32,
    }

    #[luamethods]
    impl MultiMethod {
        pub fn new(value: i32) -> Self {
            Self { value }
        }

        pub fn get_value(&self) -> i32 {
            self.value
        }
    }

    #[luamethods]
    impl MultiMethod {
        pub fn double(&self) -> i32 {
            self.value * 2
        }
    }
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_missing {
    #[luaclass]
    #[derive(Debug)]

    pub struct RequiredFields {
        pub name: String,
        pub age: i32,
    }
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_type_mismatch {
    #[luaclass]
    #[derive(Debug)]

    pub struct TypedField {
        pub count: i32,
    }
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_rename {
    #[luaclass]
    #[derive(Clone)]

    pub struct RenamedField {
        #[lua(rename = "server_port")]
        pub port: u16,
    }
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_newtype {
    #[lua_newtype(to = "string")]
    #[derive(Debug, Clone)]

    pub struct LuaPathBuf(pub String);

    impl mlua::FromLua for LuaPathBuf {
        fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
            let s: String = mlua::FromLua::from_lua(value, lua)?;
            Ok(LuaPathBuf(s))
        }
    }

    impl mlua::IntoLua for LuaPathBuf {
        fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
            self.0.into_lua(lua)
        }
    }

    #[luaclass]
    #[derive(Clone)]

    pub struct AppConfig {
        pub data_dir: LuaPathBuf,
        pub port: u16,
    }
}

#[test]
fn register_and_create_proxy() {
    let lua = Lua::new();
    api::register(&lua).expect("register");

    let config: mlua::Value = lua
        .load("return ServerConfig.new(8080)")
        .eval()
        .expect("eval");

    assert!(config.is_userdata());
}

#[test]
fn from_lua_deserializes_table() {
    let lua = Lua::new();
    api::register(&lua).expect("register");

    let config: api::ServerConfig = lua
        .load("return { bind = '0.0.0.0', port = 443, logging = { level = 'info', file = 'log.txt' } }")
        .eval()
        .expect("eval");

    assert_eq!(config.bind, "0.0.0.0");
    assert_eq!(config.port, 443);
    assert_eq!(config.internal_id, 42);
    assert!(config.logging.is_some());
    let logging = config.logging.unwrap();
    assert_eq!(logging.level, "info");
    assert_eq!(logging.file, Some("log.txt".into()));
}

#[test]
fn fields_accessible_from_lua() {
    let lua = Lua::new();
    api::register(&lua).expect("register");

    lua.load(
        r#"
        local cfg = ServerConfig.new(3000)
        assert(cfg.bind == "127.0.0.1")
        assert(cfg.port == 3000)
        cfg:set_port(4000)
        assert(cfg.port == 4000)
        assert(cfg:get_bind() == "127.0.0.1")
        "#,
    )
    .exec()
    .expect("exec");
}

#[test]
fn free_function_works() {
    let lua = Lua::new();
    api::register(&lua).expect("register");

    std::env::set_var("TEST_BRITTLE", "hello_lua");

    let result: String = lua
        .load(r#"return env("TEST_BRITTLE")"#)
        .eval()
        .expect("eval");

    assert_eq!(result, "hello_lua");
}

#[test]
fn emmy_lua_is_generated() {
    let emmy = api::EMMY_LUA;
    assert!(emmy.contains("@class ServerConfig"));
    assert!(emmy.contains("@class LogConfig"));
    assert!(emmy.contains("@field bind string"));
    assert!(emmy.contains("return {}"));
}

#[test]
fn test_empty_class() {
    let lua = Lua::new();
    api_empty::register(&lua).expect("register");

    let _: api_empty::Empty = lua.load("return {}").eval().expect("eval");

    let emmy = api_empty::EMMY_LUA;
    assert!(emmy.contains("@class Empty"));
}

#[test]
fn test_multiple_methods_blocks() {
    let lua = Lua::new();
    api_multi::register(&lua).expect("register");

    lua.load(
        r#"
        local m = MultiMethod.new(21)
        assert(m:get_value() == 21)
        assert(m:double() == 42)
        "#,
    )
    .exec()
    .expect("exec");
}

#[test]
fn test_missing_field_errors() {
    let lua = Lua::new();
    api_missing::register(&lua).expect("register");

    let result: mlua::Result<api_missing::RequiredFields> =
        lua.load("return { name = 'hello' }").eval();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("missing field"));
    assert!(err.contains("age"));
}

#[test]
fn test_type_mismatch_errors() {
    let lua = Lua::new();
    api_type_mismatch::register(&lua).expect("register");

    let result: mlua::Result<api_type_mismatch::TypedField> =
        lua.load("return { count = 'not a number' }").eval();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("TypedField"));
    assert!(err.contains("count"));
}

#[test]
fn test_renamed_field() {
    let lua = Lua::new();
    api_rename::register(&lua).expect("register");

    let settings: api_rename::RenamedField = lua
        .load("return { server_port = 8080 }")
        .eval()
        .expect("eval");

    assert_eq!(settings.port, 8080);
}

#[test]
fn test_emmylua_contains_all_annotations() {
    let emmy = api::EMMY_LUA;

    assert!(emmy.contains("@class ServerConfig"));
    assert!(emmy.contains("@class LogConfig"));

    assert!(emmy.contains("@field bind string"));
    assert!(emmy.contains("@field port integer"));
    assert!(emmy.contains("@field logging LogConfig?"));
    assert!(emmy.contains("@field level string"));
    assert!(emmy.contains("@field file string?"));

    assert!(emmy.contains("@field get_bind fun(self): string"));
    assert!(emmy.contains("@field set_port fun(self, port: integer)"));
    assert!(emmy.contains("@field new fun(port: integer): ServerConfig"));

    assert!(emmy.contains("function env(name) end"));
    assert!(emmy.contains("@param name string"));
    assert!(emmy.contains("@return string"));

    assert!(emmy.contains("return {}"));
}

#[test]
fn test_newtype_from_lua_deserializes_string() {
    let lua = Lua::new();
    api_newtype::register(&lua).expect("register");

    let config: api_newtype::AppConfig = lua
        .load("return { data_dir = '/var/lib/app', port = 8080 }")
        .eval()
        .expect("eval");

    assert_eq!(config.data_dir.0, "/var/lib/app");
    assert_eq!(config.port, 8080);
}

#[test]
fn test_newtype_emmylua_maps_to_string() {
    let emmy = api_newtype::EMMY_LUA;

    assert!(
        emmy.contains("@field data_dir string"),
        "expected `string` for data_dir, got:\n{}",
        emmy
    );
    assert!(
        !emmy.contains("LuaPathBuf"),
        "EMMY_LUA should not reference the wrapper type LuaPathBuf:\n{}",
        emmy
    );
    assert!(emmy.contains("@class AppConfig"));
    assert!(emmy.contains("@field port integer"));
    assert!(emmy.contains("return {}"));
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_signature {
    #[luaclass]
    #[derive(Clone)]

    pub struct RefParser {
        pub file_extension: String,
        #[lua(signature = "fun(string): PublicationMetadata")]
        pub parser: mlua::Function,
    }
}

#[brittle_lua::lua_module(crate = "brittle_lua")]
mod api_annotation_only {
    #[luaclass(annotation, name = "PubMeta")]
    #[derive(Clone)]

    pub struct PublicationMetadata {
        pub title: String,
        pub author: String,
        pub year: i32,
        pub journal: String,
    }
}

#[test]
fn test_signature_field_emmylua_override() {
    let emmy = api_signature::EMMY_LUA;

    assert!(
        emmy.contains("--- @field parser fun(string): PublicationMetadata"),
        "expected custom signature, got:\n{}",
        emmy
    );
    assert!(
        !emmy.contains("--- @field parser function"),
        "should not contain default `function` type, got:\n{}",
        emmy
    );
    assert!(emmy.contains("--- @class RefParser"));
}

#[test]
fn test_annotation_only_class_emmylua() {
    let emmy = api_annotation_only::EMMY_LUA;

    assert!(
        emmy.contains("--- @class PubMeta"),
        "expected @class PubMeta, got:\n{}",
        emmy
    );
    assert!(emmy.contains("--- @field title string"));
    assert!(emmy.contains("--- @field author string"));
    assert!(emmy.contains("--- @field year integer"));
    assert!(emmy.contains("--- @field journal string"));
}

#[test]
fn test_annotation_only_no_register() {
    use mlua::Lua;

    let lua = Lua::new();
    let result = api_annotation_only::register(&lua);
    assert!(result.is_ok());

    // annotation-only classes should not be registered as proxies

    let result: mlua::Result<mlua::Value> = lua.load("return PubMeta").eval();
    assert!(result.is_err() || matches!(result, Ok(mlua::Value::Nil)));
}

#[test]
fn test_signature_ref_parser_register() {
    use mlua::Lua;

    let lua = Lua::new();
    api_signature::register(&lua).expect("register");

    // Verify class is registered and callable via Lua

    let result: mlua::Result<mlua::Value> = lua.load("return RefParser").eval();
    assert!(result.is_ok());
}
