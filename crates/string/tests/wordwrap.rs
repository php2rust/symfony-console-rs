//! PHP's native `wordwrap()` against 4,005 calls recorded from the oracle
//! (`scripts/record.sh string`).
//!
//! Upstream: none

use symfony_string::php::wordwrap;

#[test]
fn native_wordwrap_matches_php() -> Result<(), Box<dyn std::error::Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/wordwrap.json");
    let cases: Vec<(String, usize, bool, String)> =
        serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let mut failures = Vec::new();
    for (mask, width, cut, expected) in &cases {
        let actual = wordwrap(mask.as_bytes(), *width, b'#', *cut);
        if actual != expected.as_bytes() {
            failures.push(format!(
                "{mask:?} width {width} cut {cut}: {:?}, PHP {expected:?}",
                String::from_utf8_lossy(&actual)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(25)].join("\n")
    );
    Ok(())
}
