use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::slice::Iter;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::{InterpreterError, MesonObject, Value};

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
        .map(|arg| match arg {
            Value::String(s) => Ok(File {
                pwd: pwd.clone(),
                path: PathBuf::from(s),
            }),
            Value::Object(obj) => {
                let file = obj.borrow();
                let file = file.downcast_ref::<File>()?;
                Ok(file.clone())
            }
            _ => Err(InterpreterError::TypeError(format!(
                "Expected arguments to be strings or File objects, got {arg:?}",
            ))),
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
