//! Port of symfony/string's `FrenchInflectorTest`, on upstream's provider
//! rows as the oracle returns them (`tests/data/french_inflector_test.json`,
//! `scripts/record.sh string`).
//!
//! Upstream: symfony/string Tests/Inflector/FrenchInflectorTest.php
#![allow(non_snake_case)]

mod common;

use common::provider;
use symfony_string::{FrenchInflector, Inflector};

const PROVIDERS: &str = include_str!("data/french_inflector_test.json");

/// `ucfirst()`: the first byte in ASCII upper case.
fn ucfirst(s: &[u8]) -> Vec<u8> {
    let mut out = s.to_vec();
    if let Some(first) = out.first_mut() {
        first.make_ascii_uppercase();
    }
    out
}

#[test]
fn testSingularize() {
    for (name, args) in provider(PROVIDERS, "testSingularize") {
        let (singular, plural) = (args[0].bytes(), args[1].bytes());
        assert_eq!(
            vec![singular.clone()],
            FrenchInflector::new().singularize(&plural).unwrap(),
            "row {name}"
        );
        // test casing: if the first letter was uppercase, it should remain so
        assert_eq!(
            vec![ucfirst(&singular)],
            FrenchInflector::new()
                .singularize(&ucfirst(&plural))
                .unwrap(),
            "row {name}, ucfirst"
        );
    }
}

#[test]
fn testPluralize() {
    for (name, args) in provider(PROVIDERS, "testPluralize") {
        let (singular, plural) = (args[0].bytes(), args[1].bytes());
        assert_eq!(
            vec![plural.clone()],
            FrenchInflector::new().pluralize(&singular).unwrap(),
            "row {name}"
        );
        // test casing: if the first letter was uppercase, it should remain so
        assert_eq!(
            vec![ucfirst(&plural)],
            FrenchInflector::new()
                .pluralize(&ucfirst(&singular))
                .unwrap(),
            "row {name}, ucfirst"
        );
    }
}
