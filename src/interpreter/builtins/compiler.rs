use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;
use core::iter;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::builtins::utils::flatten;
use crate::interpreter::error::ErrorContext as _;
use crate::interpreter::{
    Interpreter, InterpreterError, MesonObject, Value, bail_runtime_error, bail_type_error,
};
use crate::path::Path;
use crate::runtime::{CompilerInfo, TryCompileOutput};

#[derive(Debug, Clone, PartialEq)]
pub struct Compiler {
    lang: String,
    command: Path,
    flags: Vec<String>,
}

const DELIMITER: &str = r#""MESON_DELIMITER""#;

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
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let code = include_str!("compiler/compiler_id.c");
        let result = self.try_compile(&["-E"], &[], code, interp)?;
        let output = String::from_utf8_lossy(&result.artifact);
        let suffix = output.rsplit_once(DELIMITER).map(|(_, s)| s.trim());
        match suffix {
            None | Some("") => Err(InterpreterError::RuntimeError(
                "Failed to detect compiler family".into(),
            )),
            Some(family) => Ok(Value::String(family.into())),
        }
    }

    fn get_linker_id(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        // TODO: actually detect linker
        Ok(Value::String("ld.lld".into()))
    }

    fn cmd_array(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let argv = iter::once(self.command.to_string())
            .chain(self.flags.iter().cloned())
            .map(Value::String)
            .collect();
        Ok(Value::Array(argv))
    }

    fn has_argument(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(argument)) = args.first() else {
            bail_type_error!("has_argument requires a string argument");
        };
        let required = match kwargs.get("required") {
            Some(Value::Boolean(val)) => *val,
            None => false,
            _ => {
                bail_type_error!("The 'required' keyword argument must be a boolean");
            }
        };

        let result = self.try_compile(&["-c"], &[argument], "", interp)?;
        let supported = result.success;

        if supported || !required {
            Ok(Value::Boolean(supported))
        } else {
            bail_runtime_error!("Compiler does not support argument: {argument}");
        }
    }

    fn get_supported_arguments(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let args = flatten(&args)
            .map(|v| {
                v.as_string()
                    .context_type("Expected arguments to be strings")
            })
            .collect::<Result<Vec<_>, _>>()?;

        let args = args
            .into_iter()
            .filter_map(|arg| match self.try_compile(&["-c"], &[arg], "", interp) {
                Ok(TryCompileOutput { success, .. }) => success.then_some(Ok(arg)),
                Err(e) => Some(Err(e)),
            })
            .map(|arg| arg.map(|v| Value::String(v.to_string())))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Value::Array(args))
    }

    fn has_function(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(function)) = args.first() else {
            bail_type_error!("has_function requires a string argument");
        };

        let extra_args = get_extra_args(&kwargs)?;

        let code = format!("int main() {{ void *p = (void*)({function}); return 0; }}");

        let supported = self.try_compile(&[], &extra_args, &code, interp)?.success;

        Ok(Value::Boolean(supported))
    }

    fn has_link_argument(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(argument)) = args.first() else {
            bail_type_error!("has_link_argument requires a string argument");
        };

        let code = "int main() { return 0; }";

        let supported = self.try_compile(&[], &[argument], code, interp)?.success;

        Ok(Value::Boolean(supported))
    }

    fn has_multi_link_arguments(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let args = flatten(&args)
            .map(|v| {
                v.as_string()
                    .context_type("Expected arguments to be strings")
            })
            .collect::<Result<Vec<_>, _>>()?;

        let code = "int main() { return 0; }";

        let supported = self.try_compile(&[], &args, code, interp)?.success;

        Ok(Value::Boolean(supported))
    }

    fn symbols_have_underscore_prefix(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let code = include_str!("compiler/underscore_prefix.c");
        let result = self.try_compile(&["-E"], &[], code, interp)?;
        let output = String::from_utf8_lossy(&result.artifact);
        let suffix = output.rsplit_once(DELIMITER).map(|(_, s)| s.trim());
        match suffix {
            Some("_") => Ok(Value::Boolean(true)),
            Some("") => Ok(Value::Boolean(false)),
            Some(sym) => Err(InterpreterError::RuntimeError(format!(
                "Found unexpected underscore prefix {sym:?}"
            ))),
            None => Err(InterpreterError::RuntimeError(format!(
                "Failed to find underscore prefix, {}",
                String::from_utf8_lossy(&result.artifact)
            ))),
        }
    }

    fn compiles(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(code)) = args.first() else {
            bail_type_error!("compiles requires a string argument");
        };

        let extra_args = get_extra_args(&kwargs)?;

        let success = self
            .try_compile(&["-c"], &extra_args, code, interp)?
            .success;

        Ok(Value::Boolean(success))
    }

    fn links(
        &self,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
        interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(code)) = args.first() else {
            bail_type_error!("links requires a string argument");
        };

        let extra_args = get_extra_args(&kwargs)?;

        let success = self.try_compile(&[], &extra_args, code, interp)?.success;

        Ok(Value::Boolean(success))
    }

    fn try_compile(
        &self,
        args: &[&str],
        extra_args: &[&str],
        code: &str,
        interp: &Interpreter,
    ) -> Result<TryCompileOutput, InterpreterError> {
        let meson = interp.meson.borrow();
        let args = args.iter().copied();
        let cmd_args = meson
            .project_args
            .get("c")
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(String::as_str);

        let cmd_args = cmd_args.chain(args).chain(extra_args.iter().copied());
        let cmd_args = cmd_args.chain(self.flags.iter().map(String::as_str));

        let outdir = interp
            .os
            .tempdir()
            .context_runtime("Failed to create temporary directory")?;

        let out_path = outdir.path().join("output");

        let input_filename = match self.lang.as_str() {
            "c" => "input.c",
            "cpp" => "input.cpp",
            _ => {
                return Err(InterpreterError::RuntimeError(format!(
                    "Unsupported language: {}",
                    self.lang
                )));
            }
        };
        let input = outdir.path().join(input_filename);
        interp
            .os
            .write_file(&input, code.as_bytes())
            .context_runtime("Failed to write temporary source file")?;

        let cmd_args = cmd_args.chain([input.as_ref(), "-o", out_path.as_ref()]);

        let result = interp
            .os
            .run_command(&self.command, &cmd_args.collect::<Vec<_>>())
            .context_runtime("Failed to run compiler")?;

        let artifact = interp.os.read_file(&out_path).unwrap_or_default();

        let result = TryCompileOutput {
            success: result.returncode == 0,
            artifact,
        };

        Ok(result)
    }
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

