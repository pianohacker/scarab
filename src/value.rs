// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::rc::Rc;

use thiserror::Error;

use crate::types::{self, Type, Typeable};

pub type Identifier = String;

pub fn identifier(i: impl Into<String>) -> Identifier {
    i.into()
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("expected {0}, got {1}")]
    ExpectedType(Type, Type),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Nil,
    Boolean(bool),
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

    pub fn try_as_identifier(&self) -> Result<&Identifier> {
        match self {
            Value::Identifier(i) => Ok(i),
            _ => Err(Error::ExpectedType(Type::Identifier, self.type_())),
        }
    }

    pub fn try_as_integer(&self) -> Result<isize> {
        match self {
            Value::Integer(i) => Ok(*i),
            _ => Err(Error::ExpectedType(Type::Integer, self.type_())),
        }
    }

    pub fn try_as_cell(&self) -> Result<(&Value, &Value)> {
        match self {
            Value::Cell(ref l, ref r) => Ok((l, r)),
            _ => Err(Error::ExpectedType(Type::Cell, self.type_())),
        }
    }

    pub fn iter_list(&self) -> impl Iterator<Item = Result<&Self>> {
        let mut current = Some(self);

        std::iter::from_fn(move || match current.take() {
            None => None,
            Some(Value::Nil) => None,
            Some(val) => match val.try_as_cell() {
                Ok((l, r)) => {
                    current = Some(r);
                    Some(Ok(l))
                }
                Err(e) => {
                    current = None;
                    Some(Err(e))
                }
            },
        })
    }

    pub fn try_as_cell_rc(&self) -> Result<(Rc<Value>, Rc<Value>)> {
        match self {
            Value::Cell(l, r) => Ok((l.clone(), r.clone())),
            _ => Err(Error::ExpectedType(Type::Cell, self.type_())),
        }
    }

    pub fn iter_list_rc(cell: Rc<Value>) -> impl Iterator<Item = Result<Rc<Self>>> {
        let mut current = Some(cell);

        std::iter::from_fn(move || match &*current.take()? {
            Value::Nil => None,
            val => match val.try_as_cell_rc() {
                Ok((l, r)) => {
                    current = Some(r);
                    Some(Ok(l))
                }
                Err(e) => {
                    current = None;
                    Some(Err(e))
                }
            },
        })
    }
}

impl types::Typeable for Value {
    fn type_(&self) -> Type {
        match self {
            Value::Nil => Type::Nil,
            Value::Boolean(_) => Type::Boolean,
            Value::Integer(_) => Type::Integer,
            Value::String(_) => Type::String,
            Value::Identifier(_) => Type::Identifier,
            Value::Cell(_, _) => Type::Cell,
            Value::Quoted(_) => Type::Quoted,
        }
    }
}

fn format_cell_contents<'a>(
    mut left: &'a Rc<Value>,
    mut right: &'a Rc<Value>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Identifier(i) => write!(f, "{}", i),
            Value::Integer(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Cell(l, r) => {
                write!(f, "(")?;
                format_cell_contents(l, r, f)?;
                write!(f, ")")
            }
            Value::Quoted(v) => write!(f, "'{}", v),
        }
    }
}

impl std::convert::From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl std::convert::From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl std::convert::From<isize> for Value {
    fn from(i: isize) -> Self {
        Value::Integer(i.into())
    }
}

#[derive(Debug)]
pub struct ContextMap<T>(HashMap<*const Value, T>);

impl<T> ContextMap<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, k: &Rc<Value>, v: T) -> Option<T> {
        self.0.insert(Rc::as_ptr(k), v)
    }

    pub unsafe fn iter(&self) -> impl Iterator<Item = (&Value, &T)> {
        self.0
            .iter()
            .map(|(k, v)| (unsafe { k.as_ref().unwrap() }, v))
    }
}

impl<T> std::ops::Index<&Rc<Value>> for ContextMap<T> {
    type Output = T;

    fn index(&self, i: &Rc<Value>) -> &T {
        &self.0[&Rc::as_ptr(&i)]
    }
}

