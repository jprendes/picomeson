use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};
use crate::path::Path;

pub fn join_paths(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let parts = flatten(&args)
        .map(Value::as_string)
        .collect::<Result<Vec<_>, _>>()
        .context_type("All arguments to join_paths must be strings")?;

    let mut parts = parts.into_iter();

    let Some(path) = parts.next() else {
        return Ok(Value::String(String::new()));
    };

    let mut path = Path::from(path);

    for part in parts {
        path = path.join(part);
    }

    Ok(Value::String(path.to_string()))
}
