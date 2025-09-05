use std::collections::HashMap;

use crate::interpreter::builtins::filesystem::filesystem;
use crate::interpreter::{InterpreterError, MesonObject as _, Value};

pub fn import(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(module_name)) = args.get(0) else {
        return Err(InterpreterError::TypeError(
            "import requires a string argument".to_string(),
        ));
    };

    match module_name.as_str() {
        "fs" => Ok(filesystem().into_object()),
        _ => Err(InterpreterError::RuntimeError(format!(
            "No module named '{module_name}'",
        ))),
    }
}
