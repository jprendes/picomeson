use std::path::PathBuf;

use hashbrown::HashMap;

use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn join_paths(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let mut path = PathBuf::new();

    let parts = flatten(&args)
        .map(Value::as_string)
        .collect::<Result<Vec<_>, _>>()
        .context_type("All arguments to join_paths must be strings")?;

    for part in parts {
        path.push(part);
    }

    Ok(Value::String(path.to_string_lossy().to_string()))
}
