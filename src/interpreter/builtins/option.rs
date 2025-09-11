use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::{ErrorContext as _, bail_type_error};
use crate::interpreter::{Interpreter, InterpreterError, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum OptionType {
    Boolean,
    Integer(i64, i64),   // min, max
    String(Vec<String>), // allowed values for combo
    Array(Vec<String>),  // allowed values
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildOption {
    pub typ: OptionType,
    pub value: Value,
    pub description: String,
}

pub fn option(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let name: String = args
        .first()
        .context_type("First argument to option must be a string")?
        .as_string()?
        .into();

    let typ = kwargs
        .get("type")
        .context_type("Option requires a 'type' keyword argument")?
        .as_string()?;

    let description = kwargs
        .get("description")
        .map(Value::as_string)
        .transpose()
        .context_type("Expected 'description' keyword argument to be a string")?
        .unwrap_or_default();

    let choices = kwargs
        .get("choices")
        .map(Value::as_array)
        .transpose()
        .context_type("Expected 'choices' keyword argument to be an array")?
        .unwrap_or_default()
        .iter()
        .map(|v| Value::as_string(v).map(String::from))
        .collect::<Result<Vec<_>, _>>()?;

    let min = kwargs
        .get("min")
        .map(Value::as_integer)
        .transpose()
        .context_type("Expected 'min' keyword argument to be an integer")?
        .unwrap_or(i64::MIN);

    let max = kwargs
        .get("max")
        .map(Value::as_integer)
        .transpose()
        .context_type("Expected 'max' keyword argument to be an integer")?
        .unwrap_or(i64::MAX);

    let value = kwargs.get("value");
    let (value, typ) = match typ {
        "boolean" => {
            let bool_value = value.map(Value::as_boolean).transpose()?.unwrap_or(true);
            (Value::Boolean(bool_value), OptionType::Boolean)
        }
        "integer" => {
            let int_value = value.map(Value::as_integer).transpose()?.unwrap_or(0);
            (Value::Integer(int_value), OptionType::Integer(min, max))
        }
        "string" | "combo" => {
            let string_value = value
                .map(Value::as_string)
                .transpose()?
                .or_else(|| choices.first().map(String::as_str))
                .unwrap_or_default()
                .into();

            (Value::String(string_value), OptionType::String(choices))
        }
        "array" => {
            let arr_value = value
                .map(Value::as_array)
                .transpose()
                .context_type("Expected 'value' keyword argument to be an array")?
                .map(|arr| {
                    arr.iter()
                        .map(|v| v.as_string())
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .context_type("Expected all elements in 'value' array to be strings")?
                .unwrap_or_else(|| choices.iter().map(String::as_str).collect())
                .into_iter()
                .map(|v| Value::String(v.into()))
                .collect();

            (Value::Array(arr_value), OptionType::Array(choices))
        }
        ty => bail_type_error!("Unsupported option type: {ty}"),
    };

    let opt = BuildOption {
        typ,
        value: value.clone(),
        description: description.into(),
    };

    interp.options.insert(name, opt);

    Ok(Value::None)
}

pub fn get_option(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let opt: String = args
        .first()
        .context_type("First argument to get_option must be a string")?
        .as_string()?
        .into();

    match interp.options.get(&opt) {
        Some(v) => Ok(v.value.clone()),
        None => bail_type_error!("No such option: {opt}"),
    }
}
