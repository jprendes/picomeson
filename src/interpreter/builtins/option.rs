use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::error::{ErrorContext as _, bail_type_error};
use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn option(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let opt: String = args
        .first()
        .context_type("First argument to option must be a string")?
        .as_string()?
        .into();

    let typ = kwargs
        .get("type")
        .context_type("Option requires a 'type' keyword argument")?
        .as_string()?;

    let value = kwargs.get("value");
    let value = match typ {
        "boolean" => {
            let bool_value = value.unwrap_or(&Value::Boolean(true)).as_bool()?;
            Value::Boolean(bool_value)
        }
        "integer" => {
            let int_value = value.unwrap_or(&Value::Integer(0)).as_integer()?;
            Value::Integer(int_value)
        }
        "string" | "combo" => {
            let string_value = value
                .unwrap_or(&Value::String(String::new()))
                .as_string()?
                .into();
            Value::String(string_value)
        }
        "array" => {
            let arr_value = value
                .unwrap_or(&Value::Array(vec![]))
                .as_array()?
                .iter()
                .map(|v| Ok(Value::String(v.as_string()?.into())))
                .collect::<Result<Vec<Value>, _>>()?;
            Value::Array(arr_value)
        }
        ty => bail_type_error!("Unsupported option type: {ty}"),
    };

    interp.options.insert(opt, value);

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
        Some(v) => Ok(v.clone()),
        None => bail_type_error!("No such option: {opt}"),
    }
}
