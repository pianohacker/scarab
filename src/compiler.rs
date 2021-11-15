// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::Range;
use std::rc::Rc;
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
    #[error("unknown internal function: {0}")]
    UnknownInternalFunction(value::Identifier),
    #[error("placeholder")]
    Placeholder,
}

type Result<T> = std::result::Result<T, Error>;

struct RegisterAllocator {
    highest_used: code::RegisterId,
    current: RegisterAllocation,
    stack: Vec<RegisterAllocation>,
}

type RegisterAllocation = Range<code::RegisterId>;

impl RegisterAllocator {
    fn new() -> Self {
        Self {
            highest_used: 0,
            current: 0..0,
            stack: Vec::new(),
        }
    }

    fn push_range(&mut self) {
        let start = self.current.end;
        self.stack.push(std::mem::take(&mut self.current));
        self.current = start..start;
    }

    fn pop_range(&mut self) {
        self.current = self.stack.pop().unwrap();
    }

    fn extend_to(&mut self, used: code::RegisterId) {
        assert!(used >= self.current.end);
        self.current.end = used + 1;
    }

    fn current(&self) -> code::RegisterId {
        self.current.end
    }

    fn alloc(&mut self) -> code::RegisterId {
        let register_id = self.current.end;
        self.current.end += 1;

        if register_id > self.highest_used {
            self.highest_used = register_id;
        }

        register_id
    }
}

struct CompilerVisitor<'o, 'a> {
    output: &'o mut Vec<Instruction>,
    allocator: &'a mut RegisterAllocator,
}

impl<'o, 'a> CompilerVisitor<'o, 'a> {
    fn new(output: &'o mut Vec<Instruction>, allocator: &'a mut RegisterAllocator) -> Self {
        Self { output, allocator }
    }

    fn push(&mut self, i: code::Instruction) {
        self.output.push(i);
    }

    fn extend(&mut self, i: impl std::iter::IntoIterator<Item = code::Instruction>) {
        self.output.extend(i);
    }

    fn visit_if(&mut self, args: Vec<Rc<Value>>) -> Result<()> {
        use code::Instruction::*;

        self.allocator.push_range();

        let cond = self.allocator.current();
        self.visit_expr(args[0].clone())?;

        let mut true_output = Vec::new();
        {
            CompilerVisitor::new(&mut true_output, &mut self.allocator)
                .visit_program(args[1].clone())?;
        }

        let mut false_output = Vec::new();
        {
            CompilerVisitor::new(&mut false_output, &mut self.allocator)
                .visit_program(args[2].clone())?;
        }

        self.push(JumpIf {
            cond,
            distance: false_output.len() as code::PcOffset + 2,
        });
        self.extend(false_output);
        let always_cond = self.allocator.current();
        self.visit_expr(Rc::new(Value::Boolean(true)))?;
        self.push(JumpIf {
            cond: always_cond,
            distance: true_output.len() as code::PcOffset,
        });

        self.extend(true_output);

        self.allocator.pop_range();

        Ok(())
    }

    fn visit_call(&mut self, l: Rc<Value>, r: Rc<Value>) -> Result<()> {
        use code::Instruction::*;

        let args: Vec<_> = Value::iter_list_rc(r).collect::<value::Result<Vec<_>>>()?;

        let fn_name = l.try_as_identifier()?;
        match fn_name.as_str() {
            "if" => return self.visit_if(args),
            _ => {}
        }

        builtins::get(fn_name).ok_or(Error::UnknownInternalFunction(fn_name.clone()))?;

        let num_args = args.len() as code::RegisterOffset;

        self.allocator.push_range();
        let base = self.allocator.current();

        for arg in args.into_iter() {
            self.visit_expr(arg)?;
        }

        self.push(CallInternal {
            ident: fn_name.clone(),
            base,
            num_args,
        });

        self.allocator.pop_range();
        self.allocator.extend_to(base);

        Ok(())
    }

    fn visit_expr(&mut self, expr: Rc<Value>) -> Result<()> {
        use code::Instruction::*;

        match &*expr {
            Value::Integer(_) | Value::Boolean(_) => {
                let dest = self.allocator.alloc();

                self.push(LoadImmediate {
                    dest,
                    value: (*expr).clone(),
                });

                Ok(())
            }
            Value::Cell(l, r) => self.visit_call(l.clone(), r.clone()),
            _ => todo!("can't visit value: {}", expr),
        }
    }

    fn visit_program(&mut self, program: Rc<Value>) -> Result<()> {
        for maybe_item in Value::iter_list_rc(program) {
            self.visit_expr(maybe_item?)?;
        }

        Ok(())
    }
}

pub fn compile(program: Rc<Value>) -> Result<Vec<Instruction>> {
    use Instruction::*;

    let mut allocator = RegisterAllocator::new();
    let mut output = Vec::new();

    let num_registers_used = {
        let mut visitor = CompilerVisitor::new(&mut output, &mut allocator);
        visitor.visit_program(program.clone())?;

        visitor.allocator.highest_used as code::RegisterOffset + 1
    };

    output.insert(
        0,
        AllocRegisters {
            count: num_registers_used,
        },
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn compile_display(program: Value) -> Result<String> {
        Ok(compile(Rc::new(program))?
            .into_iter()
            .map(|i| format!("{}", i))
            .collect::<Vec<_>>()
            .join(";\n")
            + ";")
    }

    #[test]
    fn basic_add() -> Result<()> {
        snapshot!(
            compile_display(value!(((+ 1 2 3))))?,
            "
alloc 3;
load 0 1;
load 1 2;
load 2 3;
call + 0 3;
"
        );

        Ok(())
    }

    #[test]
    fn nested_add() -> Result<()> {
        snapshot!(
            compile_display(value!(((+ 1 (+ 2 3)))))?,
            "
alloc 3;
load 0 1;
load 1 2;
load 2 3;
call + 1 2;
call + 0 2;
"
        );

        Ok(())
    }

    #[test]
    fn double_nested_add() -> Result<()> {
        snapshot!(
            compile_display(value!(((+ 1 (+ 2 3) (+ 4 5)))))?,
            "
alloc 4;
load 0 1;
load 1 2;
load 2 3;
call + 1 2;
load 2 4;
load 3 5;
call + 2 2;
call + 0 3;
"
        );

        Ok(())
    }

    #[test]
    fn constant_if() -> Result<()> {
        snapshot!(
            compile_display(value!(((if (< 1 2) ((debug 1)) ((debug 2))))))?,
            "
alloc 4;
load 0 1;
load 1 2;
call < 0 2;
jump_if 0 4 ;
load 2 2;
call debug 2 1;
load 3 true;
jump_if 3 2 ;
load 1 1;
call debug 1 1;
"
        );

        Ok(())
    }

    #[test]
    fn nested_if() -> Result<()> {
        snapshot!(
            compile_display(value!((
                (if (< 1 2)
                    ((if (< 3 2)
                      nil
                      nil
                    ))
                    nil
                )
            )))?,
            "
alloc 3;
load 0 1;
load 1 2;
call < 0 2;
jump_if 0 2 ;
load 1 true;
jump_if 1 6 ;
load 1 3;
load 2 2;
call < 1 2;
jump_if 1 2 ;
load 2 true;
jump_if 2 0 ;
"
        );

        Ok(())
    }
}
