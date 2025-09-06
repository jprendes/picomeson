use std::collections::HashMap;
use std::env;

use super::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::{ErrorContext, InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
struct Env {
    vars: HashMap<String, String>,
}

impl MesonObject for Env {
    builtin_impl!(prepend);
}

impl Env {
    fn prepend(
        &mut self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let mut args = flatten(&args).map(|v| v.as_string());

        let separator = match kwargs.get("separator") {
            Some(Value::String(s)) => Some(s.as_str()),
            None => None,
            Some(_) => {
                return Err(InterpreterError::TypeError(
                    "Expected 'separator' keyword argument to be a string".into(),
                ));
            }
        };

        let Some(variable) = args.next().transpose()? else {
            return Err(InterpreterError::TypeError(
                "Expected at least one arguments".into(),
            ));
        };

        let values = self
            .vars
            .get(variable)
            .map(|s| Ok(s.as_str()))
            .into_iter()
            .chain(args)
            .collect::<Result<Vec<_>, _>>()?;

        let value = if let Some(sep) = separator {
            values.join(sep)
        } else {
            env::join_paths(values)
                .context_runtime("msg Failed to join values")?
                .to_string_lossy()
                .into_owned()
        };

        self.vars.insert(variable.to_string(), value);

        Ok(Value::None)
    }
}

pub fn environment(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
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
