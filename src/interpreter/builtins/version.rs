use std::collections::HashMap;

use super::builtin_impl;
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    version: semver::Version,
}

impl Version {
    pub fn version_compare(
        &self,
        args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        let Some(Value::String(req)) = args.first() else {
            return Err(InterpreterError::TypeError(
                "Expected a string as the first argument".into(),
            ));
        };

        let req = semver::VersionReq::parse(req).map_err(|e| {
            InterpreterError::RuntimeError(format!(
                "Invalid version requirement string '{req}': {e}"
            ))
        })?;

        Ok(Value::Boolean(req.matches(&self.version)))
    }
}

impl MesonObject for Version {
    builtin_impl!(version_compare);
}

pub fn version(version: impl AsRef<str>) -> Result<Value, InterpreterError> {
    let version = version.as_ref();
    let version = semver::Version::parse(version).map_err(|e| {
        InterpreterError::RuntimeError(format!("Invalid version string '{version}': {e}",))
    })?;
    Ok(Version { version }.into_object())
}
