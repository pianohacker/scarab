// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use thiserror::Error;

use super::utils::{
    PositionLabeled, PositionLabeledCharsExt, StripPositionsExt, TakeWhileUngreedyExt,
};

#[derive(Clone, Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("unexpected character: {0}")]
    UnexpectedChar(PositionLabeled<char>),
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

type Result<T> = std::result::Result<T, Error>;

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
    input: std::iter::Peekable<super::utils::PositionLabeledChars<I>>,
    stopped: bool,
}

impl<I> Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    fn tokenize_string(&mut self) -> Result<String> {
        let result = self
            .input
            .take_while_ungreedy(|x| x.contents != '\"')
            .strip_positions()
            .collect();

        if let None = self.input.next() {
            return Err(Error::UnterminatedString);
        }

        Ok(result)
    }

    fn tokenize_identifier(&mut self, first_char: PositionLabeled<char>) -> Result<String> {
        Ok(std::iter::once(first_char.contents)
            .chain(
                self.input
                    .take_while_ungreedy(|x| {
                        !char_is_token_end(x.contents) && !x.contents.is_ascii_whitespace()
                    })
                    .strip_positions(),
            )
            .collect())
    }

    fn tokenize_integer(&mut self, mut first_char: PositionLabeled<char>) -> Result<isize> {
        let sign = if first_char.contents == '-' {
            first_char = self.input.next().unwrap_or_else(|| unreachable!());
            -1
        } else {
            1
        };

        let mut base = 10;

        if first_char.contents == '0' {
            match self.input.peek().map(|x| x.contents) {
                Some('b') => {
                    base = 2;
                    self.input.next();
                    first_char = self.input.next().ok_or(Error::InvalidInteger)?;
                }
                Some('x') => {
                    base = 16;
                    self.input.next();
                    first_char = self.input.next().ok_or(Error::InvalidInteger)?;
                }
                _ => {}
            }
        }

        let s = std::iter::once(first_char.contents)
            .chain(
                self.input
                    .take_while_ungreedy(|x| !char_is_token_end(x.contents))
                    .strip_positions(),
            )
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
    type Item = Result<PositionLabeled<Token>>;

    fn next(&mut self) -> Option<Self::Item> {
        use Token::*;

        if self.stopped {
            return None;
        }

        self.input
            .take_while_ungreedy(|x| x.is_ascii_whitespace() && x.contents != '\n')
            .for_each(drop);

        let result = self.input.next().map(|c| {
            match c.contents {
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
                _ => Err(Error::UnexpectedChar(c)),
            }
            .map(|t| c.label(t))
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
        input: input.into_iter().position_labeled_chars().peekable(),
        stopped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::{assert_err_matches_regex, snapshot};

    fn try_tokenize(input: &str) -> Result<Vec<Token>> {
        tokenize(input.chars())
            .map(|ir| ir.map(|i| i.contents))
            .collect()
    }

    fn try_tokenize_uncollapsed(input: &str) -> Vec<Result<Token>> {
        tokenize(input.chars())
            .map(|ir| ir.map(|i| i.contents))
            .collect()
    }

    fn try_tokenize_full(input: &str) -> Result<Vec<PositionLabeled<Token>>> {
        tokenize(input.chars()).collect()
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
            PositionLabeled {
                contents: '\u{7}',
                line: 1,
                column: 2,
            },
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
            )?,
            r#"
[
    PositionLabeled {
        contents: Integer(
            1234,
        ),
        line: 1,
        column: 1,
    },
    PositionLabeled {
        contents: Newline,
        line: 1,
        column: 5,
    },
    PositionLabeled {
        contents: LParen,
        line: 2,
        column: 1,
    },
    PositionLabeled {
        contents: Newline,
        line: 2,
        column: 2,
    },
    PositionLabeled {
        contents: LParen,
        line: 3,
        column: 2,
    },
    PositionLabeled {
        contents: Integer(
            456,
        ),
        line: 3,
        column: 4,
    },
    PositionLabeled {
        contents: RParen,
        line: 3,
        column: 8,
    },
    PositionLabeled {
        contents: Newline,
        line: 3,
        column: 9,
    },
    PositionLabeled {
        contents: LBracket,
        line: 4,
        column: 3,
    },
    PositionLabeled {
        contents: String(
            "abc",
        ),
        line: 4,
        column: 4,
    },
    PositionLabeled {
        contents: RParen,
        line: 4,
        column: 9,
    },
]
"#
        );

        Ok(())
    }
}
