use hashbrown::HashMap;
use std::path::{Path, PathBuf};

use super::builtin_impl;
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct FileSystem;

impl MesonObject for FileSystem {
    builtin_impl!(replace_suffix, exists, is_file, is_dir);
}

impl FileSystem {
    fn is_file(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };
        Ok(Value::Boolean(Path::new(path).is_file()))
    }

    fn is_dir(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };
        Ok(Value::Boolean(Path::new(path).is_dir()))
    }

    fn exists(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };
        Ok(Value::Boolean(Path::new(path).exists()))
    }

    fn replace_suffix(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };
        let Some(Value::String(suffix)) = args.get(1) else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };
        let mut path = PathBuf::from(path);
        path.set_extension(suffix);
        Ok(Value::String(path.to_string_lossy().into_owned()))
    }
}

pub fn filesystem() -> FileSystem {
    FileSystem
}
