//! PHP's mbstring functions as symfony/string calls them, on UTF-8:
//! `mb_strtolower()` (with the conditional final sigma), `mb_strtoupper()`,
//! `mb_convert_case()` (title of one code point, full and simple fold),
//! `mb_strlen()`, `mb_substr()`, the `mb_*pos()` family, `mb_ord()`, and
//! `mb_convert_encoding()` / `mb_detect_encoding()` for the code pages
//! symfony/string's tests name.
//!
//! The case mappings are mbstring's own, recorded per code point from the
//! oracle (`case_tables.rs`, generated from `tests/data/case.tsv`); the
//! functions are written from the PHP manual (no php-src, PLAN.md decision
//! 8) and checked against `tests/data/builtins.json`.
//!
//! Upstream: none

use crate::{case_tables as t, php};

fn lookup(table: &'static [(u32, &'static str)], c: char) -> Option<&'static str> {
    table
        .binary_search_by_key(&(c as u32), |&(cp, _)| cp)
        .ok()
        .map(|i| table[i].1)
}

pub(crate) fn in_ranges(table: &[(u32, u32)], cp: u32) -> bool {
    table
        .binary_search_by(|&(lo, hi)| {
            if hi < cp {
                std::cmp::Ordering::Less
            } else if lo > cp {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn map(s: &str, table: &'static [(u32, &'static str)]) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match lookup(table, c) {
            Some(m) => out.push_str(m),
            None => out.push(c),
        }
    }
    out
}

fn is_cased(c: char) -> bool {
    in_ranges(t::CASED, c as u32)
}

fn is_case_ignorable(c: char) -> bool {
    in_ranges(t::CASE_IGNORABLE, c as u32)
}

/// `mb_strtolower($s, 'UTF-8')`. A capital sigma becomes final (`ς`) after
/// a cased letter and not before one, case-ignorable characters skipped on
/// both sides.
pub fn strtolower(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        if c == '\u{3A3}' {
            let before = chars[..i]
                .iter()
                .rev()
                .find(|&&p| !is_case_ignorable(p))
                .is_some_and(|&p| is_cased(p));
            let after = chars[i + 1..]
                .iter()
                .find(|&&n| !is_case_ignorable(n))
                .is_some_and(|&n| is_cased(n));
            out.push(if before && !after {
                '\u{3C2}'
            } else {
                '\u{3C3}'
            });
            continue;
        }
        match lookup(t::LOWER, c) {
            Some(m) => out.push_str(m),
            None => out.push(c),
        }
    }
    out
}

/// `mb_strtoupper($s, 'UTF-8')`.
pub fn strtoupper(s: &str) -> String {
    map(s, t::UPPER)
}

/// `mb_convert_case($c, MB_CASE_TITLE, 'UTF-8')` for one code point.
pub fn title_char(c: char) -> String {
    lookup(t::TITLE, c).map_or_else(|| c.to_string(), str::to_owned)
}

/// `mb_convert_case($s, MB_CASE_FOLD_SIMPLE, 'UTF-8')`.
pub fn fold_simple(s: &str) -> String {
    map(s, t::FOLD_SIMPLE)
}

/// `mb_convert_case($s, MB_CASE_FOLD, 'UTF-8')`.
pub fn fold(s: &str) -> String {
    map(s, t::FOLD)
}

/// `mb_strlen($s, 'UTF-8')`.
pub fn strlen(s: &str) -> usize {
    s.chars().count()
}

/// Byte offsets of the code points of `s`, and `s.len()` at the end.
fn char_bounds(s: &str) -> Vec<usize> {
    s.char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(s.len()))
        .collect()
}

/// `mb_substr($s, $start, $length, 'UTF-8')`.
pub fn substr(s: &str, start: i64, length: Option<i64>) -> &str {
    let bounds = char_bounds(s);
    let (from, to) = php::substr_range(bounds.len() - 1, start, length);
    &s[bounds[from]..bounds[to]]
}

fn chars_ci(s: &str, ci: bool) -> Vec<char> {
    if ci {
        fold_simple(s).chars().collect()
    } else {
        s.chars().collect()
    }
}

