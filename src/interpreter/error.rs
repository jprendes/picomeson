use core::fmt;
use core::fmt::Display;

#[derive(Debug)]
pub enum InterpreterError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    TypeError(String),
    RuntimeError(String),
}

macro_rules! bail_type_error {
    ($msg:expr, $($arg:tt)*) => { return Err(InterpreterError::TypeError(format!($msg, $($arg)*).into())) };
    ($msg:expr) =>              { return Err(InterpreterError::TypeError(format!($msg).into())) };
    () =>                       { return Err(InterpreterError::TypeError("Type mismatch".into())) };
}

macro_rules! bail_runtime_error {
    ($msg:expr, $($arg:tt)*) => { return Err(InterpreterError::RuntimeError(format!($msg, $($arg)*).into())) };
    ($msg:expr) =>              { return Err(InterpreterError::RuntimeError(format!($msg).into())) };
    () =>                       { return Err(InterpreterError::RuntimeError("Runtime error".into())) };
}

pub(crate) use {bail_runtime_error, bail_type_error};

pub trait ErrorContext: Sized {
    type Ok;
    fn context_type(self, msg: impl Display) -> Result<Self::Ok, InterpreterError> {
        self.with_context_type(|| msg)
    }
    fn context_undef_variable(self, msg: impl Display) -> Result<Self::Ok, InterpreterError> {
        self.with_context_undef_variable(|| msg)
    }
    fn context_runtime(self, msg: impl Display) -> Result<Self::Ok, InterpreterError> {
        self.with_context_runtime(|| msg)
    }
    fn with_context_type<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<Self::Ok, InterpreterError>;
    fn with_context_runtime<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<Self::Ok, InterpreterError>;
    fn with_context_undef_variable<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<Self::Ok, InterpreterError>;
}

impl<T, E: core::fmt::Display> ErrorContext for Result<T, E> {
    type Ok = T;
    fn with_context_type<R: Display>(self, f: impl FnOnce() -> R) -> Result<T, InterpreterError> {
        self.map_err(|e| InterpreterError::TypeError(format!("{}: {}", f(), e)))
    }
    fn with_context_runtime<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.map_err(|e| InterpreterError::RuntimeError(format!("{}: {}", f(), e)))
    }
    fn with_context_undef_variable<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.map_err(|e| InterpreterError::UndefinedVariable(format!("{}: {}", f(), e)))
    }
}

impl<T> ErrorContext for Option<T> {
    type Ok = T;
    fn with_context_type<R: Display>(self, f: impl FnOnce() -> R) -> Result<T, InterpreterError> {
        self.ok_or_else(|| InterpreterError::TypeError(f().to_string()))
    }
    fn with_context_runtime<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.ok_or_else(|| InterpreterError::RuntimeError(f().to_string()))
    }
    fn with_context_undef_variable<R: Display>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.ok_or_else(|| InterpreterError::UndefinedVariable(f().to_string()))
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InterpreterError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            InterpreterError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            InterpreterError::TypeError(msg) => write!(f, "Type error: {}", msg),
            InterpreterError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl core::error::Error for InterpreterError {}
