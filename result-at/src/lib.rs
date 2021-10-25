// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![feature(try_trait_v2)]

use thiserror::Error;

pub trait ResultAtReader: Sized {
    type Output;
    type Error: std::error::Error;

    fn read_with_position(&mut self) -> ResultAt<Self::Output, Self::Error>;

    fn with_positions(self) -> ResultAtInput<Self> {
        ResultAtInput {
            reader: self,
            peeked: None,
        }
    }
}

pub fn label_chars<I: Iterator<Item = char>>(input: I) -> ResultAtCharReader<I> {
    ResultAtCharReader {
        input,
        line: 1,
        column: 1,
    }
}

pub struct ResultAtCharReader<I: Iterator<Item = char>> {
    input: I,
    line: usize,
    column: usize,
}

#[derive(Copy, Clone, Error, Debug, PartialEq, Eq)]
pub enum ResultAtCharReaderError {
    #[error("EOF")]
    Eof,
}

impl<I: Iterator<Item = char>> ResultAtReader for ResultAtCharReader<I> {
    type Output = char;
    type Error = ResultAtCharReaderError;

    fn read_with_position(&mut self) -> ResultAt<char, Self::Error> {
        match self.input.next() {
            None => ResultAt(Err(ResultAtCharReaderError::Eof), (self.line, self.column)),
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

pub struct ResultAtInput<R: ResultAtReader> {
    reader: R,
    peeked: Option<ResultAt<R::Output, R::Error>>,
}

impl<R: ResultAtReader> ResultAtInput<R> {
    pub fn next(&mut self) -> ResultAt<R::Output, R::Error> {
        if let Some(x) = self.peeked.take() {
            return x;
        }

        self.reader.read_with_position()
    }

    pub fn peek(&mut self) -> &ResultAt<R::Output, R::Error> {
        let reader = &mut self.reader;

        self.peeked
            .get_or_insert_with(|| reader.read_with_position())
    }

    pub fn items_while_successful(&mut self) -> impl Iterator<Item = R::Output> + '_ {
        std::iter::from_fn(move || {
            if let ResultAt(Ok(_), _) = self.peek() {
                return Some(self.next().0.unwrap());
            }

            None
        })
    }

    pub fn items_while_successful_if<'a>(
        &'a mut self,
        mut predicate: impl FnMut(&R::Output) -> bool + 'a,
    ) -> impl Iterator<Item = R::Output> + 'a {
        std::iter::from_fn(move || {
            if let ResultAt(Ok(x), _) = self.peek() {
                if (predicate)(&x) {
                    return Some(self.next().0.unwrap());
                }
            }

            None
        })
    }

    pub fn iter(&mut self) -> impl Iterator<Item = ResultAt<R::Output, R::Error>> + '_ {
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

    pub fn iter_results(&mut self) -> impl Iterator<Item = Result<R::Output, R::Error>> + '_ {
        self.iter().map(|x| x.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResultAt<T, E>(pub Result<T, E>, pub (usize, usize));

impl<T, E> ResultAt<T, E> {
    pub fn and_then<O, E2: From<E>>(self, op: impl FnOnce(T) -> Result<O, E2>) -> ResultAt<O, E2> {
        match self {
            Self(Ok(x), at) => ResultAt(op(x), at),
            Self(Err(e), at) => ResultAt(Err(e.into()), at),
        }
    }

    pub fn and_then_at<O, E2: From<E>>(
        self,
        op: impl FnOnce(T, (usize, usize)) -> ResultAt<O, E2>,
    ) -> ResultAt<O, E2> {
        match self {
            Self(Ok(x), at) => op(x, at),
            Self(Err(e), at) => ResultAt(Err(e.into()), at),
        }
    }

    pub fn as_ref(&self) -> ResultAt<&T, &E> {
        match *self {
            Self(Ok(ref x), at) => ResultAt(Ok(x), at),
            Self(Err(ref e), at) => ResultAt(Err(e), at),
        }
    }

    pub fn map<O>(self, op: impl FnOnce(T) -> O) -> ResultAt<O, E> {
        ResultAt(self.0.map(op), self.1)
    }

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

    struct TestReader {
        next: usize,
        fails_at: usize,
    }

    fn test_reader() -> TestReader {
        TestReader {
            next: 0,
            fails_at: usize::MAX,
        }
    }

    fn test_reader_fails_at(fails_at: usize) -> TestReader {
        TestReader { next: 0, fails_at }
    }

    impl ResultAtReader for TestReader {
        type Output = usize;
        type Error = TestError;

        fn read_with_position(&mut self) -> RA<Self::Output, Self::Error> {
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
        let mut input = test_reader().with_positions();

        assert_eq!(input.next(), RA(Ok(1), (0, 1)));
        assert_eq!(input.peek(), &RA(Ok(2), (0, 2)));
        assert_eq!(input.next(), RA(Ok(2), (0, 2)));
    }

    #[test]
    fn iter_gives_result_ats() {
        assert_eq!(
            test_reader_fails_at(4)
                .with_positions()
                .iter()
                .collect::<Vec<_>>(),
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
            test_reader_fails_at(4)
                .with_positions()
                .iter_results()
                .collect::<Vec<_>>(),
            vec![Ok(1), Ok(2), Ok(3), Err(TestError {})]
        );
    }

    #[test]
    fn items_while_successful_gives_inner_values() {
        assert_eq!(
            test_reader_fails_at(4)
                .with_positions()
                .items_while_successful()
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn items_while_successful_if_gives_inner_values() {
        assert_eq!(
            test_reader_fails_at(4)
                .with_positions()
                .items_while_successful_if(|x| *x < 3)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn items_while_successful_if_leaves_peeked() {
        let mut input = test_reader_fails_at(4).with_positions();
        input.items_while_successful_if(|x| *x < 3).for_each(drop);

        assert_eq!(input.next(), RA(Ok(3), (0, 3)));
    }

    #[test]
    fn try_gives_contents_on_success() {
        assert_eq!(
            (|| {
                let (val, at) = test_reader_fails_at(2).with_positions().next()?;

                ResultAt::<_, TestError>(Ok(format!("-> {}", val)), at)
            })(),
            RA(Ok("-> 1".to_string()), (0, 1))
        );
    }

    #[test]
    fn try_gives_result_at_on_failure() {
        assert_eq!(
            (|| -> ResultAt<(), TestError> {
                test_reader_fails_at(1).with_positions().next()?;

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
