// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Nil,
    Integer(isize),
    String(String),
    Identifier(String),
    Cell(Rc<Value>, Rc<Value>),
    Quoted(Rc<Value>),
}
