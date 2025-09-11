pub mod array;
pub mod build_target;
pub mod compiler;
pub mod config_data;
pub mod debug;
pub mod dict;
pub mod env;
pub mod external_program;
pub mod files;
pub mod filesystem;
pub mod import;
pub mod include_directories;
pub mod install_headers;
pub mod join_paths;
pub mod machine;
pub mod meson;
pub mod option;
pub mod project;
pub mod run_result;
pub mod string;
pub mod subdir;
pub mod utils;
pub mod variable;
pub mod version;
pub mod add_languages;
pub mod test;

macro_rules! builtin_impl {
    ($($method:ident),* $(,)?) => {
        fn call_method(
            &mut self,
            name: &str,
            _args: alloc::vec::Vec<crate::interpreter::Value>,
            _kwargs: hashbrown::HashMap<alloc::string::String, crate::interpreter::Value>,
            _interp: &mut crate::interpreter::Interpreter,
        ) -> Result<crate::interpreter::Value, crate::interpreter::InterpreterError> {
            match name {
                $(stringify!($method) => self.$method(_args, _kwargs, _interp),)*
                "to_string" => Ok(crate::interpreter::Value::String(self.to_string())),
                _ => crate::interpreter::bail_runtime_error!("Unknown method '{name}' for {} object", core::any::type_name::<Self>()),
            }
        }

        fn clone_rc(&self) -> alloc::rc::Rc<core::cell::RefCell<dyn crate::interpreter::MesonObject>> {
            alloc::rc::Rc::new(core::cell::RefCell::new(self.clone()))
        }

        fn is_equal(&self, other: &alloc::rc::Rc<core::cell::RefCell<dyn crate::interpreter::MesonObject>>) -> bool {
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
