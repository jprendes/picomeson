use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use as_any::Downcast;

use crate::parser::{BinaryOperator, Statement, UnaryOperator, Value as AstValue};

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    None,
    Object(Arc<dyn MesonObject>),
}

impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Dict(dict) => {
                let items: Vec<String> = dict
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::None => "none".to_string(),
            Value::Object(obj) => obj.to_string(),
        }
    }

    fn to_bool(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Integer(i) => *i != 0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Dict(dict) => !dict.is_empty(),
            Value::None => false,
            Value::Object(_) => true,
        }
    }

    fn format_string(&self, args: &[Value]) -> String {
        let mut result = self.to_string();
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("@{}@", i);
            result = result.replace(&placeholder, &arg.to_string());
        }
        result
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Dict(a), Value::Dict(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::Object(a), Value::Object(b)) => Arc::ptr_eq(a, b),
            (Value::String(a), b) => a == &b.to_string(),
            (a, Value::String(b)) => &a.to_string() == b,
            _ => false,
        }
    }
}

impl Value {
    fn cloned(&self) -> Self {
        match self {
            Value::String(s) => Value::String(s.clone()),
            Value::Integer(i) => Value::Integer(*i),
            Value::Boolean(b) => Value::Boolean(*b),
            Value::Array(arr) => Value::Array(arr.iter().map(|v| v.cloned()).collect()),
            Value::Dict(dict) => {
                Value::Dict(dict.iter().map(|(k, v)| (k.clone(), v.cloned())).collect())
            }
            Value::None => Value::None,
            Value::Object(obj) => Value::Object(obj.clone_arc()),
        }
    }
}

pub trait MesonObject: std::fmt::Debug + as_any::AsAny {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>;
    fn to_string(&self) -> String;
    fn clone_arc(&self) -> Arc<dyn MesonObject>;
}