/// `mb_strpos($h, $n, $offset, 'UTF-8')` (`mb_stripos()` with `ci`), in
/// code points. `None` for no match and for an offset outside the
/// haystack (a `ValueError`).
pub fn strpos(h: &str, n: &str, offset: i64, ci: bool) -> Option<usize> {
    let (h, n) = (chars_ci(h, ci), chars_ci(n, ci));
    let len = h.len() as i64;
    let offset = if offset < 0 { offset + len } else { offset };
    if offset < 0 || offset > len || n.len() > h.len() {
        return None;
    }
    (offset as usize..=h.len() - n.len()).find(|&i| h[i..i + n.len()] == n[..])
}

/// `mb_strrpos($h, $n, $offset, 'UTF-8')` (`mb_strripos()` with `ci`).
pub fn strrpos(h: &str, n: &str, offset: i64, ci: bool) -> Option<usize> {
    let (h, n) = (chars_ci(h, ci), chars_ci(n, ci));
    let (lo, hi) = php::rpos_bounds(h.len(), n.len(), offset)?;
    if lo > hi {
        return None;
    }
    (lo..=hi).rev().find(|&i| h[i..i + n.len()] == n[..])
}

/// `mb_ord($c, 'UTF-8')` of the first code point.
pub fn ord(s: &str) -> Option<u32> {
    s.chars().next().map(|c| c as u32)
}

/// A code page `mb_convert_encoding()` or the `iconv()` fallback converts
/// to and from UTF-8 here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodePage {
    /// mbstring's `Windows-1252`.
    Windows1252,
    /// glibc iconv's `windows-1250` (mbstring has none).
    Windows1250,
}

impl CodePage {
    /// The code page an encoding name means, as mbstring and iconv match
    /// names: without regard to case.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "windows-1252" | "cp1252" => Some(Self::Windows1252),
            "windows-1250" | "cp1250" => Some(Self::Windows1250),
            _ => None,
        }
    }

    fn high(self) -> &'static [Option<char>; 128] {
        match self {
            Self::Windows1252 => &WINDOWS_1252,
            Self::Windows1250 => &WINDOWS_1250,
        }
    }

    /// The character a byte decodes to; `None` for a byte the code page
    /// leaves undefined.
    pub fn decode_byte(self, b: u8) -> Option<char> {
        if b < 0x80 {
            Some(b as char)
        } else {
            self.high()[(b - 0x80) as usize]
        }
    }

    /// The byte for a character, if the code page holds it.
    pub fn encode_char(self, c: char) -> Option<u8> {
        if (c as u32) < 0x80 {
            return Some(c as u8);
        }
        self.high()
            .iter()
            .position(|&m| m == Some(c))
            .map(|i| 0x80 + i as u8)
    }
}

/// `mb_convert_encoding($s, 'Windows-1252', 'UTF-8')`: a character the code
/// page lacks becomes `?`; or `iconv('UTF-8', 'windows-1250', $s)`, which
/// fails (`None`) on such a character.
pub fn encode(s: &str, page: CodePage) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        match (page.encode_char(c), page) {
            (Some(b), _) => out.push(b),
            (None, CodePage::Windows1252) => out.push(b'?'),
            (None, CodePage::Windows1250) => return None,
        }
    }
    Some(out)
}

/// `mb_detect_encoding($s, 'Windows-1252', true) !== false`.
pub fn detect(s: &[u8], page: CodePage) -> bool {
    s.iter().all(|&b| page.decode_byte(b).is_some())
}

/// `mb_convert_encoding($s, 'UTF-8', 'Windows-1252')` of a string
/// [`detect`] accepts, or `iconv('windows-1250', 'UTF-8', $s)` (`None` on
/// an undefined byte).
pub fn decode(s: &[u8], page: CodePage) -> Option<String> {
    s.iter().map(|&b| page.decode_byte(b)).collect()
}

const fn c(cp: u32) -> Option<char> {
    char::from_u32(cp)
}

