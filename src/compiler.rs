// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use thiserror::Error;

use crate::builtins;
use crate::value;
use crate::value::Value;
use crate::vm::code::{self, Instruction};

#[derive(Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("invalid value in program: {source}")]
    Value {
        #[from]
        source: crate::value::Error,
    },
    #[error("placeholder")]
    Placeholder,
}

type Result<T> = std::result::Result<T, Error>;

struct CompilerVisitor {
    output: Vec<Instruction>,
    next_register: code::RegisterId,
    num_allocated: code::RegisterOffset,
}

impl CompilerVisitor {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            next_register: 0,
            num_allocated: 0,
        }
    }

    fn register(&mut self) -> code::RegisterId {
        let register_id = self.next_register;
        self.next_register += 1;

        register_id
    }

    fn push_dealloc_registers(&mut self, count: code::RegisterOffset) {
        self.next_register =
            ((self.next_register as code::RegisterOffset) - count) as code::RegisterId;

        self.output
            .push(code::Instruction::AllocRegisters { count: -count });
    }

    fn visit_call(&mut self, l: &Value, r: &Value) -> Result<()> {
        use code::Instruction::*;

        let fn_name = l.try_as_identifier()?;
        let builtin = builtins::get(fn_name).ok_or(Error::Placeholder)?;

        let args: Vec<_> = r.iter_list().collect::<value::Result<Vec<_>>>()?;
        let num_args = args.len() as code::RegisterOffset;

        self.output.push(AllocRegisters {
            count: num_args as code::RegisterOffset,
        });

        for (i, arg) in args.into_iter().enumerate() {
            self.visit_expr(arg)?;
        }

        self.output.push(CallInternal {
            ident: fn_name.clone(),
            num_args,
        });

        // self.push_dealloc_registers(num_args - builtin.num_outputs);

        Ok(())
    }

    fn visit_expr(&mut self, expr: &Value) -> Result<()> {
        use code::Instruction::*;

        match expr {
            Value::Integer(_) => {
                let dest = self.register();

                self.output.push(LoadImmediate {
                    dest,
                    value: expr.clone(),
                });

                Ok(())
            }
            Value::Cell(l, r) => self.visit_call(l, r),
            _ => Err(Error::Placeholder),
        }
    }
}

pub fn compile(program: Value) -> Result<Vec<Instruction>> {
    use Instruction::*;

    let mut visitor = CompilerVisitor::new();

    for maybe_item in program.iter_list() {
        visitor.visit_expr(maybe_item?)?;
    }

    Ok(visitor.output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    #[test]
    fn basic_add() -> Result<()> {
        snapshot!(
            compile(value!(((+ 1 2 3))))?,
            r#"
[
    AllocRegisters {
        count: 3,
    },
    LoadImmediate {
        dest: 0,
        value: Integer(
            1,
        ),
    },
    LoadImmediate {
        dest: 1,
        value: Integer(
            2,
        ),
    },
    LoadImmediate {
        dest: 2,
        value: Integer(
            3,
        ),
    },
    CallInternal {
        ident: "+",
        num_args: 3,
    },
]
"#
        );

        Ok(())
    }

    #[test]
    fn nested_add() -> Result<()> {
        snapshot!(
            compile(value!(((+ 1 (+ 2 3)))))?,
            r#"
[
    AllocRegisters {
        count: 2,
    },
    LoadImmediate {
        dest: 0,
        value: Integer(
            1,
        ),
    },
    AllocRegisters {
        count: 2,
    },
    LoadImmediate {
        dest: 1,
        value: Integer(
            2,
        ),
    },
    LoadImmediate {
        dest: 2,
        value: Integer(
            3,
        ),
    },
    CallInternal {
        ident: "+",
        num_args: 2,
    },
    CallInternal {
        ident: "+",
        num_args: 2,
    },
]
"#
        );

        Ok(())
    }

    #[test]
    fn double_nested_add() -> Result<()> {
        snapshot!(
            compile(value!(((+ 1 (+ 2 3) (+ 4 5)))))?,
            r#"
[
    AllocRegisters {
        count: 3,
    },
    LoadImmediate {
        dest: 0,
        value: Integer(
            1,
        ),
    },
    AllocRegisters {
        count: 2,
    },
    LoadImmediate {
        dest: 1,
        value: Integer(
            2,
        ),
    },
    LoadImmediate {
        dest: 2,
        value: Integer(
            3,
        ),
    },
    CallInternal {
        ident: "+",
        num_args: 2,
    },
    AllocRegisters {
        count: 2,
    },
    LoadImmediate {
        dest: 3,
        value: Integer(
            4,
        ),
    },
    LoadImmediate {
        dest: 4,
        value: Integer(
            5,
        ),
    },
    CallInternal {
        ident: "+",
        num_args: 2,
    },
    CallInternal {
        ident: "+",
        num_args: 3,
    },
]
"#
        );

        Ok(())
    }
}
