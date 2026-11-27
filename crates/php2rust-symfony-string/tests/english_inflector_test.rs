//! Port of symfony/string's `EnglishInflectorTest`, on upstream's provider
//! rows as the oracle returns them (`tests/data/english_inflector_test.json`,
//! `scripts/record.sh string`).
//!
//! Upstream: symfony/string Tests/Inflector/EnglishInflectorTest.php
#![allow(non_snake_case)]

mod common;

use common::{Arg, provider};
use symfony_string::{EnglishInflector, Inflector};

const PROVIDERS: &str = include_str!("data/english_inflector_test.json");

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
        let plural = args[0].bytes();
        assert_eq!(
            forms(&args[1]),
            EnglishInflector::new().singularize(&plural).unwrap(),
            "row {name}: {}",
            String::from_utf8_lossy(&plural)
        );
    }
}

#[test]
fn testPluralize() {
    for (name, args) in provider(PROVIDERS, "testPluralize") {
        let singular = args[0].bytes();
        assert_eq!(
            forms(&args[1]),
            EnglishInflector::new().pluralize(&singular).unwrap(),
            "row {name}: {}",
            String::from_utf8_lossy(&singular)
        );
    }
}

#[test]
fn testPluralizeEmptyString() {
    let plural = EnglishInflector::new().pluralize(b"").unwrap();
    assert_eq!(vec![b"".to_vec()], plural);
}

#[test]
fn testSingularizeEmptyString() {
    let singular = EnglishInflector::new().singularize(b"").unwrap();
    assert_eq!(vec![b"".to_vec()], singular);
}
