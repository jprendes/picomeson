use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::slice::Iter;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::{InterpreterError, MesonObject, Value, bail_type_error};

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pwd: PathBuf,
    path: PathBuf,
}

impl MesonObject for File {
    builtin_impl!();
}

pub(super) fn files_impl<'a>(
    args: impl IntoIterator<Item = &'a Value, IntoIter = Iter<'a, Value>>,
) -> Result<Vec<File>, InterpreterError> {
    let pwd = env::current_dir().unwrap();
    flatten(args)
        .map(|arg| {
            if let Ok(s) = arg.as_string() {
                Ok(File {
                    pwd: pwd.clone(),
                    path: PathBuf::from(s),
                })
            } else if let Ok(file) = arg.as_object::<File>() {
                Ok(file.clone())
            } else {
                bail_type_error!("Expected arguments to be strings or File objects, got {arg:?}")
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

pub fn files(args: Vec<Value>, _kwargs: HashMap<String, Value>) -> Result<Value, InterpreterError> {
    let files = files_impl(&args)?
        .into_iter()
        .map(MesonObject::into_object)
        .collect();
    Ok(Value::Array(files))
}
