// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. .0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/.0/.

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

pub type Position = (usize, usize);

/// The `Source` trait allows for reading items with position information.
///
/// Can be used directly with [`next()`](Self::next), but [`reader()`](Self::reader) allows use of many utility methods.
pub trait Source: Sized {
    type Output;
    type Error;

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
            None => NoneAt((self.line, self.column)),
            Some(c) => {
                let result = OkAt(c, (self.line, self.column));

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

/// Create a [`Source`] from a function.
///
/// The function will be called repeatedly, and should produce a [`ResultAt<T, E>`].
///
/// # Examples
///
/// ```
/// # use result_at::*;
/// use std::convert::Infallible;
/// let mut once = false;
/// let mut reader = source_from_fn(|| -> ResultAt<_, Infallible> {
///     if once {
///         NoneAt((1, 2))
///     } else {
///         once = true;
///         OkAt(true, (1, 1))
///     }
/// }).reader();
/// assert_eq!(reader.next(), OkAt(true, (1, 1)));
/// assert_eq!(reader.next(), NoneAt((1, 2)));
///
/// let mut once = false;
/// let mut reader = source_from_fn(|| {
///     if once {
///         ErrAt("failed!", (1, 2))
///     } else {
///         once = true;
///         OkAt(true, (1, 1))
///     }
/// }).reader();
/// assert_eq!(reader.next(), OkAt(true, (1, 1)));
/// assert_eq!(reader.next(), ErrAt("failed!", (1, 2)));
/// ```
pub fn source_from_fn<T, E>(
    next_op: impl FnMut() -> ResultAt<T, E>,
) -> impl Source<Output = T, Error = E> {
    SourceFromFn { next_op }
}

struct SourceFromFn<T, E, O: FnMut() -> ResultAt<T, E>> {
    next_op: O,
}

impl<T, E, O: FnMut() -> ResultAt<T, E>> Source for SourceFromFn<T, E, O> {
    type Output = T;
    type Error = E;

    fn next(&mut self) -> ResultAt<T, E> {
        (self.next_op)()
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

    /// Returns an [`Iterator`] that yields all items up to the first error or `NoneAt`.
    pub fn items_while_successful(&mut self) -> impl Iterator<Item = S::Output> + '_ {
        std::iter::from_fn(move || {
            if let OkAt(_, _) = self.peek() {
                return Some(self.next().unwrap_or_else(|| unreachable!()).0);
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
            if let OkAt(x, _) = self.peek() {
                if (predicate)(&x) {
                    return Some(self.next().unwrap_or_else(|| unreachable!()).0);
                }
            }

            None
        })
    }

    /// Returns an [`Iterator`] that yields all [`ResultAt`]s up to and including the first `ErrAt` or `NoneAt`.
    pub fn iter(&mut self) -> impl Iterator<Item = ResultAt<S::Output, S::Error>> + '_ {
        let mut stopped = false;

        std::iter::from_fn(move || {
            if stopped {
                return None;
            }

            match self.next() {
                result @ OkAt(_, _) => Some(result),
                result @ (ErrAt(_, _) | NoneAt(_)) => {
                    stopped = true;
                    Some(result)
                }
            }
        })
    }

    /// Returns an [`Iterator`] that yields all [`Result`]s up to the first ErrAt or NoneAt.
    pub fn iter_results(&mut self) -> impl Iterator<Item = Result<S::Output, S::Error>> + '_ {
        self.iter().filter_map(|r| match r {
            OkAt(x, _) => Some(Ok(x)),
            ErrAt(e, _) => Some(Err(e)),
            NoneAt(_) => None,
        })
    }
}

/// A [`Result`] with line/column position information.
///
/// Supports `?`, returning `(T, Position)` on success.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResultAt<T, E> {
    OkAt(T, Position),
    ErrAt(E, Position),
    NoneAt(Position),
}

pub use ResultAt::*;

#[must_use]
impl<T, E> ResultAt<T, E> {
    /// Passes the contained value to the given closure on [`OkAt`], passing through [`ErrAt`] and
    /// [`NoneAt`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use result_at::*;
    /// assert_eq!(
    ///     OkAt::<_, &str>(42, (1, 1)).and_then(|x| Ok(x * 2)),
    ///     OkAt::<_, &str>(84, (1, 1))
    /// );
    /// assert_eq!(
    ///     OkAt::<_, &str>(42, (1, 1)).and_then(|_| -> Result<usize, &str> {
    ///         Err("oh no")
    ///     }),
    ///     ErrAt::<_, &str>("oh no", (1, 1))
    /// );
    /// assert_eq!(
    ///     NoneAt::<_, &str>((1, 1)).and_then(|x: usize| -> Result<usize, &str> {
    ///         Ok(x * 2)
    ///     }),
    ///     NoneAt::<_, &str>((1, 1))
    /// );
    /// ```
    pub fn and_then<O, E2: From<E>>(self, op: impl FnOnce(T) -> Result<O, E2>) -> ResultAt<O, E2> {
        match self {
            OkAt(x, at) => match op(x) {
                Ok(x) => OkAt(x, at),
                Err(e) => ErrAt(e, at),
            },
            ErrAt(e, at) => ErrAt(e.into(), at),
            NoneAt(at) => NoneAt(at),
        }
    }

