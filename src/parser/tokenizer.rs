// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum TokenizeError {
    #[error("unexpected character: {0}")]
    UnexpectedChar(char),
    #[error("placeholder")]
    Placeholder,
}

type TResult<T> = Result<T, TokenizeError>;

#[derive(Debug, Eq, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Quote,
    String(String),
}

struct TakeWhileUngreedy<'a, T, I: Iterator<Item = T>, P> {
    input: &'a mut std::iter::Peekable<I>,
    predicate: P,
}

trait TakeWhileUngreedyHelper<T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> {
    fn take_while_ungreedy(&mut self, predicate: P) -> TakeWhileUngreedy<T, I, P>;
}

impl<T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> TakeWhileUngreedyHelper<T, I, P>
    for std::iter::Peekable<I>
{
    fn take_while_ungreedy(&mut self, predicate: P) -> TakeWhileUngreedy<T, I, P> {
        TakeWhileUngreedy {
            input: self,
            predicate,
        }
    }
}

impl<'a, T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> Iterator
    for TakeWhileUngreedy<'a, T, I, P>
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.input.peek()?;

        if (self.predicate)(value) {
            self.input.next()
        } else {
            None
        }
    }
}

pub struct Tokenizer<I: Iterator<Item = char>> {
    input: std::iter::Peekable<I>,
    stopped: bool,
}

impl<I> Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    fn tokenize_string(&mut self) -> TResult<String> {
        let result = self.input.take_while_ungreedy(|x| *x != '\"').collect();

        if let None = self.input.next() {
            return Err(TokenizeError::Placeholder);
        }

        Ok(result)
    }
}

impl<I> std::iter::Iterator for Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    type Item = TResult<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        use Token::{String as TokenString, *};

        if self.stopped {
            return None;
        }

        let result = self.input.next().map(|c| match c {
            '(' => Ok(LParen),
            ')' => Ok(RParen),
            '[' => Ok(LBracket),
            ']' => Ok(RBracket),
            '{' => Ok(LBrace),
            '}' => Ok(RBrace),
            '\'' => Ok(Quote),
            '"' => Ok(TokenString(self.tokenize_string()?)),
            _ => Err(TokenizeError::UnexpectedChar(c)),
        });

        if let Some(Err(_)) = result {
            self.stopped = true;
        }

        result
    }
}

fn tokenize<I>(input: I) -> Tokenizer<I::IntoIter>
where
    I: IntoIterator<Item = char>,
{
    Tokenizer {
        input: input.into_iter().peekable(),
        stopped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn try_tokenize(input: &str) -> TResult<Vec<Token>> {
        tokenize(input.chars()).collect()
    }

    fn try_tokenize_uncollapsed(input: &str) -> Vec<TResult<Token>> {
        tokenize(input.chars()).collect()
    }

    #[test]
    fn single_character_tokens() -> TResult<()> {
        snapshot!(
            try_tokenize("()[]{}'")?,
            "
[
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Quote,
]
"
        );

        Ok(())
    }

    #[test]
    fn unexpected_single_character_tokens() -> TResult<()> {
        assert_err_matches_regex!(try_tokenize("\x07"), r#"\\u\{7\}"#);

        Ok(())
    }

    #[test]
    fn tokenizing_stops_after_error() -> TResult<()> {
        snapshot!(
            try_tokenize_uncollapsed("(\x07)"),
            r"
[
    Ok(
        LParen,
    ),
    Err(
        UnexpectedChar(
            '\u{7}',
        ),
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn basic_strings() -> TResult<()> {
        snapshot!(
            try_tokenize(r#""""a""abc""#)?,
            r#"
[
    String(
        "",
    ),
    String(
        "a",
    ),
    String(
        "abc",
    ),
]
"#
        );

        Ok(())
    }
}
