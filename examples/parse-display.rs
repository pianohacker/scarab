// Copyright (c) Jesse Weaver, 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use scarab::parser::parse_value;

use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut line = String::new();
    print!("> ");
    stdout.flush();
    while let Ok(..) = stdin.read_line(&mut line) {
        if line == "" {
            println!();
            break;
        }

        match parse_value(line.chars()) {
            Ok(value) => println!("{}", value),
            Err(e) => println!("parsing failed: {}", e),
        }

        line.clear();
        print!("> ");
        stdout.flush();
    }
}