// 0x80-0xFF as the oracle decodes them (tests/data/builtins.json,
// `encodings`): mbstring's Windows-1252 maps the five bytes the code page
// leaves undefined to the C1 controls of the same value and its strict
// detection accepts them; `None` where glibc's iconv fails.
#[rustfmt::skip]
static WINDOWS_1252: [Option<char>; 128] = [
    c(0x20AC), c(0x81), c(0x201A), c(0x0192), c(0x201E), c(0x2026), c(0x2020), c(0x2021),
    c(0x02C6), c(0x2030), c(0x0160), c(0x2039), c(0x0152), c(0x8D), c(0x017D), c(0x8F),
    c(0x90), c(0x2018), c(0x2019), c(0x201C), c(0x201D), c(0x2022), c(0x2013), c(0x2014),
    c(0x02DC), c(0x2122), c(0x0161), c(0x203A), c(0x0153), c(0x9D), c(0x017E), c(0x0178),
    c(0xA0), c(0xA1), c(0xA2), c(0xA3), c(0xA4), c(0xA5), c(0xA6), c(0xA7),
    c(0xA8), c(0xA9), c(0xAA), c(0xAB), c(0xAC), c(0xAD), c(0xAE), c(0xAF),
    c(0xB0), c(0xB1), c(0xB2), c(0xB3), c(0xB4), c(0xB5), c(0xB6), c(0xB7),
    c(0xB8), c(0xB9), c(0xBA), c(0xBB), c(0xBC), c(0xBD), c(0xBE), c(0xBF),
    c(0xC0), c(0xC1), c(0xC2), c(0xC3), c(0xC4), c(0xC5), c(0xC6), c(0xC7),
    c(0xC8), c(0xC9), c(0xCA), c(0xCB), c(0xCC), c(0xCD), c(0xCE), c(0xCF),
    c(0xD0), c(0xD1), c(0xD2), c(0xD3), c(0xD4), c(0xD5), c(0xD6), c(0xD7),
    c(0xD8), c(0xD9), c(0xDA), c(0xDB), c(0xDC), c(0xDD), c(0xDE), c(0xDF),
    c(0xE0), c(0xE1), c(0xE2), c(0xE3), c(0xE4), c(0xE5), c(0xE6), c(0xE7),
    c(0xE8), c(0xE9), c(0xEA), c(0xEB), c(0xEC), c(0xED), c(0xEE), c(0xEF),
    c(0xF0), c(0xF1), c(0xF2), c(0xF3), c(0xF4), c(0xF5), c(0xF6), c(0xF7),
    c(0xF8), c(0xF9), c(0xFA), c(0xFB), c(0xFC), c(0xFD), c(0xFE), c(0xFF),
];

#[rustfmt::skip]
static WINDOWS_1250: [Option<char>; 128] = [
    c(0x20AC), None, c(0x201A), None, c(0x201E), c(0x2026), c(0x2020), c(0x2021),
    None, c(0x2030), c(0x0160), c(0x2039), c(0x015A), c(0x0164), c(0x017D), c(0x0179),
    None, c(0x2018), c(0x2019), c(0x201C), c(0x201D), c(0x2022), c(0x2013), c(0x2014),
    None, c(0x2122), c(0x0161), c(0x203A), c(0x015B), c(0x0165), c(0x017E), c(0x017A),
    c(0xA0), c(0x02C7), c(0x02D8), c(0x0141), c(0xA4), c(0x0104), c(0xA6), c(0xA7),
    c(0xA8), c(0xA9), c(0x015E), c(0xAB), c(0xAC), c(0xAD), c(0xAE), c(0x017B),
    c(0xB0), c(0xB1), c(0x02DB), c(0x0142), c(0xB4), c(0xB5), c(0xB6), c(0xB7),
    c(0xB8), c(0x0105), c(0x015F), c(0xBB), c(0x013D), c(0x02DD), c(0x013E), c(0x017C),
    c(0x0154), c(0xC1), c(0xC2), c(0x0102), c(0xC4), c(0x0139), c(0x0106), c(0xC7),
    c(0x010C), c(0xC9), c(0x0118), c(0xCB), c(0x011A), c(0xCD), c(0xCE), c(0x010E),
    c(0x0110), c(0x0143), c(0x0147), c(0xD3), c(0xD4), c(0x0150), c(0xD6), c(0xD7),
    c(0x0158), c(0x016E), c(0xDA), c(0x0170), c(0xDC), c(0xDD), c(0x0162), c(0xDF),
    c(0x0155), c(0xE1), c(0xE2), c(0x0103), c(0xE4), c(0x013A), c(0x0107), c(0xE7),
    c(0x010D), c(0xE9), c(0x0119), c(0xEB), c(0x011B), c(0xED), c(0xEE), c(0x010F),
    c(0x0111), c(0x0144), c(0x0148), c(0xF3), c(0xF4), c(0x0151), c(0xF6), c(0xF7),
    c(0x0159), c(0x016F), c(0xFA), c(0x0171), c(0xFC), c(0xFD), c(0x0163), c(0x02D9),
];
