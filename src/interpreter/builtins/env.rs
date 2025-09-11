use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
struct Env {
    vars: HashMap<String, String>,
}

impl MesonObject for Env {
    builtin_impl!(prepend);
}

const DEFAULT_SEPARATOR: &str = ":";

impl Env {
    fn prepend(
        &mut self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let variable = args
            .first()
            .context_type("Expected the first argument to be a string representing the environment variable name")?
            .as_string()
            .context_type("Expected the first argument to be a string representing the environment variable name")?;

        let new_values = flatten(&args[1..]).map(|v| v.as_string());

        let separator = kwargs
            .get("separator")
            .map(Value::as_string)
            .transpose()
            .context_type("Expected 'separator' keyword argument to be a string")?
            .unwrap_or(DEFAULT_SEPARATOR);

        let old_value = self.vars.get(variable).map(|s| Ok(s.as_str()));
        let values = new_values.chain(old_value).collect::<Result<Vec<_>, _>>()?;
        let value = values.join(separator);

        self.vars.insert(variable.to_string(), value);

        Ok(Value::None)
    }
}

pub fn environment(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let vars = match args.first() {
        Some(Value::Dict(data)) => data
            .iter()
            .map(|(k, v)| match v {
                Value::String(s) => Ok((k.clone(), s.clone())),
                _ => Err(InterpreterError::TypeError(
                    "Expected environment values to be strings".into(),
                )),
            })
            .collect::<Result<HashMap<_, _>, _>>()?,
        None => HashMap::new(),
        Some(_) => {
            return Err(InterpreterError::TypeError(
                "Expected a dict object as the first argument".into(),
            ));
        }
    };
    Ok(Env { vars }.into_object())
}
