use hashbrown::HashMap;

use super::builtin_impl;
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

pub fn host_machine(interp: &Interpreter) -> Machine {
    let MachineInfo {
        system,
        cpu,
        endian,
    } = interp.os_env.host();

    Machine {
        system,
        cpu_family: cpu.clone(),
        cpu,
        endian,
    }
}

pub fn target_machine(interp: &Interpreter) -> Machine {
    let MachineInfo {
        system,
        cpu,
        endian,
    } = interp.os_env.target();

    Machine {
        system,
        cpu_family: cpu.clone(),
        cpu,
        endian,
    }
}