/// Rough analog of Scarab syntax (though only basic lists). `'` is replaced with `@`.
#[macro_export]
macro_rules! value {
    ((@$quoted_first:tt $($inner:tt)+)) => {
        $crate::value::Value::Cell(
            std::rc::Rc::new($crate::value::Value::Quoted(std::rc::Rc::new(value!($quoted_first)))),
            std::rc::Rc::new(value!(($($inner)+))),
        )
    };

    ((@$quoted_inner:tt)) => {
        $crate::value::Value::Cell(
            std::rc::Rc::new($crate::value::Value::Quoted(std::rc::Rc::new(value!($quoted_inner)))),
            std::rc::Rc::new($crate::value::Value::Nil),
        )
    };

    (($first:tt $($inner:tt)+)) => {
        $crate::value::Value::Cell(
            std::rc::Rc::new(value!($first)),
            std::rc::Rc::new(value!(($($inner)+))),
        )
    };

    (($inner:tt)) => {
        $crate::value::Value::Cell(
            std::rc::Rc::new(value!($inner)),
            std::rc::Rc::new($crate::value::Value::Nil),
        )
    };

    (@$quoted:tt) => {
        $crate::value::Value::Quoted(std::rc::Rc::new(value!($quoted)))
    };

    (nil) => {
        Value::Nil
    };

    (false) => {
        Value::from(false)
    };

    (true) => {
        Value::from(true)
    };

    ($ident:ident) => {
        $crate::value::Value::Identifier($crate::value::identifier(stringify!($ident)))
    };

    (<) => {
        $crate::value::Value::Identifier($crate::value::identifier("<"))
    };

    ($value:expr) => {
        Value::from($value)
    };

    ($tt:tt) => {
        $crate::value::Value::Identifier($crate::value::identifier(stringify!($tt)))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    #[test]
    fn nil_display() {
        snapshot!(format!("{}", Value::Nil), "nil");
    }

    #[test]
    fn nil_macro() {
        assert_eq!(Value::Nil, value!(nil));
    }

    #[test]
    fn boolean_display() {
        snapshot!(format!("{}", Value::Boolean(true)), "true");
    }

    #[test]
    fn boolean_macro() {
        assert_eq!(Value::Boolean(false), value!(false));
        assert_eq!(Value::Boolean(true), value!(true));
    }

    #[test]
    fn string_display() {
        snapshot!(format!("{}", Value::String("abc".to_string())), r#""abc""#);
    }

    #[test]
    fn string_macro() {
        assert_eq!(Value::String("abc".to_string()), value!("abc"));
    }

    #[test]
    fn identifier_display() {
        snapshot!(format!("{}", Value::Identifier("abc".to_string())), "abc");
    }

    #[test]
    fn identifier_macro() {
        assert_eq!(Value::Identifier("abc".to_string()), value!(abc));
        assert_eq!(Value::Identifier("+".to_string()), value!(+));
        assert_eq!(Value::Identifier("<".to_string()), value!(<));
    }

    #[test]
    fn integer_display() {
        snapshot!(format!("{}", Value::Integer(4567)), "4567");
    }

    #[test]
    fn integer_macro() {
        assert_eq!(Value::Integer(4567), value!(4567));
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
    fn quoted_macro() {
        assert_eq!(Value::Quoted(Rc::new(Value::Integer(4567))), value!(@4567),);
        assert_eq!(
            Value::Quoted(Rc::new(Value::Identifier("abc".to_string()))),
            value!(@abc),
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

    #[test]
    fn cell_macro() {
        assert_eq!(
            Value::Cell(Rc::new(Value::Integer(4567)), Rc::new(Value::Nil)),
            value!((4567))
        );

        assert_eq!(
            Value::Cell(
                Rc::new(Value::Integer(123)),
                Rc::new(Value::Cell(
                    Rc::new(Value::Identifier("abc".to_string())),
                    Rc::new(Value::Cell(
                        Rc::new(Value::String("def".to_string())),
                        Rc::new(Value::Nil)
                    ))
                ))
            ),
            value!((123 abc "def"))
        );

        assert_eq!(
            Value::Cell(
                Rc::new(Value::Integer(123)),
                Rc::new(Value::Cell(
                    Rc::new(Value::Cell(
                        Rc::new(Value::Identifier("def".to_string())),
                        Rc::new(Value::Nil)
                    )),
                    Rc::new(Value::Cell(
                        Rc::new(Value::String("def".to_string())),
                        Rc::new(Value::Nil)
                    ))
                ))
            ),
            value!((123 (def) "def"))
        );
    }

    #[test]
    fn quoted_in_cell() {
        assert_eq!(
            Value::Quoted(Rc::new(Value::Cell(
                Rc::new(Value::Integer(123)),
                Rc::new(Value::Cell(
                    Rc::new(Value::Quoted(Rc::new(Value::Cell(
                        Rc::new(Value::Identifier("def".to_string())),
                        Rc::new(Value::Cell(
                            Rc::new(Value::Quoted(Rc::new(Value::Integer(123)))),
                            Rc::new(Value::Nil)
                        ),)
                    )),)),
                    Rc::new(Value::Cell(
                        Rc::new(Value::Cell(
                            Rc::new(Value::Quoted(Rc::new(Value::Integer(123)))),
                            Rc::new(Value::Cell(
                                Rc::new(Value::Cell(
                                    Rc::new(Value::Quoted(Rc::new(Value::String(
                                        "def".to_string()
                                    )))),
                                    Rc::new(Value::Nil)
                                )),
                                Rc::new(Value::Nil)
                            )),
                        )),
                        Rc::new(Value::Nil)
                    ))
                )),
            ))),
            value!(@(123 @(def @123) (@123 (@"def"))))
        );
    }

    #[test]
    fn iter_list_gives_items_for_valid_list() -> Result<()> {
        snapshot!(
            format!(
                "{}",
                value!((1 "a" (2 3)))
                    .iter_list()
                    .map(|x| x.map(|x| format!("{}", x)))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ")
            ),
            r#"1, "a", (2 3)"#
        );

        Ok(())
    }

    #[test]
    fn iter_list_fails_for_non_lists() {
        assert_err_matches_regex!(
            value!(1).iter_list().next().unwrap(),
            "ExpectedType.*Integer"
        );
    }

    #[test]
    fn iter_list_fails_for_invalid_lists() {
        let list = Value::Cell(
            Rc::new(Value::Integer(4)),
            Rc::new(Value::String("a".to_string())),
        );
        let mut iter = list.iter_list();

        assert_eq!(iter.next(), Some(Ok(&Value::Integer(4))));
        assert_err_matches_regex!(iter.next().unwrap(), "ExpectedType.*String");
    }
}
