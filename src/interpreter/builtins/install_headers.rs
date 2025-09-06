use hashbrown::HashMap;
use std::path::PathBuf;

use crate::interpreter::builtins::files::files_impl;
use crate::interpreter::{InterpreterError, Value};

pub fn install_headers(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
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
