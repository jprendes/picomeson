use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::{
    ErrorContext, InterpreterError, MesonObject, Value, bail_runtime_error, bail_type_error,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigData {
    data: HashMap<String, (Value, String)>,
}

impl ConfigData {
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
            None => bail_runtime_error!("Key '{key}' not found in ConfigData"),
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
                bail_type_error!("Expected int or bool as the second argument");
            }
        }
        self.set(args, kwargs)
    }

    fn merge_from(
        &mut self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let other = args.first().context_type(
            "merge_from requires a ConfigData object or a dict as the first argument",
        )?;
        if let Ok(other) = other.as_object::<ConfigData>() {
            self.data.extend(other.data.clone());
        } else if let Ok(dict) = other.as_dict() {
            let iter = dict
                .iter()
                .map(|(k, v)| (k.clone(), (v.clone(), String::new())));
            self.data.extend(iter);
        } else {
            bail_type_error!("merge_from requires a ConfigData object or a dict");
        };
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
        _ => bail_type_error!("configure_file 'input' keyword argument must be a string"),
    };
    let Some(Value::String(output)) = kwargs.get("output") else {
        bail_type_error!("configure_file requires an 'output' keyword argument of type string");
    };
    let configuration = kwargs
        .get("configuration")
        .context_type(
            "configure_file requires a 'configuration' keyword argument of type ConfigData",
        )?
        .as_object::<ConfigData>()?;
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
            v => bail_type_error!("Unsupported value type for key {key}: {v:?}"),
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
