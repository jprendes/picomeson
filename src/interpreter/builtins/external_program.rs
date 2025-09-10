use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalProgram {
    full_path: Option<String>,
}

impl ExternalProgram {
    pub fn found(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Boolean(self.full_path.is_some()))
    }

    pub fn full_path(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(path) = &self.full_path else {
            return Ok(Value::None);
        };
        Ok(Value::String(path.clone()))
    }
}

impl MesonObject for ExternalProgram {
    builtin_impl!(found, full_path);
}

pub fn find_program(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let prog = args
        .first()
        .context_type("Expected a string as the first argument")?
        .as_string()
        .context_type("Expected a string as the first argument")?;

    // Simple check if program exists in PATH
    let full_path = interp.os.find_program(prog, &interp.current_dir).ok();

    let found = full_path.is_some();

    let program = ExternalProgram { full_path }.into_object();

    if found {
        return Ok(program);
    }

    let required = kwargs
        .get("required")
        .map(Value::as_bool)
        .transpose()?
        .unwrap_or(false);

    if required {
        return Err(InterpreterError::RuntimeError(format!(
            "Program '{prog}' not found"
        )));
    }

    Ok(program)
}
