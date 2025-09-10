use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    version: semver::Version,
}

impl Version {
    pub fn version_compare(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(req)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string as the first argument".into(),
            ));
        };

        let req = semver::VersionReq::parse(req)
            .with_context_runtime(|| format!("Invalid version requirement string '{req}'"))?;

        Ok(Value::Boolean(req.matches(&self.version)))
    }
}

impl MesonObject for Version {
    builtin_impl!(version_compare);
}

pub fn version(version: impl AsRef<str>) -> Result<Value, InterpreterError> {
    let version = version.as_ref();
    let version = semver::Version::parse(version)
        .with_context_runtime(|| format!("Invalid version string '{version}'"))?;
    Ok(Version { version }.into_object())
}
