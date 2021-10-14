// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[must_use]
pub struct TakeWhileUngreedyImpl<'a, T, I: Iterator<Item = T>, P> {
    input: &'a mut std::iter::Peekable<I>,
    predicate: P,
}

pub trait TakeWhileUngreedy<T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> {
    fn take_while_ungreedy(&mut self, predicate: P) -> TakeWhileUngreedyImpl<T, I, P>;
}

impl<T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> TakeWhileUngreedy<T, I, P>
    for std::iter::Peekable<I>
{
    fn take_while_ungreedy(&mut self, predicate: P) -> TakeWhileUngreedyImpl<T, I, P> {
        TakeWhileUngreedyImpl {
            input: self,
            predicate,
        }
    }
}

impl<'a, T, I: Iterator<Item = T>, P: FnMut(&T) -> bool> Iterator
    for TakeWhileUngreedyImpl<'a, T, I, P>
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
