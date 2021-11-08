// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use phf::phf_map;

use crate::value::{Identifier, Value};
use crate::vm::code;
use crate::vm::Vm;

pub(crate) struct Builtin {
    pub run: &'static (dyn Fn(&mut Vm<'_>, code::RegisterOffset) + Sync),
}

static BUILTINS: phf::Map<&'static str, Builtin> = phf_map! {
    "+" => Builtin {
        run: &|vm, num_args| {
            vm.registers[0] = Value::Integer(
                vm.registers.iter().take(num_args as usize).map(|v| v.as_isize().unwrap()).sum(),
            );
        },
    },
    "-" => Builtin {
        run: &|vm, num_args| {
            vm.registers[0] = Value::Integer(
                vm.registers.iter().take(num_args as usize).map(|v| v.as_isize().unwrap()).reduce(|a, b| a -b).unwrap(),
            );
        },
    },
    "debug" => Builtin {
        run: &|vm, num_args| {
            let output: Vec<_> = vm.registers.iter().take(num_args as usize).map(|v| format!("{}", v)).collect();
            write!(vm.debug_output, "{}\n", output.join(" ")).unwrap();

            vm.registers[0] = Value::Nil;
        }
    },
};

pub(crate) fn get(name: &Identifier) -> Option<&'static Builtin> {
    BUILTINS.get(name)
}
