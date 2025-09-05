use std::{collections::HashMap, env};

use super::builtin_impl;
use crate::interpreter::{InterpreterError, MesonObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Machine {
    system: String,
    cpu_family: String,
    cpu: String,
    endian: String,
}

impl MesonObject for Machine {
    builtin_impl!(system, cpu_family, cpu, endian);
}

impl Machine {
    pub fn new(
        system: impl Into<String>,
        cpu_family: impl Into<String>,
        cpu: impl Into<String>,
        endian: impl Into<String>,
    ) -> Self {
        Self {
            system: system.into(),
            cpu_family: cpu_family.into(),
            cpu: cpu.into(),
            endian: endian.into(),
        }
    }

    fn system(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.system.clone()))
    }

    fn cpu_family(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.cpu_family.clone()))
    }

    fn cpu(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.cpu.clone()))
    }

    fn endian(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.endian.clone()))
    }
}

pub fn host_machine() -> Machine {
    let system = env::consts::OS.to_string();
    let cpu_family = env::consts::ARCH.to_string();
    let cpu = env::consts::ARCH.to_string();
    let endian = if cfg!(target_endian = "big") {
        "big".to_string()
    } else {
        "little".to_string()
    };
    Machine::new(system, cpu_family, cpu, endian)
}

pub fn target_machine() -> Machine {
    let system = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let cpu_family = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let cpu = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let endian = if env::var("CARGO_CFG_TARGET_ENDIAN").unwrap_or_default() == "big" {
        "big".to_string()
    } else {
        "little".to_string()
    };
    Machine::new(system, cpu_family, cpu, endian)
}
