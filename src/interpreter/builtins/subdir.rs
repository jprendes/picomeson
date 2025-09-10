use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn subdir(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let dir = args
        .first()
        .context_type("First argument to subdir must be a string")?
        .as_string()
        .context_type("First argument to subdir must be a string")?;

    let pwd = interp.current_dir.clone();
    let dir = interp
        .os
        .join_paths(&[&pwd, dir])
        .context_runtime("Failed to join paths")?;

    let res = subdir_impl(&dir, interp);

    interp.current_dir = pwd;

    res?;

    Ok(Value::None)
}

fn subdir_impl(dir: &str, interp: &mut Interpreter) -> Result<(), InterpreterError> {
    interp.current_dir = dir.into();

    let file = interp
        .os
        .join_paths(&[dir, "meson.build"])
        .context_runtime("Failed to join paths")?;

    interp.interpret_file(&file)?;

    Ok(())
}
