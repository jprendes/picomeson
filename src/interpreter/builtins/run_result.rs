use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
struct RunResult {
    stdout: String,
    stderr: String,
    returncode: i64,
}

impl MesonObject for RunResult {
    builtin_impl!(stdout, stderr, returncode);
}

impl RunResult {
    fn stdout(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.stdout.clone()))
    }

    fn stderr(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.stderr.clone()))
    }

    fn returncode(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Integer(self.returncode))
    }
}

pub fn run_command(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let cmd_args = flatten(&args)
        .map(Value::as_string)
        .collect::<Result<Vec<_>, _>>()?;

    if cmd_args.is_empty() {
        return Err(InterpreterError::RuntimeError(
            "Expected at least one argument".into(),
        ));
    }

    let output = interp
        .os
        .run_command(&cmd_args)
        .context_runtime("Failed to run command")?;

    Ok(RunResult {
        stdout: output.stdout,
        stderr: output.stderr,
        returncode: output.returncode,
    }
    .into_object())
}
