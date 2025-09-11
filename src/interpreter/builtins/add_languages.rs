use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn add_languages(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let lang = args
        .first()
        .context_type("First argument to add_languages must be a string")?
        .as_string()
        .context_type("First argument to add_languages must be a string")?;

    let required = kwargs
        .get("required")
        .map(Value::as_boolean)
        .transpose()
        .context_type("'required' keyword argument must be of type bool")?
        .unwrap_or(false);

    let compiler = interp.os.get_compiler(lang);

    if required {
        compiler
            .as_ref()
            .with_context_runtime(|| format!("No compiler found for language: {lang}"))?;
    }

    Ok(Value::Boolean(compiler.is_ok()))
}
