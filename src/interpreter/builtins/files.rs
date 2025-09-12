use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use hashbrown::HashMap;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::utils::{AsValueSlice, flatten};
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value, bail_type_error};
use crate::os::Path;

#[derive(Clone, PartialEq)]
pub struct File {
    pub path: Path,
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File({})", self.path)
    }
}

impl MesonObject for File {
    builtin_impl!();
}

impl File {
    pub fn from_path(path: impl AsRef<str>) -> Self {
        Self {
            path: Path::from(path.as_ref()),
        }
    }
}

pub(super) fn files_impl<'a, 'b: 'a>(
    args: &'b (impl AsValueSlice<'a> + ?Sized),
    interp: &Interpreter,
) -> Result<Vec<File>, InterpreterError> {
    let pwd = &interp.current_dir;
    flatten(args)
        .map(|arg| {
            if let Ok(s) = arg.as_string() {
                Ok(File::from_path(pwd.join(s)))
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
