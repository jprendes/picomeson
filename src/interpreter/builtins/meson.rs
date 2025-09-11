use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::compiler::get_compiler;
use crate::interpreter::builtins::version::version;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};
use crate::os::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct Meson {
    build_dir: Path,
    source_dir: Path,
    pub project_name: String,
    pub project_version: String,
    pub project_args: HashMap<String, Vec<String>>,
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
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        version("1.3.0")
    }

    fn is_subproject(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Boolean(false))
    }

    fn get_compiler(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        get_compiler(args, kwargs, interp)
    }

    fn get_cross_property(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        // Return the default value (second argument)
        Ok(args.get(1).cloned().unwrap_or(Value::None))
    }

    fn project_version(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.project_version.clone()))
    }

    fn current_build_dir(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.build_dir.to_string()))
    }

    fn current_source_dir(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.source_dir.to_string()))
    }
}

pub fn meson(source_dir: Path, build_dir: Path) -> Meson {
    Meson {
        build_dir,
        source_dir,
        project_name: "".into(),
        project_version: "0.0.0".into(),
        project_args: HashMap::new(),
        is_subproject: false,
    }
}
