use hashbrown::HashMap;

use crate::interpreter::{Interpreter, InterpreterError, Value, error::ErrorContext};

pub fn get(
    obj: &HashMap<String, Value>,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let key = args
        .first()
        .context_type("First argument to get is required")?
        .as_string()
        .context_type("First argument to get must be a string")?;

    let fallback = args.get(1);

    obj.get(key)
        .or(fallback)
        .cloned()
        .context_runtime("Key not found and no fallback value provided")
}

pub fn has_key(
    obj: &HashMap<String, Value>,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let key = args
        .first()
        .context_type("First argument to has_key is required")?
        .as_string()
        .context_type("First argument to has_key must be a string")?;

    Ok(Value::Boolean(obj.contains_key(key)))
}

pub fn keys(
    obj: &HashMap<String, Value>,
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let keys = obj.keys().cloned().map(Value::String).collect();
    Ok(Value::Array(keys))
}

pub fn values(
    obj: &HashMap<String, Value>,
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let values = obj.values().cloned().collect();
    Ok(Value::Array(values))
}
