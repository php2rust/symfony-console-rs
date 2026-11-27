//! The PHP functions symfony/string calls, against the answers recorded
//! from PHP 8.4.27 (tests/data/builtins.json, oracle/recorders/
//! `string_builtins.php`, `scripts/record.sh string`): the str*`pos()`,
//! mb_*`pos()` and grapheme_*`pos()` families with every offset, `mb_substr()`,
//! `grapheme_substr()`, `grapheme_extract()`, `grapheme_str_split()`, whole-string
//! case mapping, `str_pad()`, `ucwords()`, `ucfirst()`, `lcfirst()`, `substr()`,
//! `substr_replace()`, `trim()` with ranges, the normalizer forms, and the
//! Windows-1252 / windows-1250 code pages.
//!
//! Upstream: none
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::format_collect,
    clippy::many_single_char_names,
    reason = "test code ports PHPUnit tests and helpers as they are"
)]

use serde_json::Value as Json;
use symfony_string::intl::{self, Form};
use symfony_string::mb::{self, CodePage};
use symfony_string::php::{self, PadType};

fn data() -> Json {
    serde_json::from_str(include_str!("data/builtins.json"))
        .unwrap_or_else(|e| panic!("builtins.json: {e}"))
}

/// Calls whose answer differs from PHP's, as the README's "Differences"
/// lists them, in the grapheme_*`pos()` family: ICU folds the ligature
/// U+FB01 like "fi" (and still does not fold U+00DF like "ss"), and
/// `grapheme_strrpos()` of a needle of ignorable code points only fails on
/// a haystack ending in a combining sequence.
fn known_difference(f: &str, h: &str, n: &str) -> bool {
    let ligature = f.ends_with("ipos") && h.contains('\u{FB01}') && (n == "fi" || n == "\u{FB01}");
    let ignorable = f.starts_with("grapheme_strr") && n == "\u{200B}";
    ligature || ignorable
}

fn s(j: &Json) -> &str {
    j.as_str().unwrap_or_else(|| panic!("not a string: {j}"))
}

fn report(failures: &[String], total: usize) {
    assert!(
        failures.is_empty(),
        "{} of {total} differ:\n{}",
        failures.len(),
        failures[..failures.len().min(40)].join("\n")
    );
}

