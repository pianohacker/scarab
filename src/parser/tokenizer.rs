// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use thiserror::Error;

use super::utils::TakeWhileUngreedy;

#[derive(Clone, Error, Debug, Eq, PartialEq)]
pub enum TokenizeError {
    #[error("unexpected character: {0}")]
    UnexpectedChar(char),
    #[error("invalid integer")]
    InvalidInteger,
    #[error("unparsable integer")]
    UnparsableInteger {
        #[from]
        source: std::num::ParseIntError,
    },
    #[error("unterminated string")]
    UnterminatedString,
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
    Newline,
    Comma,
    Integer(isize),
    String(String),
    Identifier(String),
}

fn char_is_token_end(c: char) -> bool {
    match c {
        '(' | ')' | '[' | ']' | '{' | '}' | '\'' | '"' | '\n' | ',' => true,
        _ if c.is_ascii_whitespace() => true,
        _ => false,
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
            return Err(TokenizeError::UnterminatedString);
        }

        Ok(result)
    }

    fn tokenize_identifier(&mut self, first_char: char) -> TResult<String> {
        Ok(std::iter::once(first_char)
            .chain(
                self.input
                    .take_while_ungreedy(|x| !char_is_token_end(*x) && !x.is_ascii_whitespace()),
            )
            .collect())
    }

    fn tokenize_integer(&mut self, mut first_char: char) -> TResult<isize> {
        let sign = if first_char == '-' {
            first_char = self.input.next().unwrap_or_else(|| unreachable!());
            -1
        } else {
            1
        };

        let mut base = 10;

        if first_char == '0' {
            match self.input.peek() {
                Some('b') => {
                    base = 2;
                    self.input.next();
                    first_char = self.input.next().ok_or(TokenizeError::InvalidInteger)?;
                }
                Some('x') => {
                    base = 16;
                    self.input.next();
                    first_char = self.input.next().ok_or(TokenizeError::InvalidInteger)?;
                }
                _ => {}
            }
        }

        let s = std::iter::once(first_char)
            .chain(self.input.take_while_ungreedy(|x| !char_is_token_end(*x)))
            .collect::<String>();

        isize::from_str_radix(&s, base)
            .map(|x| x * sign)
            .map_err(|e| e.into())
    }
}

impl<I> std::iter::Iterator for Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    type Item = TResult<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        use Token::*;

        if self.stopped {
            return None;
        }

        self.input
            .take_while_ungreedy(|x| x.is_ascii_whitespace() && *x != '\n')
            .for_each(drop);

        let result = self.input.next().map(|c| match c {
            '(' => Ok(LParen),
            ')' => Ok(RParen),
            '[' => Ok(LBracket),
            ']' => Ok(RBracket),
            '{' => Ok(LBrace),
            '}' => Ok(RBrace),
            '\'' => Ok(Quote),
            '\n' => Ok(Newline),
            ',' => Ok(Comma),
            '"' => Ok(String(self.tokenize_string()?)),
            _ if c.is_ascii_digit() => Ok(Integer(self.tokenize_integer(c)?)),
            '-' if self.input.peek().map_or(false, |c2| c2.is_ascii_digit()) => {
                Ok(Integer(self.tokenize_integer(c)?))
            }
            _ if !c.is_control() => Ok(Identifier(self.tokenize_identifier(c)?)),
            _ => Err(TokenizeError::UnexpectedChar(c)),
        });

        if let Some(Err(_)) = result {
            self.stopped = true;
        }

        result
    }
}

pub fn tokenize<I>(input: I) -> Tokenizer<I::IntoIter>
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
            try_tokenize("()[]{}',")?,
            "
[
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Quote,
    Comma,
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

    #[test]
    fn unterminated_string() -> TResult<()> {
        assert_err_matches_regex!(try_tokenize("\"abc"), r#"Unterminated"#);

        Ok(())
    }

    #[test]
    fn space_separated_tokens() -> TResult<()> {
        snapshot!(
            try_tokenize("( \"abc\"\t\n{}")?,
            r#"
[
    LParen,
    String(
        "abc",
    ),
    Newline,
    LBrace,
    RBrace,
]
"#
        );

        Ok(())
    }

    #[test]
    fn identifiers() -> TResult<()> {
        snapshot!(
            try_tokenize("identifier1 identifier!2?)identifier3")?,
            r#"
[
    Identifier(
        "identifier1",
    ),
    Identifier(
        "identifier!2?",
    ),
    RParen,
    Identifier(
        "identifier3",
    ),
]
"#
        );

        Ok(())
    }

    #[test]
    fn leading_dash_identifiers() -> TResult<()> {
        snapshot!(
            try_tokenize(r#"- -a"#)?,
            r#"
[
    Identifier(
        "-",
    ),
    Identifier(
        "-a",
    ),
]
"#
        );

        Ok(())
    }

    #[test]
    fn integers() -> TResult<()> {
        snapshot!(
            try_tokenize_uncollapsed("0 123 4 0b11001 0x46aF -3 -0b111 -0x77D"),
            "
[
    Ok(
        Integer(
            0,
        ),
    ),
    Ok(
        Integer(
            123,
        ),
    ),
    Ok(
        Integer(
            4,
        ),
    ),
    Ok(
        Integer(
            25,
        ),
    ),
    Ok(
        Integer(
            18095,
        ),
    ),
    Ok(
        Integer(
            -3,
        ),
    ),
    Ok(
        Integer(
            -7,
        ),
    ),
    Ok(
        Integer(
            -1917,
        ),
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn partial_integer() -> TResult<()> {
        assert_err_matches_regex!(try_tokenize("0b"), r#"InvalidInteger"#);
        assert_err_matches_regex!(try_tokenize("0x"), r#"InvalidInteger"#);

        Ok(())
    }

    #[test]
    fn invalid_integer() -> TResult<()> {
        assert_err_matches_regex!(try_tokenize("04y"), r#"UnparsableInteger.*Digit"#);
        assert_err_matches_regex!(try_tokenize("0b12"), r#"UnparsableInteger.*Digit"#);
        assert_err_matches_regex!(try_tokenize("0xAZ"), r#"UnparsableInteger.*Digit"#);

        assert_err_matches_regex!(
            try_tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFF"),
            r#"UnparsableInteger.*Overflow"#
        );

        Ok(())
    }
}
