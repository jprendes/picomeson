pub mod build_target;
pub mod compiler;
pub mod config_data;
pub mod env;
pub mod external_program;
pub mod files;
pub mod filesystem;
pub mod import;
pub mod include_directories;
pub mod install_headers;
pub mod machine;
pub mod meson;
pub mod run_result;
pub mod utils;
pub mod version;

macro_rules! builtin_impl {
    ($($method:ident),* $(,)?) => {
        fn call_method(
            &mut self,
            name: &str,
            _args: std::vec::Vec<crate::interpreter::Value>,
            _kwargs: std::collections::HashMap<String, crate::interpreter::Value>,
        ) -> Result<crate::interpreter::Value, crate::interpreter::InterpreterError> {
            match name {
                $(stringify!($method) => self.$method(_args, _kwargs),)*
                "to_string" => Ok(crate::interpreter::Value::String(self.to_string())),
                _ => Err(crate::interpreter::InterpreterError::RuntimeError(format!(
                    "Unknown method '{name}' for {} object",
                    core::any::type_name::<Self>()
                ))),
            }
        }

        fn clone_rc(&self) -> std::rc::Rc<std::cell::RefCell<dyn crate::interpreter::MesonObject>> {
            std::rc::Rc::new(std::cell::RefCell::new(self.clone()))
        }

        fn is_equal(&self, other: &std::rc::Rc<std::cell::RefCell<dyn crate::interpreter::MesonObject>>) -> bool {
            use as_any::Downcast;
            if let Some(other) = other.downcast_ref::<Self>() {
                self == other
            } else {
                false
            }
        }
    };
}

pub(crate) use builtin_impl;
