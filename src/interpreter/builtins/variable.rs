use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn set_variable(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let name = args
        .first()
        .context_type("First argument to set_variable must be a string")?
        .as_string()
        .context_type("First argument to set_variable must be a string")?;
    let value = args.get(1).unwrap_or(&Value::None).cloned();
    interp.variables.insert(name.into(), value);
    Ok(Value::None)
}

pub fn is_variable(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let name = args
        .first()
        .context_type("First argument to is_variable must be a string")?
        .as_string()
        .context_type("First argument to is_variable must be a string")?;
    Ok(Value::Boolean(interp.variables.contains_key(name)))
}

pub fn get_variable(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let name = args
        .first()
        .context_type("First argument to get_variable must be a string")?
        .as_string()
        .context_type("First argument to get_variable must be a string")?;
    match interp.variables.get(name) {
        Some(value) => Ok(value.clone()),
        None => args.get(1).cloned().context_undef_variable(name),
    }
}
