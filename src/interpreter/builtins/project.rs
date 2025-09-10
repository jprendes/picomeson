use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn project(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    // Project definition
    let project_name = args
        .first()
        .context_type("First argument to project must be a string")?
        .as_string()
        .context_type("First argument to project must be a string")?;

    let default_version = Value::String("0.0.0".into());
    let project_version = kwargs
        .get("version")
        .map(|v| match v {
            Value::None => &default_version,
            _ => v,
        })
        .unwrap_or(&default_version)
        .as_string()
        .context_type("Expected 'version' keyword argument to be a string")?;

    let mut meson = interp.meson.borrow_mut();
    meson.project_version = project_version.into();
    meson.project_name = project_name.into();

    Ok(Value::None)
}

pub fn add_project_arguments(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let language = kwargs.get("language");
    let language = flatten(&language)
        .map(Value::as_string)
        .collect::<Result<Vec<_>, _>>()?;

    let arguments = flatten(&args)
        .map(|v| Ok(Value::as_string(v)?.into()))
        .collect::<Result<Vec<_>, _>>()?;

    let mut meson = interp.meson.borrow_mut();
    for lang in language {
        meson.project_args.insert(lang.into(), arguments.clone());
    }
    Ok(Value::None)
}