impl PartialEq for Box<dyn MesonObject> {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

#[derive(Debug, Clone)]
struct Version {
    version: String,
}

impl MesonObject for Version {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "version_compare" => {
                let version = self.version.clone();
                Some(Box::new(move |args, _| {
                    // Simple version comparison stub
                    if let Some(Value::String(comp)) = args.first() {
                        let req = semver::VersionReq::parse(&comp).map_err(|e| {
                            InterpreterError::RuntimeError(format!(
                                "Invalid version requirement string '{}': {}",
                                version, e
                            ))
                        })?;
                        let version = semver::Version::parse(&version).map_err(|e| {
                            InterpreterError::RuntimeError(format!(
                                "Invalid version string '{}': {}",
                                version, e
                            ))
                        })?;
                        Ok(Value::Boolean(req.matches(&version)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }))
            }
            "to_string" => {
                let version = self.version.clone();
                Some(Box::new(move |_, _| Ok(Value::String(version.clone()))))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("Version({})", self.version)
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Default)]
struct ConfigData {
    data: Arc<Mutex<HashMap<String, (Value, String)>>>,
}

impl Clone for ConfigData {
    fn clone(&self) -> Self {
        Self {
            data: Arc::new(Mutex::new(self.data.lock().unwrap().clone())),
        }
    }
}

impl MesonObject for ConfigData {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "get" => {
                let data = self.data.clone();
                Some(Box::new(move |args, _| {
                    if let Some(Value::String(key)) = args.first() {
                        let data = data.lock().unwrap();
                        Ok(data
                            .get(key)
                            .map(|(v, _)| v)
                            .unwrap_or(&Value::None)
                            .cloned())
                    } else {
                        Ok(Value::None)
                    }
                }))
            }
            "set" => {
                let data = self.data.clone();
                Some(Box::new(move |args, kwargs| {
                    if args.len() == 2 {
                        if let Some(Value::String(key)) = args.get(0) {
                            let value = args.get(1).unwrap_or(&Value::None).cloned();
                            let desc = match kwargs.get("description") {
                                Some(Value::String(s)) => s.clone(),
                                None => String::new(),
                                _ => {
                                    return Err(InterpreterError::TypeError(
                                        "description must be a string".to_string(),
                                    ));
                                }
                            };
                            let mut data = data.lock().unwrap();
                            data.insert(key.clone(), (value, desc));
                            return Ok(Value::None);
                        }
                    }
                    Err(InterpreterError::TypeError(
                        "set requires a string key and a value".to_string(),
                    ))
                }))
            }
            "set10" => {
                let data = self.data.clone();
                Some(Box::new(move |args, kwargs| {
                    if args.len() == 2 {
                        if let Some(Value::String(key)) = args.get(0) {
                            let value = args.get(1).unwrap_or(&Value::None).cloned();
                            let value = if value.to_bool() {
                                Value::Integer(1)
                            } else {
                                Value::Integer(0)
                            };
                            let desc = match kwargs.get("description") {
                                Some(Value::String(s)) => s.clone(),
                                None => String::new(),
                                _ => {
                                    return Err(InterpreterError::TypeError(
                                        "description must be a string".to_string(),
                                    ));
                                }
                            };
                            let mut data = data.lock().unwrap();
                            data.insert(key.clone(), (value, desc));
                            return Ok(Value::None);
                        }
                    }
                    Err(InterpreterError::TypeError(
                        "set requires a string key and a value".to_string(),
                    ))
                }))
            }
            "merge_from" => {
                let data = self.data.clone();
                Some(Box::new(move |args, _| {
                    let Some(configuration) = args.get(0).and_then(|v| match v {
                        Value::Object(o) => {
                            if let Some(data) = o.as_ref().downcast_ref::<ConfigData>() {
                                Some(data.clone())
                            } else {
                                None
                            }
                        }
                        Value::Dict(dict) => Some(ConfigData::from_dict(dict.clone())),
                        _ => None,
                    }) else {
                        return Err(InterpreterError::TypeError(
                            "merge_from requires a ConfigData object or a dict".to_string(),
                        ));
                    };
                    let other_data = configuration.data.lock().unwrap();
                    let mut data = data.lock().unwrap();
                    for (k, v) in other_data.iter() {
                        data.insert(k.clone(), v.clone());
                    }
                    Ok(Value::None)
                }))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("ConfigData(num_datas={})", self.data.lock().unwrap().len())
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

impl ConfigData {
    fn from_dict(dict: HashMap<String, Value>) -> Self {
        let dict = dict
            .into_iter()
            .map(|(k, v)| (k, (v, String::new())))
            .collect();
        Self {
            data: Arc::new(Mutex::new(dict)),
        }
    }

    fn configure_file(
        &self,
        input: Option<String>,
        output: String,
    ) -> Result<Value, InterpreterError> {
        let content = if let Some(_input) = input {
            // TODO: implement this
            "".to_string()
        } else {
            let data = self.data.lock().unwrap();
            let mut content = String::from("#pragma once\n\n");
            for (key, (value, desc)) in data.iter() {
                if desc != "" {
                    content.push_str(&format!("// {}\n", desc));
                }
                match value {
                    Value::Boolean(true) => {
                        content.push_str(&format!("#define {}\n", key));
                    }
                    Value::Boolean(false) => {
                        content.push_str(&format!("#undef {}\n", key));
                    }
                    Value::Integer(i) => {
                        content.push_str(&format!("#define {} {}\n", key, i));
                    }
                    Value::String(s) => {
                        content.push_str(&format!("#define {} {}\n", key, s));
                    }
                    v => {
                        Err(InterpreterError::RuntimeError(format!(
                            "Unsupported value type for key {}: {:?}",
                            key, v
                        )))?;
                    }
                }
                content.push('\n');
            }
            content
        };

        // TODO: Handle paths properly
        println!("Should be writing output file: {}", output);
        println!("With content:\n{}", content);
        //std::fs::write(output, content).map_err(|e| {
        //    InterpreterError::RuntimeError(format!("Failed to write output file: {}", e))
        //})?;

        Ok(Value::None)
    }
}

#[derive(Debug, Clone)]
struct IncludeDirectories {
    dirs: Vec<PathBuf>,
}

impl MesonObject for IncludeDirectories {
    fn get_method(
        &self,
        _name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        None
    }

    fn to_string(&self) -> String {
        let items: Vec<String> = self
            .dirs
            .iter()
            .map(|d| d.to_string_lossy().to_string())
            .collect();
        format!("IncludeDirectories([{}])", items.join(", "))
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct File {
    path: PathBuf,
}

impl MesonObject for File {
    fn get_method(
        &self,
        _name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        None
    }

    fn to_string(&self) -> String {
        format!("File({})", self.path.to_string_lossy())
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct ExternalProgram {
    full_path: Option<String>,
}

impl MesonObject for ExternalProgram {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "found" => {
                let found = self.full_path.is_some();
                Some(Box::new(move |_, _| Ok(Value::Boolean(found))))
            }
            "full_path" => {
                let path = self.full_path.clone();
                Some(Box::new(move |_, _| {
                    if let Some(p) = path.clone() {
                        Ok(Value::String(p))
                    } else {
                        Ok(Value::None)
                    }
                }))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("ExternalProgram(found={})", self.full_path.is_some())
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct BuildTarget {
    name: String,
    target_type: String,
    sources: Vec<PathBuf>,
}

impl MesonObject for BuildTarget {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "extract_objects" => Some(Box::new(move |_, _| Ok(Value::None))),
            "extract_all_objects" => Some(Box::new(move |_, _| Ok(Value::Array(vec![])))),
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!(
            "BuildTarget(name={}, type={}, num_sources={})",
            self.name,
            self.target_type,
            self.sources.len()
        )
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct RunResult {
    stdout: String,
    stderr: String,
    returncode: i64,
}

impl MesonObject for RunResult {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "stdout" => {
                let stdout = self.stdout.clone();
                Some(Box::new(move |_, _| Ok(Value::String(stdout.clone()))))
            }
            "stderr" => {
                let stderr = self.stderr.clone();
                Some(Box::new(move |_, _| Ok(Value::String(stderr.clone()))))
            }
            "returncode" => {
                let code = self.returncode;
                Some(Box::new(move |_, _| Ok(Value::Integer(code))))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("RunResult(returncode={})", self.returncode)
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct Compiler {
    id: String,
    command: Vec<String>,
}

impl MesonObject for Compiler {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "cmd_array" => {
                let cmd = self.command.clone();
                Some(Box::new(move |_, _| {
                    Ok(Value::Array(
                        cmd.iter().map(|s| Value::String(s.clone())).collect(),
                    ))
                }))
            }
            "get_supported_arguments" => {
                Some(Box::new(move |args, _| {
                    // For now, just return the input arguments as supported
                    if let Some(Value::Array(flags)) = args.first() {
                        Ok(Value::Array(flags.clone()))
                    } else {
                        Ok(Value::Array(vec![]))
                    }
                }))
            }
            "has_function" => {
                Some(Box::new(move |_args, _| {
                    // Stub: check if function exists
                    Ok(Value::Boolean(false))
                }))
            }
            "has_link_argument" => {
                Some(Box::new(move |_args, _| {
                    // Stub: check if linker argument is supported
                    Ok(Value::Boolean(true))
                }))
            }
            "has_multi_link_arguments" => {
                Some(Box::new(move |_args, _| {
                    // Stub: check if multiple linker arguments are supported
                    Ok(Value::Boolean(true))
                }))
            }
            "symbols_have_underscore_prefix" => {
                Some(Box::new(move |_, _| {
                    // Stub: check symbol prefix
                    Ok(Value::Boolean(false))
                }))
            }
            "get_id" => {
                let id = self.id.clone();
                Some(Box::new(move |_, _| Ok(Value::String(id.clone()))))
            }
            "get_linker_id" => Some(Box::new(move |_, _| Ok(Value::String("ld.lld".into())))),
            "compiles" => {
                let cmd = self.command.clone();
                Some(Box::new(move |args, kwargs| {
                    try_compile(&cmd, &["-c"], args, kwargs).map(Value::Boolean)
                }))
            }
            "links" => {
                let cmd = self.command.clone();
                Some(Box::new(move |args, kwargs| {
                    try_compile(&cmd, &[], args, kwargs).map(Value::Boolean)
                }))
            }
            "has_argument" => {
                let cmd = self.command.clone();
                Some(Box::new(move |args, kwargs| {
                    let code = Value::String(String::new());
                    let test_arg = match args.first() {
                        Some(Value::String(s)) => s,
                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "has_argument requires a string argument".to_string(),
                            ));
                        }
                    };
                    let supported =
                        try_compile(&cmd, &[test_arg, "-c"], &[code], &Default::default())?;
                    if supported {
                        Ok(Value::Boolean(true))
                    } else if let Some(Value::Boolean(true)) = kwargs.get("required") {
                        Err(InterpreterError::RuntimeError(format!(
                            "Compiler does not support argument: {}",
                            test_arg
                        )))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("Compiler({})", self.id)
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

fn try_compile(
    base_cmd: &Vec<String>,
    extra_cmd: &[&str],
    args: &[Value],
    kwargs: &HashMap<String, Value>,
) -> Result<bool, InterpreterError> {
    use std::io::Write;

    let Some(Value::String(code)) = args.first() else {
        return Err(InterpreterError::TypeError(
            "First argument to compiles must be a string".to_string(),
        ));
    };

    let extra_args = match kwargs.get("args") {
        Some(Value::Array(arr)) => arr.as_slice(),
        _ => &[],
    };
    let extra_args = extra_args.iter().filter_map(|v| {
        if let Value::String(s) = v {
            Some(s.as_str())
        } else {
            None
        }
    });

    let mut cmd = Command::new(base_cmd.first().unwrap_or(&"cc".to_string()));

    cmd.args(&base_cmd[1..])
        .args(extra_cmd)
        .args(["-xc", "-o/dev/null", "-"])
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| InterpreterError::RuntimeError(format!("Failed to run compiler: {}", e)))?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(code.as_bytes())
        .map_err(|e| InterpreterError::RuntimeError(format!("Failed to run compiler: {}", e)))?;

    let output = child
        .wait_with_output()
        .map_err(|e| InterpreterError::RuntimeError(format!("Failed to run compiler: {}", e)))?;

    // let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    // let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let success = output.status.success();

    Ok(success)
}

#[derive(Debug, Clone)]
struct Machine {
    system: String,
    cpu_family: String,
    cpu: String,
    endian: String,
}

impl MesonObject for Machine {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "system" => {
                let system = self.system.clone();
                Some(Box::new(move |_, _| Ok(Value::String(system.clone()))))
            }
            "cpu_family" => {
                let cpu_family = self.cpu_family.clone();
                Some(Box::new(move |_, _| Ok(Value::String(cpu_family.clone()))))
            }
            "cpu" => {
                let cpu = self.cpu.clone();
                Some(Box::new(move |_, _| Ok(Value::String(cpu.clone()))))
            }
            "endian" => {
                let endian = self.endian.clone();
                Some(Box::new(move |_, _| Ok(Value::String(endian.clone()))))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!(
            "Host(system={}, cpu_family={})",
            self.system, self.cpu_family
        )
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct Env {
    vars: Arc<Mutex<HashMap<String, String>>>,
}

impl MesonObject for Env {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "prepend" => {
                let vars = self.vars.clone();
                Some(Box::new(move |args, _| {
                    let Some(Value::String(key)) = args.get(0) else {
                        return Err(InterpreterError::TypeError(
                            "First argument to env.prepend must be a string".to_string(),
                        ));
                    };
                    let Some(Value::String(value)) = args.get(1) else {
                        return Err(InterpreterError::TypeError(
                            "Second argument to env.prepend must be a string".to_string(),
                        ));
                    };
                    let mut vars = vars.lock().unwrap();
                    vars.insert(key.clone(), value.clone());
                    Ok(Value::None)
                }))
            }
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("Env(num_vars={})", self.vars.lock().unwrap().len())
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct FileSystem;

impl MesonObject for FileSystem {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "is_file" => Some(Box::new(move |args, _| {
                if let Some(Value::String(path)) = args.first() {
                    Ok(Value::Boolean(Path::new(path).is_file()))
                } else {
                    Ok(Value::Boolean(false))
                }
            })),
            "replace_suffix" => Some(Box::new(move |args, _| {
                if let Some(Value::String(path)) = args.get(0) {
                    if let Some(Value::String(suffix)) = args.get(1) {
                        let mut p = PathBuf::from(path);
                        p.set_extension(suffix);
                        Ok(Value::String(p.to_string_lossy().to_string()))
                    } else {
                        Err(InterpreterError::TypeError(
                            "Second argument to replace_suffix must be a string".to_string(),
                        ))
                    }
                } else {
                    Err(InterpreterError::TypeError(
                        "First argument to replace_suffix must be a string".to_string(),
                    ))
                }
            })),
            "is_dir" => Some(Box::new(move |args, _| {
                if let Some(Value::String(path)) = args.first() {
                    Ok(Value::Boolean(Path::new(path).is_dir()))
                } else {
                    Ok(Value::Boolean(false))
                }
            })),
            "exists" => Some(Box::new(move |args, _| {
                if let Some(Value::String(path)) = args.first() {
                    Ok(Value::Boolean(Path::new(path).exists()))
                } else {
                    Ok(Value::Boolean(false))
                }
            })),
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        "FileSystem".to_string()
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct Meson {
    version: String,
    is_subproject: bool,
}

impl MesonObject for Meson {
    fn get_method(
        &self,
        name: &str,
    ) -> Option<Box<dyn Fn(&[Value], &HashMap<String, Value>) -> Result<Value, InterpreterError>>>
    {
        match name {
            "version" => {
                let version = self.version.clone();
                Some(Box::new(move |_, _| {
                    Ok(Value::Object(Arc::new(Version {
                        version: version.clone(),
                    })))
                }))
            }
            "is_subproject" => {
                let is_sub = self.is_subproject;
                Some(Box::new(move |_, _| Ok(Value::Boolean(is_sub))))
            }
            "get_compiler" => Some(Box::new(move |args, _| {
                let lang = args
                    .first()
                    .and_then(|v| {
                        if let Value::String(s) = v {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("c");

                let compiler = Compiler {
                    id: lang.to_string(),
                    command: vec![format!("{}", if lang == "c" { "cc" } else { "c++" })],
                };
                Ok(Value::Object(Arc::new(compiler)))
            })),
            "get_cross_property" => {
                Some(Box::new(move |args, _| {
                    // Return the default value (second argument)
                    if args.len() >= 2 {
                        Ok(args[1].clone())
                    } else {
                        Ok(Value::None)
                    }
                }))
            }
            "project_version" => {
                let version = self.version.clone();
                Some(Box::new(move |_, _| Ok(Value::String(version.clone()))))
            }
            "current_source_dir" | "current_build_dir" => Some(Box::new(move |_, _| {
                // TODO: Return actual source / build dir
                Ok(Value::String(
                    env::current_dir().unwrap().to_string_lossy().to_string(),
                ))
            })),
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        format!("Meson(version={})", self.version)
    }

    fn clone_arc(&self) -> Arc<dyn MesonObject> {
        Arc::new(self.clone())
    }
}

fn flatten_files(args: &[Value]) -> Result<Vec<PathBuf>, InterpreterError> {
    let mut files = Vec::new();
    for arg in args {
        match arg {
            Value::String(s) => files.push(PathBuf::from(s)),
            Value::Object(o) => {
                if let Some(f) = o.as_ref().downcast_ref::<File>() {
                    files.push(f.path.clone());
                } else {
                    return Err(InterpreterError::TypeError(format!(
                        "Expected File object, got {}",
                        o.to_string()
                    )));
                }
            }
            Value::Array(arr) => {
                let nested_files = flatten_files(arr)?;
                files.extend(nested_files);
            }
            _ => {
                return Err(InterpreterError::TypeError(format!(
                    "Expected string or File object, got {:?}",
                    arg
                )));
            }
        }
    }
    Ok(files)
}

pub struct Interpreter {
    variables: HashMap<String, Value>,
    options: HashMap<String, Value>,
    break_flag: bool,
    continue_flag: bool,
}

#[derive(Debug)]
pub enum InterpreterError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    TypeError(String),
    RuntimeError(String),
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InterpreterError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            InterpreterError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            InterpreterError::TypeError(msg) => write!(f, "Type error: {}", msg),
            InterpreterError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for InterpreterError {}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            variables: HashMap::new(),
            options: HashMap::new(),
            break_flag: false,
            continue_flag: false,
        };

        // Initialize built-in variables
        interpreter.init_builtins();
        interpreter
    }

    fn init_builtins(&mut self) {
        // Meson object
        self.variables.insert(
            "meson".to_string(),
            Value::Object(Arc::new(Meson {
                version: "1.3.0".to_string(),
                is_subproject: false,
            })),
        );

        // Host machine
        self.variables.insert(
            "host_machine".to_string(),
            Value::Object(Arc::new(Machine {
                system: env::consts::OS.to_string(),
                cpu_family: env::consts::ARCH.to_string(),
                cpu: env::consts::ARCH.to_string(),
                endian: if cfg!(target_endian = "big") {
                    "big"
                } else {
                    "little"
                }
                .to_string(),
            })),
        );

        // Target machine
        self.variables.insert(
            "target_machine".to_string(),
            Value::Object(Arc::new(Machine {
                system: env::var("CARGO_CFG_TARGET_OS").unwrap_or_default(),
                cpu_family: env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default(),
                cpu: env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default(),
                endian: if env::var("CARGO_CFG_TARGET_ENDIAN").unwrap_or_default() == "big" {
                    "big"
                } else {
                    "little"
                }
                .to_string(),
            })),
        );

        // Build machine (same as host for now)
        self.variables.insert(
            "build_machine".to_string(),
            Value::Object(Arc::new(Machine {
                system: env::consts::OS.to_string(),
                cpu_family: env::consts::ARCH.to_string(),
                cpu: env::consts::ARCH.to_string(),
                endian: if cfg!(target_endian = "big") {
                    "big"
                } else {
                    "little"
                }
                .to_string(),
            })),
        );

        // File system object
        self.variables
            .insert("fs".to_string(), Value::Object(Arc::new(FileSystem)));
    }

    pub fn interpret(&mut self, statements: Vec<Statement>) -> Result<(), InterpreterError> {
        for statement in statements {
            self.execute_statement(statement)?;

            if self.break_flag || self.continue_flag {
                break;
            }
        }
        Ok(())
    }

    fn execute_statement(&mut self, statement: Statement) -> Result<(), InterpreterError> {
        match statement {
            Statement::Assignment(name, value) => {
                let evaluated = self.evaluate_value(value)?;
                self.variables.insert(name, evaluated);
            }
            Statement::AddAssignment(name, value) => {
                let new_value = self.evaluate_value(value)?;
                if let Some(existing) = self.variables.get(&name) {
                    let combined = self.add_values(existing, &new_value)?;
                    self.variables.insert(name, combined);
                } else {
                    self.variables.insert(name, new_value);
                }
            }
            Statement::Expression(value) => {
                self.evaluate_value(value)?;
            }
            Statement::If(condition, then_branch, elif_branches, else_branch) => {
                let cond_value = self.evaluate_value(condition)?;
                if cond_value.to_bool() {
                    self.execute_block(then_branch)?;
                } else {
                    let mut executed = false;
                    for (elif_cond, elif_body) in elif_branches {
                        let elif_value = self.evaluate_value(elif_cond)?;
                        if elif_value.to_bool() {
                            self.execute_block(elif_body)?;
                            executed = true;
                            break;
                        }
                    }
                    if !executed {
                        if let Some(else_body) = else_branch {
                            self.execute_block(else_body)?;
                        }
                    }
                }
            }
            Statement::Foreach(var, iterable, body) => {
                let iter_value = self.evaluate_value(iterable)?;
                match iter_value {
                    Value::Array(items) => {
                        for item in items {
                            self.variables.insert(var.clone(), item);
                            self.execute_block(body.clone())?;

                            if self.break_flag {
                                self.break_flag = false;
                                break;
                            }
                            if self.continue_flag {
                                self.continue_flag = false;
                                continue;
                            }
                        }
                    }
                    Value::String(s) => {
                        for ch in s.chars() {
                            self.variables
                                .insert(var.clone(), Value::String(ch.to_string()));
                            self.execute_block(body.clone())?;

                            if self.break_flag {
                                self.break_flag = false;
                                break;
                            }
                            if self.continue_flag {
                                self.continue_flag = false;
                                continue;
                            }
                        }
                    }
                    _ => {
                        return Err(InterpreterError::TypeError(
                            "Cannot iterate over non-iterable".to_string(),
                        ));
                    }
                }
            }
            Statement::Break => {
                self.break_flag = true;
            }
            Statement::Continue => {
                self.continue_flag = true;
            }
        }
        Ok(())
    }

    fn execute_block(&mut self, statements: Vec<Statement>) -> Result<(), InterpreterError> {
        for statement in statements {
            self.execute_statement(statement)?;
            if self.break_flag || self.continue_flag {
                break;
            }
        }
        Ok(())
    }

    fn evaluate_value(&mut self, value: AstValue) -> Result<Value, InterpreterError> {
        match value {
            AstValue::String(s) => Ok(Value::String(s)),
            AstValue::FormatString(s) => Ok(Value::String(s)),
            AstValue::Integer(i) => Ok(Value::Integer(i)),
            AstValue::Boolean(b) => Ok(Value::Boolean(b)),
            AstValue::Array(items) => {
                let mut evaluated = Vec::new();
                for item in items {
                    evaluated.push(self.evaluate_value(item)?);
                }
                Ok(Value::Array(evaluated))
            }
            AstValue::Dict(dict) => {
                let mut evaluated = HashMap::new();
                for (k, v) in dict {
                    evaluated.insert(k, self.evaluate_value(v)?);
                }
                Ok(Value::Dict(evaluated))
            }
            AstValue::Identifier(name) => self
                .variables
                .get(&name)
                .cloned()
                .ok_or_else(|| InterpreterError::UndefinedVariable(name)),
            AstValue::FunctionCall(name, args, kwargs) => self.call_function(&name, args, kwargs),
            AstValue::MethodCall(object, method, args, kwargs) => {
                let obj = self.evaluate_value(*object)?;
                self.call_method(obj, &method, args, kwargs)
            }
            AstValue::BinaryOp(left, op, right) => {
                let left_val = self.evaluate_value(*left)?;
                let right_val = self.evaluate_value(*right)?;
                self.apply_binary_op(left_val, op, right_val)
            }
            AstValue::UnaryOp(op, expr) => {
                let val = self.evaluate_value(*expr)?;
                self.apply_unary_op(op, val)
            }
            AstValue::Subscript(object, index) => {
                let obj = self.evaluate_value(*object)?;
                let idx = self.evaluate_value(*index)?;
                self.subscript(obj, idx)
            }
            AstValue::TernaryOp(condition, true_val, false_val) => {
                let cond = self.evaluate_value(*condition)?;
                if cond.to_bool() {
                    self.evaluate_value(*true_val)
                } else {
                    self.evaluate_value(*false_val)
                }
            }
        }
    }

    fn call_function(
        &mut self,
        name: &str,
        args: Vec<AstValue>,
        kwargs: HashMap<String, AstValue>,
    ) -> Result<Value, InterpreterError> {
        // Evaluate arguments
        let mut eval_args = Vec::new();
        for arg in args {
            eval_args.push(self.evaluate_value(arg)?);
        }

        let mut eval_kwargs = HashMap::new();
        for (k, v) in kwargs {
            eval_kwargs.insert(k, self.evaluate_value(v)?);
        }

        // Built-in functions
        match name {
            "project" => {
                // Project definition
                if let Some(Value::String(proj_name)) = eval_args.first() {
                    self.variables
                        .insert("project_name".to_string(), Value::String(proj_name.clone()));
                }
                Ok(Value::None)
            }
            "option" => {
                if let Some(Value::String(opt)) = eval_args.first() {
                    // Set an option
                    let Some(Value::String(ty)) = eval_kwargs.get("type").cloned() else {
                        return Err(InterpreterError::TypeError(
                            "Option requires a 'type' keyword argument of type string".to_string(),
                        ));
                    };
                    let value = eval_kwargs.get("value");
                    let opt = opt.clone();
                    match ty.as_str() {
                        "boolean" => {
                            let bool_value = match value {
                                Some(Value::Boolean(v)) => *v,
                                None => false,
                                _ => {
                                    return Err(InterpreterError::TypeError(
                                        "Boolean option requires a boolean value".to_string(),
                                    ));
                                }
                            };
                            self.options.insert(opt, Value::Boolean(bool_value));
                        }
                        "string" | "combo" => {
                            let string_value = match value {
                                Some(Value::String(v)) => v.clone(),
                                None => "".to_string(),
                                _ => {
                                    return Err(InterpreterError::TypeError(
                                        "String option requires a string value".to_string(),
                                    ));
                                }
                            };
                            self.options.insert(opt, Value::String(string_value));
                        }
                        "integer" => {
                            let int_value = match value {
                                Some(Value::Integer(v)) => *v,
                                None => 0,
                                _ => {
                                    return Err(InterpreterError::TypeError(
                                        "Integer option requires an integer value".to_string(),
                                    ));
                                }
                            };
                            self.options.insert(opt, Value::Integer(int_value));
                        }
                        "array" => {
                            let arr_value = match value {
                                Some(Value::Array(v)) => v.clone(),
                                None => vec![],
                                _ => {
                                    return Err(InterpreterError::TypeError(
                                        "Array option requires an array value".to_string(),
                                    ));
                                }
                            };
                            self.options.insert(opt, Value::Array(arr_value));
                        }
                        _ => {
                            return Err(InterpreterError::TypeError(format!(
                                "Unsupported option type: {}",
                                ty
                            )));
                        }
                    }
                    Ok(Value::None)
                } else {
                    return Err(InterpreterError::TypeError(
                        "First argument to option must be a string".to_string(),
                    ));
                }
            }
            "get_option" => {
                if let Some(Value::String(opt)) = eval_args.first() {
                    // Return a default value for options
                    Ok(match opt.as_str() {
                        "buildtype" => Value::String("debug".to_string()),
                        "prefix" => Value::String("/usr/local".to_string()),
                        "libdir" => Value::String("lib".to_string()),
                        "includedir" => Value::String("include".to_string()),
                        //_ if opt.ends_with("-tests") => Value::Boolean(false),
                        _ => self.options.get(opt).unwrap_or(&Value::None).cloned(),
                    })
                } else {
                    Ok(Value::None)
                }
            }
            "import" => {
                if let Some(Value::String(module)) = eval_args.first() {
                    match module.as_str() {
                        "fs" => Ok(Value::Object(Arc::new(FileSystem))),
                        _ => Ok(Value::None),
                    }
                } else {
                    Ok(Value::None)
                }
            }
            "run_command" => {
                let mut cmd_args = Vec::new();
                for arg in &eval_args {
                    match arg {
                        Value::String(s) => cmd_args.push(s.clone()),
                        Value::Array(arr) => {
                            for item in arr {
                                if let Value::String(s) = item {
                                    cmd_args.push(s.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if cmd_args.is_empty() {
                    return Ok(Value::Object(Arc::new(RunResult {
                        stdout: String::new(),
                        stderr: String::new(),
                        returncode: 1,
                    })));
                }

                let (stdout, stderr, status_code) = Command::new(&cmd_args[0])
                    .args(&cmd_args[1..])
                    .output()
                    .map(|output| (output.stdout, output.stderr, output.status.code()))
                    .unwrap_or_else(|e| (Vec::new(), e.to_string().into_bytes(), Some(1)));

                Ok(Value::Object(Arc::new(RunResult {
                    stdout: String::from_utf8_lossy(&stdout).to_string(),
                    stderr: String::from_utf8_lossy(&stderr).to_string(),
                    returncode: status_code.unwrap_or(1) as i64,
                })))
            }
            "set_variable" => {
                if eval_args.len() != 2 {
                    return Err(InterpreterError::RuntimeError(
                        "set_variable requires 2 arguments".to_string(),
                    ));
                }
                let Some(Value::String(name)) = eval_args.first() else {
                    return Err(InterpreterError::TypeError(
                        "First argument to set_variable must be a string".to_string(),
                    ));
                };
                let value = eval_args.get(1).unwrap_or(&Value::None).cloned();
                self.variables.insert(name.clone(), value);
                Ok(Value::None)
            }
            "configuration_data" => Ok(Value::Object(Arc::new(ConfigData::default()))),
            "configure_file" => {
                let input = eval_kwargs.get("input").and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                });
                let Some(output) = eval_kwargs.get("output").and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                }) else {
                    return Err(InterpreterError::TypeError(
                        "configure_file requires an 'output' keyword argument of type string"
                            .to_string(),
                    ));
                };
                let Some(configuration) = eval_kwargs.get("configuration").and_then(|v| match v {
                    Value::Object(o) => {
                        if let Some(data) = o.as_ref().downcast_ref::<ConfigData>() {
                            Some(data.clone())
                        } else {
                            None
                        }
                    }
                    Value::Dict(dict) => Some(ConfigData::from_dict(dict.clone())),
                    _ => None,
                }) else {
                    return Err(InterpreterError::TypeError(
                        "configure_file requires a 'configuration' keyword argument of type ConfigData"
                            .to_string(),
                    ));
                };
                configuration.configure_file(input, output)
            }
            "is_variable" => {
                if let Some(Value::String(var)) = eval_args.first() {
                    Ok(Value::Boolean(self.variables.contains_key(var)))
                } else {
                    Ok(Value::Boolean(false))
                }
            }
            "get_variable" => {
                if let Some(Value::String(var)) = eval_args.first() {
                    match self.variables.get(var) {
                        Some(value) => Ok(value.clone()),
                        None => match eval_args.get(1).cloned() {
                            Some(v) => Ok(v),
                            None => Err(InterpreterError::UndefinedVariable(var.clone())),
                        },
                    }
                } else {
                    Err(InterpreterError::TypeError(
                        "First argument to get_variable must be a string".to_string(),
                    ))
                }
            }
            "include_directories" => {
                let pwd = env::current_dir().unwrap();
                let mut dirs = flatten_files(&eval_args)?;
                for dir in &mut dirs {
                    *dir = pwd.join(&dir);
                }
                Ok(Value::Object(Arc::new(IncludeDirectories { dirs })))
            }
            "add_project_arguments" => {
                // Ignore for now
                // TODO: Implement this
                Ok(Value::None)
            }
            "files" => {
                let pwd = env::current_dir().unwrap();
                let mut files = flatten_files(&eval_args)?;
                for file in &mut files {
                    *file = pwd.join(&file);
                }
                let files = files
                    .into_iter()
                    .map(|path| Value::Object(Arc::new(File { path })))
                    .collect();
                Ok(Value::Array(files))
            }
            "subdir" => {
                let Some(Value::String(dir)) = eval_args.first() else {
                    return Err(InterpreterError::TypeError(
                        "First argument to subdir must be a string".to_string(),
                    ));
                };
                let pwd = env::current_dir().unwrap();
                struct Restore(PathBuf);
                impl Drop for Restore {
                    fn drop(&mut self) {
                        env::set_current_dir(&self.0).unwrap();
                    }
                }
                let _restore = Restore(pwd.clone());
                env::set_current_dir(dir).map_err(|e| {
                    InterpreterError::RuntimeError(format!("Failed to enter subdir {}: {}", dir, e))
                })?;
                let meson_code = std::fs::read_to_string("meson.build").map_err(|e| {
                    InterpreterError::RuntimeError(format!(
                        "Failed to read meson.build in subdir {}: {}",
                        dir, e
                    ))
                })?;
                match crate::parser::parse_meson_file(&meson_code) {
                    Ok(statements) => {
                        self.interpret(statements)?;
                    }
                    Err(e) => {
                        return Err(InterpreterError::RuntimeError(format!(
                            "Failed to parse meson.build in subdir {}: {}",
                            dir, e
                        )));
                    }
                }
                // TODO: Implement subdir handling
                Ok(Value::None)
            }
            "environment" => {
                if let Some(Value::Dict(vars)) = eval_args.first() {
                    let vars = vars
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect();
                    Ok(Value::Object(Arc::new(Env {
                        vars: Arc::new(Mutex::new(vars)),
                    })))
                } else {
                    Err(InterpreterError::TypeError(
                        "First argument to environment must be a dictionary".to_string(),
                    ))
                }
            }
            "join_paths" => {
                let mut path = PathBuf::new();
                for part in &eval_args {
                    let Value::String(part) = part else {
                        return Err(InterpreterError::TypeError(
                            "All arguments to join_paths must be strings".to_string(),
                        ));
                    };
                    path.push(part);
                }
                // Path joining in Meson
                Ok(Value::String(path.to_string_lossy().to_string()))
            }
            "static_library" => {
                // TODO: Implement static_library
                let Some(Value::String(name)) = eval_args.first().cloned() else {
                    return Err(InterpreterError::TypeError(
                        "First argument to static_library must be a string".to_string(),
                    ));
                };
                let sources = flatten_files(&eval_args[1..])?;
                println!("Creating static library {name} with:\n{:?}", sources);
                Ok(Value::Object(Arc::new(BuildTarget {
                    name,
                    target_type: "static_library".into(),
                    sources,
                })))
            }
            "executable" => {
                // TODO: Implement executable
                let Some(Value::String(name)) = eval_args.first().cloned() else {
                    return Err(InterpreterError::TypeError(
                        "First argument to static_library must be a string".to_string(),
                    ));
                };
                let sources = flatten_files(&eval_args[1..])?;
                println!("Creating executable {name} with:\n{:?}", sources);
                Ok(Value::Object(Arc::new(BuildTarget {
                    name,
                    target_type: "executable".into(),
                    sources,
                })))
            }
            "custom_target" => {
                // TODO: implement custom_target
                Ok(Value::None)
            }
            "find_program" => {
                if let Some(Value::String(prog)) = eval_args.first() {
                    // Simple check if program exists in PATH
                    let full_path = Command::new("which")
                        .arg(prog)
                        .output()
                        .map(|o| {
                            o.status.success().then_some(
                                String::from_utf8_lossy(&o.stdout)
                                    .as_ref()
                                    .trim()
                                    .to_string(),
                            )
                        })
                        .unwrap_or(None);

                    let found = full_path.is_some();
                    let program = Value::Object(Arc::new(ExternalProgram { full_path }));

                    if found {
                        Ok(program)
                    } else {
                        let required = eval_kwargs
                            .get("required")
                            .and_then(|v| {
                                if let Value::Boolean(b) = v {
                                    Some(*b)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(true);

                        if required {
                            Err(InterpreterError::RuntimeError(format!(
                                "Program '{}' not found",
                                prog
                            )))
                        } else {
                            Ok(program)
                        }
                    }
                } else {
                    Ok(Value::None)
                }
            }
            "install_headers" => {
                let headers = eval_args
                    .iter()
                    .flat_map(|v| {
                        if let Value::Array(arr) = v {
                            arr.as_slice()
                        } else {
                            core::slice::from_ref(v)
                        }
                    })
                    .map(|v| {
                        if let Value::Object(s) = v {
                            if let Some(file) = s.as_ref().downcast_ref::<File>() {
                                return Value::String(file.path.to_string_lossy().into_owned());
                            }
                        }
                        v.cloned()
                    })
                    .map(|v| match v {
                        Value::String(s) => Ok(PathBuf::from(s)),
                        _ => {
                            Err(InterpreterError::TypeError(
                                "All arguments to static_library after the first must be strings"
                                    .to_string(),
                            ))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                println!("Installing headers: {:?}", headers);
                Ok(Value::None)
            }
            "assert" => {
                let Some(Value::Boolean(cond)) = eval_args.first() else {
                    return Err(InterpreterError::TypeError(
                        "First argument to assert must be a boolean".to_string(),
                    ));
                };
                if !cond {
                    let msg = if eval_args.len() >= 2 {
                        let msg = eval_args[1].to_string();
                        format!("Assertion failed: {}", msg.trim_matches('"'))
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(InterpreterError::RuntimeError(msg));
                }
                Ok(Value::None)
            }
            "message" => {
                for arg in eval_args {
                    print!("{} ", arg.to_string());
                }
                println!();
                Ok(Value::None)
            }
            "error" => {
                let msg = eval_args
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                Err(InterpreterError::RuntimeError(msg))
            }
            "warning" => {
                print!("WARNING: ");
                for arg in eval_args {
                    print!("{} ", arg.to_string());
                }
                println!();
                Ok(Value::None)
            }
            _ => Err(InterpreterError::UndefinedFunction(name.to_string())),
        }
    }

    fn call_method(
        &mut self,
        object: Value,
        method: &str,
        args: Vec<AstValue>,
        kwargs: HashMap<String, AstValue>,
    ) -> Result<Value, InterpreterError> {
        // Evaluate arguments
        let mut eval_args = Vec::new();
        for arg in args {
            eval_args.push(self.evaluate_value(arg)?);
        }

        let mut eval_kwargs = HashMap::new();
        for (k, v) in kwargs {
            eval_kwargs.insert(k, self.evaluate_value(v)?);
        }

        match object {
            Value::String(ref s) => match method {
                "format" => Ok(Value::String(
                    Value::String(s.clone()).format_string(&eval_args),
                )),
                "split" => {
                    let separator = eval_args
                        .first()
                        .and_then(|v| {
                            if let Value::String(s) = v {
                                Some(s.as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or(" ");

                    let parts: Vec<Value> = s
                        .split(separator)
                        .map(|p| Value::String(p.to_string()))
                        .collect();
                    Ok(Value::Array(parts))
                }
                "join" => {
                    let result = eval_args
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(s);
                    Ok(Value::String(result))
                }
                "strip" => Ok(Value::String(s.trim().to_string())),
                "startswith" => {
                    if let Some(Value::String(prefix)) = eval_args.first() {
                        Ok(Value::Boolean(s.starts_with(prefix)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                "endswith" => {
                    if let Some(Value::String(suffix)) = eval_args.first() {
                        Ok(Value::Boolean(s.ends_with(suffix)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                "substring" => {
                    let start = eval_args
                        .get(0)
                        .and_then(|v| {
                            if let Value::Integer(i) = v {
                                Some(*i as usize)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                    let len = eval_args
                        .get(1)
                        .and_then(|v| {
                            if let Value::Integer(i) = v {
                                Some(*i as usize)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(1);

                    let result = s.chars().skip(start).take(len).collect::<String>();
                    Ok(Value::String(result))
                }
                "contains" => {
                    if let Some(Value::String(substr)) = eval_args.first() {
                        Ok(Value::Boolean(s.contains(substr)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                "underscorify" => {
                    let underscored = s
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect();
                    Ok(Value::String(underscored))
                }
                "to_upper" => Ok(Value::String(s.to_uppercase())),
                "to_lower" => Ok(Value::String(s.to_lowercase())),
                _ => Err(InterpreterError::RuntimeError(format!(
                    "Unknown method '{}' for string",
                    method
                ))),
            },
            Value::Array(ref arr) => match method {
                "get" => {
                    let idx = eval_args
                        .first()
                        .and_then(|v| {
                            if let Value::Integer(i) = v {
                                Some(*i as usize)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);

                    if idx < arr.len() {
                        Ok(arr[idx].clone())
                    } else if eval_args.len() >= 2 {
                        Ok(eval_args[1].clone())
                    } else {
                        Ok(Value::None)
                    }
                }
                "contains" => {
                    if let Some(item) = eval_args.first() {
                        Ok(Value::Boolean(arr.contains(item)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                "length" => Ok(Value::Integer(arr.len() as i64)),
                _ => Err(InterpreterError::RuntimeError(format!(
                    "Unknown method '{}' for array",
                    method
                ))),
            },
            Value::Dict(ref dict) => match method {
                "get" => {
                    if let Some(Value::String(key)) = eval_args.first() {
                        if let Some(value) = dict.get(key) {
                            Ok(value.clone())
                        } else if eval_args.len() >= 2 {
                            Ok(eval_args[1].clone())
                        } else {
                            Ok(Value::None)
                        }
                    } else {
                        Ok(Value::None)
                    }
                }
                "has_key" => {
                    if let Some(Value::String(key)) = eval_args.first() {
                        Ok(Value::Boolean(dict.contains_key(key)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                "keys" => {
                    let keys: Vec<Value> = dict.keys().map(|k| Value::String(k.clone())).collect();
                    Ok(Value::Array(keys))
                }
                "values" => {
                    let values: Vec<Value> = dict.values().cloned().collect();
                    Ok(Value::Array(values))
                }
                _ => Err(InterpreterError::RuntimeError(format!(
                    "Unknown method '{}' for dict",
                    method
                ))),
            },
            Value::Object(ref obj) => {
                if let Some(method_fn) = obj.get_method(method) {
                    method_fn(&eval_args, &eval_kwargs)
                } else {
                    Err(InterpreterError::RuntimeError(format!(
                        "Unknown method '{}' for object",
                        method
                    )))
                }
            }
            _ => Err(InterpreterError::TypeError(format!(
                "Cannot call method '{}' on {:?}",
                method, object
            ))),
        }
    }

    fn apply_binary_op(
        &self,
        left: Value,
        op: BinaryOperator,
        right: Value,
    ) -> Result<Value, InterpreterError> {
        match op {
            BinaryOperator::Add => self.add_values(&left, &right),
            BinaryOperator::Sub => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                _ => Err(InterpreterError::TypeError(
                    "Cannot subtract non-integers".to_string(),
                )),
            },
            BinaryOperator::Mul => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                (Value::String(s), Value::Integer(n)) | (Value::Integer(n), Value::String(s)) => {
                    Ok(Value::String(s.repeat(n as usize)))
                }
                _ => Err(InterpreterError::TypeError(
                    "Invalid operands for multiplication".to_string(),
                )),
            },
            BinaryOperator::Div => {
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            Err(InterpreterError::RuntimeError(
                                "Division by zero".to_string(),
                            ))
                        } else {
                            Ok(Value::Integer(a / b))
                        }
                    }
                    (Value::String(s), Value::String(sep)) => {
                        // Path joining in Meson
                        let path = PathBuf::from(s).join(sep);
                        Ok(Value::String(path.to_string_lossy().to_string()))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "Invalid operands for division".to_string(),
                    )),
                }
            }
            BinaryOperator::Mod => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if b == 0 {
                        Err(InterpreterError::RuntimeError("Modulo by zero".to_string()))
                    } else {
                        Ok(Value::Integer(a % b))
                    }
                }
                _ => Err(InterpreterError::TypeError(
                    "Cannot modulo non-integers".to_string(),
                )),
            },
            BinaryOperator::Eq => Ok(Value::Boolean(left == right)),
            BinaryOperator::Ne => Ok(Value::Boolean(left != right)),
            BinaryOperator::Lt => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
                _ => Err(InterpreterError::TypeError(
                    "Cannot compare incompatible types".to_string(),
                )),
            },
            BinaryOperator::Le => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a <= b)),
                _ => Err(InterpreterError::TypeError(
                    "Cannot compare incompatible types".to_string(),
                )),
            },
            BinaryOperator::Gt => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a > b)),
                _ => Err(InterpreterError::TypeError(
                    "Cannot compare incompatible types".to_string(),
                )),
            },
            BinaryOperator::Ge => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a >= b)),
                _ => Err(InterpreterError::TypeError(
                    "Cannot compare incompatible types".to_string(),
                )),
            },
            BinaryOperator::And => Ok(Value::Boolean(left.to_bool() && right.to_bool())),
            BinaryOperator::Or => Ok(Value::Boolean(left.to_bool() || right.to_bool())),
            BinaryOperator::In => match right {
                Value::Array(arr) => Ok(Value::Boolean(arr.contains(&left))),
                Value::String(s) => {
                    if let Value::String(needle) = left {
                        Ok(Value::Boolean(s.contains(&needle)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                Value::Dict(dict) => {
                    if let Value::String(key) = left {
                        Ok(Value::Boolean(dict.contains_key(&key)))
                    } else {
                        Ok(Value::Boolean(false))
                    }
                }
                _ => Ok(Value::Boolean(false)),
            },
            BinaryOperator::NotIn => match right {
                Value::Array(arr) => Ok(Value::Boolean(!arr.contains(&left))),
                Value::String(s) => {
                    if let Value::String(needle) = left {
                        Ok(Value::Boolean(!s.contains(&needle)))
                    } else {
                        Ok(Value::Boolean(true))
                    }
                }
                Value::Dict(dict) => {
                    if let Value::String(key) = left {
                        Ok(Value::Boolean(!dict.contains_key(&key)))
                    } else {
                        Ok(Value::Boolean(true))
                    }
                }
                _ => Ok(Value::Boolean(true)),
            },
        }
    }

    fn apply_unary_op(&self, op: UnaryOperator, value: Value) -> Result<Value, InterpreterError> {
        match op {
            UnaryOperator::Not => Ok(Value::Boolean(!value.to_bool())),
            UnaryOperator::Minus => match value {
                Value::Integer(i) => Ok(Value::Integer(-i)),
                _ => Err(InterpreterError::TypeError(
                    "Cannot negate non-integer".to_string(),
                )),
            },
        }
    }

    fn subscript(&self, object: Value, index: Value) -> Result<Value, InterpreterError> {
        match object {
            Value::Array(arr) => {
                if let Value::Integer(idx) = index {
                    let idx = if idx < 0 {
                        (arr.len() as i64 + idx) as usize
                    } else {
                        idx as usize
                    };

                    arr.get(idx).cloned().ok_or_else(|| {
                        InterpreterError::RuntimeError("Index out of bounds".to_string())
                    })
                } else {
                    Err(InterpreterError::TypeError(
                        "Array index must be integer".to_string(),
                    ))
                }
            }
            Value::Dict(dict) => {
                if let Value::String(key) = index {
                    dict.get(&key).cloned().ok_or_else(|| {
                        InterpreterError::RuntimeError(format!("Key '{}' not found", key))
                    })
                } else {
                    Err(InterpreterError::TypeError(
                        "Dictionary key must be string".to_string(),
                    ))
                }
            }
            Value::String(s) => {
                if let Value::Integer(idx) = index {
                    let idx = if idx < 0 {
                        (s.len() as i64 + idx) as usize
                    } else {
                        idx as usize
                    };

                    s.chars()
                        .nth(idx)
                        .map(|c| Value::String(c.to_string()))
                        .ok_or_else(|| {
                            InterpreterError::RuntimeError("Index out of bounds".to_string())
                        })
                } else {
                    Err(InterpreterError::TypeError(
                        "String index must be integer".to_string(),
                    ))
                }
            }
            _ => Err(InterpreterError::TypeError(
                "Cannot subscript this type".to_string(),
            )),
        }
    }

    fn add_values(&self, left: &Value, right: &Value) -> Result<Value, InterpreterError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::Array(a), Value::Array(b)) => {
                let mut result = a.clone();
                result.extend(b.clone());
                Ok(Value::Array(result))
            }
            (Value::Array(a), b) => {
                let mut result = a.clone();
                result.push(b.clone());
                Ok(Value::Array(result))
            }
            _ => Err(InterpreterError::TypeError(format!(
                "Cannot add incompatible types {left:?} + {right:?}"
            ))),
        }
    }
}

// Helper function to run interpreter on parsed AST
pub fn run_interpreter(statements: Vec<Statement>) -> Result<(), InterpreterError> {
    let mut interpreter = Interpreter::new();
    interpreter.interpret(statements)
}
