// Copyright (c) Jesse Weaver, 2021, "123"
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod tokenizer;

use std::rc::Rc;
use thiserror::Error;

use self::tokenizer::{tokenize, Token, Tokenizer};
use crate::value::{self, Value};
use result_at::{Position, Reader, ResultAt, ResultAt::*};

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
type IResultAt<T> = ResultAt<T, ErrorInternal>;

fn result_from_result_at<T>(result_at: IResultAt<T>) -> Result<T> {
    match result_at {
        OkAt(x, _) => Ok(x),
        ErrAt(e, at) => Err(Error::from_internal_at(e, at)),
        NoneAt(at) => Err(Error::from_internal_at(ErrorInternal::Eof, at)),
    }
}

pub type PositionMap = value::ContextMap<Position>;

pub struct Parser<I: Iterator<Item = char>> {
    input: Reader<Tokenizer<I>>,
    positions: PositionMap,
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

macro_rules! expect_match_at {
    ($token_at:expr, { $($pat:pat => $body:expr),+$(,)? }) => {
        {
            match $token_at {
                $($pat => $body,)+
                (token, at) => return ErrAt(
                    ErrorInternal::UnexpectedToken(token),
                    at,
                ),
            }
        }
    };
}

impl<I: Iterator<Item = char>> Parser<I> {
    fn elements_to_list(elements: Vec<Rc<Value>>) -> Rc<Value> {
        Rc::new(
            elements
                .into_iter()
                .rev()
                .fold(Value::Nil, |accum, elem| Value::Cell(elem, Rc::new(accum))),
        )
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
    ) -> IResultAt<Rc<Value>> {
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

    fn parse_operator_list(&mut self, at: (usize, usize)) -> IResultAt<Rc<Value>> {
        let (first, _) = self.parse_value()?;

        let (operator, operator_at) = expect_match_at!( self.next()?, {
            (Token::Identifier(i), at) => (i, at),
        });

        let operator_value = Rc::new(Value::Identifier(value::identifier(operator.clone())));
        self.positions.insert(&operator_value, operator_at);

        let (second, _) = self.parse_value()?;

        let mut elements = vec![operator_value, first, second];

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
    ) -> IResultAt<Rc<Value>> {
        let (list, _) = self.parse_list(terminator_predicate, at)?;

        self.input
            .items_while_successful_if(|t| *t == Token::Semicolon || *t == Token::Newline)
            .for_each(drop);

        OkAt(list, at)
    }

    fn parse_form_list(&mut self, mut at: (usize, usize)) -> IResultAt<Rc<Value>> {
        let mut lists = vec![];

        loop {
            let (next, next_at) = self.peek_or(ErrorInternal::UnterminatedList)?;
            if *next == Token::RBrace {
                break;
            }

            let (list, _) = self.parse_form_list_item(
                |t| {
                    t.map(|t| *t == Token::Semicolon || *t == Token::Newline || *t == Token::RBrace)
                },
                next_at,
            )?;
            if *list != Value::Nil {
                self.positions.insert(&list, next_at);
                lists.push(list);
            }
        }

        expect_match!( self.next()?, {
            Token::RBrace => (),
        });

        OkAt(Self::elements_to_list(lists), at)
    }

    pub fn parse_implicit_form_list(&mut self) -> IResultAt<Rc<Value>> {
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
                    t.map(|t| *t == Token::Semicolon || *t == Token::Newline)
                        .none_as_value(true)
                },
                at,
            )?;
            if *list != Value::Nil {
                self.positions.insert(&list, at);
                lists.push(list);
            }
        }

        OkAt(Self::elements_to_list(lists), at)
    }

    pub fn parse_value(&mut self) -> IResultAt<Rc<Value>> {
        self.next()
            .and_then_at(|t, at| {
                expect_match!( (t, at), {
                    Token::Integer(i) => OkAt(Rc::new(Value::Integer(i)), at),
                    Token::String(s) => OkAt(Rc::new(Value::String(s)), at),
                    Token::Identifier(i) => {
                        OkAt(match i.as_str() {
                            "nil" => Rc::new(Value::Nil),
                            "true" => Rc::new(Value::Boolean(true)),
                            "false" => Rc::new(Value::Boolean(false)),
                            _ => Rc::new(Value::Identifier(i)),
                        }, at)
                    },
                    Token::LParen => {
                        let (result, _) = self.parse_list(|t| t.map(|t| *t == Token::RParen), at)?;
                        self.next()?;
                        OkAt(result, at)
                    },
                    Token::LBracket => self.parse_operator_list(at),
                    Token::LBrace => self.parse_form_list(at),
                    Token::Quote => self.parse_value().map(|v| Rc::new(Value::Quoted(v))),
                })
            })
            .and_then_at(|t, at| {
                self.positions.insert(&t, at);

                OkAt(t, at)
            })
    }
}

