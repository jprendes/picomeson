use std::process::Command;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::{InterpreterError, MesonObject, Value};

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
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.stdout.clone()))
    }

    fn stderr(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.stderr.clone()))
    }

    fn returncode(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Integer(self.returncode))
    }
}

pub fn run_command(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let cmd_args = flatten(&args)
        .map(Value::as_string)
        .collect::<Result<Vec<_>, _>>()?;

    if cmd_args.is_empty() {
        return Err(InterpreterError::RuntimeError(
            "Expected at least one argument".into(),
        ));
    }

    let (stdout, stderr, status_code) = Command::new(cmd_args[0])
        .args(&cmd_args[1..])
        .output()
        .map(|output| (output.stdout, output.stderr, output.status.code()))
        .unwrap_or_else(|e| (Vec::new(), e.to_string().into_bytes(), Some(1)));

    Ok(RunResult {
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
        returncode: status_code.unwrap_or(1) as i64,
    }
    .into_object())
}
