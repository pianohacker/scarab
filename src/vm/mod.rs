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

type Result<T> = std::result::Result<T, Error>;
type IResult<T> = std::result::Result<T, ErrorInternal>;

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    error: ErrorInternal,
    pc: code::Pc,
}

impl Error {
    fn from_internal(error: ErrorInternal, pc: code::Pc) -> Self {
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
    #[error("{source}")]
    Runtime {
        #[from]
        source: code::Error,
    },
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
                Copy { dest, src } => {
                    self.registers[dest] = self.registers[src].clone();
                    Ok(())
                }
                CallInternal {
                    ident,
                    base,
                    num_args,
                } => self.call_internal(ident, base, num_args),
                JumpIf { cond, distance } => {
                    if self.registers[cond] == Value::Boolean(true) {
                        pc = pc.wrapping_add(distance as code::Pc);
                    }

                    Ok(())
                }
            } {
                return Err(Error::from_internal(e, cur_pc));
            }
        }

        Ok(())
    }

    fn call_internal(
        &mut self,
        ident: value::Identifier,
        base: code::RegisterId,
        num_args: code::RegisterOffset,
    ) -> IResult<()> {
        self.registers.push_window_starting(base);

        (builtins::get(&ident)
            .ok_or(ErrorInternal::UnknownInternalFunction(ident.clone()))?
            .run)(self, num_args)?;

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
    use crate::{instructions, value};

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
    fn copy() -> Result<()> {
        snapshot!(
            run_into_registers(instructions! {
                alloc 2;
                load 0 22;
                copy 1 0;
            })?,
            "
[
    Integer(
        22,
    ),
    Integer(
        22,
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn basic_add() -> Result<()> {
        snapshot!(
            run_into_registers(instructions! {
                alloc 2;
                load 0 42;
                load 1 93;
                call + 0 2;
            })?,
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
    fn invalid_add() -> Result<()> {
        assert_err_matches_regex!(
            run_into_registers(instructions! {
                alloc 2;
                load 0 true;
                load 1 "abc";
                call + 0 2;
            }),
            "ExpectedType"
        );

        Ok(())
    }

    #[test]
    fn subtract_and_add() -> Result<()> {
        snapshot!(
            run_into_registers(instructions! {
                alloc 3;
                load 0 22;
                load 1 100;
                load 2 89;
                call - 1 2;
                call + 0 2;
            })?,
            "
[
    Integer(
        33,
    ),
    Integer(
        11,
    ),
    Integer(
        89,
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
                base: 0,
                num_args: 0,
            },]),
            "UnknownInternal"
        );

        Ok(())
    }

    #[test]
    fn debug() -> Result<()> {
        snapshot!(
            run_into_output(instructions! {
                alloc 3;
                load 0 "blah";
                load 1 100;
                load 2 (abc);
                call debug 0 3;
            })?,
            r#"
"blah" 100 (abc)

"#
        );

        Ok(())
    }

    #[test]
    fn jump_if_basic() -> Result<()> {
        snapshot!(
            run_into_registers(instructions! {
                alloc 3;
                load 0 true;
                jump_if 0 1;
                load 1 1;

                load 0 false;
                jump_if 0 1;
                load 2 2;
            })?,
            "
[
    Boolean(
        false,
    ),
    Nil,
    Integer(
        2,
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn jump_if_loop() -> Result<()> {
        snapshot!(
            run_into_registers(instructions! {
                alloc 4;
                load 0 0;
                load 1 1;
                load 3 10;
                call + 0 2;
                copy 2 0;
                call < 2 2;
                jump_if 2 -4;
            })?,
            "
[
    Integer(
        10,
    ),
    Integer(
        1,
    ),
    Boolean(
        false,
    ),
    Integer(
        10,
    ),
]
"
        );

        Ok(())
    }
}
