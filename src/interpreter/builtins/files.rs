use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::utils::{AsValueSlice, flatten};
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value, bail_type_error};
use crate::os::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    path: Path,
}

impl MesonObject for File {
    builtin_impl!();
}

pub(super) fn files_impl<'a, 'b: 'a>(
    args: &'b (impl AsValueSlice<'a> + ?Sized),
    interp: &Interpreter,
) -> Result<Vec<File>, InterpreterError> {
    let pwd = &interp.current_dir;
    flatten(args)
        .map(|arg| {
            if let Ok(s) = arg.as_string() {
                Ok(File {
                    path: pwd.join(s),
                })
            } else if let Ok(file) = arg.as_object::<File>() {
                Ok(file.clone())
            } else {
                bail_type_error!("Expected arguments to be strings or File objects, got {arg:?}")
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

pub fn files(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let files = files_impl(&args, interp)?
        .into_iter()
        .map(MesonObject::into_object)
        .collect();
    Ok(Value::Array(files))
}
