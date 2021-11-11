// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use phf::phf_map;

use crate::value::{self, Identifier, Value};
use crate::vm::code;
use crate::vm::Vm;

pub(crate) struct Builtin {
    pub run: &'static (dyn Fn(&mut Vm<'_>, code::RegisterOffset) -> Result<(), code::Error> + Sync),
}

fn iter_as_integers(
    registers: &code::Registers,
    num_args: code::RegisterOffset,
) -> Result<impl Iterator<Item = isize>, value::Error> {
    Ok(registers
        .iter()
        .take(num_args as usize)
        .map(|v| v.try_as_integer())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter())
}

static BUILTINS: phf::Map<&'static str, Builtin> = phf_map! {
    "+" => Builtin {
        run: &|vm, num_args| {
            vm.registers[0] = Value::Integer(
                iter_as_integers(&vm.registers, num_args)?.sum(),
            );

            Ok(())
        },
    },
    "-" => Builtin {
        run: &|vm, num_args| {
            vm.registers[0] = Value::Integer(
                iter_as_integers(&vm.registers, num_args)?.reduce(|a, b| a -b).unwrap(),
            );

            Ok(())
        },
    },
    "<" => Builtin {
        run: &|vm, num_args| {
            vm.registers[0] = Value::Boolean(
                vm.registers[0].try_as_integer()? < vm.registers[1].try_as_integer()?
            );

            Ok(())
        },
    },
    "debug" => Builtin {
        run: &|vm, num_args| {
            let output: Vec<_> = vm.registers.iter().take(num_args as usize).map(|v| format!("{}", v)).collect();
            write!(vm.debug_output, "{}\n", output.join(" ")).unwrap();

            vm.registers[0] = Value::Nil;

            Ok(())
        }
    },
};

pub(crate) fn get(name: &Identifier) -> Option<&'static Builtin> {
    BUILTINS.get(name)
}
