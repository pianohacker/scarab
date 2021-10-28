// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use thiserror::Error;

use result_at::{CharReaderError, CharSource, Reader, ResultAt, Source};

#[derive(Clone, Error, Debug, Eq, PartialEq)]
pub enum Error {
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
    #[error("EOF")]
    Eof {
        #[from]
        source: CharReaderError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
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
    input: Reader<CharSource<I>>,
    stopped: bool,
}

impl<I> Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    fn tokenize_string(&mut self) -> Result<String> {
        let result = self
            .input
            .items_while_successful_if(|x| *x != '\"')
            .collect();

        if let ResultAt(Err(_), _) = self.input.next() {
            return Err(Error::UnterminatedString);
        }

        Ok(result)
    }

    fn tokenize_identifier(&mut self, first_char: char) -> Result<String> {
        Ok(std::iter::once(first_char)
            .chain(
                self.input.items_while_successful_if(|x| {
                    !char_is_token_end(*x) && !x.is_ascii_whitespace()
                }),
            )
            .collect())
    }

    fn tokenize_integer(&mut self, mut first_char: char) -> Result<isize> {
        let sign = if first_char == '-' {
            first_char = self.input.next().0.unwrap_or_else(|_| unreachable!());
            -1
        } else {
            1
        };

        let mut base = 10;

        if first_char == '0' {
            match self.input.peek() {
                ResultAt(Ok('b'), _) => {
                    base = 2;
                    self.input.next().0.map_err(|_| Error::InvalidInteger)?;
                    first_char = self.input.next().0.map_err(|_| Error::InvalidInteger)?;
                }
                ResultAt(Ok('x'), _) => {
                    base = 16;
                    self.input.next().0.map_err(|_| Error::InvalidInteger)?;
                    first_char = self.input.next().0.map_err(|_| Error::InvalidInteger)?;
                }
                _ => {}
            }
        }

        let s = std::iter::once(first_char)
            .chain(
                self.input
                    .items_while_successful_if(|x| !char_is_token_end(*x)),
            )
            .collect::<String>();

        isize::from_str_radix(&s, base)
            .map(|x| x * sign)
            .map_err(|e| e.into())
    }
}

impl<I> Source for Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    type Output = Token;
    type Error = Error;

    fn next(&mut self) -> ResultAt<Token, Error> {
        use Token::*;

        self.input
            .items_while_successful_if(|x| x.is_ascii_whitespace() && *x != '\n')
            .for_each(drop);

        let result = self
            .input
            .next()
            .map_err(Error::from)
            .and_then(|c| match c {
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
                '-' if self.input.peek().0.map_or(false, |c2| c2.is_ascii_digit()) => {
                    Ok(Integer(self.tokenize_integer(c)?))
                }
                _ if !c.is_control() => Ok(Identifier(self.tokenize_identifier(c)?)),
                _ => Err(Error::UnexpectedChar(c)),
            });

        if let ResultAt(Err(_), _) = result {
            self.stopped = true;
        }

        result
    }
}

pub fn tokenize<I>(input: I) -> Reader<Tokenizer<I::IntoIter>>
where
    I: IntoIterator<Item = char>,
{
    Tokenizer {
        input: CharSource::new(input.into_iter()).reader(),
        stopped: false,
    }
    .reader()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn try_tokenize(input: &str) -> Result<Vec<Token>> {
        tokenize(input.chars())
            .iter_results()
            .filter(|r| match r {
                Err(Error::Eof { .. }) => false,
                _ => true,
            })
            .collect()
    }

    fn try_tokenize_uncollapsed(input: &str) -> Vec<Result<Token>> {
        tokenize(input.chars()).iter_results().collect()
    }

    fn try_tokenize_full(input: &str) -> Vec<ResultAt<Token, Error>> {
        tokenize(input.chars()).iter().collect()
    }

    #[test]
    fn single_character_tokens() -> Result<()> {
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
    fn unexpected_single_character_tokens() -> Result<()> {
        assert_err_matches_regex!(try_tokenize("\x07"), r#"\\u\{7\}"#);

        Ok(())
    }

    #[test]
    fn tokenizing_stops_after_error() -> Result<()> {
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
    fn basic_strings() -> Result<()> {
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
    fn unterminated_string() -> Result<()> {
        assert_err_matches_regex!(try_tokenize("\"abc"), r#"Unterminated"#);

        Ok(())
    }

    #[test]
    fn space_separated_tokens() -> Result<()> {
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
    fn identifiers() -> Result<()> {
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
    fn leading_dash_identifiers() -> Result<()> {
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
    fn integers() -> Result<()> {
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
    Err(
        Eof {
            source: Eof,
        },
    ),
]
"
        );

        Ok(())
    }

    #[test]
    fn partial_integer() -> Result<()> {
        assert_err_matches_regex!(try_tokenize("0b"), r#"InvalidInteger"#);
        assert_err_matches_regex!(try_tokenize("0x"), r#"InvalidInteger"#);

        Ok(())
    }

    #[test]
    fn invalid_integer() -> Result<()> {
        assert_err_matches_regex!(try_tokenize("04y"), r#"UnparsableInteger.*Digit"#);
        assert_err_matches_regex!(try_tokenize("0b12"), r#"UnparsableInteger.*Digit"#);
        assert_err_matches_regex!(try_tokenize("0xAZ"), r#"UnparsableInteger.*Digit"#);

        assert_err_matches_regex!(
            try_tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFF"),
            r#"UnparsableInteger.*Overflow"#
        );

        Ok(())
    }

    #[test]
    fn multiline() -> Result<()> {
        snapshot!(
            try_tokenize_full(
                "1234
(
\t( 456 )
  [\"abc\")"
            ),
            r#"
[
    ResultAt(
        Ok(
            Integer(
                1234,
            ),
        ),
        (
            1,
            1,
        ),
    ),
    ResultAt(
        Ok(
            Newline,
        ),
        (
            1,
            5,
        ),
    ),
    ResultAt(
        Ok(
            LParen,
        ),
        (
            2,
            1,
        ),
    ),
    ResultAt(
        Ok(
            Newline,
        ),
        (
            2,
            2,
        ),
    ),
    ResultAt(
        Ok(
            LParen,
        ),
        (
            3,
            2,
        ),
    ),
    ResultAt(
        Ok(
            Integer(
                456,
            ),
        ),
        (
            3,
            4,
        ),
    ),
    ResultAt(
        Ok(
            RParen,
        ),
        (
            3,
            8,
        ),
    ),
    ResultAt(
        Ok(
            Newline,
        ),
        (
            3,
            9,
        ),
    ),
    ResultAt(
        Ok(
            LBracket,
        ),
        (
            4,
            3,
        ),
    ),
    ResultAt(
        Ok(
            String(
                "abc",
            ),
        ),
        (
            4,
            4,
        ),
    ),
    ResultAt(
        Ok(
            RParen,
        ),
        (
            4,
            9,
        ),
    ),
    ResultAt(
        Err(
            Eof {
                source: Eof,
            },
        ),
        (
            4,
            10,
        ),
    ),
]
"#
        );

        Ok(())
    }
}
