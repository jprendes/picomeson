use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

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
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };
        let is_file = interp
            .os_env
            .is_file(path)
            .context_runtime("Failed to check if path is a file")?;
        Ok(Value::Boolean(is_file))
    }

    fn is_dir(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };

        let is_dir = interp
            .os_env
            .is_dir(path)
            .context_runtime("Failed to check if path is a directory")?;
        Ok(Value::Boolean(is_dir))
    }

    fn exists(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(path)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string argument".into(),
            ));
        };

        let exists = interp
            .os_env
            .exists(path)
            .context_runtime("Failed to check if path exists")?;
        Ok(Value::Boolean(exists))
    }

    fn replace_suffix(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
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

        // replace the extension of the path in `path` with `suffix` without using `PathBuf`.

        let path = set_extension(path, suffix);
        Ok(Value::String(path))
    }
}

pub fn filesystem() -> FileSystem {
    FileSystem
}

fn set_extension(path: &str, suffix: &str) -> String {
    // replace the extension of the path in `path` with `suffix` without using `PathBuf`.

    // Find the last occurrence of '/' or '\'
    let last_separator = path.rfind(['/', '\\']);

    // Find the last occurrence of '.' after the last separator (or from start if no separator)
    let search_start = last_separator.map(|i| i + 1).unwrap_or(0);
    let last_dot = path[search_start..].rfind('.').map(|i| search_start + i);

    if let Some(dot_pos) = last_dot {
        // Replace everything after the last dot with the new suffix
        let mut new_path = String::from(&path[..dot_pos]);
        if !suffix.starts_with('.') {
            new_path.push('.');
        }
        new_path.push_str(suffix);
        new_path
    } else {
        // No extension found, append the suffix
        let mut new_path = String::from(path);
        if !suffix.starts_with('.') {
            new_path.push('.');
        }
        new_path.push_str(suffix);
        new_path
    }
}
