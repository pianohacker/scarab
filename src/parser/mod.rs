// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod tokenizer;
mod utils;

use std::rc::Rc;
use thiserror::Error;

use self::tokenizer::{tokenize, Token, Tokenizer};
use self::utils::{PositionLabeled, TakeWhileUngreedyExt};
use crate::value::Value;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("unexpected end of input")]
    EOF,
    #[error("mismatched operator list; operator {1} does not match initial operator {0}")]
    MismatchedOperatorList(String, String),
    #[error("{source}")]
    Tokenize {
        #[from]
        source: self::tokenizer::Error,
    },
    #[error("unexpected token")]
    UnexpectedToken(PositionLabeled<Token>),
    #[error("unterminated list")]
    UnterminatedList,
    #[error("placeholder")]
    Placeholder,
}

type Result<T> = std::result::Result<T, Error>;

pub struct Parser<I: Iterator<Item = char>> {
    input: std::iter::Peekable<Tokenizer<I>>,
}

macro_rules! expect_match {
    ($self:expr$(, $pat:pat => $body:expr)+$(,)?) => {
        {
            let instance = $self.expect_next()?;
            match instance.contents {
                $($pat => $body,)+
                _ => return Err(Error::UnexpectedToken(instance)),
            }
        }
    }
}

impl<I: Iterator<Item = char>> Parser<I> {
    fn elements_to_list(elements: Vec<Value>) -> Value {
        elements.into_iter().rev().fold(Value::Nil, |accum, elem| {
            Value::Cell(Rc::new(elem), Rc::new(accum))
        })
    }

    fn expect_next(&mut self) -> Result<PositionLabeled<Token>> {
        self.input
            .next()
            .ok_or(Error::EOF)?
            .map_err(|e| Error::from(e.clone()))
    }

    fn expect_peek_or(&mut self, error: Error) -> Result<&PositionLabeled<Token>> {
        self.input
            .peek()
            .ok_or(error)?
            .as_ref()
            .map_err(|e| Error::from(e.clone()))
    }

    fn parse_list(&mut self, terminator_predicate: impl Fn(&Token) -> bool) -> Result<Value> {
        let mut elements = vec![];

        while !terminator_predicate(&self.expect_peek_or(Error::UnterminatedList)?.contents) {
            elements.push(self.parse_value()?);
        }

        assert!(terminator_predicate(
            &self.expect_peek_or(Error::UnterminatedList)?.contents
        ));

        Ok(Self::elements_to_list(elements))
    }

    fn parse_operator_list(&mut self) -> Result<Value> {
        let first = self.parse_value()?;

        let operator = expect_match! { self,
            Token::Identifier(i) => i,
        };

        let second = self.parse_value()?;

        let mut elements = vec![Value::Identifier(operator.clone()), first, second];

        while self.expect_peek_or(Error::UnterminatedList)?.contents != Token::RBracket {
            let next_operator = expect_match! { self,
                Token::Identifier(i) => i,
            };

            if next_operator != operator {
                return Err(Error::MismatchedOperatorList(operator, next_operator));
            }

            elements.push(self.parse_value()?);
        }

        expect_match! { self,
            Token::RBracket => {},
        };

        Ok(Self::elements_to_list(elements))
    }

    fn parse_form_list(&mut self) -> Result<Value> {
        let mut lists = vec![];

        while self.expect_peek_or(Error::UnterminatedList)?.contents != Token::RBrace {
            let list = self.parse_list(|t| {
                *t == Token::Comma || *t == Token::Newline || *t == Token::RBrace
            })?;

            if list != Value::Nil {
                lists.push(list);
            }

            self.input
                .take_while_ungreedy(|t| {
                    t.as_ref().map_or(false, |t| {
                        t.contents == Token::Comma || t.contents == Token::Newline
                    })
                })
                .for_each(drop);
        }

        expect_match! { self,
            Token::RBrace => {},
        };

        Ok(Self::elements_to_list(lists))
    }

    pub fn parse_value(&mut self) -> Result<Value> {
        expect_match! { self,
            Token::Integer(i) => Ok(Value::Integer(i)),
            Token::String(s) => Ok(Value::String(s)),
            Token::Identifier(i) => Ok(Value::Identifier(i)),
            Token::LParen => {
                let result = self.parse_list(|t| *t == Token::RParen)?;
                self.expect_next()?;
                Ok(result)
            },
            Token::LBracket => self.parse_operator_list(),
            Token::LBrace => self.parse_form_list(),
            Token::Quote => Ok(Value::Quoted(Rc::new(self.parse_value()?))),
        }
    }
}