pub fn parse_value<I>(input: I) -> Result<(Rc<Value>, PositionMap)>
where
    I: IntoIterator<Item = char>,
{
    let mut parser = Parser {
        input: tokenize(input.into_iter()),
        positions: value::ContextMap::new(),
    };
    result_from_result_at(parser.parse_value()).map(|v| (v, parser.positions))
}

pub fn parse_implicit_form_list<I>(input: I) -> Result<(Rc<Value>, PositionMap)>
where
    I: IntoIterator<Item = char>,
{
    let mut parser = Parser {
        input: tokenize(input.into_iter()),
        positions: value::ContextMap::new(),
    };
    result_from_result_at(parser.parse_implicit_form_list()).map(|v| (v, parser.positions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn try_parse_display(input: &str) -> Result<String> {
        parse_value(input.chars()).map(|(v, _)| format!("{}", v))
    }

    fn try_into_display_positions<'a>(
        parser: impl FnOnce(std::str::Chars<'a>) -> Result<(Rc<Value>, PositionMap)>,
        input: &'a str,
    ) -> Result<String> {
        parser(input.chars()).map(|(_, p)| {
            let mut entries: Vec<_> =
                unsafe { p.iter().map(|(k, p)| (p, format!("{}", k))).collect() };

            entries.sort();

            let formatted_entries: Vec<_> = entries
                .into_iter()
                .map(|((l, c), k)| format!("    ({}, {}): {}", l, c, k))
                .collect();

            format!("{{\n{}\n}}", formatted_entries.join(",\n"))
        })
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
        snapshot!(try_parse_display("nil")?, "nil");
        snapshot!(try_parse_display("true")?, "true");
        snapshot!(try_parse_display("false")?, "false");

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
        snapshot!(try_parse_display("{a b; c d; 1}")?, "((a b) (c d) (1))");

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
                .0
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

    #[test]
    fn single_value_saves_position() -> Result<()> {
        snapshot!(
            try_into_display_positions(parse_value, "123")?,
            "
{
    (1, 1): 123
}
"
        );

        Ok(())
    }

    #[test]
    fn list_saves_positions() -> Result<()> {
        snapshot!(
            try_into_display_positions(parse_value, "(\"a\" b 1 (c 1))")?,
            r#"
{
    (1, 1): ("a" b 1 (c 1)),
    (1, 2): "a",
    (1, 6): b,
    (1, 8): 1,
    (1, 10): (c 1),
    (1, 11): c,
    (1, 13): 1
}
"#
        );

        Ok(())
    }

    #[test]
    fn operator_list_saves_positions() -> Result<()> {
        snapshot!(
            try_into_display_positions(parse_value, "[1 + 2 + 4]")?,
            "
{
    (1, 1): (+ 1 2 4),
    (1, 2): 1,
    (1, 4): +,
    (1, 6): 2,
    (1, 10): 4
}
"
        );

        Ok(())
    }

    #[test]
    fn form_list_saves_positions() -> Result<()> {
        snapshot!(
            try_into_display_positions(parse_value, "{a b; def d}")?,
            "
{
    (1, 1): ((a b) (def d)),
    (1, 2): (a b),
    (1, 2): a,
    (1, 4): b,
    (1, 7): (def d),
    (1, 7): def,
    (1, 11): d
}
"
        );

        Ok(())
    }

    #[test]
    fn implicit_form_list_saves_positions() -> Result<()> {
        snapshot!(
            try_into_display_positions(parse_implicit_form_list, "a b\ndef d")?,
            "
{
    (1, 1): (a b),
    (1, 1): a,
    (1, 3): b,
    (2, 1): (def d),
    (2, 1): def,
    (2, 5): d
}
"
        );

        Ok(())
    }
}
