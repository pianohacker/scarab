// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Wrapper type for [Result] with line/column information; useful for parsers and tokenizers.
//!
//! Implementing tokenizers as an [`Iterator`] adapter has a few problems:
//! * Each item needs position information, including errors. Embedding position information in
//!   errors isn't enough, as successful reads from an upstream source might cause downstream
//!   errors. For instance, a successfully read character might be an invalid character for a
//!   tokenizer. This means the [`Iterator`] has to yield either something like `(Result, Position)` or `Result<ItemWithPosition, ErrorWithPosition>`.
//! * Hitting the end of an iterator and hitting an error inside the iterator can often be handled
//!   the same way, but doing this with either option is tedious.
//! * Most syntaxes need the ability to peek ahead (during tokenizing and parsing).
//!   [`Iterator::peekable()`] allows this, but adds another layer of complexity.
//!
//! The [`Source`] trait (along with the utility methods in [`Reader`]) manage this with minimal
//! boilerplate.
#![feature(try_trait_v2)]

use thiserror::Error;

/// The `Source` trait allows for reading items with position information.
///
/// Can be used directly with [`next()`](Self::next), but [`reader()`](Self::reader) allows use of many utility methods.
pub trait Source: Sized {
    type Output;
    type Error: std::error::Error;

    fn next(&mut self) -> ResultAt<Self::Output, Self::Error>;

    fn reader(self) -> Reader<Self> {
        Reader {
            reader: self,
            peeked: None,
        }
    }
}

/// A [Source] that labels characters with line and column based on newlines.
pub struct CharSource<I: Iterator<Item = char>> {
    input: I,
    line: usize,
    column: usize,
}

impl<I: Iterator<Item = char>> CharSource<I> {
    pub fn new(input: I) -> Self {
        Self {
            input,
            line: 1,
            column: 1,
        }
    }
}

/// Errors produced by a [CharSource].
#[derive(Copy, Clone, Error, Debug, PartialEq, Eq)]
pub enum CharReaderError {
    #[error("EOF")]
    Eof,
}

impl<I: Iterator<Item = char>> Source for CharSource<I> {
    type Output = char;
    type Error = CharReaderError;

    fn next(&mut self) -> ResultAt<char, Self::Error> {
        match self.input.next() {
            None => ResultAt(Err(CharReaderError::Eof), (self.line, self.column)),
            Some(c) => {
                let result = ResultAt(Ok(c), (self.line, self.column));

                if c == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }

                result
            }
        }
    }
}

/// Utility wrapper around a [`Source`].
pub struct Reader<S: Source> {
    reader: S,
    peeked: Option<ResultAt<S::Output, S::Error>>,
}

impl<S: Source> Reader<S> {
    /// Fetch the next result from the source.
    ///
    /// Will take the last peeked item, if any.
    pub fn next(&mut self) -> ResultAt<S::Output, S::Error> {
        if let Some(x) = self.peeked.take() {
            return x;
        }

        self.reader.next()
    }

    /// Peek at the next item without consuming it.
    pub fn peek(&mut self) -> &ResultAt<S::Output, S::Error> {
        let reader = &mut self.reader;

        self.peeked.get_or_insert_with(|| reader.next())
    }

    /// Returns an [`Iterator`] that yields all items up to the first error.
    pub fn items_while_successful(&mut self) -> impl Iterator<Item = S::Output> + '_ {
        std::iter::from_fn(move || {
            if let ResultAt(Ok(_), _) = self.peek() {
                return Some(self.next().0.unwrap());
            }

            None
        })
    }

    /// Returns an [`Iterator`] that yields all items up to the first error which also match the
    /// given predicate.
    ///
    /// The final, non-matching item will not be consumed.
    pub fn items_while_successful_if<'a>(
        &'a mut self,
        mut predicate: impl FnMut(&S::Output) -> bool + 'a,
    ) -> impl Iterator<Item = S::Output> + 'a {
        std::iter::from_fn(move || {
            if let ResultAt(Ok(x), _) = self.peek() {
                if (predicate)(&x) {
                    return Some(self.next().0.unwrap());
                }
            }

            None
        })
    }

    /// Returns an [`Iterator`] that yields all [`ResultAt`]s up to and including the first error.
    pub fn iter(&mut self) -> impl Iterator<Item = ResultAt<S::Output, S::Error>> + '_ {
        let mut stopped = false;

        std::iter::from_fn(move || {
            if stopped {
                return None;
            }

            match self.next() {
                ResultAt(Ok(x), at) => Some(ResultAt(Ok(x), at)),
                ResultAt(Err(e), at) => {
                    stopped = true;
                    Some(ResultAt(Err(e), at))
                }
            }
        })
    }

    /// Returns an [`Iterator`] that yields all [`Result`]s up to and including the first error.
    pub fn iter_results(&mut self) -> impl Iterator<Item = Result<S::Output, S::Error>> + '_ {
        self.iter().map(|x| x.0)
    }
}

/// A [`Result`] with line/column position information.
///
/// Supports `?`, returning `(T, (usize, usize))` on success.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResultAt<T, E>(
    /// The contained [`Result`].
    pub Result<T, E>,
    /// The line and column.
    pub (usize, usize),
);

impl<T, E> ResultAt<T, E> {
    /// Wraps [`Result::and_then()`] for the contained result.
    pub fn and_then<O, E2: From<E>>(self, op: impl FnOnce(T) -> Result<O, E2>) -> ResultAt<O, E2> {
        match self {
            Self(Ok(x), at) => ResultAt(op(x), at),
            Self(Err(e), at) => ResultAt(Err(e.into()), at),
        }
    }

