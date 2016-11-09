extern crate scarab;

use scarab::tokenizer::Tokenizer;
use std::env;

fn main() {
    let chars = env::args().nth(1).unwrap();
    let tokenizer = Tokenizer::new("<string>", &chars);

    for item in tokenizer {
        match item {
            Err(e) => {
                println!("Error: {:?}", e);
            },
            Ok(token) => println!("Token: {:?}", token),
        }
    }
}
