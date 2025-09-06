use hashbrown::HashMap;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::files::{File, files_impl};
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
enum TargetType {
    StaticLibrary,
    Executable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildTarget {
    name: String,
    target_type: TargetType,
    sources: Vec<File>,
}

impl BuildTarget {
    fn extract_objects(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        // Placeholder implementation
        Ok(Value::None)
    }

    fn extract_all_objects(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        // Placeholder implementation
        Ok(Value::None)
    }
}

impl MesonObject for BuildTarget {
    builtin_impl!(extract_objects, extract_all_objects);
}

pub fn static_library(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(name)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "First argument to static_library must be a string (name)".into(),
        ));
    };

    let sources = files_impl(&args[1..])?;

    let lib = BuildTarget {
        name: name.clone(),
        target_type: TargetType::StaticLibrary,
        sources,
    };

    println!("Created static library: {:?}", lib);

    Ok(lib.into_object())
}

pub fn executable(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(name)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "First argument to executable must be a string (name)".into(),
        ));
    };

    let sources = files_impl(&args[1..])?;

    let bin = BuildTarget {
        name: name.clone(),
        target_type: TargetType::Executable,
        sources,
    };

    Ok(bin.into_object())
}
