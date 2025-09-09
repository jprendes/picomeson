use std::env;
use std::path::PathBuf;

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

    let pwd = env::current_dir().unwrap();
    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            env::set_current_dir(&self.0).unwrap();
        }
    }
    let _restore = Restore(pwd.clone());
    env::set_current_dir(dir)
        .with_context_runtime(|| format!("Failed to change directory to {}", dir))?;

    let meson_code = std::fs::read_to_string("meson.build")
        .with_context_runtime(|| format!("Failed to read meson.build in subdir {}", dir))?;
    let statements = crate::parser::parse_meson_file(&meson_code)
        .with_context_runtime(|| format!("Failed to parse meson.build in subdir {}", dir))?;

    interp.interpret(statements)?;

    Ok(Value::None)
}
