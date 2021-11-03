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
    pub num_outputs: code::RegisterOffset,
    pub run: &'static (dyn Fn(&mut Vm<'_>) + Sync),
}

static BUILTINS: phf::Map<&'static str, Builtin> = phf_map! {
    "+" => Builtin {
        num_outputs: 1,
        run: &|vm| {
            vm.registers[0] = Value::Integer(
                vm.registers.iter().map(|v| v.as_isize().unwrap()).sum(),
            );
        },
    },
    "-" => Builtin {
        num_outputs: 1,
        run: &|vm| {
            vm.registers[0] = Value::Integer(
                vm.registers.iter().map(|v| v.as_isize().unwrap()).reduce(|a, b| a -b).unwrap(),
            );
        },
    },
    "debug" => Builtin {
        num_outputs: 0,
        run: &|vm| {
            let output: Vec<_> = vm.registers.iter().map(|v| format!("{}", v)).collect();
            write!(vm.debug_output, "{}\n", output.join(" ")).unwrap();
        }
    },
};

pub(crate) fn get(name: &Identifier) -> Option<&'static Builtin> {
    BUILTINS.get(name)
}
