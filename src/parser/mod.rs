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
use result_at::{Reader, ResultAt, ResultAt::*};

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum ErrorInternal {
    #[error("unexpected end of input")]
    Eof,
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
type IResult<T> = std::result::Result<T, ErrorInternal>;
type IResultAt<T> = ResultAt<T, ErrorInternal>;

fn result_from_result_at<T>(result_at: IResultAt<T>) -> Result<T> {
    match result_at {
        OkAt(x, _) => Ok(x),
        ErrAt(e, at) => Err(Error::from_internal_at(e, at)),
        NoneAt(at) => Err(Error::from_internal_at(ErrorInternal::Eof, at)),
    }
}

pub struct Parser<I: Iterator<Item = char>> {
    input: Reader<Tokenizer<I>>,
}

macro_rules! expect_match {
    ($token_at:expr, { $($pat:pat => $body:expr),+$(,)? }) => {
        {
            let (token, at) = $token_at;
            match token {
                $($pat => $body,)+
                _ => return ErrAt(
                    ErrorInternal::UnexpectedToken(token),
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
        self.input
            .items_while_successful_if(|t| *t == Token::Newline)
            .for_each(drop);

        self.input.next().map_err(|e| e.into())
    }

    fn peek(&mut self) -> ResultAt<&Token, ErrorInternal> {
        self.input
            .items_while_successful_if(|t| *t == Token::Newline)
            .for_each(drop);

        self.input.peek().as_ref().map_err(|e| e.clone().into())
    }

    fn peek_with_newlines(&mut self) -> ResultAt<&Token, ErrorInternal> {
        self.input.peek().as_ref().map_err(|e| e.clone().into())
    }

    fn peek_or(&mut self, error: ErrorInternal) -> ResultAt<&Token, ErrorInternal> {
        self.input
            .items_while_successful_if(|t| *t == Token::Newline)
            .for_each(drop);

        self.peek().none_as_err(error)
    }

    fn parse_list(
        &mut self,
        terminator_predicate: impl Fn(IResultAt<&Token>) -> IResultAt<bool>,
        at: (usize, usize),
    ) -> IResultAt<Value> {
        let mut elements = vec![];

        loop {
            let result = self.peek_with_newlines();

            if terminator_predicate(result)
                .none_as_err(ErrorInternal::UnterminatedList)?
                .0
            {
                break;
            }
            elements.push(self.parse_value()?.0);
        }

        OkAt(Self::elements_to_list(elements), at)
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
                return ErrAt(
                    ErrorInternal::MismatchedOperatorList(operator, next_operator),
                    at,
                );
            }

            elements.push(self.parse_value()?.0);
        }

        expect_match!( self.next()?, {
            Token::RBracket => (),
        });

        OkAt(Self::elements_to_list(elements), at)
    }

    fn parse_form_list_item(
        &mut self,
        terminator_predicate: impl Fn(IResultAt<&Token>) -> IResultAt<bool>,
        at: (usize, usize),
    ) -> IResultAt<Value> {
        let (list, _) = self.parse_list(terminator_predicate, at)?;

        self.input
            .items_while_successful_if(|t| *t == Token::Comma || *t == Token::Newline)
            .for_each(drop);

        OkAt(list, at)
    }

    fn parse_form_list(&mut self, mut at: (usize, usize)) -> IResultAt<Value> {
        let mut lists = vec![];

        loop {
            let (next, next_at) = self.peek_or(ErrorInternal::UnterminatedList)?;
            if *next == Token::RBrace {
                break;
            }

            let (list, _) = self.parse_form_list_item(
                |t| t.map(|t| *t == Token::Comma || *t == Token::Newline || *t == Token::RBrace),
                at,
            )?;
            if list != Value::Nil {
                lists.push(list);
            }

            at = next_at;
        }

        expect_match!( self.next()?, {
            Token::RBrace => (),
        });

        OkAt(Self::elements_to_list(lists), at)
    }

    pub fn parse_implicit_form_list(&mut self) -> IResultAt<Value> {
        let mut lists = vec![];
        let mut at = (1, 1);

        loop {
            let next_at = match self.input.peek() {
                OkAt(_, at) => at,
                ErrAt(e, at) => return ErrAt(e.clone().into(), *at),
                NoneAt(_) => break,
            };
            at = *next_at;

            let (list, _) = self.parse_form_list_item(
                |t| {
                    t.map(|t| *t == Token::Comma || *t == Token::Newline)
                        .none_as_value(true)
                },
                at,
            )?;
            if list != Value::Nil {
                lists.push(list);
            }
        }

        OkAt(Self::elements_to_list(lists), at)
    }

    pub fn parse_value(&mut self) -> IResultAt<Value> {
        self.next().and_then_at(|t, at| {
            expect_match!( (t, at), {
                Token::Integer(i) => OkAt(Value::Integer(i), at),
                Token::String(s) => OkAt(Value::String(s), at),
                Token::Identifier(i) => OkAt(Value::Identifier(i), at),
                Token::LParen => {
                    let (result, _) = self.parse_list(|t| t.map(|t| *t == Token::RParen), at)?;
                    self.next()?;
                    OkAt(result, at)
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
    result_from_result_at(
        Parser {
            input: tokenize(input.into_iter()),
        }
        .parse_value(),
    )
}

pub fn parse_implicit_form_list<I>(input: I) -> Result<Value>
where
    I: IntoIterator<Item = char>,
{
    result_from_result_at(
        Parser {
            input: tokenize(input.into_iter()),
        }
        .parse_implicit_form_list(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn try_parse_display(input: &str) -> Result<String> {
        parse_value(input.chars()).map(|v| format!("{}", v))
    }

    #[test]
    fn empty_parse_fails() -> Result<()> {
        assert_err_matches_regex!(try_parse_display(""), "Eof");

        Ok(())
    }

    #[test]
    fn unexpected_tokens_fail() -> Result<()> {
        assert_err_matches_regex!(try_parse_display(")"), "Unexpected.*RParen");

        Ok(())
    }

    #[test]
    fn single_values() -> Result<()> {
        snapshot!(try_parse_display("123")?, "123");
        snapshot!(try_parse_display("\"abc\"")?, r#""abc""#);
        snapshot!(try_parse_display("blah")?, "blah");

        Ok(())
    }

    #[test]
    fn quoted_values() -> Result<()> {
        snapshot!(try_parse_display("'abc")?, r"'abc");

        snapshot!(try_parse_display("''123")?, r"''123");

        snapshot!(try_parse_display("'(1 2 3))")?, r"'(1 2 3)");

        Ok(())
    }

    #[test]
    fn simple_list() -> Result<()> {
        snapshot!(try_parse_display("(+ 123 456))")?, "(+ 123 456)");

        Ok(())
    }

    #[test]
    fn simple_list_containing_newline() -> Result<()> {
        snapshot!(try_parse_display("(+ 123\n456))")?, "(+ 123 456)");

        Ok(())
    }

    #[test]
    fn nested_list() -> Result<()> {
        snapshot!(try_parse_display("(+ ((-)) 123) 456)")?, "(+ ((-)) 123)");

        Ok(())
    }

    #[test]
    fn simple_operator_list() -> Result<()> {
        snapshot!(try_parse_display("[1 + 2 + 'a]")?, r"(+ 1 2 'a)");

        Ok(())
    }

    #[test]
    fn mismatched_operator_list() -> Result<()> {
        assert_err_matches_regex!(try_parse_display("[1 + 2 * 3]"), "MismatchedOperatorList");

        Ok(())
    }

    #[test]
    fn single_line_form_list() -> Result<()> {
        snapshot!(try_parse_display("{a b, c d, 1}")?, "((a b) (c d) (1))");

        Ok(())
    }

    #[test]
    fn multi_line_form_list() -> Result<()> {
        snapshot!(
            try_parse_display(
                "{
                    d c 1
                    e f \"yo\"
                }"
            )?,
            r#"((d c 1) (e f "yo"))"#
        );

        Ok(())
    }

    #[test]
    fn unterminated_lists() -> Result<()> {
        assert_err_matches_regex!(try_parse_display("(1 2"), "UnterminatedList");
        assert_err_matches_regex!(try_parse_display("[1 + 2"), "UnterminatedList");
        assert_err_matches_regex!(try_parse_display("{a c, d"), "UnterminatedList");

        Ok(())
    }

    #[test]
    fn tokenize_errors_passed_through() -> Result<()> {
        assert_err_matches_regex!(try_parse_display("\"abc"), "Tokenize.*String");
        assert_err_matches_regex!(try_parse_display("(\"abc"), "Tokenize.*String");

        Ok(())
    }

    #[test]
    fn multi_line_implicit_form_list() -> Result<()> {
        snapshot!(
            format!(
                "{}",
                parse_implicit_form_list(
                    "
                        d c 1

                        e f \"yo\"
                    "
                    .chars()
                )?
            ),
            r#"((d c 1) (e f "yo"))"#
        );

        Ok(())
    }

    #[test]
    fn multi_line_implicit_form_list_cannot_end_with_brace() -> Result<()> {
        assert_err_matches_regex!(
            parse_implicit_form_list("d c 1}".chars()),
            "UnexpectedToken.*Brace"
        );

        Ok(())
    }
}
