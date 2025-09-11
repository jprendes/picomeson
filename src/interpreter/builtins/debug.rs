use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn assert(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let cond = args
        .first()
        .context_type("assert requires at least one argument")?
        .as_boolean()
        .context_type("First argument to assert must be a boolean")?;

    let msg = args
        .get(1)
        .map(Value::as_string)
        .transpose()
        .context_type("Second argument to assert must be a string")?;

    if cond {
        return Ok(Value::None);
    }

    let mut err_msg = String::from("Assertion failed");
    if let Some(msg) = msg {
        err_msg.push_str(": ");
        err_msg.push_str(msg.trim_matches('"'));
    }

    Err(InterpreterError::RuntimeError(err_msg))
}

pub fn message(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let mut output = String::new();
    for arg in args {
        output.push_str(&arg.coerce_string());
        output.push(' ');
    }
    interp.os.print(&output);
    Ok(Value::None)
}

pub fn error(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let msg = args
        .iter()
        .map(|v| v.coerce_string())
        .collect::<Vec<_>>()
        .join(" ");
    Err(InterpreterError::RuntimeError(msg))
}

pub fn warning(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let mut output = String::from("WARNING: ");
    for arg in args {
        output.push_str(&arg.coerce_string());
        output.push(' ');
    }
    interp.os.print(&output);
    Ok(Value::None)
}
