//! PHP's intl functions as symfony/string calls them: the `Normalizer`
//! forms (`normalizer_normalize()`, `normalizer_is_normalized()`, `NFKC_CF`
//! included) and the grapheme functions (`grapheme_strlen()`,
//! `grapheme_substr()`, the `grapheme_*pos()` family, `grapheme_extract()`
//! with `GRAPHEME_EXTR_MAXBYTES`, `grapheme_str_split()`).
//!
//! NFC, NFD, NFKC and NFKD come from `unicode-normalization` and grapheme
//! clusters from `unicode-segmentation` (PLAN.md stack row), checked
//! against the oracle's ICU 72.1 (`tests/unicode.rs`); `NFKC_CF` applies
//! ICU's per-code-point mapping recorded from the oracle (`case_tables.rs`)
//! and recomposes. The functions are written from the PHP manual (no
//! php-src, PLAN.md decision 8) and checked against
//! `tests/data/builtins.json`.
//!
//! Upstream: none

use unicode_normalization::{
    IsNormalized, UnicodeNormalization, is_nfc_quick, is_nfd_quick, is_nfkc_quick, is_nfkd_quick,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{case_tables as t, mb, php};

/// A `Normalizer` form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
    NfkcCf,
}

impl Form {
    /// The form for `Normalizer::NFD` (4), `NFKD` (8), `NFC` (16), `NFKC`
    /// (32) and `NFKC_CF` (48), the values PHP gives these constants.
    pub fn from_php(value: i64) -> Option<Self> {
        match value {
            4 => Some(Self::Nfd),
            8 => Some(Self::Nfkd),
            16 => Some(Self::Nfc),
            32 => Some(Self::Nfkc),
            48 => Some(Self::NfkcCf),
            _ => None,
        }
    }

    /// The constant's value in PHP.
    pub fn php_value(self) -> i64 {
        match self {
            Self::Nfd => 4,
            Self::Nfkd => 8,
            Self::Nfc => 16,
            Self::Nfkc => 32,
            Self::NfkcCf => 48,
        }
    }
}

/// `normalizer_normalize($s, $form)`.
pub fn normalize(s: &str, form: Form) -> String {
    match form {
        Form::Nfc => s.nfc().collect(),
        Form::Nfd => s.nfd().collect(),
        Form::Nfkc => s.nfkc().collect(),
        Form::Nfkd => s.nfkd().collect(),
        Form::NfkcCf => {
            let mut mapped = String::with_capacity(s.len());
            for c in s.nfd() {
                let cp = c as u32;
                if mb::in_ranges(t::NFKC_CF_REMOVED, cp) {
                    continue;
                }
                match t::NFKC_CF.binary_search_by_key(&cp, |&(k, _)| k) {
                    Ok(i) => mapped.push_str(t::NFKC_CF[i].1),
                    Err(_) => mapped.push(c),
                }
            }
            mapped.nfc().collect()
        }
    }
}

/// `normalizer_is_normalized($s, $form)`.
pub fn is_normalized(s: &str, form: Form) -> bool {
    let quick = match form {
        Form::Nfc => is_nfc_quick(s.chars()),
        Form::Nfd => is_nfd_quick(s.chars()),
        Form::Nfkc => is_nfkc_quick(s.chars()),
        Form::Nfkd => is_nfkd_quick(s.chars()),
        Form::NfkcCf => IsNormalized::Maybe,
    };
    match quick {
        IsNormalized::Yes => true,
        IsNormalized::No => false,
        IsNormalized::Maybe => normalize(s, form) == s,
    }
}

/// `$s` normalized to `$form` unless it already is: upstream's
/// `normalizer_is_normalized($s, $form) ?: $s = normalizer_normalize(...)`.
pub fn normalized(s: &str, form: Form) -> String {
    if is_normalized(s, form) {
        s.to_owned()
    } else {
        normalize(s, form)
    }
}

/// Byte offsets of the grapheme boundaries of `s`, `0` and `s.len()`
/// included.
pub fn boundaries(s: &str) -> Vec<usize> {
    s.grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(s.len()))
        .collect()
}

/// `grapheme_strlen($s)`.
pub fn strlen(s: &str) -> usize {
    s.graphemes(true).count()
}

