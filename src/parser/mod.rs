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
use crate::value::Value;
use result_at::{Reader, ResultAt};

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum ErrorInternal {
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
    UnexpectedToken(Token),
    #[error("unterminated list")]
    UnterminatedList,
    #[error("placeholder")]
    Placeholder,
}

impl From<&ErrorInternal> for ErrorInternal {
    fn from(e: &ErrorInternal) -> ErrorInternal {
        e.clone()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Error {
    error: ErrorInternal,
    line: usize,
    column: usize,
}

impl Error {
    fn from_internal_at(error: ErrorInternal, at: (usize, usize)) -> Self {
        let (line, column) = at;
        Error {
            error,
            line,
            column,
        }
    }
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
type IResultAt<T> = ResultAt<T, ErrorInternal>;

pub struct Parser<I: Iterator<Item = char>> {
    input: Reader<Tokenizer<I>>,
}

macro_rules! expect_match {
    ($token_at:expr, { $($pat:pat => $body:expr),+$(,)? }) => {
        {
            let (token, at) = $token_at;
            match token {
                $($pat => $body,)+
                _ => return ResultAt(
                    Err(
                        ErrorInternal::UnexpectedToken(token),
                    ),
                    at,
                ),
            }
        }
    };
}

impl<I: Iterator<Item = char>> Parser<I> {
    fn elements_to_list(elements: Vec<Value>) -> Value {
        elements.into_iter().rev().fold(Value::Nil, |accum, elem| {
            Value::Cell(Rc::new(elem), Rc::new(accum))
        })
    }

    fn next(&mut self) -> IResultAt<Token> {
        self.input.next().map_err(|e| ErrorInternal::from(e))
    }

    fn peek_or(&mut self, error: ErrorInternal) -> ResultAt<&Token, ErrorInternal> {
        self.input.peek().as_ref().map_err(|e| match e {
            tokenizer::Error::Eof { .. } => error,
            _ => e.clone().into(),
        })
    }

    fn parse_list(
        &mut self,
        terminator_predicate: impl Fn(&Token) -> bool,
        at: (usize, usize),
    ) -> IResultAt<Value> {
        let mut elements = vec![];

        while !terminator_predicate(&self.peek_or(ErrorInternal::UnterminatedList)?.0) {
            elements.push(self.parse_value()?.0);
        }

        assert!(terminator_predicate(
            &self.peek_or(ErrorInternal::UnterminatedList)?.0
        ));

        ResultAt(Ok(Self::elements_to_list(elements)), at)
    }

    fn parse_operator_list(&mut self, at: (usize, usize)) -> IResultAt<Value> {
        let (first, _) = self.parse_value()?;

        let operator = expect_match!( self.next()?, {
            Token::Identifier(i) => i,
        });

        let (second, _) = self.parse_value()?;

        let mut elements = vec![Value::Identifier(operator.clone()), first, second];

        while *self.peek_or(ErrorInternal::UnterminatedList)?.0 != Token::RBracket {
            let (next, at) = self.next()?;
            let next_operator = expect_match!( (next, at), {
                Token::Identifier(i) => i,
            });

            if next_operator != operator {
                return ResultAt(
                    Err(ErrorInternal::MismatchedOperatorList(
                        operator,
                        next_operator,
                    )),
                    at,
                );
            }

            elements.push(self.parse_value()?.0);
        }

        expect_match!( self.next()?, {
            Token::RBracket => (),
        });

        ResultAt(Ok(Self::elements_to_list(elements)), at)
    }

    fn parse_form_list(&mut self, at: (usize, usize)) -> IResultAt<Value> {
        let mut lists = vec![];

        while *self.peek_or(ErrorInternal::UnterminatedList)?.0 != Token::RBrace {
            let (list, _) = self.parse_list(
                |t| *t == Token::Comma || *t == Token::Newline || *t == Token::RBrace,
                at,
            )?;

            if list != Value::Nil {
                lists.push(list);
            }

            self.input
                .items_while_successful_if(|t| *t == Token::Comma || *t == Token::Newline)
                .for_each(drop);
        }

        expect_match!( self.next()?, {
            Token::RBrace => (),
        });

        ResultAt(Ok(Self::elements_to_list(lists)), at)
    }

    pub fn parse_value(&mut self) -> IResultAt<Value> {
        self.next().and_then_at(|t, at| {
            expect_match!( (t, at), {
                Token::Integer(i) => ResultAt(Ok(Value::Integer(i)), at),
                Token::String(s) => ResultAt(Ok(Value::String(s)), at),
                Token::Identifier(i) => ResultAt(Ok(Value::Identifier(i)), at),
                Token::LParen => {
                    let (result, _) = self.parse_list(|t| *t == Token::RParen, at)?;
                    self.next()?;
                    ResultAt(Ok(result), at)
                },
                Token::LBracket => self.parse_operator_list(at),
                Token::LBrace => self.parse_form_list(at),
                Token::Quote => self.parse_value().map(|v| Value::Quoted(Rc::new(v))),
            })
        })
    }
}

pub fn parse_value<I>(input: I) -> Result<Value>
where
    I: IntoIterator<Item = char>,
{
    match (Parser {
        input: tokenize(input.into_iter()),
    }
    .parse_value())
    {
        ResultAt(Ok(x), _) => Ok(x),
        ResultAt(Err(e), at) => Err(Error::from_internal_at(e, at)),
    }
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
        assert_err_matches_regex!(try_parse(""), "Eof");

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
