use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

use tempfile::tempdir;

use crate::interpreter::{InterpreterError, MesonObject, Value, builtins::utils::flatten};

use super::builtin_impl;

#[derive(Debug, Clone, PartialEq)]
pub struct Compiler {
    command: Vec<String>,
}

impl MesonObject for Compiler {
    builtin_impl!(
        get_id,
        get_linker_id,
        cmd_array,
        has_argument,
        get_supported_arguments,
        has_function,
        has_link_argument,
        has_multi_link_arguments,
        symbols_have_underscore_prefix,
        compiles,
        links,
    );
}

impl Compiler {
    fn get_id(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        // TODO: actually detect compiler
        Ok(Value::String("cc".to_string()))
    }

    fn get_linker_id(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        // TODO: actually detect linker
        Ok(Value::String("ld.lld".to_string()))
    }

    fn cmd_array(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::Array(
            self.command.iter().cloned().map(Value::String).collect(),
        ))
    }

    fn has_argument(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(argument)) = args.first() else {
            return Err(InterpreterError::RuntimeError(
                "has_argument requires a string argument".to_string(),
            ));
        };
        let required = match kwargs.get("required") {
            Some(Value::Boolean(val)) => *val,
            None => false,
            _ => {
                return Err(InterpreterError::TypeError(
                    "The 'required' keyword argument must be a boolean".into(),
                ));
            }
        };

        let result = self.try_compile(&["-c"], &[argument], "")?;
        let supported = result.success;

        if supported || !required {
            Ok(Value::Boolean(supported))
        } else {
            Err(InterpreterError::RuntimeError(format!(
                "Compiler does not support argument: {argument}",
            )))
        }
    }

    fn get_supported_arguments(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let args = flatten(&args)
            .map(|v| match v {
                Value::String(s) => Ok(s),
                _ => Err(InterpreterError::TypeError(format!(
                    "Expected arguments to be strings, found {v:?}"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;

        let args = args
            .into_iter()
            .filter_map(|arg| match self.try_compile(&["-c"], &[&arg], "") {
                Ok(TryCompileResult { success, .. }) => success.then_some(Ok(arg)),
                Err(e) => Some(Err(e)),
            })
            .map(|arg| arg.cloned().map(Value::String))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Value::Array(args))
    }

    fn has_function(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(function)) = args.first() else {
            return Err(InterpreterError::RuntimeError(
                "has_function requires a string argument".to_string(),
            ));
        };

        let extra_args = get_extra_args(&kwargs)?;

        let code = format!("int main() {{ void *p = (void*)({function}); return 0; }}");

        let supported = self.try_compile(&[], &extra_args, &code)?.success;

        Ok(Value::Boolean(supported))
    }

    fn has_link_argument(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(argument)) = args.first() else {
            return Err(InterpreterError::RuntimeError(
                "has_link_argument requires a string argument".to_string(),
            ));
        };

        let code = "int main() { return 0; }";

        let supported = self.try_compile(&[], &[argument], code)?.success;

        Ok(Value::Boolean(supported))
    }

    fn has_multi_link_arguments(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let args = flatten(&args)
            .map(|v| match v {
                Value::String(s) => Ok(s.as_str()),
                _ => Err(InterpreterError::TypeError(format!(
                    "Expected arguments to be strings, found {v:?}"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;

        let code = "int main() { return 0; }";

        let supported = self.try_compile(&[], &args, code)?.success;

        Ok(Value::Boolean(supported))
    }

    fn symbols_have_underscore_prefix(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let delimiter = r#""MESON_HAVE_UNDERSCORE_DELIMITER" "#;
        let code = format!(
            "
#ifndef __USER_LABEL_PREFIX__
#define MESON_UNDERSCORE_PREFIX unsupported
#else
#define MESON_UNDERSCORE_PREFIX __USER_LABEL_PREFIX__
#endif
{delimiter}MESON_UNDERSCORE_PREFIX
"
        );
        let result = self.try_compile(&["-c", "-E"], &[], &code)?;
        let output = String::from_utf8_lossy(&result.artifact);
        let suffix = output.rsplit_once(delimiter).map(|(_, s)| s.trim());
        match suffix {
            Some("_") => Ok(Value::Boolean(true)),
            Some("") => Ok(Value::Boolean(false)),
            _ => self.symbols_have_underscore_prefix_searchbin(),
        }
    }

    fn symbols_have_underscore_prefix_searchbin(&self) -> Result<Value, InterpreterError> {
        let symbol_name = "meson_uscore_prefix";
        let code = format!(
            "
#ifdef __cplusplus
extern \"C\" {{
#endif
void {symbol_name}(void) {{}}
#ifdef __cplusplus
}}
#endif
"
        );
        let artifact = self.try_compile(&["-c"], &[], &code)?.artifact;
        let artifact = String::from_utf8_lossy(&artifact);
        if artifact.contains(&format!("_{symbol_name}")) {
            Ok(Value::Boolean(true))
        } else if artifact.contains(symbol_name) {
            Ok(Value::Boolean(false))
        } else {
            Err(InterpreterError::RuntimeError(
                "Failed to find symbol in compiler output".into(),
            ))
        }
    }

    fn compiles(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(code)) = args.first() else {
            return Err(InterpreterError::RuntimeError(
                "compiles requires a string argument".to_string(),
            ));
        };

        let extra_args = get_extra_args(&kwargs)?;

        let success = self.try_compile(&["-c"], &extra_args, code)?.success;

        Ok(Value::Boolean(success))
    }

    fn links(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(code)) = args.first() else {
            return Err(InterpreterError::RuntimeError(
                "links requires a string argument".to_string(),
            ));
        };

        let extra_args = get_extra_args(&kwargs)?;

        let success = self.try_compile(&[], &extra_args, code)?.success;

        Ok(Value::Boolean(success))
    }

    fn try_compile(
        &self,
        args: &[&str],
        extra_args: &[&str],
        code: &str,
    ) -> Result<TryCompileResult, InterpreterError> {
        use std::io::Write;

        let tmp_dir = tempdir().map_err(|e| {
            InterpreterError::RuntimeError(format!("Failed to create temporary directory: {}", e))
        })?;

        let Some(arg0) = self.command.first() else {
            return Err(InterpreterError::RuntimeError(
                "Compiler command is empty".into(),
            ));
        };

        let mut cmd = Command::new(arg0);

        let out_path = tmp_dir.path().join("output");
        cmd.args(&self.command[1..])
            .args(args)
            .args(["-xc", "-", "-o"])
            .arg(&out_path)
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            InterpreterError::RuntimeError(format!("Failed to run compiler: {}", e))
        })?;

        child
            .stdin
            .take()
            .unwrap()
            .write_all(code.as_bytes())
            .map_err(|e| {
                InterpreterError::RuntimeError(format!("Failed to run compiler: {}", e))
            })?;

        let output = child.wait_with_output().map_err(|e| {
            InterpreterError::RuntimeError(format!("Failed to run compiler: {}", e))
        })?;

        let artifact = std::fs::read(&out_path).unwrap_or_default();
        let success = output.status.success();

        Ok(TryCompileResult { success, artifact })
    }
}

