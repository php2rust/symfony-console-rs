//! The English, French and Spanish inflectors on the word corpus recorded
//! from PHP (`tests/data/inflector.json`, `oracle/recorders/string_inflector.php`):
//! upstream's provider words and the inflectors' own table entries, with
//! case variants, prefixes, a trailing newline and bytes that are not
//! UTF-8. Every answer must equal PHP's.
//!
//! Upstream: none

use php_value::base64;
use serde_json::Value as Json;
use symfony_string::{EnglishInflector, FrenchInflector, Inflector, SpanishInflector};

const DATA: &str = include_str!("data/inflector.json");

fn bytes(json: &Json) -> Vec<u8> {
    base64::decode_strict(json.as_str().unwrap_or_default().as_bytes())
        .unwrap_or_else(|| panic!("base64: {json}"))
}

fn forms(json: &Json) -> Vec<Vec<u8>> {
    json.as_array().into_iter().flatten().map(bytes).collect()
}

fn replay(name: &str, inflector: &dyn Inflector) {
    let data: Json = serde_json::from_str(DATA).unwrap_or_else(|e| panic!("data: {e}"));
    let rows = data[name].as_array().unwrap_or_else(|| panic!("no {name}"));
    assert!(rows.len() > 10_000, "{name}: {} words", rows.len());
    let mut differ = Vec::new();
    for row in rows {
        let word = bytes(&row[0]);
        let singular = inflector
            .singularize(&word)
            .unwrap_or_else(|e| panic!("{e}"));
        let plural = inflector.pluralize(&word).unwrap_or_else(|e| panic!("{e}"));
        if singular != forms(&row[1]) || plural != forms(&row[2]) {
            differ.push(String::from_utf8_lossy(&word).into_owned());
        }
    }
    assert!(
        differ.is_empty(),
        "{name}: {} of {} words differ from PHP, e.g. {:?}",
        differ.len(),
        rows.len(),
        &differ[..differ.len().min(20)]
    );
}

#[test]
fn english_matches_php() {
    replay("english", &EnglishInflector::new());
}

#[test]
fn french_matches_php() {
    replay("french", &FrenchInflector::new());
}

#[test]
fn spanish_matches_php() {
    replay("spanish", &SpanishInflector::new());
}
