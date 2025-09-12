use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::builtins::files::files_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, Value};
use crate::path::Path;

pub fn install_headers(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let headers = files_impl(&args, interp)?
        .into_iter()
        .map(|file| file.path)
        .collect::<Vec<_>>();

    let install_dir = kwargs
        .get("install_dir")
        .map(Value::as_string)
        .transpose()
        .context_type("'install_dir' keyword argument must be of type string")?
        .unwrap_or("");

    interp
        .steps
        .install_headers(&Path::from(install_dir), &headers);

    Ok(Value::None)
}
