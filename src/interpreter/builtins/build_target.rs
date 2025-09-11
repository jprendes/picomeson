use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::files::{File, files_impl};
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

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
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        // Placeholder implementation
        Ok(Value::None)
    }

    fn extract_all_objects(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        // Placeholder implementation
        Ok(Value::None)
    }

    fn full_path(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let name = match self.target_type {
            TargetType::StaticLibrary => format!("lib{}.a", self.name),
            TargetType::Executable => self.name.clone(),
        };
        let path = interp.meson.borrow().build_dir.join(name);
        // Placeholder implementation
        Ok(Value::String(path.to_string()))
    }
}

impl MesonObject for BuildTarget {
    builtin_impl!(extract_objects, extract_all_objects, full_path);
}

pub fn static_library(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(name)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "First argument to static_library must be a string (name)".into(),
        ));
    };

    let sources = files_impl(&args[1..], interp)?;

    let lib = BuildTarget {
        name: name.clone(),
        target_type: TargetType::StaticLibrary,
        sources,
    };

    interp
        .os
        .print(&format!("Created static library: {:?}", lib));

    Ok(lib.into_object())
}

pub fn executable(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(name)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "First argument to executable must be a string (name)".into(),
        ));
    };

    let sources = files_impl(&args[1..], interp)?;

    let bin = BuildTarget {
        name: name.clone(),
        target_type: TargetType::Executable,
        sources,
    };

    interp
        .os
        .print(&format!("Created executable library: {:?}", bin));

    Ok(bin.into_object())
}

pub fn custom_target(
    _args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    _interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    // Placeholder implementation
    // TODO: Implement custom_target
    Ok(Value::None)
}
