// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use result_at::Position;
use thiserror::Error;

use crate::builtins;
use crate::parser;
use crate::types::{self, Typeable};
use crate::value;
use crate::value::Value;
use crate::vm::code::{self, Instruction};

#[derive(Error, Debug, Eq, PartialEq)]
pub enum ErrorInternal {
    #[error("invalid value in program: {source}")]
    Value {
        #[from]
        source: value::Error,
    },
    #[error("type error: {source}")]
    Type {
        #[from]
        source: types::Error,
    },
    #[error("unknown internal function: {0}")]
    UnknownInternalFunction(value::Identifier),
    #[error("placeholder")]
    Placeholder,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Error {
    error: ErrorInternal,
    line: usize,
    column: usize,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "{} (at line {}, column {})",
            self.error, self.line, self.column
        )
    }
}

type Result<T> = std::result::Result<T, Error>;
type IResult<T> = std::result::Result<T, ErrorInternal>;

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

trait Visitor<'p> {
    fn get_positions(&self) -> &'p parser::PositionMap;

    fn label_with_value<T, E: Into<ErrorInternal>>(
        &self,
        r: std::result::Result<T, E>,
        value: &Rc<Value>,
    ) -> Result<T> {
        r.map_err(|e| {
            let (line, column) = self.get_positions()[value];
            Error {
                error: e.into(),
                line,
                column,
            }
        })
    }

    fn label_with_position<T, E: Into<ErrorInternal>>(
        &self,
        r: std::result::Result<T, E>,
        (line, column): Position,
    ) -> Result<T> {
        r.map_err(|e| Error {
            error: e.into(),
            line,
            column,
        })
    }

    fn collect_call<'l>(
        &self,
        l: &'l Rc<Value>,
        r: Rc<Value>,
    ) -> Result<(&'l String, Vec<Rc<Value>>)> {
        Ok((
            self.label_with_value(l.try_as_identifier(), &l)?,
            self.label_with_value(
                Value::iter_list_rc(r).collect::<value::Result<Vec<_>>>(),
                &l,
            )?,
        ))
    }

    fn get_builtin(&self, name: &Rc<Value>) -> Result<&'static builtins::Builtin> {
        let fn_name = self.label_with_value(name.try_as_identifier(), &name)?;

        self.label_with_value(
            builtins::get(fn_name).ok_or(ErrorInternal::UnknownInternalFunction(fn_name.clone())),
            &name,
        )
    }

    fn visit_statement(&mut self, statement: Rc<Value>) -> Result<()>;

    fn visit_program(&mut self, program: Rc<Value>) -> Result<()> {
        for maybe_item in Value::iter_list_rc(program) {
            self.visit_statement(self.label_with_position(maybe_item, (1, 1))?)?;
        }

        Ok(())
    }
}

struct TypeCheckVisitor<'p> {
    variables: HashMap<String, types::Type>,
    positions: &'p parser::PositionMap,
}

impl<'p> Visitor<'p> for TypeCheckVisitor<'p> {
    fn get_positions(&self) -> &'p parser::PositionMap {
        self.positions
    }

    fn visit_statement(&mut self, statement: Rc<Value>) -> Result<()> {
        self.visit_expr(statement.clone())?;

        Ok(())
    }
}

impl<'p> TypeCheckVisitor<'p> {
    fn new(positions: &'p parser::PositionMap) -> Self {
        Self {
            variables: HashMap::new(),
            positions,
        }
    }

    fn visit_set(&mut self, r: Vec<Rc<Value>>) -> Result<types::Type> {
        let name = r[0].try_as_identifier().unwrap();
        let value_type = self.visit_expr(r[1].clone())?;

        self.variables.insert(name.to_string(), value_type);

        Ok(value_type)
    }

    fn visit_call(&mut self, l: Rc<Value>, r: Rc<Value>) -> Result<types::Type> {
        let (fn_name, args) = self.collect_call(&l, r.clone())?;

        let builtin = self.get_builtin(&l)?;

        self.label_with_value(builtin.signature.check_arguments_length(args.len()), &r)?;

        for (position, (arg, arg_spec)) in args
            .iter()
            .zip(builtin.signature.specs_by_position())
            .enumerate()
        {
            let type_ = if arg_spec.is_raw() {
                arg.type_()
            } else {
                self.visit_expr(arg.clone())?
            };

            self.label_with_value(arg_spec.check_at(type_, position), &arg)?;
        }

        match fn_name.as_str() {
            "set" => self.visit_set(args),
            "if" => Ok(types::Type::Nil),
            _ => Ok(builtin.signature.return_type),
        }
    }

    fn visit_expr(&mut self, expr: Rc<Value>) -> Result<types::Type> {
        match &*expr {
            Value::Integer(_) | Value::Boolean(_) | Value::String(_) | Value::Nil => {
                Ok(expr.type_())
            }
            Value::Identifier(i) => Ok(self.variables[i]),
            Value::Cell(l, r) => self.visit_call(l.clone(), r.clone()),
            _ => todo!("can't visit value: {}", expr),
        }
    }
}

