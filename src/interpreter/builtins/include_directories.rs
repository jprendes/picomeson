use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::files::files_impl;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};
use crate::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeDirectories {
    pub dirs: Vec<Path>,
}

impl MesonObject for IncludeDirectories {
    builtin_impl!();
}

pub fn include_directories(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let dirs = files_impl(&args, interp)?
        .into_iter()
        .map(|f| f.path)
        .collect();
    let inc_dirs = IncludeDirectories { dirs };
    Ok(inc_dirs.into_object())
}
