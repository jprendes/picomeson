use std::collections::HashMap;

use crate::interpreter::builtins::filesystem::filesystem;
use crate::interpreter::{
    InterpreterError, MesonObject as _, Value, bail_runtime_error, bail_type_error,
};

pub fn import(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(module_name)) = args.first() else {
        bail_type_error!("import requires a string argument");
    };

    match module_name.as_str() {
        "fs" => Ok(filesystem().into_object()),
        _ => bail_runtime_error!("No module named '{module_name}'"),
    }
}
