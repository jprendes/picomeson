use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn get(
    obj: &[Value],
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let len: i64 = obj
        .len()
        .try_into()
        .context_type("Array length exceeds i64")?;

    let idx = args
        .first()
        .context_type("First argument to get is required")?
        .as_integer()
        .context_type("First argument to get must be an integer")?;

    let idx = idx + if idx < 0 { len } else { 0 };

    let fallback = args.get(1);

    idx.try_into()
        .ok()
        .and_then(|idx: usize| obj.get(idx))
        .or(fallback)
        .cloned()
        .context_runtime("Index out of range and no fallback value provided")
}

pub fn length(
    obj: &[Value],
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let len: i64 = obj
        .len()
        .try_into()
        .context_type("Array length exceeds i64")?;
    Ok(Value::Integer(len))
}

pub fn contains(
    obj: &[Value],
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let item = args
        .first()
        .context_type("First argument to contains is required")?;

    Ok(Value::Boolean(obj.contains(item)))
}