#[test]
fn positions_match_php() {
    let d = data();
    let haystacks: Vec<&str> = d["haystacks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(s)
        .collect();
    let needles: Vec<&str> = d["needles"]
        .as_array()
        .into_iter()
        .flatten()
        .map(s)
        .collect();
    let offsets: Vec<i64> = d["offsets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Json::as_i64)
        .collect();
    let mut failures = Vec::new();
    let mut total = 0;
    for (f, by_h) in d["pos"].as_object().into_iter().flatten() {
        for (hi, by_n) in by_h.as_array().into_iter().flatten().enumerate() {
            for (ni, answers) in by_n.as_array().into_iter().flatten().enumerate() {
                let (h, n) = (haystacks[hi], needles[ni]);
                for (oi, expected) in answers.as_array().into_iter().flatten().enumerate() {
                    if expected == "skipped" {
                        continue;
                    }
                    total += 1;
                    let o = offsets[oi];
                    let actual = match f.as_str() {
                        "strpos" => php::strpos(h.as_bytes(), n.as_bytes(), o, false),
                        "stripos" => php::strpos(h.as_bytes(), n.as_bytes(), o, true),
                        "strrpos" => php::strrpos(h.as_bytes(), n.as_bytes(), o, false),
                        "strripos" => php::strrpos(h.as_bytes(), n.as_bytes(), o, true),
                        "mb_strpos" => mb::strpos(h, n, o, false),
                        "mb_stripos" => mb::strpos(h, n, o, true),
                        "mb_strrpos" => mb::strrpos(h, n, o, false),
                        "mb_strripos" => mb::strrpos(h, n, o, true),
                        "grapheme_strpos" => intl::strpos(h, n, o, false),
                        "grapheme_stripos" => intl::strpos(h, n, o, true),
                        "grapheme_strrpos" => intl::strrpos(h, n, o, false),
                        "grapheme_strripos" => intl::strrpos(h, n, o, true),
                        other => panic!("{other}"),
                    };
                    // a ValueError and false are both null to symfony/string
                    let expected = expected.as_u64().map(|v| v as usize);
                    if actual != expected && !known_difference(f, h, n) {
                        failures.push(format!(
                            "{f}({h:?}, {n:?}, {o}): {actual:?}, PHP {expected:?}"
                        ));
                    }
                }
            }
        }
    }
    report(&failures, total);
}

#[test]
fn calls_match_php() {
    let d = data();
    let mut failures = Vec::new();
    let calls = d["calls"].as_array().cloned().unwrap_or_default();
    for call in &calls {
        let f = s(&call[0]);
        let a = call[1].as_array().cloned().unwrap_or_default();
        let expected = &call[2];
        let int = |i: usize| a[i].as_i64().unwrap_or_default();
        let opt = |i: usize| a[i].as_i64();
        let st = |i: usize| s(&a[i]).to_owned();
        let text = |v: Option<String>| v.map_or(Json::Bool(false), Json::String);
        let actual: Json = match f {
            "mb_substr" => Json::String(mb::substr(&st(0), int(1), opt(2)).to_owned()),
            "grapheme_substr" => Json::String(intl::substr(&st(0), int(1), opt(2)).to_owned()),
            "grapheme_extract" => {
                text(intl::extract_maxbytes(&st(0), int(1) as usize, int(3)).map(str::to_owned))
            }
            "grapheme_str_split" => Json::Array(
                intl::str_split(&st(0))
                    .into_iter()
                    .map(|g| Json::String(g.to_owned()))
                    .collect(),
            ),
            "mb_strtolower" => Json::String(mb::strtolower(&st(0))),
            "mb_strtoupper" => Json::String(mb::strtoupper(&st(0))),
            "ucwords" => {
                Json::String(String::from_utf8_lossy(&php::ucwords(st(0).as_bytes())).into_owned())
            }
            "ucfirst" => {
                Json::String(String::from_utf8_lossy(&php::ucfirst(st(0).as_bytes())).into_owned())
            }
            "lcfirst" => {
                Json::String(String::from_utf8_lossy(&php::lcfirst(st(0).as_bytes())).into_owned())
            }
            "normalizer_normalize" => {
                let form = Form::from_php(int(1)).unwrap_or_else(|| panic!("form {}", int(1)));
                Json::String(intl::normalize(&st(0), form))
            }
            "normalizer_is_normalized" => {
                let form = Form::from_php(int(1)).unwrap_or_else(|| panic!("form {}", int(1)));
                Json::Bool(intl::is_normalized(&st(0), form))
            }
            "str_pad" => {
                let kind = match int(3) {
                    0 => PadType::Left,
                    1 => PadType::Right,
                    _ => PadType::Both,
                };
                match php::str_pad(st(0).as_bytes(), int(1), st(2).as_bytes(), kind) {
                    Some(v) => Json::String(String::from_utf8_lossy(&v).into_owned()),
                    None => serde_json::json!({"throws": "ValueError"}),
                }
            }
            "substr" => Json::String(
                String::from_utf8_lossy(php::substr(st(0).as_bytes(), int(1), opt(2))).into_owned(),
            ),
            "substr_replace" => Json::String(
                String::from_utf8_lossy(&php::substr_replace(
                    st(0).as_bytes(),
                    st(1).as_bytes(),
                    int(2),
                    opt(3),
                ))
                .into_owned(),
            ),
            "trim" | "ltrim" | "rtrim" => {
                let (l, r) = match f {
                    "trim" => (true, true),
                    "ltrim" => (true, false),
                    _ => (false, true),
                };
                Json::String(
                    String::from_utf8_lossy(&php::trim(st(0).as_bytes(), st(1).as_bytes(), l, r))
                        .into_owned(),
                )
            }
            other => panic!("{other}"),
        };
        if actual != *expected {
            failures.push(format!("{f}({a:?}): {actual}, PHP {expected}"));
        }
    }
    report(&failures, calls.len());
}

#[test]
fn code_pages_match_php() {
    let d = data();
    for (name, page) in [
        ("Windows-1252", CodePage::Windows1252),
        ("windows-1250", CodePage::Windows1250),
    ] {
        for (b, hex) in d["encodings"][name]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let expected = hex.as_str().map(|h| {
                let bytes: Vec<u8> = (0..h.len())
                    .step_by(2)
                    .filter_map(|i| u8::from_str_radix(&h[i..i + 2], 16).ok())
                    .collect();
                String::from_utf8(bytes).unwrap_or_default()
            });
            let actual = page.decode_byte(b as u8).map(String::from);
            assert_eq!(expected, actual, "{name} byte {b:#X}");
        }
    }
    for (b, ok) in d["detect"]["Windows-1252"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        assert_eq!(
            ok.as_bool(),
            Some(mb::detect(&[b as u8], CodePage::Windows1252)),
            "detect {b:#X}"
        );
    }
    let hex = |v: &[u8]| v.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let snowman = "\u{17E}s\u{17E}s\u{FD} \u{2603} \u{65E5}\u{672C}";
    assert_eq!(
        d["unmappable"]["Windows-1252"],
        hex(&mb::encode(snowman, CodePage::Windows1252).unwrap_or_default())
    );
    assert_eq!(d["unmappable"]["windows-1250"], "false");
    assert!(mb::encode("\u{17E}s\u{17E}s\u{FD} \u{2603}", CodePage::Windows1250).is_none());
    assert_eq!(
        d["unmappable"]["windows-1250-mappable"],
        hex(&mb::encode("\u{17E}s\u{17E}s\u{FD}", CodePage::Windows1250).unwrap_or_default())
    );
}
