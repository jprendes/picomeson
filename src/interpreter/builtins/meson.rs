use std::path::PathBuf;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::compiler::get_compiler;
use crate::interpreter::builtins::version::version;
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Meson {
    build_dir: PathBuf,
    source_dir: PathBuf,
    pub project_version: String,
    is_subproject: bool,
}

impl MesonObject for Meson {
    builtin_impl!(
        version,
        is_subproject,
        get_compiler,
        get_cross_property,
        project_version,
        current_build_dir,
        current_source_dir
    );
}

impl Meson {
    fn version(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        version("1.3.0")
    }

    fn is_subproject(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Boolean(false))
    }

    fn get_compiler(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        get_compiler(args, kwargs)
    }

    fn get_cross_property(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        // Return the default value (second argument)
        Ok(args.get(1).cloned().unwrap_or(Value::None))
    }

    fn project_version(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.project_version.clone()))
    }

    fn current_build_dir(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.build_dir.to_string_lossy().into_owned()))
    }

    fn current_source_dir(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(
            self.source_dir.to_string_lossy().into_owned(),
        ))
    }
}

pub fn meson(build_dir: impl Into<PathBuf>, source_dir: impl Into<PathBuf>) -> Meson {
    Meson {
        build_dir: build_dir.into(),
        source_dir: source_dir.into(),
        project_version: "0.0.0".into(),
        is_subproject: false,
    }
}
