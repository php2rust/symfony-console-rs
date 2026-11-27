//! Port of symfony/string's `SpanishInflectorTest`, on upstream's provider
//! rows as the oracle returns them (`tests/data/spanish_inflector_test.json`,
//! `scripts/record.sh string`).
//!
//! Upstream: symfony/string Tests/Inflector/SpanishInflectorTest.php
#![allow(non_snake_case)]

mod common;

use common::{Arg, provider};
use symfony_string::{Inflector, SpanishInflector};

const PROVIDERS: &str = include_str!("data/spanish_inflector_test.json");

/// `\is_array($x) ? $x : [$x]`.
fn forms(arg: &Arg) -> Vec<Vec<u8>> {
    match arg {
        Arg::Array(_) => arg.values().iter().map(Arg::bytes).collect(),
        other => vec![other.bytes()],
    }
}

#[test]
fn testSingularize() {
    for (name, args) in provider(PROVIDERS, "testSingularize") {
        assert_eq!(
            forms(&args[1]),
            SpanishInflector::new()
                .singularize(&args[0].bytes())
                .unwrap(),
            "row {name}"
        );
    }
}

#[test]
fn testPluralize() {
    for (name, args) in provider(PROVIDERS, "testPluralize") {
        assert_eq!(
            forms(&args[1]),
            SpanishInflector::new().pluralize(&args[0].bytes()).unwrap(),
            "row {name}"
        );
    }
}

#[test]
fn testUninflected() {
    for (name, args) in provider(PROVIDERS, "testUninflected") {
        let word = args[0].bytes();
        assert_eq!(
            vec![word.clone()],
            SpanishInflector::new().pluralize(&word).unwrap(),
            "row {name}"
        );
    }
}