struct CompilerVisitor<'o, 'a, 'p> {
    output: &'o mut Vec<Instruction>,
    allocator: &'a mut RegisterAllocator,
    positions: &'p parser::PositionMap,
    variables: HashMap<String, code::RegisterId>,
}

impl<'p> Visitor<'p> for CompilerVisitor<'_, '_, 'p> {
    fn get_positions(&self) -> &'p parser::PositionMap {
        self.positions
    }

    fn visit_statement(&mut self, statement: Rc<Value>) -> Result<()> {
        self.visit_expr(statement)
    }
}

impl<'o, 'a, 'p> CompilerVisitor<'o, 'a, 'p> {
    fn new(
        output: &'o mut Vec<Instruction>,
        allocator: &'a mut RegisterAllocator,
        positions: &'p parser::PositionMap,
    ) -> Self {
        Self {
            output,
            allocator,
            positions,
            variables: HashMap::new(),
        }
    }

    fn push(&mut self, i: code::Instruction) {
        self.output.push(i);
    }

    fn extend(&mut self, i: impl std::iter::IntoIterator<Item = code::Instruction>) {
        self.output.extend(i);
    }

    fn visit_set(&mut self, args: Vec<Rc<Value>>) -> Result<()> {
        let name = args[0].try_as_identifier().unwrap();

        self.variables
            .insert(name.to_string(), self.allocator.current());
        self.visit_expr(args[1].clone())?;

        Ok(())
    }

    fn visit_if(&mut self, args: Vec<Rc<Value>>) -> Result<()> {
        use code::Instruction::*;

        self.allocator.push_range();

        let cond = self.allocator.current();
        self.visit_expr(args[0].clone())?;

        let mut true_output = Vec::new();
        {
            CompilerVisitor::new(&mut true_output, &mut self.allocator, &self.positions)
                .visit_program(args[1].clone())?;
        }

        let mut false_output = Vec::new();
        {
            CompilerVisitor::new(&mut false_output, &mut self.allocator, &self.positions)
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

        let (fn_name, args) = self.collect_call(&l, r)?;

        match fn_name.as_str() {
            "if" => return self.visit_if(args),
            "set" => return self.visit_set(args),
            _ => {}
        }

        self.get_builtin(&l)?;

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
            Value::Integer(_) | Value::Boolean(_) | Value::String(_) | Value::Nil => {
                let dest = self.allocator.alloc();

                self.push(LoadImmediate {
                    dest,
                    value: (*expr).clone(),
                });

                Ok(())
            }
            Value::Identifier(i) => {
                let dest = self.allocator.alloc();

                self.push(Copy {
                    dest,
                    src: self.variables[i],
                });

                Ok(())
            }
            Value::Cell(l, r) => self.visit_call(l.clone(), r.clone()),
            _ => todo!("can't visit value: {}", expr),
        }
    }
}

pub fn compile(program: Rc<Value>, positions: parser::PositionMap) -> Result<Vec<Instruction>> {
    use Instruction::*;

    TypeCheckVisitor::new(&positions).visit_program(program.clone())?;

    let mut allocator = RegisterAllocator::new();
    let mut output = Vec::new();

    let num_registers_used = {
        let mut visitor = CompilerVisitor::new(&mut output, &mut allocator, &positions);
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

    fn compile_display(program: &str) -> Result<String> {
        let (program, positions) = parser::parse_implicit_form_list(program.chars()).unwrap();

        Ok(compile(program, positions)?
            .into_iter()
            .map(|i| format!("{}", i))
            .collect::<Vec<_>>()
            .join(";\n")
            + ";")
    }

    #[test]
    fn basic_add() -> Result<()> {
        snapshot!(
            compile_display("+ 1 2 3")?,
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
    fn basic_mixed() -> Result<()> {
        snapshot!(
            compile_display("debug 1 \"a\" true nil")?,
            r#"
alloc 4;
load 0 1;
load 1 "a";
load 2 true;
load 3 nil;
call debug 0 4;
"#
        );

        Ok(())
    }

    #[test]
    fn unknown_internal_func_fails() -> Result<()> {
        assert_err_matches_regex!(compile_display("-unknown-"), "Unknown.*line.*1.*1");

        Ok(())
    }

    #[test]
    #[ignore]
    fn incorrect_add_fails() -> Result<()> {
        assert_err_matches_regex!(compile_display("+ \"a\" 1"), "InvalidArgument");

        Ok(())
    }

    #[test]
    fn nested_add() -> Result<()> {
        snapshot!(
            compile_display("+ 1 (+ 2 3)")?,
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
            compile_display("+ 1 (+ 2 3) (+ 4 5)")?,
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
            compile_display("if (< 1 2) {debug 1} {debug 2}")?,
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
            compile_display(
                "
                if (< 1 2) {
                    if (< 3 2) nil nil
                } nil
                "
            )?,
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

    #[test]
    fn basic_variables() -> Result<()> {
        snapshot!(compile_display(
            "
                set a 1
                set b 2
                + a b
            "
        )?, "
alloc 4;
load 0 1;
load 1 2;
copy 2 0;
copy 3 1;
call + 2 2;
");

        Ok(())
    }
}
