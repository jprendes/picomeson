use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};
use crate::os::Path;

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
        let path = args
            .first()
            .context_type("Expected a string argument")?
            .as_string()
            .context_type("Expected a string argument")?;

        let path = interp.current_dir.join(path);

        let is_file = interp
            .os
            .is_file(&path)
            .context_runtime("Failed to check if path is a file")?;
        Ok(Value::Boolean(is_file))
    }

    fn is_dir(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let path = args
            .first()
            .context_type("Expected a string argument")?
            .as_string()
            .context_type("Expected a string argument")?;

        let path = interp.current_dir.join(path);

        let is_dir = interp
            .os
            .is_dir(&path)
            .context_runtime("Failed to check if path is a directory")?;
        Ok(Value::Boolean(is_dir))
    }

    fn exists(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let path = args
            .first()
            .context_type("Expected a string argument")?
            .as_string()
            .context_type("Expected a string argument")?;

        let path = interp.current_dir.join(path);

        let exists = interp
            .os
            .exists(&path)
            .context_runtime("Failed to check if path exists")?;
        Ok(Value::Boolean(exists))
    }

    fn replace_suffix(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let path = args
            .first()
            .map(Value::as_string)
            .context_type("First argument to replace_suffix must be a string")?
            .context_type("First argument to replace_suffix must be a string")?;

        let suffix = args
            .get(1)
            .map(Value::as_string)
            .context_type("Second argument to replace_suffix must be a string")?
            .context_type("Second argument to replace_suffix must be a string")?;

        // replace the extension of the path in `path` with `suffix` without using `PathBuf`.

        let path = Path::from(path).set_extension(suffix);
        Ok(Value::String(path.to_string()))
    }
}

pub fn filesystem() -> FileSystem {
    FileSystem
}
