use alloc::string::{String, ToString as _};
use alloc::vec::Vec;
use alloc::{format, vec};

use hashbrown::HashMap;

use crate::interpreter::builtins::builtin_impl;
use crate::interpreter::builtins::files::{File, files_impl};
use crate::interpreter::builtins::include_directories::IncludeDirectories;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};
use crate::os::Path;

#[derive(Debug, Clone, PartialEq, Copy)]
enum TargetType {
    StaticLibrary,
    Executable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildTarget {
    name: String,
    target_type: TargetType,
    sources: Vec<Path>,
    install: bool,
    include_dirs: Vec<Path>,
    install_dir: Path,
    flags: Vec<String>,
}

impl BuildTarget {
    fn extract_objects(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let sources = self.sources.clone();
        let extracted = ExtractedObjects { sources };
        Ok(extracted.into_object())
    }

    fn extract_all_objects(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let sources = self.sources.clone();
        let extracted = ExtractedObjects { sources };
        Ok(extracted.into_object())
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

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedObjects {
    sources: Vec<Path>,
}

impl MesonObject for ExtractedObjects {
    builtin_impl!();
}

impl MesonObject for BuildTarget {
    builtin_impl!(extract_objects, extract_all_objects, full_path);
}

pub fn static_library(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    add_target_impl(TargetType::StaticLibrary, args, kwargs, interp)
}

pub fn executable(
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    add_target_impl(TargetType::Executable, args, kwargs, interp)
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

fn add_target_impl(
    target_type: TargetType,
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(name)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "First argument must be a string (name)".into(),
        ));
    };

    let sources = files_impl(&args[1..], interp)?
        .into_iter()
        .map(|f| f.path)
        .collect::<Vec<_>>();

    let objects = kwargs
        .get("objects")
        .cloned()
        .unwrap_or(Value::Array(vec![]));

    let objects = flatten([objects].as_slice())
        .flat_map(|v| {
            if let Ok(s) = v.as_string() {
                vec![Ok(Path::from(interp.current_dir.join(s)))]
            } else if let Ok(file) = v.as_object::<File>() {
                vec![Ok(file.path.clone())]
            } else if let Ok(objs) = v.as_object::<ExtractedObjects>() {
                objs.sources.iter().map(|f| Ok(f.clone())).collect::<Vec<_>>()
            } else {
                vec![Err(InterpreterError::TypeError(
                    "Expected elements of 'objects' to be strings, File, or ExtractedObjects object".into(),
                ))]
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .context_type("Expected 'objects' keyword argument to be an array")?;

    let install = kwargs
        .get("install")
        .map(Value::as_boolean)
        .transpose()
        .context_type("Expected 'install' keyword argument to be a boolean")?
        .unwrap_or(false);

    let install_dir = kwargs
        .get("install_dir")
        .map(Value::as_string)
        .transpose()
        .context_type("Expected 'install_dir' keyword argument to be a string")?;

    let install_dir = match install_dir {
        Some(dir) => Some(dir.into()),
        None => match target_type {
            TargetType::StaticLibrary => get_dir(interp, "libdir")?,
            TargetType::Executable => get_dir(interp, "bindir")?,
        },
    }
    .map(Path::from)
    .context_runtime("Could not determine install directory")?;

    let include_dirs = kwargs
        .get("include_directories")
        .cloned()
        .unwrap_or(Value::Array(vec![]));

    let include_dirs = flatten([include_dirs].as_slice())
        .flat_map(|v| {
            if let Ok(s) = v.as_string() {
                vec![Ok(Path::from(interp.current_dir.join(s)))]
            } else if let Ok(inc) = v.as_object::<IncludeDirectories>() {
                inc.dirs.iter().map(|f| Ok(f.clone())).collect::<Vec<_>>()
            } else {
                vec![Err(InterpreterError::TypeError(
                    "Expected elements of 'include_directories' to be strings or include_directories objects".into(),
                ))]
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let flags = kwargs
        .get("c_args")
        .map(Value::as_array)
        .transpose()
        .context_type("Expected 'c_args' keyword argument to be an array")?
        .unwrap_or_default()
        .iter()
        .map(|v| v.as_string().map(String::from))
        .collect::<Result<Vec<_>, _>>()
        .context_type("Expected elements of 'c_args' to be strings")?;

    let mut sources = sources;
    sources.extend(objects);

    let lib = BuildTarget {
        name: name.clone(),
        target_type,
        sources,
        install,
        include_dirs,
        install_dir,
        flags,
    };

    interp
        .os
        .print(&format!("Created {target_type:?}: {:?}", lib));

    Ok(lib.into_object())
}

fn get_dir(interp: &Interpreter, key: &str) -> Result<Option<String>, InterpreterError> {
    interp
        .get_option(key)
        .map(|v| v.as_string().map(String::from))
        .transpose()
        .with_context_type(|| format!("Expected '{key}' option to be a string"))
}
