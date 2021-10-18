// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::rc::Rc;

pub type Identifier = String;

pub fn identifier(i: impl Into<String>) -> Identifier {
    i.into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Nil,
    Integer(isize),
    String(String),
    Identifier(Identifier),
    Cell(Rc<Value>, Rc<Value>),
    Quoted(Rc<Value>),
}

impl Value {
    pub fn is_cell(&self) -> bool {
        match self {
            Value::Cell(..) => true,
            _ => false,
        }
    }

    pub fn is_nil(&self) -> bool {
        match self {
            Value::Nil => true,
            _ => false,
        }
    }

    pub fn unwrap_cell(self) -> (Rc<Value>, Rc<Value>) {
        match self {
            Value::Cell(l, r) => (l, r),
            _ => panic!("tried to unwrap {:?} as cell", self),
        }
    }

    pub fn unwrap_cell_ref(&self) -> (&Rc<Value>, &Rc<Value>) {
        match self {
            Value::Cell(l, r) => (l, r),
            _ => panic!("tried to unwrap {:?} as cell", self),
        }
    }

    pub fn as_isize(&self) -> Option<isize> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }
}

fn format_cell_contents<'a>(
    mut left: &'a Rc<Value>,
    mut right: &'a Rc<Value>,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    loop {
        match (&**left, &**right) {
            (lv, &Value::Nil) => break write!(f, "{}", lv),
            (lv, &Value::Cell(ref ilv, ref irv)) => {
                write!(f, "{} ", lv)?;
                left = &ilv;
                right = &irv;
            }
            _ => todo!("left: {:?}, right: {:?}", left, right),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Value::Identifier(i) => write!(f, "{}", i),
            Value::Integer(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Cell(l, r) => {
                write!(f, "(")?;
                format_cell_contents(l, r, f)?;
                write!(f, ")")
            }
            Value::Quoted(v) => write!(f, "'{}", v),
            _ => todo!("{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::snapshot;

    #[test]
    fn string_display() {
        snapshot!(format!("{}", Value::String("abc".to_string())), r#""abc""#);
    }

    #[test]
    fn identifier_display() {
        snapshot!(format!("{}", Value::Identifier("abc".to_string())), "abc");
    }

    #[test]
    fn integer_display() {
        snapshot!(format!("{}", Value::Integer(4567)), "4567");
    }

    #[test]
    fn integer_conversion() {
        assert_eq!(Value::Integer(42).as_isize(), Some(42));
        assert_eq!(Value::Nil.as_isize(), None);
    }

    #[test]
    fn quoted_display() {
        snapshot!(
            format!("{}", Value::Quoted(Rc::new(Value::Integer(4567)))),
            "'4567"
        );
        snapshot!(
            format!(
                "{}",
                Value::Quoted(Rc::new(Value::Identifier("abc".to_string())))
            ),
            "'abc"
        );
    }

    #[test]
    fn cell_display() {
        snapshot!(
            format!(
                "{}",
                Value::Cell(Rc::new(Value::Integer(4567)), Rc::new(Value::Nil))
            ),
            "(4567)"
        );

        snapshot!(
            format!(
                "{}",
                Value::Cell(
                    Rc::new(Value::Integer(123)),
                    Rc::new(Value::Cell(
                        Rc::new(Value::Identifier("abc".to_string())),
                        Rc::new(Value::Cell(
                            Rc::new(Value::String("def".to_string())),
                            Rc::new(Value::Nil)
                        ))
                    ))
                )
            ),
            "(123 abc \"def\")"
        );
    }
}
