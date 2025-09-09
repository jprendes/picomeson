use core::fmt;
use std::borrow::Cow;

#[derive(Debug)]
pub enum InterpreterError {
    UndefinedVariable(Cow<'static, str>),
    UndefinedFunction(Cow<'static, str>),
    TypeError(Cow<'static, str>),
    RuntimeError(Cow<'static, str>),
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
    fn context_type(self, msg: impl Into<Cow<'static, str>>) -> Result<Self::Ok, InterpreterError> {
        self.with_context_type(|| msg)
    }
    fn context_runtime(
        self,
        msg: impl Into<Cow<'static, str>>,
    ) -> Result<Self::Ok, InterpreterError> {
        self.with_context_runtime(|| msg)
    }
    fn with_context_type<R: Into<Cow<'static, str>>>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<Self::Ok, InterpreterError>;
    fn with_context_runtime<R: Into<Cow<'static, str>>>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<Self::Ok, InterpreterError>;
}

impl<T, E: core::fmt::Display> ErrorContext for Result<T, E> {
    type Ok = T;
    fn with_context_type<R: Into<Cow<'static, str>>>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.map_err(|e| InterpreterError::TypeError(Cow::from(format!("{}: {}", f().into(), e))))
    }
    fn with_context_runtime<R: Into<Cow<'static, str>>>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.map_err(|e| {
            InterpreterError::RuntimeError(Cow::from(format!("{}: {}", f().into(), e)))
        })
    }
}

impl<T> ErrorContext for Option<T> {
    type Ok = T;
    fn with_context_type<R: Into<Cow<'static, str>>>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.ok_or_else(|| InterpreterError::TypeError(Cow::from(format!("{}", f().into()))))
    }
    fn with_context_runtime<R: Into<Cow<'static, str>>>(
        self,
        f: impl FnOnce() -> R,
    ) -> Result<T, InterpreterError> {
        self.ok_or_else(|| InterpreterError::RuntimeError(Cow::from(format!("{}", f().into()))))
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
