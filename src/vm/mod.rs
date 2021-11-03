// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod code;

use std::io;
use thiserror::Error;

use crate::builtins;
use crate::value::{self, Value};

type Pc = usize;

type Result<T> = std::result::Result<T, Error>;
type IResult<T> = std::result::Result<T, ErrorInternal>;

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    error: ErrorInternal,
    pc: Pc,
}

impl Error {
    fn from_internal(error: ErrorInternal, pc: Pc) -> Self {
        Error { error, pc }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{} (at PC 0x{:x})", self.error, self.pc)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
enum ErrorInternal {
    #[error("unknown internal function: {0}")]
    UnknownInternalFunction(value::Identifier),
    #[error("placeholder")]
    Placeholder,
}

pub struct Vm<'a> {
    instructions: Vec<code::Instruction>,
    pub(crate) registers: code::Registers,
    pub(crate) debug_output: &'a mut dyn io::Write,
}

impl<'a> Vm<'a> {
    pub fn new(debug_output: &'a mut impl io::Write) -> Self {
        Self {
            instructions: vec![],
            registers: code::Registers::new(),
            debug_output,
        }
    }

    pub fn load(&mut self, instructions: Vec<code::Instruction>) {
        self.instructions = instructions;
    }

    pub fn run(&mut self) -> Result<()> {
        use code::Instruction::*;

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
                LoadImmediate { dest, value } => {
                    self.registers[dest] = value;
                    Ok(())
                }
                CallInternal { ident, num_args } => self.call_internal(ident, num_args),
            } {
                return Err(Error::from_internal(e, cur_pc));
            }
        }

        Ok(())
    }

    fn call_internal(
        &mut self,
        ident: value::Identifier,
        num_args: code::RegisterOffset,
    ) -> IResult<()> {
        self.registers.push_window(num_args);

        (builtins::get(&ident)
            .ok_or(ErrorInternal::UnknownInternalFunction(ident.clone()))?
            .run)(self);

        self.registers.pop_window();

        Ok(())
    }

    fn into_registers(self) -> Vec<Value> {
        self.registers.into_values()
    }
}

#[cfg(test)]
mod tests {
    use super::code::Instruction as I;
    use super::*;
    use crate::value;

    use k9::{assert_err_matches_regex, snapshot};

    fn run_into_registers(instructions: Vec<code::Instruction>) -> Result<Vec<Value>> {
        let mut debug_output = Vec::new();
        let registers = {
            let mut vm = Vm::new(&mut debug_output);
            vm.load(instructions);
            vm.run()?;
            Ok(vm.into_registers())
        }?;

        if debug_output.len() != 0 {
            dbg!(std::str::from_utf8(&debug_output).unwrap());
        }

        Ok(registers)
    }

    fn run_into_output(instructions: Vec<code::Instruction>) -> Result<String> {
        let mut debug_output = Vec::new();
        {
            let mut vm = Vm::new(&mut debug_output);
            vm.load(instructions);
            vm.run()?;
        }

        Ok(String::from_utf8(debug_output).unwrap())
    }

    #[test]
    fn basic_add() -> Result<()> {
        snapshot!(
            run_into_registers(vec![
                I::AllocRegisters { count: 2 },
                I::LoadImmediate {
                    dest: 0,
                    value: value!(42)
                },
                I::LoadImmediate {
                    dest: 1,
                    value: value!(93)
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
    fn subtract_and_add() -> Result<()> {
        snapshot!(
            run_into_registers(vec![
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
                I::AllocRegisters { count: -1 },
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
    fn unknown_internal_func_fails() -> Result<()> {
        assert_err_matches_regex!(
            run_into_registers(vec![I::CallInternal {
                ident: value::identifier("unknown"),
                num_args: 0,
            },]),
            "UnknownInternal"
        );

        Ok(())
    }

    #[test]
    fn debug() -> Result<()> {
        snapshot!(
            run_into_output(vec![
                I::AllocRegisters { count: 3 },
                I::LoadImmediate {
                    dest: 0,
                    value: value!("blah"),
                },
                I::LoadImmediate {
                    dest: 1,
                    value: value!(100),
                },
                I::LoadImmediate {
                    dest: 2,
                    value: value!((abc)),
                },
                I::CallInternal {
                    ident: value::identifier("debug"),
                    num_args: 3,
                },
            ])?,
            r#"
"blah" 100 (abc)

"#
        );

        Ok(())
    }
}