    /// Similar to [`and_then()`](Self::and_then), but passes the location to the closure and expects a
    /// [`ResultAt`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use result_at::*;
    /// assert_eq!(
    ///     OkAt::<_, &str>(42, (1, 1)).and_then_at(|x, _| OkAt(x * 2, (4, 4))),
    ///     OkAt::<_, &str>(84, (4, 4))
    /// );
    /// assert_eq!(
    ///     OkAt::<_, &str>(42, (1, 1)).and_then_at(|_, at| -> ResultAt<usize, &str> {
    ///         ErrAt("oh no", at)
    ///     }),
    ///     ErrAt::<_, &str>("oh no", (1, 1))
    /// );
    /// assert_eq!(
    ///     NoneAt::<_, &str>((1, 1)).and_then_at(|_: usize, _| -> ResultAt<usize, &str> {
    ///         unreachable!();
    ///     }),
    ///     NoneAt::<_, &str>((1, 1))
    /// );
    /// ```
    pub fn and_then_at<O, E2: From<E>>(
        self,
        op: impl FnOnce(T, Position) -> ResultAt<O, E2>,
    ) -> ResultAt<O, E2> {
        match self {
            OkAt(x, at) => op(x, at),
            ErrAt(e, at) => ErrAt(e.into(), at),
            NoneAt(at) => NoneAt(at),
        }
    }

    /// Returns a [`ResultAt`] with a reference to the contained `T` or `E`.
    pub fn as_ref(&self) -> ResultAt<&T, &E> {
        match *self {
            OkAt(ref x, at) => OkAt(x, at),
            ErrAt(ref e, at) => ErrAt(e, at),
            NoneAt(at) => NoneAt(at),
        }
    }

    /// Wraps [`Result::map()`] for the contained result.
    pub fn map<O>(self, op: impl FnOnce(T) -> O) -> ResultAt<O, E> {
        match self {
            OkAt(x, at) => OkAt(op(x), at),
            ErrAt(e, at) => ErrAt(e, at),
            NoneAt(at) => NoneAt(at),
        }
    }

    /// Wraps [`Result::map_err()`] for the contained result.
    pub fn map_err<O>(self, op: impl FnOnce(E) -> O) -> ResultAt<T, O> {
        match self {
            OkAt(x, at) => OkAt(x, at),
            ErrAt(e, at) => ErrAt(op(e), at),
            NoneAt(at) => NoneAt(at),
        }
    }

    /// Returns the contained `OkAt` value and position or panics.
    pub fn unwrap(self) -> (T, Position)
    where
        E: std::fmt::Debug,
    {
        match self {
            OkAt(x, at) => (x, at),
            ErrAt(e, at) => panic!(
                "called unwrap on a ResultAt containing ErrAt({:?}, {:?})",
                e, at
            ),
            NoneAt(at) => panic!("called unwrap on a ResultAt containing NoneAt({:?})", at),
        }
    }

    /// Returns the contained `OkAt` value or computes it from a closure.
    pub fn unwrap_or_else(self, op: impl FnOnce() -> T) -> (T, Position) {
        match self {
            OkAt(x, at) => (x, at),
            ErrAt(_, at) => (op(), at),
            NoneAt(at) => (op(), at),
        }
    }

    /// Unfolds into a `Result<Option<T>, E>`, dropping position information.
    pub fn unfold_contents(self) -> Result<Option<T>, E> {
        match self {
            OkAt(x, _) => Ok(Some(x)),
            ErrAt(e, _) => Err(e),
            NoneAt(_) => Ok(None),
        }
    }

    /// Returns an ok/erroring `ResultAt` from the given `Result` with the given position.
    pub fn from_result<E2: Into<E>>(result: Result<T, E2>, at: Position) -> Self {
        match result {
            Ok(x) => OkAt(x, at),
            Err(e) => ErrAt(e.into(), at),
        }
    }

    /// Maps `NoneAt` to the given value.
    pub fn none_as_value(self, default: T) -> Self {
        match self {
            result @ (OkAt(_, _) | ErrAt(_, _)) => result,
            NoneAt(at) => OkAt(default, at),
        }
    }

    /// Maps `NoneAt` to the given error.
    pub fn none_as_err<E2: From<E>>(self, e: E2) -> ResultAt<T, E2> {
        match self {
            OkAt(x, at) => OkAt(x, at),
            ErrAt(e, at) => ErrAt(e.into(), at),
            NoneAt(at) => ErrAt(e, at),
        }
    }

    /// Runs the given function on `OkAt`, defaulting to the given value otherwise.
    pub fn map_or<O>(self, default: O, op: impl FnOnce(T) -> O) -> O {
        match self {
            OkAt(x, _) => op(x),
            ErrAt(_, _) => default,
            NoneAt(_) => default,
        }
    }
}