fn get_compiler_argv0(interp: &mut Interpreter, lang: &str) -> Result<Path, InterpreterError> {
    let compiler_info = interp
        .os
        .get_compiler(lang)
        .with_context_runtime(|| format!("Failed to get compiler for language: {lang}"))?;

    Ok(compiler_info.bin)
}

fn get_compiler_flags(
    interp: &mut Interpreter,
    lang: &str,
) -> Result<Vec<String>, InterpreterError> {
    let mut flags = Vec::new();

    if let Some(f) = interp.options.get(&format!("{lang}_args")) {
        let f = f
            .value
            .as_array()
            .context_type("Expected compiler flags option to be an array")?
            .iter()
            .map(|v| v.as_string().map(String::from))
            .collect::<Result<Vec<String>, _>>()
            .context_type("Expected compiler flags option to be an array of strings")?;

        flags.extend(f);
    }

    if let Ok(CompilerInfo { flags: f, .. }) = interp.os.get_compiler(lang) {
        flags.extend(f);
    }

    Ok(flags)
}

pub fn get_compiler(
    args: Vec<Value>,
    _kwargs: HashMap<String, Value>,
    interp: &mut Interpreter,
) -> Result<Value, InterpreterError> {
    let lang = args
        .first()
        .context_type("Expected a string as the first argument")?
        .as_string()
        .context_type("Expected a string as the first argument")?;

    let command = get_compiler_argv0(interp, lang).with_context_runtime(|| {
        format!("Failed to determine compiler command for language: {lang}")
    })?;

    let flags = get_compiler_flags(interp, lang).with_context_runtime(|| {
        format!("Failed to determine compiler flags for language: {lang}")
    })?;

    Ok(Compiler {
        command,
        flags,
        lang: lang.into(),
    }
    .into_object())
}
