use hashbrown::HashMap;

use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn join_paths(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let parts = flatten(&args)
        .map(Value::as_string)
        .collect::<Result<Vec<_>, _>>()
        .context_type("All arguments to join_paths must be strings")?;

    let path = interp.os_env.join_paths(&parts);

    Ok(Value::String(path))
}