pub fn parse_value<I>(input: I) -> Result<Value>
where
    I: IntoIterator<Item = char>,
{
    Parser {
        input: tokenize(input.into_iter()).peekable(),
    }
    .parse_value()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn try_parse(input: &str) -> Result<Value> {
        parse_value(input.chars())
    }

    #[test]
    fn empty_parse_fails() -> Result<()> {
        assert_err_matches_regex!(try_parse(""), "EOF");

        Ok(())
    }

    #[test]
    fn unexpected_tokens_fail() -> Result<()> {
        assert_err_matches_regex!(try_parse(")"), "Unexpected.*RParen");

        Ok(())
    }

    #[test]
    fn single_values() -> Result<()> {
        snapshot!(
            try_parse("123"),
            "
Ok(
    Integer(
        123,
    ),
)
"
        );
        snapshot!(
            try_parse("\"abc\""),
            r#"
Ok(
    String(
        "abc",
    ),
)
"#
        );
        snapshot!(
            try_parse("blah"),
            r#"
Ok(
    Identifier(
        "blah",
    ),
)
"#
        );

        Ok(())
    }

    #[test]
    fn quoted_values() -> Result<()> {
        snapshot!(
            try_parse("'abc"),
            r#"
Ok(
    Quoted(
        Identifier(
            "abc",
        ),
    ),
)
"#
        );

        snapshot!(
            try_parse("''123"),
            "
Ok(
    Quoted(
        Quoted(
            Integer(
                123,
            ),
        ),
    ),
)
"
        );

        snapshot!(
            try_parse("'(1 2 3)"),
            "
Ok(
    Quoted(
        Cell(
            Integer(
                1,
            ),
            Cell(
                Integer(
                    2,
                ),
                Cell(
                    Integer(
                        3,
                    ),
                    Nil,
                ),
            ),
        ),
    ),
)
"
        );

        Ok(())
    }

    #[test]
    fn simple_list() -> Result<()> {
        snapshot!(
            try_parse("(+ 123 456)"),
            r#"
Ok(
    Cell(
        Identifier(
            "+",
        ),
        Cell(
            Integer(
                123,
            ),
            Cell(
                Integer(
                    456,
                ),
                Nil,
            ),
        ),
    ),
)
"#
        );

        Ok(())
    }

    #[test]
    fn nested_list() -> Result<()> {
        snapshot!(
            try_parse("(+ ((-) 123) 456)"),
            r#"
Ok(
    Cell(
        Identifier(
            "+",
        ),
        Cell(
            Cell(
                Cell(
                    Identifier(
                        "-",
                    ),
                    Nil,
                ),
                Cell(
                    Integer(
                        123,
                    ),
                    Nil,
                ),
            ),
            Cell(
                Integer(
                    456,
                ),
                Nil,
            ),
        ),
    ),
)
"#
        );

        Ok(())
    }

    #[test]
    fn simple_operator_list() -> Result<()> {
        snapshot!(
            try_parse("[1 + 2 + 'a]")?,
            r#"
Cell(
    Identifier(
        "+",
    ),
    Cell(
        Integer(
            1,
        ),
        Cell(
            Integer(
                2,
            ),
            Cell(
                Quoted(
                    Identifier(
                        "a",
                    ),
                ),
                Nil,
            ),
        ),
    ),
)
"#
        );

        Ok(())
    }

    #[test]
    fn mismatched_operator_list() -> Result<()> {
        assert_err_matches_regex!(try_parse("[1 + 2 * 3]"), "MismatchedOperatorList");

        Ok(())
    }

    #[test]
    fn single_line_form_list() -> Result<()> {
        snapshot!(
            try_parse("{a b, c d, 1}")?,
            r#"
Cell(
    Cell(
        Identifier(
            "a",
        ),
        Cell(
            Identifier(
                "b",
            ),
            Nil,
        ),
    ),
    Cell(
        Cell(
            Identifier(
                "c",
            ),
            Cell(
                Identifier(
                    "d",
                ),
                Nil,
            ),
        ),
        Cell(
            Cell(
                Integer(
                    1,
                ),
                Nil,
            ),
            Nil,
        ),
    ),
)
"#
        );

        Ok(())
    }

    #[test]
    fn multi_line_form_list() -> Result<()> {
        snapshot!(
            try_parse(
                "{
                    d c 1
                    e f \"yo\"
                }"
            )?,
            r#"
Cell(
    Cell(
        Identifier(
            "d",
        ),
        Cell(
            Identifier(
                "c",
            ),
            Cell(
                Integer(
                    1,
                ),
                Nil,
            ),
        ),
    ),
    Cell(
        Cell(
            Identifier(
                "e",
            ),
            Cell(
                Identifier(
                    "f",
                ),
                Cell(
                    String(
                        "yo",
                    ),
                    Nil,
                ),
            ),
        ),
        Nil,
    ),
)
"#
        );

        Ok(())
    }

    #[test]
    fn unterminated_lists() -> Result<()> {
        assert_err_matches_regex!(try_parse("(1 2"), "UnterminatedList");
        assert_err_matches_regex!(try_parse("[1 + 2"), "UnterminatedList");
        assert_err_matches_regex!(try_parse("{a c, d"), "UnterminatedList");

        Ok(())
    }

    #[test]
    fn tokenize_errors_passed_through() -> Result<()> {
        assert_err_matches_regex!(try_parse("\"abc"), "Tokenize.*String");
        assert_err_matches_regex!(try_parse("(\"abc"), "Tokenize.*String");

        Ok(())
    }
}
