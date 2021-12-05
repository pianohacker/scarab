// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use bitflags::bitflags;
use thiserror::Error;

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("expected {expected}, got {actual}")]
    ExpectedType { expected: Type, actual: Type },

    #[error("argument {position} invalid: {source}")]
    InvalidArgument { position: usize, source: Box<Error> },

    #[error("too many arguments; expected less than {expected}, got {actual}")]
    TooManyArguments { expected: usize, actual: usize },

    #[error("not enough arguments; expected at least {expected}, got {actual}")]
    NotEnoughArguments { expected: usize, actual: usize },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Nil,
    Boolean,
    Integer,
    String,
    Identifier,
    Cell,
    Quoted,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Type::Nil => "nil",
                Type::Boolean => "boolean",
                Type::Integer => "integer",
                Type::String => "string",
                Type::Identifier => "identifier",
                Type::Cell => "cell",
                Type::Quoted => "quoted",
            }
        )
    }
}

pub trait Typeable {
    fn type_(&self) -> Type;
}

impl Typeable for Type {
    fn type_(&self) -> Type {
        *self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TypeSpec {
    Any,
    Base(Type),
    List,
}

impl TypeSpec {
    fn check(&self, actual: Type) -> Result<()> {
        use TypeSpec::*;

        match *self {
            Any => Ok(()),
            Base(expected) => {
                if actual == expected {
                    Ok(())
                } else {
                    Err(Error::ExpectedType { expected, actual })
                }
            }
            List => {
                if actual == Type::Nil || actual == Type::Cell {
                    Ok(())
                } else {
                    Err(Error::ExpectedType {
                        // TODO: Improve error
                        expected: Type::Cell,
                        actual,
                    })
                }
            }
        }
    }
}

impl From<Type> for TypeSpec {
    fn from(t: Type) -> TypeSpec {
        TypeSpec::Base(t)
    }
}

#[derive(Debug)]
pub struct ArgumentSpec {
    attributes: ArgumentAttributes,
    type_spec: TypeSpec,
}

impl ArgumentSpec {
    pub fn new(type_spec: impl Into<TypeSpec>) -> ArgumentSpecBuilder {
        ArgumentSpecBuilder(Self {
            type_spec: type_spec.into(),
            attributes: ArgumentAttributes::empty(),
        })
    }

    pub fn check_at(&self, type_: Type, position: usize) -> Result<()> {
        self.type_spec
            .check(type_)
            .map_err(|e| Error::InvalidArgument {
                position,
                source: Box::new(e),
            })
    }

    pub fn is_raw(&self) -> bool {
        self.attributes.contains(ArgumentAttributes::RAW)
    }
}

bitflags! {
    pub struct ArgumentAttributes: u8 {
        const RAW = 1 << 0;
    }
}

#[derive(Debug)]
pub struct ArgumentSpecBuilder(ArgumentSpec);

impl ArgumentSpecBuilder {
    pub fn raw(mut self, raw: bool) -> Self {
        self.0.attributes.set(ArgumentAttributes::RAW, raw);

        self
    }
}

impl std::convert::From<ArgumentSpecBuilder> for ArgumentSpec {
    fn from(b: ArgumentSpecBuilder) -> Self {
        b.0
    }
}

impl<T: Into<TypeSpec>> std::convert::From<T> for ArgumentSpec {
    fn from(t: T) -> Self {
        ArgumentSpec::new(t).into()
    }
}

#[derive(Debug)]
pub struct Signature {
    pub return_type: Type,
    argument_specs: Vec<ArgumentSpec>,
    rest_argument_spec: Option<ArgumentSpec>,
}

impl Signature {
    pub fn new() -> SignatureBuilder {
        SignatureBuilder(Self {
            return_type: Type::Nil,
            argument_specs: Vec::new(),
            rest_argument_spec: None,
        })
    }

    pub fn check_arguments_length(&self, actual: usize) -> Result<()> {
        let expected = self.argument_specs.len();

        if actual < expected {
            Err(Error::NotEnoughArguments { expected, actual })
        } else if actual > expected && self.rest_argument_spec.is_none() {
            Err(Error::TooManyArguments { expected, actual })
        } else {
            Ok(())
        }
    }

    pub fn specs_by_position(&self) -> impl Iterator<Item = &ArgumentSpec> + '_ {
        let mut arg_specs = self.argument_specs.iter();
        let mut arg_spec = arg_specs.next();

        std::iter::from_fn(move || match arg_spec {
            None => match &self.rest_argument_spec {
                None => None,
                Some(spec) => Some(spec),
            },
            Some(spec) => {
                arg_spec = arg_specs.next();

                Some(spec)
            }
        })
    }
}

#[must_use]
pub struct SignatureBuilder(Signature);

impl SignatureBuilder {
    pub fn build(self) -> Signature {
        self.0
    }