    /// Similar to [`and_then()`](Self::and_then), but passes the location to the closure and expects a
    /// [`ResultAt`].
    pub fn and_then_at<O, E2: From<E>>(
        self,
        op: impl FnOnce(T, (usize, usize)) -> ResultAt<O, E2>,
    ) -> ResultAt<O, E2> {
        match self {
            Self(Ok(x), at) => op(x, at),
            Self(Err(e), at) => ResultAt(Err(e.into()), at),
        }
    }

    /// Wraps [`Result::as_ref()`] for the contained result.
    pub fn as_ref(&self) -> ResultAt<&T, &E> {
        match *self {
            Self(Ok(ref x), at) => ResultAt(Ok(x), at),
            Self(Err(ref e), at) => ResultAt(Err(e), at),
        }
    }

    /// Wraps [`Result::map()`] for the contained result.
    pub fn map<O>(self, op: impl FnOnce(T) -> O) -> ResultAt<O, E> {
        ResultAt(self.0.map(op), self.1)
    }

    /// Wraps [`Result::map_err()`] for the contained result.
    pub fn map_err<O>(self, op: impl FnOnce(E) -> O) -> ResultAt<T, O> {
        ResultAt(self.0.map_err(op), self.1)
    }
}

impl<T, E: From<E2>, E2> std::ops::FromResidual<ResultAt<std::convert::Infallible, E2>>
    for ResultAt<T, E>
{
    fn from_residual(residual: ResultAt<std::convert::Infallible, E2>) -> Self {
        Self(Result::from_residual(residual.0), residual.1)
    }
}

impl<T, E> std::ops::Try for ResultAt<T, E> {
    type Output = (T, (usize, usize));
    type Residual = ResultAt<std::convert::Infallible, E>;

    fn from_output(output: Self::Output) -> Self {
        let (x, at) = output;

        Self(Ok(x), at)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            ResultAt(Ok(x), at) => std::ops::ControlFlow::Continue((x, at)),
            ResultAt(Err(e), at) => std::ops::ControlFlow::Break(ResultAt(Err(e), at)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ResultAt as RA, *};

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    struct TestError {}

    impl std::error::Error for TestError {}

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test error")
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    struct TestError2 {}

    impl std::error::Error for TestError2 {}

    impl std::fmt::Display for TestError2 {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test error")
        }
    }

    impl From<TestError2> for TestError {
        fn from(_: TestError2) -> TestError {
            TestError {}
        }
    }

    struct TestSource {
        next: usize,
        fails_at: usize,
    }

    fn test_source() -> TestSource {
        TestSource {
            next: 0,
            fails_at: usize::MAX,
        }
    }

    fn test_source_fails_at(fails_at: usize) -> TestSource {
        TestSource { next: 0, fails_at }
    }

    impl Source for TestSource {
        type Output = usize;
        type Error = TestError;

        fn next(&mut self) -> RA<Self::Output, Self::Error> {
            self.next += 1;

            if self.next == self.fails_at {
                RA(Err(TestError {}), (0, self.next))
            } else {
                RA(Ok(self.next), (0, self.next))
            }
        }
    }

    #[test]
    fn input_next_gives_peeked() {
        let mut input = test_source().reader();

        assert_eq!(input.next(), RA(Ok(1), (0, 1)));
        assert_eq!(input.peek(), &RA(Ok(2), (0, 2)));
        assert_eq!(input.next(), RA(Ok(2), (0, 2)));
    }

    #[test]
    fn iter_gives_result_ats() {
        assert_eq!(
            test_source_fails_at(4).reader().iter().collect::<Vec<_>>(),
            vec![
                RA(Ok(1), (0, 1)),
                RA(Ok(2), (0, 2)),
                RA(Ok(3), (0, 3)),
                RA(Err(TestError {}), (0, 4))
            ]
        );
    }

    #[test]
    fn iter_results_gives_inner_results() {
        assert_eq!(
            test_source_fails_at(4)
                .reader()
                .iter_results()
                .collect::<Vec<_>>(),
            vec![Ok(1), Ok(2), Ok(3), Err(TestError {})]
        );
    }

    #[test]
    fn items_while_successful_gives_inner_values() {
        assert_eq!(
            test_source_fails_at(4)
                .reader()
                .items_while_successful()
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn items_while_successful_if_gives_inner_values() {
        assert_eq!(
            test_source_fails_at(4)
                .reader()
                .items_while_successful_if(|x| *x < 3)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn items_while_successful_if_leaves_peeked() {
        let mut input = test_source_fails_at(4).reader();
        input.items_while_successful_if(|x| *x < 3).for_each(drop);

        assert_eq!(input.next(), RA(Ok(3), (0, 3)));
    }

    #[test]
    fn try_gives_contents_on_success() {
        assert_eq!(
            (|| {
                let (val, at) = test_source_fails_at(2).reader().next()?;

                ResultAt::<_, TestError>(Ok(format!("-> {}", val)), at)
            })(),
            RA(Ok("-> 1".to_string()), (0, 1))
        );
    }

    #[test]
    fn try_gives_result_at_on_failure() {
        assert_eq!(
            (|| -> ResultAt<(), TestError> {
                test_source_fails_at(1).reader().next()?;

                panic!();
            })(),
            RA(Err(TestError {}), (0, 1))
        );
    }

    #[test]
    fn try_can_convert_error_on_failure() {
        assert_eq!(
            (|| -> ResultAt<(), TestError> {
                ResultAt::<(), _>(Err(TestError2 {}), (1, 0))?;

                panic!();
            })(),
            RA(Err(TestError {}), (1, 0))
        );
    }
}