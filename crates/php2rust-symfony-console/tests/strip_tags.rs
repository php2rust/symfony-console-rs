//! PHP's `strip_tags()` against 6,017 calls recorded from the oracle
//! (`scripts/record.sh console`).
//!
//! Upstream: none

use symfony_console::php::strip_tags;

#[test]
fn strip_tags_matches_php() -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<(String, String)> = serde_json::from_str(include_str!("data/strip_tags.json"))?;
    let failures: Vec<String> = cases
        .iter()
        .filter(|(input, expected)| strip_tags(input) != *expected)
        .map(|(input, expected)| format!("{input:?}: {:?}, PHP {expected:?}", strip_tags(input)))
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(30)].join("\n")
    );
    Ok(())
}
