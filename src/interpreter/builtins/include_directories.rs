use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::files::{File, files_impl};
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeDirectories {
    dirs: Vec<File>,
}

impl MesonObject for IncludeDirectories {
    builtin_impl!();
}

pub fn include_directories(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let dirs = files_impl(&args)?;
    let inc_dirs = IncludeDirectories { dirs };
    Ok(inc_dirs.into_object())
}
