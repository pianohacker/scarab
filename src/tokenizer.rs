/*
 * Copyright (C) 2015 Jesse Weaver <pianohacker@gmail.com>
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 3 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin St, Fifth Floor, Boston, MA  02110-1301  USA
 */

//# Tokenizer

//## Imports
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Bytes, Cursor, ErrorKind, Read};
use std::iter::Peekable;
use unicode_reader::CodePoints;

//## Types
//
//### `TokenContents`
//
// This describes the actual contents of the token; Scarab has a fairly simple token set,
// consisting of a few reserved characters and numbers, identifiers and strings.
#[derive(Debug, PartialEq)]
pub enum TokenContents {
    Quote,
    Comma,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Newline,
    Integer(i64),
    Real(f64),
    Identifier(String),
    Str(String),
    // Should _never_ leave the inner tokenization loop
    NoMatch
}

//### `Token`
//
// A particular token, with contents, a line and column, and byte position within the source.
#[derive(Debug, PartialEq)]
pub struct Token {
    contents: TokenContents,
    position: usize,
    line: usize,
    col: usize
}

//### `TokenErrorKind`, `TokenError`
// Describe errors during tokenization, including lower level I/O errors.
#[derive(Debug)]
pub enum TokenErrorKind {
    IO(ErrorKind, String),
    IncompleteInput(String),
    InvalidChar(char, String)
}

#[derive(Debug)]
pub struct TokenError {
    kind: TokenErrorKind,

    position: usize,
    line: usize,
    col: usize
}

//### `Tokenizer`
//
// A tokenization state object; should be treated as opaque except for the filename field.
pub struct Tokenizer<R: Read> {
    pub filename: String,

    reader: Peekable<CodePoints<Bytes<R>>>,

    done: bool,
    position: usize,
    line: usize,
    col: usize
}

// Tokenizers can be created either directly from a string (in which case the desired "filename"
// should be supplied)...
impl<'a> Tokenizer<Cursor<&'a str>> {
    pub fn new(filename: &str, chars: &'a str) -> Tokenizer<Cursor<&'a str>> {
        Tokenizer {
            filename: String::from(filename),
            reader: CodePoints::from(Cursor::new(chars)).peekable(),
            done: false,
            position: 0,
            line: 1,
            col: 1
        }
    }
}

// or from a file.
impl Tokenizer<File> {
    pub fn new_from_file(filename: &str) -> io::Result<Tokenizer<File>> {
        Ok(Tokenizer {
            filename: String::from(filename),
            reader: CodePoints::from(try!(File::open(filename))).peekable(),
            done: false,
            position: 0,
            line: 1,
            col: 1
        })
    }
}

//## Tokenization
//
// This tokenizer is a simple iterator.
type TokenResult = Result<Token, TokenError>;

impl<R: Read> Iterator for Tokenizer<R> {
    type Item = TokenResult;

