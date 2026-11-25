//! PHP built-ins Console calls: `strip_tags()` (for `OUTPUT_PLAIN`), and
//! for Application's command lookup `levenshtein()`, `strnatcasecmp()`
//! (`ksort()` with `SORT_NATURAL | SORT_FLAG_CASE`) and `sort()`'s default
//! order of strings. They are written as symfony-config and
//! symfony-finder write them (from the manual and recorded PHP answers),
//! and checked here against `tests/data/builtins.json`, which found that
//! natural order does not skip whitespace right after a run of digits.
//!
//! Written from the PHP manual (no php-src, PLAN.md decision 8) and checked
//! against 6,137 recorded calls (`tests/data/strip_tags.json`,
//! `scripts/record.sh console`). The manual: HTML and PHP tags and
//! comments are removed. As observed:
//! - a `<` followed by whitespace opens nothing, in a tag or not;
//! - in a tag, `<` nests and `>` closes one level, and quotes hide both;
//!   once a nested `<` is followed by `!`, the rest of the tag is a
//!   declaration, in which `<` no longer nests;
//! - `<!--` runs to `-->`;
//! - `<?` at the top level runs to `?>` outside quotes, the opening `?`
//!   included (`<?>` is complete); inside a tag, quotes do not count, the
//!   first `>` is passed over and the next `?>` ends the whole tag, and in
//!   a nested tag it never ends;
//! - an unclosed tag runs to the end.
//!
//! Upstream: none

/// `strip_tags($string)` with no allowed tags.
pub fn strip_tags(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'<' || opens_nothing(b, i) {
            out.push(b[i]);
            i += 1;
            continue;
        }
        i = skip_tag(b, i);
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn opens_nothing(b: &[u8], at: usize) -> bool {
    b.get(at + 1).is_some_and(u8::is_ascii_whitespace)
}

/// From the `<` at `start`, the index after the tag (or the end).
fn skip_tag(b: &[u8], start: usize) -> usize {
    if b[start..].starts_with(b"<!--") {
        return find_after(b, start + 4, b"-->");
    }
    if b.get(start + 1) == Some(&b'?') {
        return skip_php(b, start + 1);
    }
    let mut declaration = b.get(start + 1) == Some(&b'!');
    let mut depth = 1;
    let mut quote: Option<u8> = None;
    let mut i = start + 1;
    while i < b.len() {
        let c = b[i];
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                b'"' | b'\'' => quote = Some(c),
                b'<' if !declaration && !opens_nothing(b, i) => {
                    if b.get(i + 1) == Some(&b'?') {
                        return skip_nested_php(b, i + 2, depth);
                    }
                    depth += 1;
                    if b.get(i + 1) == Some(&b'!') {
                        declaration = true;
                    }
                }
                b'>' => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            },
        }
        i += 1;
    }
    b.len()
}

/// A PHP section from `from`: to the first `?>` outside quotes.
fn skip_php(b: &[u8], from: usize) -> usize {
    let mut quote: Option<u8> = None;
    let mut i = from;
    while i < b.len() {
        let c = b[i];
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                b'"' | b'\'' => quote = Some(c),
                b'?' if b.get(i + 1) == Some(&b'>') => return i + 2,
                _ => {}
            },
        }
        i += 1;
    }
    b.len()
}

/// A PHP section opened inside a tag, from after its `<?`: quotes do not
/// count, the first `>` is passed over, and the next `?>` ends the whole
/// tag; inside a nested tag (depth 2 or more) it never ends.
fn skip_nested_php(b: &[u8], from: usize, depth: usize) -> usize {
    if depth > 1 {
        return b.len();
    }
    let Some(first) = b[from.min(b.len())..].iter().position(|&c| c == b'>') else {
        return b.len();
    };
    find_after(b, from + first + 1, b"?>")
}

fn find_after(b: &[u8], from: usize, end: &[u8]) -> usize {
    b[from.min(b.len())..]
        .windows(end.len())
        .position(|w| w == end)
        .map_or(b.len(), |p| from + p + end.len())
}

use std::cmp::Ordering;

use php_value::numeric::{is_numeric, string_to_float, string_to_int};

/// `levenshtein($a, $b)` with insertion, replacement and deletion costs 1,
/// on bytes.
pub use php_value::string::levenshtein;

