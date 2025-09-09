use alloc::rc::Rc;
use core::cell::{Ref, RefCell};
use core::fmt;

use as_any::Downcast;
use hashbrown::HashMap;

use crate::os::Os;
use crate::parser::{BinaryOperator, Statement, UnaryOperator, Value as AstValue};

mod builtins;

use builtins::build_target::{custom_target, executable, static_library};
use builtins::config_data::{configuration_data, configure_file};
use builtins::debug::{assert, error as error_fn, message, warning};
use builtins::env::environment;
use builtins::external_program::find_program;
use builtins::files::files;
use builtins::filesystem::filesystem;
use builtins::import::import;
use builtins::include_directories::include_directories;
use builtins::install_headers::install_headers;
use builtins::join_paths::join_paths;
use builtins::machine::{host_machine, target_machine};
use builtins::meson::{Meson, meson};
use builtins::option::{get_option, option};
use builtins::project::{add_project_arguments, project};
use builtins::run_result::run_command;
use builtins::subdir::subdir;
use builtins::variable::{get_variable, is_variable, set_variable};
use builtins::{array as builtin_array, dict as builtin_dict, string as builtin_string};

pub mod error;

pub use error::InterpreterError;
use error::{ErrorContext as _, bail_runtime_error, bail_type_error};

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
        interp: &mut Interpreter,
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
    current_dir: String,
    os_env: Box<dyn Os>,
}

impl Interpreter {
    pub fn new(os_env: impl Os) -> Self {
        let meson = meson(".", "./build");
        let meson = Rc::new(RefCell::new(meson));

        let mut interpreter = Self {
            variables: HashMap::new(),
            options: HashMap::new(),
            break_flag: false,
            continue_flag: false,
            meson,
            current_dir: ".".into(),
            os_env: Box::new(os_env),
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
            .insert("host_machine".to_string(), host_machine(self).into_object());

        // Target machine
        self.variables.insert(
            "target_machine".to_string(),
            target_machine(self).into_object(),
        );

        // Build machine (same as host for now)
        self.variables.insert(
            "build_machine".to_string(),
            host_machine(self).into_object(),
        );

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
                .ok_or(InterpreterError::UndefinedVariable(name)),
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
            "project" => project(eval_args, eval_kwargs, self),
            "option" => option(eval_args, eval_kwargs, self),
            "get_option" => get_option(eval_args, eval_kwargs, self),
            "import" => import(eval_args, eval_kwargs, self),
            "run_command" => run_command(eval_args, eval_kwargs, self),
            "set_variable" => set_variable(eval_args, eval_kwargs, self),
            "configuration_data" => configuration_data(eval_args, eval_kwargs, self),
            "configure_file" => configure_file(eval_args, eval_kwargs, self),
            "is_variable" => is_variable(eval_args, eval_kwargs, self),
            "get_variable" => get_variable(eval_args, eval_kwargs, self),
            "include_directories" => include_directories(eval_args, eval_kwargs, self),
            "add_project_arguments" => add_project_arguments(eval_args, eval_kwargs, self),
            "files" => files(eval_args, eval_kwargs, self),
            "subdir" => subdir(eval_args, eval_kwargs, self),
            "environment" => environment(eval_args, eval_kwargs, self),
            "join_paths" => join_paths(eval_args, eval_kwargs, self),
            "static_library" => static_library(eval_args, eval_kwargs, self),
            "executable" => executable(eval_args, eval_kwargs, self),
            "custom_target" => custom_target(eval_args, eval_kwargs, self),
            "find_program" => find_program(eval_args, eval_kwargs, self),
            "install_headers" => install_headers(eval_args, eval_kwargs, self),
            "assert" => assert(eval_args, eval_kwargs, self),
            "message" => message(eval_args, eval_kwargs, self),
            "error" => error_fn(eval_args, eval_kwargs, self),
            "warning" => warning(eval_args, eval_kwargs, self),
            _ => Err(InterpreterError::UndefinedFunction(name.into())),
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
                "format" => builtin_string::format(s, eval_args, eval_kwargs, self),
                "split" => builtin_string::split(s, eval_args, eval_kwargs, self),
                "join" => builtin_string::join(s, eval_args, eval_kwargs, self),
                "strip" => builtin_string::strip(s, eval_args, eval_kwargs, self),
                "startswith" => builtin_string::startswith(s, eval_args, eval_kwargs, self),
                "endswith" => builtin_string::endswith(s, eval_args, eval_kwargs, self),
                "substring" => builtin_string::substring(s, eval_args, eval_kwargs, self),
                "contains" => builtin_string::contains(s, eval_args, eval_kwargs, self),
                "underscorify" => builtin_string::underscorify(s, eval_args, eval_kwargs, self),
                "to_upper" => builtin_string::to_upper(s, eval_args, eval_kwargs, self),
                "to_lower" => builtin_string::to_lower(s, eval_args, eval_kwargs, self),
                _ => bail_runtime_error!("Unknown method '{method}' for string"),
            },
            Value::Array(ref arr) => match method {
                "get" => builtin_array::get(arr, eval_args, eval_kwargs, self),
                "contains" => builtin_array::contains(arr, eval_args, eval_kwargs, self),
                "length" => builtin_array::length(arr, eval_args, eval_kwargs, self),
                _ => bail_runtime_error!("Unknown method '{method}' for array"),
            },
            Value::Dict(ref dict) => match method {
                "get" => builtin_dict::get(dict, eval_args, eval_kwargs, self),
                "has_key" => builtin_dict::has_key(dict, eval_args, eval_kwargs, self),
                "keys" => builtin_dict::keys(dict, eval_args, eval_kwargs, self),
                "values" => builtin_dict::values(dict, eval_args, eval_kwargs, self),
                _ => bail_runtime_error!("Unknown method '{method}' for dict"),
            },
            Value::Object(ref obj) => {
                let mut obj = obj.as_ref().borrow_mut();
                obj.call_method(method, eval_args, eval_kwargs, self)
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
                    (Value::String(a), Value::String(b)) => {
                        // Path joining in Meson
                        let joined = self.os_env.join_paths(&[a, b]);
                        Ok(Value::String(joined))
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
pub fn run_interpreter(
    os_env: impl Os,
    statements: Vec<Statement>,
) -> Result<(), InterpreterError> {
    let mut interpreter = Interpreter::new(os_env);
    interpreter.interpret(statements)
}
