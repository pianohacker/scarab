// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::value::{Identifier, Value};

pub type RegisterId = u8;
pub type RegisterOffset = i16;

#[derive(Debug)]
pub struct Registers {
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
    pub fn new() -> Self {
        Self {
            values: vec![],
            offset_stack: vec![],
            offset: 0,
        }
    }

    pub fn allocate(&mut self, count: RegisterOffset) {
        self.values.resize_with(
            (self.values.len() as RegisterOffset + count) as usize,
            || Value::Nil,
        );
    }

    pub fn push_window(&mut self, size: RegisterOffset) {
        self.offset_stack.push(self.offset);

        self.offset = (self.values.len() as RegisterOffset - size).max(0) as usize
    }

    pub fn pop_window(&mut self) {
        self.offset = self.offset_stack.pop().unwrap_or(0);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.values[self.offset..].iter()
    }

    pub fn into_values(self) -> Vec<Value> {
        self.values
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Instruction {
    // Allocate or drop`count`registers.
    AllocRegisters {
        count: RegisterOffset,
    },
    // Load a register with the given value.
    LoadImmediate {
        dest: RegisterId,
        value: Value,
    },
    // Call the given function, passing the last `num_args` registers as the registers visible to
    // the function.
    CallInternal {
        ident: Identifier,
        num_args: RegisterOffset,
    },
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Instruction::*;

        match self {
            AllocRegisters { count } => write!(f, "alloc {}", count),
            LoadImmediate { dest, value } => write!(f, "load {} {}", dest, value),
            CallInternal { ident, num_args } => write!(f, "call {} {}", ident, num_args),
        }
    }
}

#[macro_export]
macro_rules! instructions {
    ( @inner ($($accum:tt)*) alloc $count:expr; $($rest:tt)* ) => {
        instructions!(
            @inner
            (
                $($accum)*
                $crate::vm::code::Instruction::AllocRegisters {
                    count: $count,
                },
            )
            $($rest)*
        )
    };
    ( @inner ($($accum:tt)*) load $dest:tt $value:tt; $($rest:tt)* ) => {
        instructions!(
            @inner
            (
                $($accum)*
                $crate::vm::code::Instruction::LoadImmediate {
                    dest: $dest,
                    value: $crate::value!($value),
                },
            )
            $($rest)*
        )
    };
    ( @inner ($($accum:tt)*) call $ident:tt $num_args:expr; $($rest:tt)* ) => {
        instructions!(
            @inner
            (
                $($accum)*
                $crate::vm::code::Instruction::CallInternal {
                    ident: $crate::value::identifier(stringify!($ident)),
                    num_args: $num_args,
                },
            )
            $($rest)*
        )
    };
    ( @inner ($($accum:tt)*) ) => {
        vec![$($accum)*]
    };
    ( $($input:tt)* ) => {
        instructions!( @inner () $($input)* )
    }
}
