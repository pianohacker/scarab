mod common;

use k9::snapshot;

#[test]
fn basic_math() {
    snapshot!(common::exec("debug [1 + 2 + [6 - 3]]"), "6");
}

#[test]
fn constant_if() {
    snapshot!(common::exec("if (< 1 2) {debug 1} {debug 2}"), "1");
}

#[test]
fn nested_if() {
    snapshot!(
        common::exec("if (< 1 2) {if (< 3 2) {debug 3} {debug 2}} {debug 1}"),
        "2"
    );
}