/// `$a <=> $b` for two strings, as `sort()` orders them by default (PHP
/// 8): as numbers when both are numeric, else byte by byte.
pub fn compare_strings(a: &[u8], b: &[u8]) -> Ordering {
    if is_numeric(a) && is_numeric(b) {
        let number = |s: &[u8]| {
            let integer_form = !s.iter().any(|&c| matches!(c, b'.' | b'e' | b'E'));
            let f = string_to_float(s);
            if integer_form {
                let i = string_to_int(s);
                if (i != i64::MAX && i != i64::MIN) || (i as f64) == f {
                    return (Some(i), f);
                }
            }
            (None, f)
        };
        return match (number(a), number(b)) {
            ((Some(x), _), (Some(y), _)) => x.cmp(&y),
            ((_, x), (_, y)) => x.partial_cmp(&y).unwrap_or(Ordering::Less),
        };
    }
    a.cmp(b)
}

/// `sort($strings)` with the default flags (stable, as PHP 8's is).
pub fn sort_strings(strings: &mut [String]) {
    strings.sort_by(|a, b| compare_strings(a.as_bytes(), b.as_bytes()));
}

/// The sign of `strnatcasecmp($a, $b)`.
pub use php_value::string::strnatcasecmp;

/// `sprintf($format, ...$args)` for the formats Table and `SymfonyStyle`
/// use: `%s`, `%N$s` and `%%`. Another conversion is refused (PHP would
/// format a number, or throw a `ValueError` for an unknown one).
pub fn sprintf_s(format: &str, args: &[&str]) -> crate::exception::Result<String> {
    let mut out = String::with_capacity(format.len() + args.iter().map(|a| a.len()).sum::<usize>());
    let mut next = 0;
    let mut rest = format;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let spec = &rest[at + 1..];
        if let Some(after) = spec.strip_prefix('%') {
            out.push('%');
            rest = after;
            continue;
        }
        let digits = spec.bytes().take_while(u8::is_ascii_digit).count();
        let (index, conversion) = if digits > 0 && spec[digits..].starts_with('$') {
            let n: usize = spec[..digits].parse().unwrap_or(0);
            (n.checked_sub(1), &spec[digits + 1..])
        } else {
            let index = next;
            next += 1;
            (Some(index), spec)
        };
        let Some(after) = conversion.strip_prefix('s') else {
            return Err(crate::exception::ConsoleError::Other {
                class: "ValueError".to_owned(),
                message: format!("Unsupported format in \"{format}\""),
            });
        };
        match index.and_then(|i| args.get(i)) {
            Some(arg) => out.push_str(arg),
            None => {
                return Err(crate::exception::ConsoleError::Other {
                    class: "ArgumentCountError".to_owned(),
                    message: format!(
                        "{} arguments are required, {} given",
                        args.len() + 2,
                        args.len() + 1
                    ),
                });
            }
        }
        rest = after;
    }
    out.push_str(rest);
    Ok(out)
}

/// `str_pad($string, $length, $padString, $padType)`, on bytes: the pad
/// string is repeated and cut to fill `$length` bytes. A cut multi-byte
/// pad string is repaired lossily (PHP would keep the partial bytes).
pub fn str_pad(
    input: &str,
    length: i64,
    pad: &str,
    pad_type: crate::helper::table_style::PadType,
) -> String {
    use crate::helper::table_style::PadType;
    let missing = length - input.len() as i64;
    if missing <= 0 || pad.is_empty() {
        return input.to_owned();
    }
    let missing = missing as usize;
    let (left, right) = match pad_type {
        PadType::Left => (missing, 0),
        PadType::Right => (0, missing),
        PadType::Both => (missing / 2, missing - missing / 2),
    };
    let fill = |n: usize| -> Vec<u8> { pad.bytes().cycle().take(n).collect() };
    let mut bytes = fill(left);
    bytes.extend_from_slice(input.as_bytes());
    bytes.extend(fill(right));
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// `mb_substr($string, $start, $length)` on code points, for a start and
/// length that are not negative.
pub fn mb_substr(s: &str, start: usize, length: Option<usize>) -> String {
    let chars = s.chars().skip(start);
    match length {
        Some(n) => chars.take(n).collect(),
        None => chars.collect(),
    }
}

/// `mb_str_split($string, $length)`: chunks of `$length` code points.
pub fn mb_str_split(s: &str, length: usize) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    chars
        .chunks(length.max(1))
        .map(|c| c.iter().collect())
        .collect()
}

/// `urlencode($string)`: letters, digits and `-_.` kept, a space as `+`,
/// every other byte as `%XX` (upper case).
pub fn urlencode(s: &str) -> String {
    // The shared encoder produces ASCII only.
    String::from_utf8_lossy(&php_value::url::urlencode(s.as_bytes())).into_owned()
}

/// `http_build_query($pairs, '', $separator)` of string pairs.
pub fn http_build_query(pairs: &[(&str, &str)], separator: &str) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join(separator)
}
