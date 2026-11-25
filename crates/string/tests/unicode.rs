//! The Unicode crates and the PCRE2 `\X` emulation against the oracle's ICU
//! 72.1 and PCRE2 10.44 (tests/data/unicode.json, `scripts/record.sh string`).
//!
//! Upstream: none
#![allow(
    clippy::cast_possible_truncation,
    reason = "test code ports PHPUnit tests and helpers as they are"
)]

use serde_json::Value as Json;
use symfony_string::unicode_string::pcre_graphemes;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

fn data() -> Json {
    serde_json::from_str(include_str!("data/unicode.json")).unwrap_or_else(|e| panic!("{e}"))
}

fn lengths(json: &Json) -> Vec<usize> {
    json.as_array()
        .into_iter()
        .flatten()
        .filter_map(Json::as_u64)
        .map(|n| n as usize)
        .collect()
}

#[test]
fn nfc_matches_icu_for_every_decomposable_code_point() {
    let data = data();
    let rows = data["nfc"].as_array().unwrap_or_else(|| panic!("nfc"));
    assert_eq!(rows.len(), 13233);
    for row in rows {
        let cp = row[0]
            .as_u64()
            .and_then(|n| char::from_u32(n as u32))
            .unwrap_or_else(|| panic!("{row}"));
        let nfd: String = cp.to_string().nfd().collect();
        assert_eq!(
            nfd.nfc().collect::<String>(),
            row[1].as_str().unwrap_or_default(),
            "U+{:04X}",
            cp as u32
        );
        assert_eq!(
            cp.to_string().nfc().collect::<String>(),
            row[2].as_str().unwrap_or_default(),
            "U+{:04X}",
            cp as u32
        );
    }
}

#[test]
fn graphemes_match_icu_and_pcre() {
    let data = data();
    let sequences = data["sequences"]
        .as_array()
        .unwrap_or_else(|| panic!("sequences"));
    assert_eq!(sequences.len(), 4105);
    let (mut icu, mut pcre) = (Vec::new(), Vec::new());
    for row in sequences {
        let text = row[0].as_str().unwrap_or_default();
        let ours: Vec<usize> = text.graphemes(true).map(str::len).collect();
        if ours != lengths(&row[1]) {
            icu.push(format!(
                "{:X?}",
                text.chars().map(u32::from).collect::<Vec<_>>()
            ));
        }
        let ours: Vec<usize> = pcre_graphemes(text).into_iter().map(str::len).collect();
        if ours != lengths(&row[2]) {
            pcre.push(format!(
                "{:X?} {ours:?} PCRE {:?}",
                text.chars().map(u32::from).collect::<Vec<_>>(),
                lengths(&row[2])
            ));
        }
    }
    assert!(icu.is_empty(), "ICU: {}", icu.join("\n"));
    assert!(
        pcre.is_empty(),
        "PCRE: {} differ\n{}",
        pcre.len(),
        pcre[..pcre.len().min(20)].join("\n")
    );
}

/// Single code points: which extend a base and which prepend, by ICU.
/// The only differences are code points the oracle's Unicode 15.0 leaves
/// unassigned and the crate's newer data classifies (README, "Differences").
#[test]
fn per_code_point_classes_differ_only_after_unicode_15() {
    let data = data();
    let ranges = |json: &Json| -> Vec<(u32, u32)> {
        json.as_array()
            .into_iter()
            .flatten()
            .map(|r| {
                (
                    r[0].as_u64().unwrap_or(0) as u32,
                    r[1].as_u64().unwrap_or(0) as u32,
                )
            })
            .collect()
    };
    let inside = |r: &[(u32, u32)], cp: u32| r.iter().any(|&(a, b)| a <= cp && cp <= b);
    let (extend, prepend) = (
        ranges(&data["extend"]["icu"]),
        ranges(&data["prepend"]["icu"]),
    );
    let (mut e, mut p) = (Vec::new(), Vec::new());
    for cp in 0..=0x0010_FFFF_u32 {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        if (format!("a{c}").graphemes(true).count() == 1) != inside(&extend, cp) {
            e.push(cp);
        }
        if (format!("{c}a").graphemes(true).count() == 1) != inside(&prepend, cp) {
            p.push(cp);
        }
    }
    assert_eq!((e.len(), p.len()), (93, 2), "{e:X?} {p:X?}");
    assert_eq!(p, [0x113D1, 0x11A3A]);
}
