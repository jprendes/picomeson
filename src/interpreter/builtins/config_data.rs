use std::collections::HashMap;

use super::builtin_impl;
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigData {
    data: HashMap<String, (Value, String)>,
}

impl ConfigData {
    fn from_dict(dict: HashMap<String, Value>) -> Self {
        let data = dict
            .into_iter()
            .map(|(k, v)| (k, (v, String::new())))
            .collect();
        Self { data }
    }

    fn get(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(key)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string as the first argument".into(),
            ));
        };

        match self.data.get(key) {
            Some((value, _)) => Ok(value.clone()),
            None => Err(InterpreterError::RuntimeError(format!(
                "Key '{key}' not found in ConfigData"
            ))),
        }
    }

    fn set(
        &mut self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(key)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string as the first argument".into(),
            ));
        };
        let value = match args.get(1) {
            Some(Value::String(s)) => Value::String(s.clone()),
            Some(Value::Integer(i)) => Value::Integer(*i),
            Some(Value::Boolean(b)) => Value::Boolean(*b),
            _ => {
                return Err(InterpreterError::TypeError(
                    "Expected a str, int or bool as the second argument".into(),
                ));
            }
        };
        let description = match kwargs.get("description") {
            Some(Value::String(s)) => s,
            None => &String::new(),
            _ => {
                return Err(InterpreterError::TypeError(
                    "Expected a string for the 'description' keyword argument".into(),
                ));
            }
        };

        self.data
            .insert(key.clone(), (value.clone(), description.clone()));
        Ok(Value::None)
    }

    fn set10(
        &mut self,
        mut args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        match args.get_mut(1) {
            Some(v @ Value::Boolean(true)) => {
                *v = Value::Integer(1);
            }
            Some(v @ Value::Boolean(false)) => {
                *v = Value::Integer(1);
            }
            Some(v @ Value::Integer(..=0)) => {
                *v = Value::Integer(0);
            }
            Some(v @ Value::Integer(1..)) => {
                *v = Value::Integer(1);
            }
            _ => {
                return Err(InterpreterError::TypeError(
                    "Expected int or bool as the second argument".into(),
                ));
            }
        }
        self.set(args, kwargs)
    }

    fn merge_from(
        &mut self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let other = match args.first() {
            Some(Value::Object(other)) => other.borrow().downcast_ref::<ConfigData>()?.clone(),
            Some(Value::Dict(dict)) => ConfigData::from_dict(dict.clone()),
            _ => {
                return Err(InterpreterError::TypeError(
                    "merge_from requires a ConfigData object or a dict".to_string(),
                ));
            }
        };
        self.data.extend(other.data);
        Ok(Value::None)
    }
}

impl MesonObject for ConfigData {
    builtin_impl!(get, set, set10, merge_from);
}

pub fn configure_file(
    _args: Vec<Value>,
    kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let input = match kwargs.get("input") {
        Some(Value::String(s)) => Some(s.clone()),
        None => None,
        _ => {
            return Err(InterpreterError::TypeError(
                "configure_file 'input' keyword argument must be a string".to_string(),
            ));
        }
    };

    let Some(Value::String(output)) = kwargs.get("output") else {
        return Err(InterpreterError::TypeError(
            "configure_file requires an 'output' keyword argument of type string".to_string(),
        ));
    };

    let Some(Value::Object(configuration)) = kwargs.get("configuration") else {
        return Err(InterpreterError::TypeError(
            "configure_file requires a 'configuration' keyword argument of type ConfigData"
                .to_string(),
        ));
    };

    let configuration = configuration.borrow();
    let configuration = configuration.downcast_ref::<ConfigData>()?;

    if input.is_some() {
        // TODO: implement this
        return Ok(Value::None);
    }

    let mut content = String::from("#pragma once\n\n");

    for (key, (value, desc)) in configuration.data.iter() {
        if !desc.is_empty() {
            content.push_str(&format!("// {}\n", desc));
        }
        match value {
            Value::Boolean(true) => {
                content.push_str(&format!("#define {}\n", key));
            }
            Value::Boolean(false) => {
                content.push_str(&format!("#undef {}\n", key));
            }
            Value::Integer(i) => {
                content.push_str(&format!("#define {} {}\n", key, i));
            }
            Value::String(s) => {
                content.push_str(&format!("#define {} {}\n", key, s));
            }
            v => {
                Err(InterpreterError::RuntimeError(format!(
                    "Unsupported value type for key {}: {:?}",
                    key, v
                )))?;
            }
        }
        content.push('\n');
    }

    // TODO: Actualy output this file, and handle paths correctly
    println!("Should be writing output file: {}", output);
    println!("With content:\n{}", content);

    Ok(Value::None)
}

pub fn configuration_data(
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    Ok(ConfigData::default().into_object())
}
