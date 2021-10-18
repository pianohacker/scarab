// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::value::{self, Value};
use thiserror::Error;

type Pc = usize;

type VResult<T> = Result<T, VmError>;
type VIResult<T> = Result<T, VmErrorInternal>;

#[derive(Debug, PartialEq, Eq)]
pub struct VmError {
    error: VmErrorInternal,
    pc: Pc,
}

impl VmError {
    fn from_internal(error: VmErrorInternal, pc: Pc) -> Self {
        VmError { error, pc }
    }
}

impl std::error::Error for VmError {}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} (at PC 0x{:x})", self.error, self.pc)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
enum VmErrorInternal {
    #[error("unknown internal function: {0}")]
    UnknownInternalFunction(value::Identifier),
    #[error("placeholder")]
    Placeholder,
}

type RegisterId = u8;
type RegisterOffset = u8;
type RegisterOffsetIntermediate = i16;

#[derive(Debug)]
struct Registers {
    values: Vec<Value>,
    offset_stack: Vec<usize>,
    offset: usize,
}

impl std::ops::Index<RegisterId> for Registers {
    type Output = Value;

    fn index(&self, index: u8) -> &Value {
        &self.values[self.offset + index as usize]
    }
}

impl std::ops::IndexMut<RegisterId> for Registers {
    fn index_mut(&mut self, index: u8) -> &mut Value {
        &mut self.values[self.offset + index as usize]
    }
}

impl Registers {
    fn new() -> Self {
        Self {
            values: vec![],
            offset_stack: vec![],
            offset: 0,
        }
    }

    fn allocate(&mut self, count: usize) {
        self.values
            .resize_with(self.values.len() + count, || Value::Nil);
    }

    fn deallocate(&mut self, count: usize) {
        self.values
            .resize_with(self.values.len() - count, || Value::Nil);
    }

    fn push_window(&mut self, size: RegisterOffset) {
        self.offset_stack.push(self.offset);

        self.offset = (self.values.len() as RegisterOffsetIntermediate
            - size as RegisterOffsetIntermediate)
            .max(0) as usize
    }

    fn pop_window(&mut self) {
        self.offset = self.offset_stack.pop().unwrap_or(0);
    }

    fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.values[self.offset..].iter()
    }

    fn into_values(self) -> Vec<Value> {
        self.values
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Instruction {
    // Allocate `count` more registers.
    AllocRegisters {
        count: usize,
    },
    // Drop `count` registers.
    DeallocRegisters {
        count: usize,
    },
    // Load a register with the given value.
    LoadImmediate {
        dest: RegisterId,
        value: Value,
    },
    // Call the given function, passing the last `num_args` registers as the registers visible to
    // the function.
    CallInternal {
        ident: value::Identifier,
        num_args: RegisterOffset,
    },
}

struct Vm {
    instructions: Vec<Instruction>,
    registers: Registers,
}

impl Vm {
    fn new() -> Self {
        Self {
            instructions: vec![],
            registers: Registers::new(),
        }
    }

    fn load(&mut self, instructions: Vec<Instruction>) {
        self.instructions = instructions;
    }

    fn run(&mut self) -> VResult<()> {
        use Instruction::*;

        let mut pc = 0;
        while pc < self.instructions.len() {
            let instruction = self.instructions[pc].clone();
            let cur_pc = pc;
            pc += 1;

            if let Err(e) = match instruction {
                AllocRegisters { count } => {
                    self.registers.allocate(count);
                    Ok(())
                }
                DeallocRegisters { count } => {
                    self.registers.deallocate(count);
                    Ok(())
                }
                LoadImmediate { dest, value } => {
                    self.registers[dest] = value;
                    Ok(())
                }
                CallInternal { ident, num_args } => self.call_internal(ident, num_args),
            } {
                return Err(VmError::from_internal(e, cur_pc));
            }
        }

        Ok(())
    }

    fn call_internal(
        &mut self,
        ident: value::Identifier,
        num_args: RegisterOffset,
    ) -> VIResult<()> {
        self.registers.push_window(num_args);

        let result = match ident.as_str() {
            "+" => self.registers.iter().map(|v| v.as_isize().unwrap()).sum(),
            "-" => self
                .registers
                .iter()
                .map(|v| v.as_isize().unwrap())
                .reduce(|a, b| a - b)
                .unwrap(),
            _ => return Err(VmErrorInternal::UnknownInternalFunction(ident)),
        };

        self.registers[0] = Value::Integer(result);

        self.registers.pop_window();

        Ok(())
    }

    fn into_registers(self) -> Vec<Value> {
        self.registers.into_values()
    }
}

#[cfg(test)]
mod tests {
    use super::Instruction as I;
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn run(instructions: Vec<Instruction>) -> VResult<Vec<Value>> {
        let mut vm = Vm::new();
        vm.load(instructions);
        vm.run()?;
        Ok(vm.into_registers())
    }

    #[test]
    fn basic_add() -> VResult<()> {
        snapshot!(
            run(vec![
                I::AllocRegisters { count: 2 },
                I::LoadImmediate {
                    dest: 0,
                    value: Value::Integer(42)
                },
                I::LoadImmediate {
                    dest: 1,
                    value: Value::Integer(93)
                },
                I::CallInternal {
                    ident: value::identifier("+"),
                    num_args: 2,
                },
            ])?,
            "
[
    Integer(
        135,
    ),
    Integer(
        93,
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn subtract_and_add() -> VResult<()> {
        snapshot!(
            run(vec![
                I::AllocRegisters { count: 3 },
                I::LoadImmediate {
                    dest: 0,
                    value: Value::Integer(22)
                },
                I::LoadImmediate {
                    dest: 1,
                    value: Value::Integer(100)
                },
                I::LoadImmediate {
                    dest: 2,
                    value: Value::Integer(89)
                },
                I::CallInternal {
                    ident: value::identifier("-"),
                    num_args: 2,
                },
                I::DeallocRegisters { count: 1 },
                I::CallInternal {
                    ident: value::identifier("+"),
                    num_args: 2,
                },
            ])?,
            "
[
    Integer(
        33,
    ),
    Integer(
        11,
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn unknown_internal_func_fails() -> VResult<()> {
        assert_err_matches_regex!(
            run(vec![I::CallInternal {
                ident: value::identifier("unknown"),
                num_args: 0,
            },]),
            "UnknownInternal"
        );

        Ok(())
    }
}
