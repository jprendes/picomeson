use alloc::rc::Rc;
use core::cell::{Ref, RefCell};
use core::fmt;
use std::env;
use std::path::PathBuf;

use hashbrown::HashMap;

use crate::interpreter::error::ErrorContext as _;
use crate::parser::{BinaryOperator, Statement, UnaryOperator, Value as AstValue};

mod builtins;

use as_any::Downcast;
use builtins::build_target::{executable, static_library};
use builtins::config_data::{configuration_data, configure_file};
use builtins::env::environment;
use builtins::external_program::find_program;
use builtins::files::files;
use builtins::filesystem::filesystem;
use builtins::import::import;
use builtins::include_directories::include_directories;
use builtins::install_headers::install_headers;
use builtins::machine::{host_machine, target_machine};
use builtins::meson::{Meson, meson};
use builtins::run_result::run_command;

pub mod error;

pub use error::InterpreterError;
use error::{bail_runtime_error, bail_type_error};

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    None,
    Object(Rc<RefCell<dyn MesonObject>>),
}

impl Value {
    fn coerce_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.coerce_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Dict(dict) => {
                let items: Vec<String> = dict
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.coerce_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::None => "none".to_string(),
            Value::Object(obj) => obj.borrow().to_string(),
        }
    }

    fn coerce_bool(&self) -> bool {
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

    fn as_bool(&self) -> Result<bool, InterpreterError> {
        match self {
            Value::Boolean(b) => Ok(*b),
            _ => bail_type_error!("Expected a boolean, found {:?}", self),
        }
    }

    fn as_string(&self) -> Result<&str, InterpreterError> {
        match self {
            Value::String(s) => Ok(s.as_str()),
            _ => bail_type_error!("Expected a string, found {:?}", self),
        }
    }

    fn as_array(&self) -> Result<&[Value], InterpreterError> {
        match self {
            Value::Array(arr) => Ok(arr.as_slice()),
            _ => bail_type_error!("Expected an array, found {:?}", self),
        }
    }

    fn as_integer(&self) -> Result<i64, InterpreterError> {
        match self {
            Value::Integer(i) => Ok(*i),
            _ => bail_type_error!("Expected an integer, found {:?}", self),
        }
    }

    fn as_object<T: MesonObject>(&self) -> Result<Ref<'_, T>, InterpreterError> {
        match self {
            Value::Object(obj) => {
                let src_typename = obj.borrow().object_type();
                let dst_typename = core::any::type_name::<T>();
                borrow_downcast::<T>(obj).with_context_type(|| {
                    format!("Object type mismatch, expected {dst_typename}, found {src_typename}")
                })
            }
            _ => Err(InterpreterError::TypeError("Expected an object".into())),
        }
    }

    fn as_dict(&self) -> Result<&HashMap<String, Value>, InterpreterError> {
        match self {
            Value::Dict(d) => Ok(d),
            _ => bail_type_error!("Expected a dict, found {:?}", self),
        }
    }

    fn format_string(&self, args: &[Value]) -> String {
        let mut result = self.coerce_string();
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("@{}@", i);
            result = result.replace(&placeholder, &arg.coerce_string());
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
            (Value::Object(a), Value::Object(b)) => a.borrow().is_equal(b),
            (Value::String(a), b) => a == &b.coerce_string(),
            (a, Value::String(b)) => &a.coerce_string() == b,
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
            Value::Object(obj) => Value::Object(obj.borrow().clone_rc()),
        }
    }
}

pub trait MesonObject: fmt::Debug + as_any::AsAny {
    fn call_method(
        &mut self,
        name: &str,
        args: Vec<Value>,
        kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError>;
    fn clone_rc(&self) -> Rc<RefCell<dyn MesonObject>>;
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
    fn is_equal(&self, other: &Rc<RefCell<dyn MesonObject>>) -> bool;
    fn into_object(self) -> Value
    where
        Self: Sized + 'static,
    {
        Value::Object(Rc::new(RefCell::new(self)))
    }
    fn object_type(&'_ self) -> &'static str {
        core::any::type_name::<Self>()
    }
}

pub fn borrow_downcast<'a, T: MesonObject>(
    cell: &'a RefCell<dyn MesonObject>,
) -> Option<Ref<'a, T>> {
    let r = cell.borrow();
    if (*r).type_id() == core::any::TypeId::of::<T>() {
        Some(Ref::map(r, |x| x.downcast_ref::<T>().unwrap()))
    } else {
        None
    }
}

pub struct Interpreter {
    variables: HashMap<String, Value>,
    options: HashMap<String, Value>,
    break_flag: bool,
    continue_flag: bool,
    meson: Rc<RefCell<Meson>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let src_dir = env::current_dir().unwrap();
        let bld_dir = src_dir.join("build");
        let meson = meson(src_dir, bld_dir);
        let meson = Rc::new(RefCell::new(meson));