#[derive(Debug)]
struct TryCompileResult {
    success: bool,
    artifact: Vec<u8>,
}

fn get_extra_args(kwargs: &HashMap<String, Value>) -> Result<Vec<&str>, InterpreterError> {
    match kwargs.get("args") {
        Some(Value::Array(arr)) => flatten(arr)
            .map(|v| match v {
                Value::String(s) => Ok(s.as_str()),
                _ => Err(InterpreterError::TypeError(
                    "The 'args' keyword argument must be an array of strings".into(),
                )),
            })
            .collect(),
        None => Ok(Vec::new()),
        _ => Err(InterpreterError::TypeError(
            "The 'args' keyword argument must be an array of strings".into(),
        )),
    }
}

pub fn get_compiler(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
) -> Result<Value, InterpreterError> {
    let Some(Value::String(lang)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "Expected a string as the first argument".into(),
        ));
    };

    match lang.as_str() {
        "c" => {
            let command = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
            return Ok(Compiler {
                command: vec![command],
            }
            .into_object());
        }
        "cpp" => {
            let command = std::env::var("CXX").unwrap_or_else(|_| "c++".to_string());
            return Ok(Compiler {
                command: vec![command],
            }
            .into_object());
        }
        lang => {
            return Err(InterpreterError::RuntimeError(format!(
                "Unsupported language '{lang}'"
            )));
        }
    }
}
