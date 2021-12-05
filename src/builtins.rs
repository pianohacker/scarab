// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use lazy_static::lazy_static;

use crate::types::{ArgumentSpec, Signature, Type, TypeSpec};
use crate::value::{self, Identifier, Value};
use crate::vm::code;
use crate::vm::Vm;

pub(crate) struct Builtin {
    pub signature: Signature,
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

lazy_static! {
    static ref BUILTINS: std::collections::HashMap<&'static str, Builtin> = {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "+",
            Builtin {
                signature: Signature::new()
                    .add_rest(Type::Integer)
                    .return_type(Type::Integer)
                    .build(),
                run: &|vm, num_args| {
                    vm.registers[0] =
                        Value::Integer(iter_as_integers(&vm.registers, num_args)?.sum());

                    Ok(())
                },
            },
        );
        map.insert(
            "-",
            Builtin {
                signature: Signature::new()
                    .add_rest(Type::Integer)
                    .return_type(Type::Integer)
                    .build(),
                run: &|vm, num_args| {
                    vm.registers[0] = Value::Integer(
                        iter_as_integers(&vm.registers, num_args)?
                            .reduce(|a, b| a - b)
                            .unwrap_or(0),
                    );

                    Ok(())
                },
            },
        );
        map.insert(
            "<",
            Builtin {
                signature: Signature::new()
                    .add(Type::Integer)
                    .add(Type::Integer)
                    .return_type(Type::Boolean)
                    .build(),
                run: &|vm, _| {
                    vm.registers[0] = Value::Boolean(
                        vm.registers[0].try_as_integer()? < vm.registers[1].try_as_integer()?,
                    );

                    Ok(())
                },
            },
        );
        map.insert(
            "debug",
            Builtin {
                signature: Signature::new()
                    .add_rest(TypeSpec::Any)
                    .return_type(Type::Nil)
                    .build(),
                run: &|vm, num_args| {
                    let output: Vec<_> = vm
                        .registers
                        .iter()
                        .take(num_args as usize)
                        .map(|v| format!("{}", v))
                        .collect();
                    write!(vm.debug_output, "{}\n", output.join(" ")).unwrap();

                    vm.registers[0] = Value::Nil;

                    Ok(())
                },
            },
        );
        map.insert(
            "if",
            Builtin {
                signature: Signature::new()
                    .add(Type::Boolean)
                    .add(ArgumentSpec::new(TypeSpec::List).raw(true))
                    .add(ArgumentSpec::new(TypeSpec::List).raw(true))
                    .return_type(Type::Nil)
                    .build(),
                run: &|_, _| unreachable!(),
            },
        );
        map.insert(
            "set",
            Builtin {
                signature: Signature::new()
                    .add(ArgumentSpec::new(Type::Identifier).raw(true))
                    .add(TypeSpec::Any)
                    .return_type(Type::Nil)
                    .build(),
                run: &|_, _| unreachable!(),
            },
        );

        map
    };
}

pub(crate) fn get(name: &Identifier) -> Option<&'static Builtin> {
    BUILTINS.get(name.as_str())
}
