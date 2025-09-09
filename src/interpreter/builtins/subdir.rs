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
    let dir = interp.os_env.join_paths(&[&pwd, dir]);

    let res = subdir_impl(&dir, interp);

    interp.current_dir = pwd;

    res?;

    Ok(Value::None)
}

fn subdir_impl(dir: &str, interp: &mut Interpreter) -> Result<(), InterpreterError> {
    interp.current_dir = dir.into();

    let file = interp.os_env.join_paths(&[dir, "meson.build"]);

    let meson_code = interp
        .os_env
        .read_file(&file)
        .with_context_runtime(|| format!("Failed to read meson.build in subdir {}", dir))?;

    let meson_code = String::from_utf8(meson_code)
        .with_context_runtime(|| format!("File is not utf-8 encoded in subdir {}", dir))?;

    let statements = crate::parser::parse_meson_file(&meson_code)
        .with_context_runtime(|| format!("Failed to parse meson.build in subdir {}", dir))?;

    interp.interpret(statements)?;

    Ok(())
}
