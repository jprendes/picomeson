use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};
use crate::path::Path;

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
    let dir = pwd.join(dir);

    let res = subdir_impl(&dir, interp);

    interp.current_dir = pwd;

    res?;

    Ok(Value::None)
}

fn subdir_impl(dir: &Path, interp: &mut Interpreter) -> Result<(), InterpreterError> {
    interp.current_dir = dir.clone();

    let file = dir.join("meson.build");

    interp.interpret_file(&file)?;

    Ok(())
}
