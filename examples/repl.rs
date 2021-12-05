use std::io::{self, Write};

use scarab::compiler::compile;
use scarab::parser::parse_implicit_form_list;
use scarab::vm::Vm;

macro_rules! try_or_bail {
    ($expr:expr, $msg_prefix:expr $(,)?) => {
        match $expr {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{}: {}", $msg_prefix, e);
                return;
            }
        }
    };
}

pub fn run_line(code: &str, output: &mut impl Write) {
    let (program, positions) =
        try_or_bail!(parse_implicit_form_list(code.chars()), "parsing failed",);

    let instructions = try_or_bail!(compile(program, positions), "compilation failed");

    let mut vm = Vm::new(output);
    vm.load(instructions);
    try_or_bail!(vm.run(), "running failed");
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut line = String::new();
    print!("> ");
    stdout.flush().unwrap();
    while let Ok(..) = stdin.read_line(&mut line) {
        if line == "" {
            println!();
            break;
        }

        run_line(&line, &mut stdout);

        line.clear();
        print!("> ");
        stdout.flush().unwrap();
    }
}
