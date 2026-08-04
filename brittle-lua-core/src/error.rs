use mlua::Error;

pub fn field_missing(struct_name: &str, field_name: &str) -> Error {
    Error::external(format!(
        "failed to deserialize `{}`: missing field `{}`",
        struct_name, field_name
    ))
}

pub fn field_type_mismatch(struct_name: &str, field_name: &str, details: String) -> Error {
    Error::external(format!(
        "failed to deserialize `{}`: field `{}`: {}",
        struct_name, field_name, details
    ))
}

pub fn wrong_arguments_count(expected: usize, got: usize) -> Error {
    Error::external(format!(
        "wrong number of arguments: expected {}, got {}",
        expected, got
    ))
}
