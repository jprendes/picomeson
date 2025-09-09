use std::process::Command;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::{
    Interpreter, InterpreterError, MesonObject, Value, bail_runtime_error, bail_type_error,
};

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
    _interp: &mut Interpreter,
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

    let program = ExternalProgram { full_path }.into_object();

    if found {
        return Ok(program);
    }

    match kwargs.get("required") {
        None | Some(Value::Boolean(false)) => Ok(program),
        Some(Value::Boolean(true)) => bail_runtime_error!("Program '{prog}' not found"),
        _ => bail_type_error!("The 'required' keyword argument must be a boolean"),
    }
}
