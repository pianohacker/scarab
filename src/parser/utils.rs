// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[must_use]
pub struct TakeWhileUngreedy<'a, T, I: Iterator<Item = T>, P> {
    input: &'a mut std::iter::Peekable<I>,
    predicate: P,
}

pub trait TakeWhileUngreedyExt<T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> {
    fn take_while_ungreedy(&mut self, predicate: P) -> TakeWhileUngreedy<T, I, P>;
}

impl<T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> TakeWhileUngreedyExt<T, I, P>
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

#[derive(Debug, Eq, PartialEq)]
pub struct PositionLabeled<T> {
    pub contents: T,
    pub line: usize,
    pub column: usize,
}

impl<T> PositionLabeled<T> {
    pub fn label<T2>(&self, contents: T2) -> PositionLabeled<T2> {
        PositionLabeled {
            contents,
            line: self.line,
            column: self.column,
        }
    }
}

impl<T: std::marker::Copy> std::marker::Copy for PositionLabeled<T> {}

impl<T: std::clone::Clone> std::clone::Clone for PositionLabeled<T> {
    fn clone(&self) -> Self {
        Self {
            contents: self.contents.clone(),
            line: self.line,
            column: self.column,
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for PositionLabeled<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.contents)
    }
}

impl<T> std::ops::Deref for PositionLabeled<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.contents
    }
}

#[must_use]
pub struct PositionLabeledChars<I: Iterator<Item = char>> {
    input: I,
    line: usize,
    column: usize,
}

pub trait PositionLabeledCharsExt<I: Iterator<Item = char>> {
    fn position_labeled_chars(self) -> PositionLabeledChars<I>;
}

impl<I: Iterator<Item = char>> PositionLabeledCharsExt<I> for I {
    fn position_labeled_chars(self) -> PositionLabeledChars<I> {
        PositionLabeledChars {
            input: self,
            line: 1,
            column: 1,
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for PositionLabeledChars<I> {
    type Item = PositionLabeled<char>;

    fn next(&mut self) -> Option<Self::Item> {
        self.input.next().map(|c| {
            let result = PositionLabeled {
                contents: c,
                line: self.line,
                column: self.column,
            };

            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }

            result
        })
    }
}

#[must_use]
pub struct StripPositions<T, I: Iterator<Item = PositionLabeled<T>>> {
    input: I,
}

pub trait StripPositionsExt<T, I: Iterator<Item = PositionLabeled<T>>> {
    fn strip_positions(self) -> StripPositions<T, I>;
}

impl<T, I: Iterator<Item = PositionLabeled<T>>> StripPositionsExt<T, I> for I {
    fn strip_positions(self) -> StripPositions<T, I> {
        StripPositions { input: self }
    }
}

impl<T, I: Iterator<Item = PositionLabeled<T>>> Iterator for StripPositions<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.input.next().map(|c| c.contents)
    }
}
