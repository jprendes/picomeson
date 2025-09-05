use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

use crate::parser::{BinaryOperator, Statement, UnaryOperator, Value as AstValue};

mod builtins;

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

thread_local! {
    pub static CURRENT_INTERPRETER: RefCell<Option<Rc<RefCell<Interpreter>>>> = RefCell::new(None);
}

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
            Value::Object(obj) => obj.borrow().to_string(),
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
            (Value::Object(a), Value::Object(b)) => a.borrow().is_equal(b),
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
            Value::Object(obj) => Value::Object(obj.borrow().clone_rc()),
        }
    }
}

pub trait MesonObject: std::fmt::Debug + as_any::AsAny {
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
}

impl dyn MesonObject {
    fn downcast_ref<T: MesonObject>(&self) -> Result<&T, InterpreterError> {
        let src_type_name = self.type_name();
        let dst_type_name = core::any::type_name::<T>();
        self.as_any().downcast_ref::<T>().ok_or_else(|| {
            InterpreterError::TypeError(format!(
                "Expected object of type {dst_type_name}, got {src_type_name}",
            ))
        })
    }

    /*
    fn downcast_mut<T: MesonObject>(&mut self) -> Result<&mut T, InterpreterError> {
        let src_type_name = self.type_name();
        let dst_type_name = core::any::type_name::<T>();
        self.as_any_mut().downcast_mut::<T>().ok_or_else(|| {
            InterpreterError::TypeError(format!(
                "Expected object of type {dst_type_name}, got {src_type_name}",
            ))
        })
    }
    */
}

pub struct Interpreter {
    variables: HashMap<String, Value>,
    options: HashMap<String, Value>,
    break_flag: bool,
    continue_flag: bool,
    meson: Rc<RefCell<Meson>>,
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
                let Some(Value::String(_project_name)) = eval_args.first() else {
                    return Err(InterpreterError::TypeError(
                        "First argument to project must be a string".to_string(),
                    ));
                };

                let project_version = match eval_kwargs.get("version") {
                    Some(Value::String(v)) => v.clone(),
                    None => "0.0.0".to_string(),
                    Some(_) => {
                        return Err(InterpreterError::TypeError(
                            "Expected 'version' keyword argument to be a string".into(),
                        ));
                    }
                };

                self.meson.borrow_mut().project_version = project_version;

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
            "import" => import(eval_args, eval_kwargs),
            "run_command" => run_command(eval_args, eval_kwargs),
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
                            None => Err(InterpreterError::UndefinedVariable(var.clone())),
                        },
                    }
                } else {
                    Err(InterpreterError::TypeError(
                        "First argument to get_variable must be a string".to_string(),
                    ))
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
            "environment" => environment(eval_args, eval_kwargs),
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
                let mut obj = obj.as_ref().borrow_mut();
                obj.call_method(method, eval_args, eval_kwargs)
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
