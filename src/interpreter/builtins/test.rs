use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::{Interpreter, InterpreterError, Value};

pub fn test(
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    // TODO> implement the `test` builtin function
    Ok(Value::None)
}
