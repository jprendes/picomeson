use hashbrown::HashMap;

use crate::interpreter::builtins::files::files_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn install_headers(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let headers = files_impl(&args, interp)?;

    let install_dir = kwargs
        .get("install_dir")
        .map(Value::as_string)
        .transpose()
        .context_type("'install_dir' keyword argument must be of type string")?
        .unwrap_or("");

    // TODO: do something with this
    println!("Installing headers at {install_dir:?}:\n{headers:?}");

    Ok(Value::None)
}