/// `grapheme_substr($s, $start, $length)`: a negative start counts from
/// the end; a negative length leaves that many graphemes off the end; out
/// of range gives `''`.
pub fn substr(s: &str, start: i64, length: Option<i64>) -> &str {
    let bounds = boundaries(s);
    let (from, to) = php::substr_range(bounds.len() - 1, start, length);
    &s[bounds[from]..bounds[to]]
}

/// `grapheme_str_split($s)`.
pub fn str_split(s: &str) -> Vec<&str> {
    s.graphemes(true).collect()
}

/// Whether ICU's collation ignores `c` when grapheme_*`pos()` compares:
/// the code points `NFKC_CF` removes (default ignorables such as U+200B).
fn ignorable(c: char) -> bool {
    mb::in_ranges(t::NFKC_CF_REMOVED, c as u32)
}

/// What grapheme_*`pos()` compares: the canonical decomposition without
/// ignorable code points, case folded (simple folding) with `ci`.
fn key(s: &str, ci: bool) -> String {
    let folded;
    let s = if ci {
        folded = mb::fold_simple(s);
        folded.as_str()
    } else {
        s
    };
    s.nfd().filter(|&c| !ignorable(c)).collect()
}

/// The grapheme indexes `k` at which `needle` matches `haystack`, as ICU's
/// collation-based search finds them (recorded in tests/data/
/// builtins.json): from the boundary `k` to a later boundary, the
/// haystack is canonically equivalent to the needle, ignorable code points
/// skipped, case folded with `ci`; a match neither starts nor ends with an
/// ignorable code point. A needle of ignorable code points only matches at
/// every grapheme.
fn matches(haystack: &str, needle: &str, ci: bool) -> Vec<usize> {
    let bounds = boundaries(haystack);
    let needle = key(needle, ci);
    if needle.is_empty() {
        return (0..bounds.len() - 1).collect();
    }
    let mut found = Vec::new();
    for (k, &from) in bounds.iter().enumerate() {
        if haystack[from..].chars().next().is_none_or(ignorable) {
            continue;
        }
        for &to in &bounds[k + 1..] {
            let piece = &haystack[from..to];
            let piece_key = key(piece, ci);
            if piece_key.len() > needle.len() {
                break;
            }
            if piece_key == needle && piece.chars().next_back().is_some_and(|c| !ignorable(c)) {
                found.push(k);
                break;
            }
        }
    }
    found
}

/// `grapheme_strpos($h, $n, $offset)` (`grapheme_stripos()` with `ci`), in
/// graphemes. `None` for no match and for an offset outside the haystack
/// (a `ValueError`).
pub fn strpos(h: &str, n: &str, offset: i64, ci: bool) -> Option<usize> {
    let len = strlen(h) as i64;
    let offset = if offset < 0 { offset + len } else { offset };
    if offset < 0 || offset > len {
        return None;
    }
    matches(h, n, ci).into_iter().find(|&k| k as i64 >= offset)
}

/// `grapheme_strrpos($h, $n, $offset)` (`grapheme_strripos()` with `ci`).
pub fn strrpos(h: &str, n: &str, offset: i64, ci: bool) -> Option<usize> {
    let len = strlen(h) as i64;
    if offset > len || offset < -len {
        return None;
    }
    let found = matches(h, n, ci);
    if offset >= 0 {
        found.into_iter().rev().find(|&k| k as i64 >= offset)
    } else {
        let needle_len = strlen(n) as i64;
        found
            .into_iter()
            .rev()
            .find(|&k| k as i64 + needle_len <= len + offset + needle_len)
    }
}

/// `grapheme_extract($s, $size, GRAPHEME_EXTR_MAXBYTES, $start)`: the
/// whole graphemes from byte `start` (moved forward to a code point
/// boundary) that fit in `size` bytes. `None` (`false`) for a start
/// outside the string.
pub fn extract_maxbytes(s: &str, size: usize, start: i64) -> Option<&str> {
    if start < 0 || start as usize >= s.len() {
        return None;
    }
    let mut start = start as usize;
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    let rest = &s[start..];
    let mut end = 0;
    for (at, g) in rest.grapheme_indices(true) {
        if at + g.len() > size {
            break;
        }
        end = at + g.len();
    }
    Some(&rest[..end])
}