impl<T, E: From<E2>, E2> std::ops::FromResidual<ResultAt<std::convert::Infallible, E2>>
    for ResultAt<T, E>
{
    fn from_residual(residual: ResultAt<std::convert::Infallible, E2>) -> Self {
        match residual {
            OkAt(_, _) => unreachable!(),
            ErrAt(e, at) => ErrAt(e.into(), at),
            NoneAt(at) => NoneAt(at),
        }
    }
}

impl<T, E> std::ops::Try for ResultAt<T, E> {
    type Output = (T, Position);
    type Residual = ResultAt<std::convert::Infallible, E>;

    fn from_output(output: Self::Output) -> Self {
        let (x, at) = output;

        OkAt(x, at)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            OkAt(x, at) => std::ops::ControlFlow::Continue((x, at)),
            ErrAt(e, at) => std::ops::ControlFlow::Break(ErrAt(e, at)),
            NoneAt(at) => std::ops::ControlFlow::Break(NoneAt(at)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ends_at: usize,
    }

    fn test2_source() -> TestSource {
        TestSource {
            next: 0,
            fails_at: usize::MAX,
            ends_at: usize::MAX,
        }
    }

    fn test2_source_fails_at(fails_at: usize) -> TestSource {
        TestSource {
            next: 0,
            fails_at,
            ends_at: fails_at + 1,
        }
    }

    fn test2_source_ends_at(ends_at: usize) -> TestSource {
        TestSource {
            next: 0,
            fails_at: usize::MAX,
            ends_at,
        }
    }

    impl Source for TestSource {
        type Output = usize;
        type Error = TestError;

        fn next(&mut self) -> ResultAt<Self::Output, Self::Error> {
            self.next += 1;

            if self.next == self.fails_at {
                ErrAt(TestError {}, (0, self.next))
            } else if self.next == self.ends_at {
                NoneAt((0, self.next))
            } else {
                OkAt(self.next, (0, self.next))
            }
        }
    }

    #[test]
    fn input_next_gives_peeked() {
        let mut input = test2_source().reader();

        assert_eq!(input.next(), OkAt(1, (0, 1)));
        assert_eq!(input.peek(), &OkAt(2, (0, 2)));
        assert_eq!(input.next(), OkAt(2, (0, 2)));
    }

    #[test]
    fn iter_gives_result_ats() {
        assert_eq!(
            test2_source_fails_at(4).reader().iter().collect::<Vec<_>>(),
            vec![
                OkAt(1, (0, 1)),
                OkAt(2, (0, 2)),
                OkAt(3, (0, 3)),
                ErrAt(TestError {}, (0, 4)),
            ]
        );

        assert_eq!(
            test2_source_ends_at(4).reader().iter().collect::<Vec<_>>(),
            vec![
                OkAt(1, (0, 1)),
                OkAt(2, (0, 2)),
                OkAt(3, (0, 3)),
                NoneAt((0, 4))
            ]
        );
    }

    #[test]
    fn iter_results_gives_inner_results() {
        assert_eq!(
            test2_source_fails_at(4)
                .reader()
                .iter_results()
                .collect::<Vec<_>>(),
            vec![Ok(1), Ok(2), Ok(3), Err(TestError {})]
        );

        assert_eq!(
            test2_source_ends_at(4)
                .reader()
                .iter_results()
                .collect::<Vec<_>>(),
            vec![Ok(1), Ok(2), Ok(3)]
        );
    }

    #[test]
    fn items_while_successful_gives_inner_values() {
        assert_eq!(
            test2_source_fails_at(4)
                .reader()
                .items_while_successful()
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn items_while_successful_if_gives_inner_values() {
        assert_eq!(
            test2_source_fails_at(4)
                .reader()
                .items_while_successful_if(|x| *x < 3)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn items_while_successful_if_leaves_peeked() {
        let mut input = test2_source_fails_at(4).reader();
        input.items_while_successful_if(|x| *x < 3).for_each(drop);

        assert_eq!(input.next(), OkAt(3, (0, 3)));
    }

    #[test]
    fn try_gives_contents_on_success() {
        assert_eq!(
            (|| { OkAt::<_, TestError>(test2_source().reader().next()?, (0, 1)) })(),
            OkAt((1, (0, 1)), (0, 1))
        );
    }

    #[test]
    fn try_gives_result_at_on_failure() {
        assert_eq!(
            (|| -> ResultAt<(), TestError> {
                test2_source_fails_at(1).reader().next()?;

                panic!();
            })(),
            ErrAt(TestError {}, (0, 1))
        );

        assert_eq!(
            (|| -> ResultAt<(), TestError> {
                test2_source_ends_at(1).reader().next()?;

                panic!();
            })(),
            NoneAt((0, 1))
        );
    }

    #[test]
    fn try_can_convert_error_on_failure() {
        assert_eq!(
            (|| -> ResultAt<(), TestError> {
                ErrAt::<(), _>(TestError2 {}, (1, 0))?;

                panic!();
            })(),
            ErrAt(TestError {}, (1, 0))
        );
    }
}
