use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn format(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    Ok(Value::String(
        Value::String(obj.into()).format_string(&args),
    ))
}

pub fn split(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let separator = args
        .first()
        .map(Value::as_string)
        .transpose()
        .context_type("First argument to split must be a string")?
        .unwrap_or(" ");

    let parts: Vec<Value> = obj
        .split(separator)
        .map(|p| Value::String(p.into()))
        .collect();

    Ok(Value::Array(parts))
}

pub fn join(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let result = args
        .iter()
        .map(|v| v.coerce_string())
        .collect::<Vec<_>>()
        .join(obj);
    Ok(Value::String(result))
}

pub fn strip(
    obj: &str,
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    Ok(Value::String(obj.trim().into()))
}

pub fn startswith(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let prefix = args
        .first()
        .context_type("First argument to startswith must be a string")?
        .as_string()
        .context_type("First argument to startswith must be a string")?;

    Ok(Value::Boolean(obj.starts_with(prefix)))
}

pub fn endswith(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let suffix = args
        .first()
        .context_type("First argument to endswith must be a string")?
        .as_string()
        .context_type("First argument to endswith must be a string")?;

    Ok(Value::Boolean(obj.ends_with(suffix)))
}

pub fn substring(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let len: i64 = obj
        .len()
        .try_into()
        .context_type("String length exceeds i64")?;

    let start = args
        .first()
        .map(Value::as_integer)
        .transpose()
        .context_type("First argument to substring must be an integer")?
        .unwrap_or(0);

    let end = args
        .get(1)
        .map(Value::as_integer)
        .transpose()
        .context_type("Second argument to substring must be an integer")?
        .unwrap_or(len);

    if len == 0 {
        return Ok(Value::String(String::new()));
    }

    let start = start + if start < 0 { len - 1 } else { 0 };
    let end = end + if end < 0 { len - 1 } else { 0 };

    let start = start.clamp(0, len - 1);
    let end = end.clamp(start, len - 1);
    let start = start as usize;
    let end = end as usize;

    Ok(Value::String(obj[start..end].into()))
}

pub fn contains(
    obj: &str,
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let substring = args
        .first()
        .context_type("First argument to contains must be a string")?
        .as_string()
        .context_type("First argument to contains must be a string")?;

    Ok(Value::Boolean(obj.contains(substring)))
}

pub fn underscorify(
    obj: &str,
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let underscorified = obj
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    Ok(Value::String(underscorified))
}

pub fn to_upper(
    obj: &str,
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    Ok(Value::String(obj.to_uppercase()))
}

pub fn to_lower(
    obj: &str,
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    Ok(Value::String(obj.to_lowercase()))
}