        let mut interpreter = Self {
            variables: HashMap::new(),
            options: HashMap::new(),
            break_flag: false,
            continue_flag: false,
            meson,
        };

        // Initialize built-in variables
        interpreter.init_builtins();
        interpreter
    }

    fn init_builtins(&mut self) {
        // Meson object
        self.variables
            .insert("meson".to_string(), Value::Object(self.meson.clone()));

        // Host machine
        self.variables
            .insert("host_machine".to_string(), host_machine().into_object());

        // Target machine
        self.variables
            .insert("target_machine".to_string(), target_machine().into_object());

        // Build machine (same as host for now)
        self.variables
            .insert("build_machine".to_string(), host_machine().into_object());

        // File system object
        self.variables
            .insert("fs".to_string(), filesystem().into_object());
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
                if cond_value.coerce_bool() {
                    self.execute_block(then_branch)?;
                } else {
                    let mut executed = false;
                    for (elif_cond, elif_body) in elif_branches {
                        let elif_value = self.evaluate_value(elif_cond)?;
                        if elif_value.coerce_bool() {
                            self.execute_block(elif_body)?;
                            executed = true;
                            break;
                        }
                    }
                    if !executed && let Some(else_body) = else_branch {
                        self.execute_block(else_body)?;
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
                        bail_type_error!("Cannot iterate over non-iterable");
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
                .ok_or(InterpreterError::UndefinedVariable(name.into())),
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
                if cond.coerce_bool() {
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
                let Some(Value::String(_project_name)) = eval_args.first() else {
                    bail_type_error!("First argument to project must be a string");
                };

                let project_version = match eval_kwargs.get("version") {
                    Some(Value::String(v)) => v.clone(),
                    None => "0.0.0".to_string(),
                    Some(_) => {
                        bail_type_error!("Expected 'version' keyword argument to be a string");
                    }
                };

                self.meson.borrow_mut().project_version = project_version;

                Ok(Value::None)
            }
            "option" => {
                let opt: String = eval_args
                    .first()
                    .context_type("First argument to option must be a string")?
                    .as_string()?
                    .into();

                let typ = eval_kwargs
                    .get("type")
                    .context_type("Option requires a 'type' keyword argument")?
                    .as_string()?;

                let value = eval_kwargs.get("value");
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

                self.options.insert(opt, value);

                Ok(Value::None)
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
            "import" => import(eval_args, eval_kwargs),
            "run_command" => run_command(eval_args, eval_kwargs),
            "set_variable" => {
                if eval_args.len() != 2 {
                    bail_runtime_error!("set_variable requires 2 arguments");
                }
                let Some(Value::String(name)) = eval_args.first() else {
                    bail_type_error!("First argument to set_variable must be a string");
                };
                let value = eval_args.get(1).unwrap_or(&Value::None).cloned();
                self.variables.insert(name.clone(), value);
                Ok(Value::None)
            }
            "configuration_data" => configuration_data(eval_args, eval_kwargs),
            "configure_file" => configure_file(eval_args, eval_kwargs),
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
                            None => {
                                Err(InterpreterError::UndefinedVariable(var.to_string().into()))
                            }
                        },
                    }
                } else {
                    bail_type_error!("First argument to get_variable must be a string");
                }
            }
            "include_directories" => include_directories(eval_args, eval_kwargs),
            "add_project_arguments" => {
                // Ignore for now
                // TODO: Implement this
                Ok(Value::None)
            }
            "files" => files(eval_args, eval_kwargs),
            "subdir" => {
                let Some(Value::String(dir)) = eval_args.first() else {
                    bail_type_error!("First argument to subdir must be a string");
                };
                let pwd = env::current_dir().unwrap();
                struct Restore(PathBuf);
                impl Drop for Restore {
                    fn drop(&mut self) {
                        env::set_current_dir(&self.0).unwrap();
                    }
                }
                let _restore = Restore(pwd.clone());
                env::set_current_dir(dir)
                    .with_context_runtime(|| format!("Failed to change directory to {}", dir))?;
                let meson_code =
                    std::fs::read_to_string("meson.build").with_context_runtime(|| {
                        format!("Failed to read meson.build in subdir {}", dir)
                    })?;
                let statements = crate::parser::parse_meson_file(&meson_code)
                    .with_context_runtime(|| {
                        format!("Failed to parse meson.build in subdir {}", dir)
                    })?;
                self.interpret(statements)?;
                Ok(Value::None)
            }
            "environment" => environment(eval_args, eval_kwargs),
            "join_paths" => {
                let mut path = PathBuf::new();
                for part in &eval_args {
                    let part = part
                        .as_string()
                        .context_type("All arguments to join_paths must be strings")?;
                    path.push(part);
                }
                // Path joining in Meson
                Ok(Value::String(path.to_string_lossy().to_string()))
            }
            "static_library" => static_library(eval_args, eval_kwargs),
            "executable" => executable(eval_args, eval_kwargs),
            "custom_target" => {
                // TODO: implement custom_target
                Ok(Value::None)
            }
            "find_program" => find_program(eval_args, eval_kwargs),
            "install_headers" => install_headers(eval_args, eval_kwargs),
            "assert" => {
                let Some(Value::Boolean(cond)) = eval_args.first() else {
                    bail_type_error!("First argument to assert must be a boolean");
                };
                if !cond {
                    let msg = if eval_args.len() >= 2 {
                        let msg = eval_args[1].coerce_string();
                        format!("Assertion failed: {}", msg.trim_matches('"'))
                    } else {
                        "Assertion failed".to_string()
                    };
                    bail_runtime_error!("Assert failure: {msg}");
                }
                Ok(Value::None)
            }
            "message" => {
                for arg in eval_args {
                    print!("{} ", arg.coerce_string());
                }
                println!();
                Ok(Value::None)
            }
            "error" => {
                let msg = eval_args
                    .iter()
                    .map(|v| v.coerce_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                bail_runtime_error!("{msg}");
            }
            "warning" => {
                print!("WARNING: ");
                for arg in eval_args {
                    print!("{} ", arg.coerce_string());
                }
                println!();
                Ok(Value::None)
            }
            _ => Err(InterpreterError::UndefinedFunction(name.to_string().into())),
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
                        .map(|v| v.coerce_string())
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
                        .first()
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
                _ => bail_runtime_error!("Unknown method '{method}' for string"),
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
                _ => bail_runtime_error!("Unknown method '{method}' for array"),
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
                _ => bail_runtime_error!("Unknown method '{method}' for dict"),
            },
            Value::Object(ref obj) => {
                let mut obj = obj.as_ref().borrow_mut();
                obj.call_method(method, eval_args, eval_kwargs)
            }
            _ => bail_type_error!("Cannot call method '{method}' on {object:?}"),
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
                _ => bail_type_error!("Cannot subtract non-integers"),
            },
            BinaryOperator::Mul => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                (Value::String(s), Value::Integer(n)) | (Value::Integer(n), Value::String(s)) => {
                    Ok(Value::String(s.repeat(n as usize)))
                }
                _ => bail_type_error!("Invalid operands for multiplication"),
            },
            BinaryOperator::Div => {
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            bail_runtime_error!("Division by zero");
                        } else {
                            Ok(Value::Integer(a / b))
                        }
                    }
                    (Value::String(s), Value::String(sep)) => {
                        // Path joining in Meson
                        let path = PathBuf::from(s).join(sep);
                        Ok(Value::String(path.to_string_lossy().to_string()))
                    }
                    _ => bail_type_error!("Invalid operands for division"),
                }
            }
            BinaryOperator::Mod => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if b == 0 {
                        bail_runtime_error!("Modulo by zero");
                    } else {
                        Ok(Value::Integer(a % b))
                    }
                }
                _ => bail_type_error!("Cannot modulo non-integers"),
            },
            BinaryOperator::Eq => Ok(Value::Boolean(left == right)),
            BinaryOperator::Ne => Ok(Value::Boolean(left != right)),
            BinaryOperator::Lt => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
                _ => bail_type_error!("Cannot compare incompatible types"),
            },
            BinaryOperator::Le => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a <= b)),
                _ => bail_type_error!("Cannot compare incompatible types"),
            },
            BinaryOperator::Gt => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a > b)),
                _ => bail_type_error!("Cannot compare incompatible types"),
            },
            BinaryOperator::Ge => match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a >= b)),
                _ => bail_type_error!("Cannot compare incompatible types"),
            },
            BinaryOperator::And => Ok(Value::Boolean(left.coerce_bool() && right.coerce_bool())),
            BinaryOperator::Or => Ok(Value::Boolean(left.coerce_bool() || right.coerce_bool())),
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
            UnaryOperator::Not => Ok(Value::Boolean(!value.coerce_bool())),
            UnaryOperator::Minus => match value {
                Value::Integer(i) => Ok(Value::Integer(-i)),
                _ => bail_type_error!("Cannot negate non-integer"),
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

                    arr.get(idx).cloned().context_runtime("Index out of bounds")
                } else {
                    bail_type_error!("Array index must be integer")
                }
            }
            Value::Dict(dict) => {
                if let Value::String(key) = index {
                    dict.get(&key)
                        .cloned()
                        .context_runtime(format!("Key '{}' not found", key))
                } else {
                    bail_type_error!("Dictionary key must be string")
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
                        .context_runtime("String index out of bounds")
                } else {
                    bail_type_error!("String index must be integer")
                }
            }
            _ => bail_type_error!("Cannot subscript this type"),
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
            _ => bail_type_error!("Cannot add incompatible types {left:?} + {right:?}"),
        }
    }
}

// Helper function to run interpreter on parsed AST
pub fn run_interpreter(statements: Vec<Statement>) -> Result<(), InterpreterError> {
    let mut interpreter = Interpreter::new();
    interpreter.interpret(statements)
}