    fn next(&mut self) -> Option<TokenResult> {
        if self.done { return None; }

        use self::TokenContents::*;

        //### Convenience macros
        //
        // Constructs and returns an error at the current location.
        macro_rules! error {
            ($error_kind:expr) => {{
                use self::TokenErrorKind::*;
                self.done = true;

                return Some(Err(TokenError {
                    kind: $error_kind,
                    position: self.position,
                    line: self.line,
                    col: self.col
                }));
            }};
        }

        //
        // Peeks at the next character, automatically failing if an I/O error is encountered.
        macro_rules! peek {
            () => {
                match self.reader.peek() {
                    Some(result) => {
                        match result {
                            &Err(ref error) => {
                                // We have to manually copy over parts of the error, because we're
                                // only given a reference.
                                error!(IO(error.kind(), String::from(error.description())));
                            },
                            &Ok(ch) => {
                                Some(ch)
                            }
                        }
                    },
                    None => None,
                }
            };
        }

        // Peeks at the next character and consumes/returns it if it matches the provided expression.
        macro_rules! next_if {
            (|$ch2:ident| -> $guard:expr) => {
                match peek!() {
                    Some($ch2) => {
                        if $guard {
                            consume!();
                            Some($ch2)
                        } else {
                            None
                        }
                    },
                    None => None
                }
            };
        }

        // Grabs the next character and advances.
        macro_rules! next {
            () => {
                match self.reader.next() {
                    Some(result) => {
                        match result {
                            Err(ref error) => {
                                error!(IO(error.kind(), String::from(error.description())));
                            },
                            Ok(ch) => {
                                // We have to advance our counters when we actually consume a
                                // character; the original position of the token is saved at the
                                // start of `next()`.
                                if ch == '\n' {
                                    self.line += 1;
                                    self.col = 1;
                                } else {
                                    self.col += 1;
                                }
                                self.position += ch.len_utf8();
                                Some(ch)
                            }
                        }
                    },
                    None => None,
                }
            };
        }

        // Advances (usually used when we know the character from peek!).
        macro_rules! consume {
            () => {
                next!();
            };
        }

        // Advances a character and gives back a single-character token of the given type.
        macro_rules! yield_char {
            ($token_type:ident) => {{
                let token = Some(Ok(Token { contents: $token_type, position: self.position, line: self.line, col: self.col }));
                consume!();
                return token;
            }};
        }

        // Creates a token at the current position.
        macro_rules! make_token {
            ($token_type:ident, $( $args:expr ),* ) => {
                Token { contents: $token_type( $( $args ),* ), position: self.position, line: self.line, col: self.col }
            };
        }

        // Checks to see if a value is in a given set.
        macro_rules! val_in {
            ($val:expr, $( $set:expr ),+ ) => {
                $( $val == $set )||+
            };
        }

        let position = self.position;
        let line = self.line;
        let col = self.col;

        let mut token_contents = NoMatch;

        while let Some(ch) = peek!() {
            consume!();

            match ch {
                // Ignore whitespace and allow the loop to continue.
                ' ' | '\t' => { continue },
                // There are several base characters that go through the tokenizer unmolested.
                '\'' => { token_contents = Quote },
                ',' => { token_contents = Comma },
                '{' => { token_contents = LBrace },
                '}' => { token_contents = RBrace },
                '[' => { token_contents = LBracket },
                ']' => { token_contents = RBracket },
                '(' => { token_contents = LParen },
                ')' => { token_contents = RParen },
                '\n' => { token_contents = Newline },
                // Normal strings can contain escapes (\n, \r, \t, \" or \\), and must be on a
                // single line.
                '"' => {
                    let mut contents = String::new();

                    while let Some(next_ch) = next_if!(|ch2| -> ch2 != '"') {
                        if next_ch == '\n' {
                            error!(InvalidChar('\n', String::from("unexpected newline in string")));
                        } else if next_ch == '\\' {
                            if let Some(next_ch) = next!() {
                                contents.push(match next_ch {
                                    '\\' => '\\',
                                    '"' => '"',
                                    'n' => '\n',
                                    'r' => '\r',
                                    't' => '\t',
                                    _ => { error!(InvalidChar(next_ch, String::from("unknown escaped character"))) },
                                });
                            } else {
                                error!(IncompleteInput(String::from("expected character after \\ in string")));
                            }
                        } else {
                            contents.push(next_ch);
                        }
                    }

                    if let None = next!() {
                        error!(IncompleteInput(String::from("expected \" to end string")));
                    }

                    token_contents = Str(contents);
                },
                // Backquoted strings can span multiple lines, and leave any escaped characters as
                // is (including \`).
                '`' => {
                    let mut contents = String::new();

                    while let Some(next_ch) = next_if!(|ch2| -> ch2 != '`') {
                        if next_ch == '\\' {
                            if let Some(next_ch) = next!() {
                                contents.push('\\');
                                contents.push(next_ch);
                            } else {
                                error!(IncompleteInput(String::from("expected character after \\ in string")));
                            }
                        } else {
                            contents.push(next_ch);
                        }
                    }

                    if let None = next!() {
                        error!(IncompleteInput(String::from("expected ` to end string")));
                    }

                    token_contents = Str(contents);
                },
                _ if ch == '-' || ch.is_digit(10) => {
                    let mut contents = String::new();
                    contents.push(ch);

                    while let Some(next_ch) = next_if!(|ch2| -> ch2.is_digit(10)) {
                        contents.push(next_ch);
                    }

                    if let Some(dot_ch) = next_if!(|ch2| -> ch2 == '.') {
                        contents.push(dot_ch);

                        while let Some(next_ch) = next_if!(|ch2| -> ch2.is_digit(10)) {
                            contents.push(next_ch);
                        }

                        token_contents = Real(contents.parse::<f64>().unwrap())
                    } else {
                        token_contents = Integer(contents.parse::<i64>().unwrap())
                    }
                },
                _ => {
                    let mut contents = String::new();
                    contents.push(ch);

                    while let Some(next_ch) = next_if!(|ch2| -> !val_in!(ch2, '\'', ',', '{', '}', '[', ']', '(', ')', '\n', ' ', '\t', '"', '`')) {
                        contents.push(next_ch);
                    }

                    token_contents = Identifier(contents)
                }
            }

            break;
        }

        if let NoMatch = token_contents {
            self.done = true;
            None
        } else {
            Some(Ok(Token { contents: token_contents, position: position, line: line, col: col }))
        }
    }
}

#cfg(test)
mod tests {

}