    pub fn return_type(self, return_type: Type) -> Self {
        Self(Signature {
            return_type,
            ..self.0
        })
    }

    pub fn add(mut self, argument_spec: impl Into<ArgumentSpec>) -> Self {
        self.0.argument_specs.push(argument_spec.into());

        self
    }

    pub fn add_rest(mut self, argument_spec: impl Into<ArgumentSpec>) -> Self {
        self.0.rest_argument_spec = Some(argument_spec.into());

        self
    }
}

impl std::convert::From<SignatureBuilder> for Signature {
    fn from(b: SignatureBuilder) -> Self {
        b.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use k9::assert_err_matches_regex;

    fn check_args(signature: &Signature, args: Vec<Type>) -> Result<()> {
        signature.check_arguments_length(args.len())?;

        signature
            .specs_by_position()
            .zip(args.into_iter())
            .enumerate()
            .map(|(i, (arg_spec, type_))| arg_spec.check_at(type_, i))
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    #[test]
    fn any_takes_any_type() -> Result<()> {
        for type_ in vec![Type::Nil, Type::Boolean, Type::Integer, Type::String] {
            TypeSpec::Any.check(type_)?;
        }

        Ok(())
    }

    #[test]
    fn specific_type_takes_only_that_type() -> Result<()> {
        let spec = TypeSpec::Base(Type::Boolean);

        spec.check(Type::Boolean)?;
        assert_err_matches_regex!(spec.check(Type::Integer), "ExpectedType.*Boolean.*Integer");

        Ok(())
    }

    #[test]
    fn list_takes_cell_or_nil() -> Result<()> {
        let spec = TypeSpec::List;

        spec.check(Type::Nil)?;
        spec.check(Type::Cell)?;
        assert_err_matches_regex!(spec.check(Type::Integer), "ExpectedType.*Cell.*Integer");

        Ok(())
    }

    #[test]
    fn function_taking_any_takes_any_type() -> Result<()> {
        let signature = Signature::new().add_rest(TypeSpec::Any).build();

        check_args(
            &signature,
            vec![Type::Nil, Type::Boolean, Type::Integer, Type::String],
        )?;

        Ok(())
    }

    #[test]
    fn function_taking_type_takes_only_that_type() -> Result<()> {
        let signature = Signature::new().add_rest(Type::Integer).build();

        check_args(&signature, vec![Type::Integer])?;
        assert_err_matches_regex!(
            check_args(&signature, vec![Type::String]),
            "InvalidArgument.*0.*Integer.*String"
        );

        Ok(())
    }

    #[test]
    fn function_taking_fixed_and_rest_arguments_rejects_less() -> Result<()> {
        let signature = Signature::new()
            .add(TypeSpec::Any)
            .add_rest(TypeSpec::Any)
            .build();

        check_args(&signature, vec![Type::Integer])?;
        check_args(&signature, vec![Type::Integer, Type::Integer])?;
        assert_err_matches_regex!(check_args(&signature, vec![]), "NotEnoughArguments.*1.*0");

        Ok(())
    }

    #[test]
    fn function_taking_fixed_arguments_rejects_more_or_less() -> Result<()> {
        let signature = Signature::new()
            .add(TypeSpec::Any)
            .add(TypeSpec::Any)
            .build();

        check_args(&signature, vec![Type::Integer, Type::Integer])?;
        assert_err_matches_regex!(
            check_args(
                &signature,
                vec![Type::Integer, Type::Integer, Type::Integer]
            ),
            "TooManyArguments.*2.*3"
        );
        assert_err_matches_regex!(
            check_args(&signature, vec![Type::Integer]),
            "NotEnoughArguments.*2.*1"
        );

        Ok(())
    }

    #[test]
    fn function_taking_infinite_arguments_accepts_any() -> Result<()> {
        let signature = Signature::new().add_rest(TypeSpec::Any).build();

        check_args(&signature, Vec::new())?;
        check_args(&signature, vec![Type::Integer, Type::Integer])?;

        Ok(())
    }

    #[test]
    fn function_taking_mixed_arguments_enforces_types() -> Result<()> {
        let signature = Signature::new()
            .add(Type::Integer)
            .add(Type::Boolean)
            .build();

        check_args(&signature, vec![Type::Integer, Type::Boolean])?;

        assert_err_matches_regex!(
            check_args(&signature, vec![Type::String, Type::Boolean]),
            "InvalidArgument.*0.*Integer.*String"
        );
        assert_err_matches_regex!(
            check_args(&signature, vec![Type::Integer, Type::String]),
            "InvalidArgument.*1.*Boolean.*String"
        );

        Ok(())
    }
}
