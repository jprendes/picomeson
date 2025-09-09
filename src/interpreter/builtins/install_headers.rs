use std::path::PathBuf;

use hashbrown::HashMap;

use crate::interpreter::builtins::files::files_impl;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn install_headers(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let headers = files_impl(&args)?;

    let install_dir = match kwargs.get("install_dir") {
        Some(Value::String(s)) => PathBuf::from(s),
        None => PathBuf::from(""),
        _ => {
            return Err(InterpreterError::TypeError(
                "'install_dir' keyword argument must be of type string".into(),
            ));
        }
    };
    // TODO: do something with this
    println!("Installing headers at {install_dir:?}:\n{:?}", headers);
    Ok(Value::None)
}
