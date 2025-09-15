use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::build_target::get_dir;
use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{
    Interpreter, InterpreterError, MesonObject, Value, bail_runtime_error, bail_type_error,
};
use crate::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigData {
    data: HashMap<String, (Value, String)>,
}

impl ConfigData {
    fn get(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
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
        _interp: &mut Interpreter,
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
        interp: &mut Interpreter,
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
        self.set(args, kwargs, interp)
    }

    fn merge_from(
        &mut self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
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

pub struct ConfigureFile {
    pub build_dir: Path,
    pub filename: Path,
    pub content: String,
    pub install_dir: Path,
    pub install: bool,
}

pub fn configure_file(
    _args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let input = kwargs
        .get("input")
        .map(Value::as_string)
        .transpose()
        .context_type("configure_file 'input' keyword argument must be a string")?;

    let output = kwargs
        .get("output")
        .context_type("configure_file requires an 'output' keyword argument")?
        .as_string()
        .context_type("configure_file 'output' keyword argument must be a string")?;

    let configuration = kwargs
        .get("configuration")
        .context_type(
            "configure_file requires a 'configuration' keyword argument of type ConfigData",
        )?
        .as_object::<ConfigData>()?;

    let prefix =
        get_dir(interp, "prefix")?.context_runtime("Could not determine installation prefix")?;

    let install_dir = kwargs
        .get("install_dir")
        .map(Value::as_string)
        .transpose()
        .context_type("configure_file 'install_dir' keyword argument must be a string")?
        .map(|s| prefix.join(s))
        .unwrap_or_default();

    let install = kwargs
        .get("install")
        .map(Value::as_boolean)
        .transpose()
        .context_type("configure_file 'install' keyword argument must be a bool")?
        .unwrap_or(false);

    if input.is_some() {
        // TODO: implement this
        return Ok(Value::None);
    }

    let mut data = configuration.data.iter().collect::<Vec<_>>();
    data.sort_by_key(|a| a.0);

    let mut content = String::from("#pragma once\n\n");
    for (key, (value, desc)) in data.iter() {
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

    let file = ConfigureFile {
        build_dir: interp.build_dir.clone(),
        filename: Path::from(output),
        content,
        install_dir,
        install,
    };

    interp.steps.configure_file(&file);

    Ok(Value::None)
}

pub fn configuration_data(
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    Ok(ConfigData::default().into_object())
}
