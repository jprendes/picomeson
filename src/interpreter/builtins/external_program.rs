use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::builtin_impl;
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalProgram {
    full_path: Option<PathBuf>,
}

impl ExternalProgram {
    pub fn found(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Boolean(self.full_path.is_some()))
    }

    pub fn full_path(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(path) = &self.full_path else {
            return Ok(Value::None);
        };
        Ok(Value::String(path.to_string_lossy().into()))
    }
}

impl MesonObject for ExternalProgram {
    builtin_impl!(found, full_path);
}

pub fn find_program(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(prog)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "Expected a string as the first argument".into(),
        ));
    };

    // Simple check if program exists in PATH
    let full_path = Command::new("which")
        .arg(prog)
        .output()
        .map(|o| {
            o.status.success().then_some(
                String::from_utf8_lossy(&o.stdout)
                    .as_ref()
                    .trim()
                    .to_string(),
            )
        })
        .unwrap_or(None);

    let found = full_path.is_some();

    let program = ExternalProgram {
        full_path: full_path.map(PathBuf::from),
    }
    .into_object();

    if found {
        return Ok(program);
    }

    match kwargs.get("required") {
        Some(Value::Boolean(true)) => {
            return Err(InterpreterError::RuntimeError(format!(
                "Program '{prog}' not found",
            )));
        }
        None | Some(Value::Boolean(false)) => Ok(program),
        _ => {
            return Err(InterpreterError::TypeError(
                "The 'required' keyword argument must be a boolean".into(),
            ));
        }
    }
}
