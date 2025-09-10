use alloc::string::String;
use alloc::vec::Vec;

use hashbrown::HashMap;

use super::builtin_impl;
use crate::interpreter::error::ErrorContext;
use crate::interpreter::{Interpreter, InterpreterError, MesonObject, Value};
use crate::os::MachineInfo;

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
    fn system(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.system.clone()))
    }

    fn cpu_family(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.cpu_family.clone()))
    }

    fn cpu(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.cpu.clone()))
    }

    fn endian(
        &self,
        _args: Vec<Value>,
        _kwargs: HashMap<String, Value>,
        _interp: &mut Interpreter,
    ) -> Result<Value, InterpreterError> {
        Ok(Value::String(self.endian.clone()))
    }
}

pub fn build_machine(interp: &Interpreter) -> Result<Machine, InterpreterError> {
    let MachineInfo {
        system,
        cpu,
        endian,
    } = interp
        .os
        .build_machine()
        .context_runtime("Failed to get build machine info")?;

    Ok(Machine {
        system,
        cpu_family: cpu.clone(),
        cpu,
        endian,
    })
}

pub fn host_machine(interp: &Interpreter) -> Result<Machine, InterpreterError> {
    let MachineInfo {
        system,
        cpu,
        endian,
    } = interp
        .os
        .host_machine()
        .context_runtime("Failed to get host machine info")?;

    Ok(Machine {
        system,
        cpu_family: cpu.clone(),
        cpu,
        endian,
    })
}
