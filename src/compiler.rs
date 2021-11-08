// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashSet;
use std::ops::Range;
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
        self.highest_used = register_id;

        register_id
    }
}

struct CompilerVisitor<'o> {
    output: Option<&'o mut Vec<Instruction>>,
    allocator: RegisterAllocator,
}

impl<'o> CompilerVisitor<'o> {
    fn new(output: Option<&'o mut Vec<Instruction>>) -> Self {
        Self {
            output,
            allocator: RegisterAllocator::new(),
        }
    }

    fn push(&mut self, i: code::Instruction) {
        if let Some(ref mut output) = self.output {
            output.push(i);
        }
    }

    fn visit_call(&mut self, l: &Value, r: &Value) -> Result<()> {
        use code::Instruction::*;

        let fn_name = l.try_as_identifier()?;
        builtins::get(fn_name).ok_or(Error::Placeholder)?;

        let args: Vec<_> = r.iter_list().collect::<value::Result<Vec<_>>>()?;
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

    fn visit_expr(&mut self, expr: &Value) -> Result<()> {
        use code::Instruction::*;

        match expr {
            Value::Integer(_) => {
                let dest = self.allocator.alloc();

                self.push(LoadImmediate {
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

    let mut register_use_visitor = CompilerVisitor::new(None);

    for maybe_item in program.iter_list() {
        register_use_visitor.visit_expr(maybe_item?)?;
    }

    let mut output = Vec::new();
    output.push(AllocRegisters {
        count: register_use_visitor.allocator.highest_used as code::RegisterOffset + 1,
    });

    let mut visitor = CompilerVisitor::new(Some(&mut output));

    for maybe_item in program.iter_list() {
        visitor.visit_expr(maybe_item?)?;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn compile_display(program: Value) -> Result<String> {
        Ok(compile(program)?
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
}
