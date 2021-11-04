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
    next_register: code::RegisterId,
    allocations: HashSet<RegisterAllocation>,
}

type RegisterAllocation = Range<code::RegisterId>;

impl RegisterAllocator {
    fn new() -> Self {
        Self {
            next_register: 0,
            allocations: HashSet::new(),
        }
    }

    fn highest_allocation_end(&self) -> code::RegisterId {
        self.allocations.iter().map(|r| r.end).max().unwrap_or(0)
    }

    fn alloc_delta(&self, prev_max: code::RegisterId) -> code::RegisterOffset {
        self.highest_allocation_end() as code::RegisterOffset - prev_max as code::RegisterOffset
    }

    fn alloc_registers(
        &mut self,
        count: code::RegisterOffset,
    ) -> (code::RegisterOffset, RegisterAllocation) {
        let allocation = self.next_register
            ..((self.next_register as code::RegisterOffset + count) as code::RegisterId);

        let prev_max = self.highest_allocation_end();
        self.allocations.insert(allocation.clone());

        (self.alloc_delta(prev_max), allocation)
    }

    fn use_register(&mut self) -> code::RegisterId {
        let register_id = self.next_register;
        self.next_register += 1;
        assert!(register_id < self.highest_allocation_end());

        register_id
    }

    fn drop_allocation(&mut self, allocation: RegisterAllocation) -> code::RegisterOffset {
        let prev_max = self.highest_allocation_end();
        self.allocations.remove(&allocation);

        self.next_register = self.next_register.min(self.highest_allocation_end());

        self.alloc_delta(prev_max)
    }
}

struct CompilerVisitor {
    output: Vec<Instruction>,
    allocator: RegisterAllocator,
}

impl CompilerVisitor {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            allocator: RegisterAllocator::new(),
        }
    }

    fn push_alloc_registers(&mut self, diff: code::RegisterOffset) {
        if diff != 0 {
            self.output
                .push(code::Instruction::AllocRegisters { count: diff });
        }
    }

    fn alloc_registers(&mut self, count: code::RegisterOffset) -> RegisterAllocation {
        let (diff, allocation) = self.allocator.alloc_registers(count);
        self.push_alloc_registers(diff);

        allocation
    }

    fn drop_allocation(&mut self, allocation: RegisterAllocation) {
        let diff = self.allocator.drop_allocation(allocation);
        self.push_alloc_registers(diff);
    }

    fn visit_call(&mut self, l: &Value, r: &Value) -> Result<()> {
        use code::Instruction::*;

        let fn_name = l.try_as_identifier()?;
        builtins::get(fn_name).ok_or(Error::Placeholder)?;

        let args: Vec<_> = r.iter_list().collect::<value::Result<Vec<_>>>()?;
        let num_args = args.len() as code::RegisterOffset;

        let allocation = self.alloc_registers(num_args);

        for arg in args.into_iter() {
            self.visit_expr(arg)?;
        }

        self.output.push(CallInternal {
            ident: fn_name.clone(),
            num_args,
        });

        self.drop_allocation(allocation);

        Ok(())
    }

    fn visit_expr(&mut self, expr: &Value) -> Result<()> {
        use code::Instruction::*;

        match expr {
            Value::Integer(_) => {
                let dest = self.allocator.use_register();

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
call + 3;
alloc -3;
"
        );

        Ok(())
    }

    #[test]
    fn nested_add() -> Result<()> {
        snapshot!(
            compile_display(value!(((+ 1 (+ 2 3)))))?,
            "
alloc 2;
load 0 1;
alloc 1;
load 1 2;
load 2 3;
call + 2;
alloc -1;
call + 2;
alloc -2;
"
        );

        Ok(())
    }

    #[test]
    fn double_nested_add() -> Result<()> {
        snapshot!(
            compile_display(value!(((+ 1 (+ 2 3) (+ 4 5)))))?,
            "
alloc 3;
load 0 1;
load 1 2;
load 2 3;
call + 2;
alloc 2;
load 3 4;
load 4 5;
call + 2;
alloc -2;
call + 3;
alloc -3;
"
        );

        Ok(())
    }
}
