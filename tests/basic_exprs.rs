mod common;

use k9::snapshot;

#[test]
fn basic_addition() {
    snapshot!(common::exec("debug [1 + 2 + [6 - 3]]"), "6");
}
