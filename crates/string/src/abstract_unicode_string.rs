//! Port of Symfony\Component\String\AbstractUnicodeString.php: what
//! `CodePointString` and `UnicodeString` share, as functions both call.
//!
//! The case functions use mbstring's mappings ([`crate::mb`]), the fixed
//! patterns of `camel()`, `snake()` and `title()` run on php-pcre, and
//! `trim()`'s character class is matched by hand (code points, with
//! upstream's `x..y` ranges). `ascii()` and the `locale*()` methods need
//! ICU's transliterators and are refused (README, "Not ported").
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/string AbstractUnicodeString.php

use crate::abstract_string::{AbstractString, Needle};
use crate::exception::{Error, Result};
use crate::intl::{self, Form};
use crate::preg::Regex;
use crate::{mb, php, tables};

/// `" \t\n\r\0\x0B\x0C\u{A0}\u{FEFF}"`, the Unicode classes' default
/// `trim()` list.
pub(crate) const TRIM_CHARS: &str = " \t\n\r\0\x0B\x0C\u{A0}\u{FEFF}";

/// The subset of folded case mappings that is not in lower case mappings
/// (`FOLD_FROM`, `FOLD_TO`), applied in order by `folded(false)`.
#[rustfmt::skip]
const FOLD_FROM_TO: [(&str, &str); 115] = [
    ("\u{130}", "i\u{307}"),
    ("\u{B5}", "\u{3BC}"),
    ("\u{17F}", "s"),
    ("\u{345}", "\u{3B9}"),
    ("\u{3C2}", "\u{3C3}"),
    ("\u{3D0}", "\u{3B2}"),
    ("\u{3D1}", "\u{3B8}"),
    ("\u{3D5}", "\u{3C6}"),
    ("\u{3D6}", "\u{3C0}"),
    ("\u{3F0}", "\u{3BA}"),
    ("\u{3F1}", "\u{3C1}"),
    ("\u{3F5}", "\u{3B5}"),
    ("\u{1E9B}", "\u{1E61}"),
    ("\u{1FBE}", "\u{3B9}"),
    ("\u{DF}", "ss"),
    ("\u{149}", "\u{2BC}n"),
    ("\u{1F0}", "\u{1F0}"),
    ("\u{390}", "\u{390}"),
    ("\u{3B0}", "\u{3B0}"),
    ("\u{587}", "\u{565}\u{582}"),
    ("\u{1E96}", "\u{1E96}"),
    ("\u{1E97}", "\u{1E97}"),
    ("\u{1E98}", "\u{1E98}"),
    ("\u{1E99}", "\u{1E99}"),
    ("\u{1E9A}", "a\u{2BE}"),
    ("\u{1E9E}", "ss"),
    ("\u{1F50}", "\u{1F50}"),
    ("\u{1F52}", "\u{1F52}"),
    ("\u{1F54}", "\u{1F54}"),
    ("\u{1F56}", "\u{1F56}"),
    ("\u{1F80}", "\u{1F00}\u{3B9}"),
    ("\u{1F81}", "\u{1F01}\u{3B9}"),
    ("\u{1F82}", "\u{1F02}\u{3B9}"),
    ("\u{1F83}", "\u{1F03}\u{3B9}"),
    ("\u{1F84}", "\u{1F04}\u{3B9}"),
    ("\u{1F85}", "\u{1F05}\u{3B9}"),
    ("\u{1F86}", "\u{1F06}\u{3B9}"),
    ("\u{1F87}", "\u{1F07}\u{3B9}"),
    ("\u{1F88}", "\u{1F00}\u{3B9}"),
    ("\u{1F89}", "\u{1F01}\u{3B9}"),
    ("\u{1F8A}", "\u{1F02}\u{3B9}"),
    ("\u{1F8B}", "\u{1F03}\u{3B9}"),
    ("\u{1F8C}", "\u{1F04}\u{3B9}"),
    ("\u{1F8D}", "\u{1F05}\u{3B9}"),
    ("\u{1F8E}", "\u{1F06}\u{3B9}"),
    ("\u{1F8F}", "\u{1F07}\u{3B9}"),
    ("\u{1F90}", "\u{1F20}\u{3B9}"),
    ("\u{1F91}", "\u{1F21}\u{3B9}"),
    ("\u{1F92}", "\u{1F22}\u{3B9}"),
    ("\u{1F93}", "\u{1F23}\u{3B9}"),
    ("\u{1F94}", "\u{1F24}\u{3B9}"),
    ("\u{1F95}", "\u{1F25}\u{3B9}"),
    ("\u{1F96}", "\u{1F26}\u{3B9}"),
    ("\u{1F97}", "\u{1F27}\u{3B9}"),
    ("\u{1F98}", "\u{1F20}\u{3B9}"),
    ("\u{1F99}", "\u{1F21}\u{3B9}"),
    ("\u{1F9A}", "\u{1F22}\u{3B9}"),
    ("\u{1F9B}", "\u{1F23}\u{3B9}"),
    ("\u{1F9C}", "\u{1F24}\u{3B9}"),
    ("\u{1F9D}", "\u{1F25}\u{3B9}"),
    ("\u{1F9E}", "\u{1F26}\u{3B9}"),
    ("\u{1F9F}", "\u{1F27}\u{3B9}"),
    ("\u{1FA0}", "\u{1F60}\u{3B9}"),
    ("\u{1FA1}", "\u{1F61}\u{3B9}"),
    ("\u{1FA2}", "\u{1F62}\u{3B9}"),
    ("\u{1FA3}", "\u{1F63}\u{3B9}"),
    ("\u{1FA4}", "\u{1F64}\u{3B9}"),
    ("\u{1FA5}", "\u{1F65}\u{3B9}"),
    ("\u{1FA6}", "\u{1F66}\u{3B9}"),
    ("\u{1FA7}", "\u{1F67}\u{3B9}"),
    ("\u{1FA8}", "\u{1F60}\u{3B9}"),
    ("\u{1FA9}", "\u{1F61}\u{3B9}"),
    ("\u{1FAA}", "\u{1F62}\u{3B9}"),
    ("\u{1FAB}", "\u{1F63}\u{3B9}"),
    ("\u{1FAC}", "\u{1F64}\u{3B9}"),
    ("\u{1FAD}", "\u{1F65}\u{3B9}"),
    ("\u{1FAE}", "\u{1F66}\u{3B9}"),
    ("\u{1FAF}", "\u{1F67}\u{3B9}"),
    ("\u{1FB2}", "\u{1F70}\u{3B9}"),
    ("\u{1FB3}", "\u{3B1}\u{3B9}"),
    ("\u{1FB4}", "\u{3AC}\u{3B9}"),
    ("\u{1FB6}", "\u{1FB6}"),
    ("\u{1FB7}", "\u{1FB6}\u{3B9}"),
    ("\u{1FBC}", "\u{3B1}\u{3B9}"),
    ("\u{1FC2}", "\u{1F74}\u{3B9}"),
    ("\u{1FC3}", "\u{3B7}\u{3B9}"),
    ("\u{1FC4}", "\u{3AE}\u{3B9}"),
    ("\u{1FC6}", "\u{1FC6}"),
    ("\u{1FC7}", "\u{1FC6}\u{3B9}"),
    ("\u{1FCC}", "\u{3B7}\u{3B9}"),
    ("\u{1FD2}", "\u{1FD2}"),
    ("\u{1FD6}", "\u{1FD6}"),
    ("\u{1FD7}", "\u{1FD7}"),
    ("\u{1FE2}", "\u{1FE2}"),
    ("\u{1FE4}", "\u{1FE4}"),
    ("\u{1FE6}", "\u{1FE6}"),
    ("\u{1FE7}", "\u{1FE7}"),
    ("\u{1FF2}", "\u{1F7C}\u{3B9}"),
    ("\u{1FF3}", "\u{3C9}\u{3B9}"),
    ("\u{1FF4}", "\u{3CE}\u{3B9}"),
    ("\u{1FF6}", "\u{1FF6}"),
    ("\u{1FF7}", "\u{1FF6}\u{3B9}"),
    ("\u{1FFC}", "\u{3C9}\u{3B9}"),
    ("\u{FB00}", "ff"),
    ("\u{FB01}", "fi"),
    ("\u{FB02}", "fl"),
    ("\u{FB03}", "ffi"),
    ("\u{FB04}", "ffl"),
    ("\u{FB05}", "st"),
    ("\u{FB06}", "st"),
    ("\u{FB13}", "\u{574}\u{576}"),
    ("\u{FB14}", "\u{574}\u{565}"),
    ("\u{FB15}", "\u{574}\u{56B}"),
    ("\u{FB16}", "\u{57E}\u{576}"),
    ("\u{FB17}", "\u{574}\u{56D}"),
];

pub(crate) fn utf8(bytes: &[u8]) -> Result<&str> {
    std::str::from_utf8(bytes).map_err(|_| Error::invalid_utf8())
}

/// The string of a Unicode class (always valid UTF-8).
pub(crate) fn text<S: AbstractString>(s: &S) -> &str {
    std::str::from_utf8(s.as_bytes()).unwrap_or_default()
}

/// `fromCodePoints(...$codes)`: each code (modulo 0x200000) as UTF-8, then
/// `new static()`, which refuses what is not valid UTF-8.
pub(crate) fn from_code_points<S: AbstractString>(codes: &[i64]) -> Result<S> {
    let mut string = Vec::new();
    for &code in codes {
        let code = code % 0x0020_0000;
        if code < 0x80 {
            string.push(code as u8);
        } else if code < 0x800 {
            string.extend([0xC0 | (code >> 6) as u8, 0x80 | (code & 0x3F) as u8]);
        } else if code < 0x10000 {
            string.extend([
                0xE0 | (code >> 12) as u8,
                0x80 | (code >> 6 & 0x3F) as u8,
                0x80 | (code & 0x3F) as u8,
            ]);
        } else {
            string.extend([
                0xF0 | (code >> 18) as u8,
                0x80 | (code >> 12 & 0x3F) as u8,
                0x80 | (code >> 6 & 0x3F) as u8,
                0x80 | (code & 0x3F) as u8,
            ]);
        }
    }
    S::create(&string)
}

/// `camel()`.
pub(crate) fn camel<S: AbstractString>(s: &S) -> Result<S> {
    let words =
        Regex::new(b"/[^\\pL0-9]++/u", "preg_replace")?.replace(s.as_bytes(), b" ", None)?;
    let mut first = true;
    let out = Regex::new(b"/\\b.(?!\\p{Lu})/u", "preg_replace_callback")?.replace_callback(
        &words,
        None,
        |m| {
            let c = std::str::from_utf8(m[0].unwrap_or_default()).unwrap_or_default();
            let out = if first {
                if c == "\u{130}" {
                    "i\u{307}".to_owned()
                } else {
                    mb::strtolower(c)
                }
            } else {
                c.chars().map(mb::title_char).collect()
            };
            first = false;
            Ok(out.into_bytes())
        },
    )?;
    Ok(s.with_string(out.into_iter().filter(|&b| b != b' ').collect()))
}

/// `codePointsAt($offset)` (`AbstractUnicodeString`'s).
pub(crate) fn code_points_at<S: AbstractString>(s: &S, offset: i64) -> Vec<u32> {
    let str = s.slice(offset, Some(1));
    text(&str).chars().map(|c| c as u32).collect()
}

/// `folded($compat)`.
pub(crate) fn folded<S: AbstractString>(s: &S, compat: bool) -> S {
    let t = text(s);
    if compat {
        return s.with_string(intl::normalize(t, Form::NfkcCf).into_bytes());
    }
    let mut string = intl::normalize(t, Form::Nfc);
    for (from, to) in FOLD_FROM_TO {
        if string.contains(from) {
            string = string.replace(from, to);
        }
    }
    s.with_string(mb::strtolower(&string).into_bytes())
}

/// `join($strings, $lastGlue)` (`AbstractUnicodeString`'s).
pub(crate) fn join<S: AbstractString>(
    s: &S,
    strings: &[&[u8]],
    last_glue: Option<&[u8]>,
) -> Result<S> {
    let joined = crate::byte_string::join_bytes(s.as_bytes(), strings, last_glue);
    utf8(&joined)?;
    Ok(s.with_string(joined))
}

/// `lower()`.
pub(crate) fn lower<S: AbstractString>(s: &S) -> S {
    s.with_string(mb::strtolower(&text(s).replace('\u{130}', "i\u{307}")).into_bytes())
}

/// `upper()`.
pub(crate) fn upper<S: AbstractString>(s: &S) -> S {
    s.with_string(mb::strtoupper(text(s)).into_bytes())
}

/// `AbstractUnicodeString::normalize()`: `CodePointString`'s.
pub(crate) fn normalize<S: AbstractString>(s: &S, form: i64) -> Result<S> {
    let Some(form @ (Form::Nfc | Form::Nfd | Form::Nfkc | Form::Nfkd)) = Form::from_php(form)
    else {
        return Err(Error::invalid("Unsupported normalization form."));
    };
    Ok(s.with_string(intl::normalized(text(s), form).into_bytes()))
}

/// The `$padStr` checks of `padBoth()`, `padEnd()` and `padStart()`.
fn pad_string(pad: &[u8]) -> Result<()> {
    if pad.is_empty() {
        return Err(Error::invalid_utf8());
    }
    utf8(pad).map(|_| ())
}

/// `pad($len, $pad, $type)` after the `$padStr` checks.
pub(crate) fn pad<S: AbstractString>(s: &S, len: i64, pad: &[u8], kind: php::PadType) -> Result<S> {
    pad_string(pad)?;
    let pad = s.with_string(pad.to_vec());
    let s_len = s.length() as i64;
    if len <= s_len {
        return Ok(s.cloned());
    }
    let pad_len = pad.length() as i64;
    let free_len = len - s_len;
    let piece = |n: i64| -> Vec<u8> {
        let rest = n % pad_len;
        let mut out = pad.as_bytes().repeat((n / pad_len) as usize);
        if rest != 0 {
            out.extend_from_slice(pad.slice(0, Some(rest)).as_bytes());
        }
        out
    };
    match kind {
        php::PadType::Right => s.append(&[&piece(free_len)]),
        php::PadType::Left => s.prepend(&[&piece(free_len)]),
        php::PadType::Both => {
            // $freeLen /= 2: ceil() to the right, floor() to the left
            let right = free_len - free_len / 2;
            let left = free_len / 2;
            s.append(&[&piece(right)])?.prepend(&[&piece(left)])
        }
    }
}

/// `replaceMatches()`'s check of a replacement string.
pub(crate) fn check_replacement(to: &[u8]) -> Result<()> {
    if !to.is_empty() {
        utf8(to)?;
    }
    Ok(())
}

/// `reverse()`: the graphemes in reverse order.
pub(crate) fn reverse<S: AbstractString>(s: &S) -> S {
    let t = text(s);
    let out: String = intl::str_split(t).into_iter().rev().collect();
    s.with_string(out.into_bytes())
}

/// `snake()`.
pub(crate) fn snake<S: AbstractString>(s: &S) -> Result<S> {
    let camel = s.camel()?;
    let first = Regex::new(b"/(\\p{Lu}+)(\\p{Lu}\\p{Ll})/u", "preg_replace")?.replace(
        camel.as_bytes(),
        b"\\1_\\2",
        None,
    )?;
    let second = Regex::new(b"/([\\p{Ll}0-9])(\\p{Lu})/u", "preg_replace")?
        .replace(&first, b"\\1_\\2", None)?;
    Ok(s.with_string(mb::strtolower(utf8(&second)?).into_bytes()))
}

/// `title($allWords)`.
pub(crate) fn title<S: AbstractString>(s: &S, all_words: bool) -> Result<S> {
    let limit = if all_words { None } else { Some(1) };
    let out = Regex::new(b"/\\b./u", "preg_replace_callback")?.replace_callback(
        s.as_bytes(),
        limit,
        |m| {
            let c = std::str::from_utf8(m[0].unwrap_or_default()).unwrap_or_default();
            Ok(c.chars()
                .map(mb::title_char)
                .collect::<String>()
                .into_bytes())
        },
    )?;
    Ok(s.with_string(out))
}

/// The code points a `trim()` list names: each one, and with `..` in the
/// list, `x..y` as the range from `x` to `y` (`quoteCharRanges()`).
fn char_class(chars: &[u8]) -> Result<Vec<(u32, u32)>> {
    let chars = std::str::from_utf8(chars).map_err(|_| Error::invalid("Invalid UTF-8 chars."))?;
    let list: Vec<char> = chars.chars().collect();
    if !chars.contains("..") {
        return Ok(list.iter().map(|&c| (c as u32, c as u32)).collect());
    }
    let mut class = Vec::new();
    let n = list.len();
    let mut i = 0;
    while i < n {
        if i + 3 < n && list[i + 1] == '.' && list[i + 2] == '.' && list[i] <= list[i + 3] {
            class.push((list[i] as u32, list[i + 3] as u32));
            i += 4;
            continue;
        }
        if list[i] != '.' || list.get(i + 1) != Some(&'.') {
            class.push((list[i] as u32, list[i] as u32));
        }
        i += 1;
    }
    Ok(class)
}

/// `trim()`, `trimStart()` and `trimEnd()`: `preg_replace()` of runs of the
/// listed code points at either end.
pub(crate) fn trim<S: AbstractString>(
    s: &S,
    chars: Option<&[u8]>,
    left: bool,
    right: bool,
) -> Result<S> {
    let chars = chars.unwrap_or(TRIM_CHARS.as_bytes());
    if chars.is_empty() {
        return Ok(s.cloned());
    }
    let class = char_class(chars)?;
    let member = |c: char| {
        class
            .iter()
            .any(|&(lo, hi)| (lo..=hi).contains(&(c as u32)))
    };
    let mut t = text(s);
    if left {
        t = t.trim_start_matches(member);
    }
    if right {
        t = t.trim_end_matches(member);
    }
    Ok(s.with_string(t.as_bytes().to_vec()))
}

/// `AbstractUnicodeString::trimPrefix()` / `trimSuffix()` under `ignoreCase()`:
/// `preg_replace("{^(?:$prefix)}iuD", '')` with each prefix quoted.
pub(crate) fn trim_affix_ignore_case<S: AbstractString>(
    s: &S,
    affix: Needle,
    suffix: bool,
) -> Result<S> {
    let list: Vec<Vec<u8>> = match affix {
        Needle::One(a) => vec![a],
        Needle::Many(a) => a,
    };
    let alternatives: Vec<Vec<u8>> = list.iter().map(|a| php::preg_quote(a)).collect();
    let body = alternatives.join(&b'|');
    let pattern = if suffix {
        [b"{(?:".as_slice(), &body, b")$}iuD"].concat()
    } else {
        [b"{^(?:".as_slice(), &body, b")}iuD"].concat()
    };
    let out = Regex::new(&pattern, "preg_replace")?.replace(s.as_bytes(), b"", None)?;
    Ok(s.with_string(out))
}

/// `width($ignoreAnsiDecoration)`: the widest line, in terminal columns.
pub(crate) fn width(string: &str, ignore_ansi_decoration: bool) -> usize {
    let mut s = string.to_owned();
    if ignore_ansi_decoration && s.contains('\x07') {
        s = remove_bel_osc(&s);
    }
    s.retain(|c| !matches!(c, '\x00' | '\x05' | '\x07'));
    if s.contains('\r') {
        s = s.replace("\r\n", "\n").replace('\r', "\n");
    }
    if !ignore_ansi_decoration {
        s.retain(|c| !is_cc(c));
    }
    let mut width = 0;
    for line in s.split('\n') {
        let line = if ignore_ansi_decoration {
            remove_ansi(line)
        } else {
            line.to_owned()
        };
        let line_width = wcswidth(&line);
        if line_width > width as i64 {
            width = line_width as usize;
        }
    }
    width
}

/// `\p{Cc}` or DEL: C0 and C1 controls.
fn is_cc(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{1F}' | '\u{7F}'..='\u{9F}')
}

/// `preg_replace('/\x1B\][^\x07\x1B\n\r]*+\x07/', '', $s)`.
fn remove_bel_osc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find("\x1B]") {
        let body = &rest[at + 2..];
        let end = body
            .find(['\x07', '\x1B', '\n', '\r'])
            .filter(|&e| body[e..].starts_with('\x07'));
        if let Some(e) = end {
            out.push_str(&rest[..at]);
            rest = &body[e + 1..];
        } else {
            out.push_str(&rest[..at + 2]);
            rest = body;
        }
    }
    out.push_str(rest);
    out
}

/// The per-line `preg_replace()` of `width()`: escape sequences (CSI; DCS,
/// OSC, SOS, PM and APC up to ST; any other ESC with its intermediate and
/// final bytes) and runs of control characters.
fn remove_ansi(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    let in_range = |b: u8, lo: u8, hi: u8| (lo..=hi).contains(&b);
    while at < bytes.len() {
        if bytes[at] == 0x1B {
            let next = at + 1;
            // \[ [\x30-\x3F]*+ [\x20-\x2F]*+ [\x40-\x7E]
            if bytes.get(next) == Some(&b'[') {
                let mut k = next + 1;
                while bytes.get(k).is_some_and(|&b| in_range(b, 0x30, 0x3F)) {
                    k += 1;
                }
                while bytes.get(k).is_some_and(|&b| in_range(b, 0x20, 0x2F)) {
                    k += 1;
                }
                if bytes.get(k).is_some_and(|&b| in_range(b, 0x40, 0x7E)) {
                    at = k + 1;
                    continue;
                }
            }
            // [P\]X^_] .*? \x1B\\
            if bytes.get(next).is_some_and(|b| b"P]X^_".contains(b))
                && let Some(end) = line[next + 1..].find("\x1B\\")
            {
                at = next + 1 + end + 2;
                continue;
            }
            // [\x20-\x2F]*+ [\x30-\x7E]?
            let mut k = next;
            while bytes.get(k).is_some_and(|&b| in_range(b, 0x20, 0x2F)) {
                k += 1;
            }
            if bytes.get(k).is_some_and(|&b| in_range(b, 0x30, 0x7E)) {
                k += 1;
            }
            at = k;
            continue;
        }
        let c = line[at..].chars().next().unwrap_or('\0');
        if !is_cc(c) {
            out.push(c);
        }
        at += c.len_utf8();
    }
    out
}

/// `EMOJI_REGEX` (PCRE2 10.40 and later): `[\p{ExtPict}\x{1F1E6}-\x{1F1FF}#*0-9]`.
pub(crate) fn is_emoji(c: char) -> bool {
    let cp = c as u32;
    matches!(c, '#' | '*' | '0'..='9')
        || (0x1F1E6..=0x1F1FF).contains(&cp)
        || in_table(tables::EXTPICT, cp)
}

pub(crate) fn in_table(table: &[(u32, u32)], cp: u32) -> bool {
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

/// `wcswidth()`: -1 if a control character remains.
fn wcswidth(string: &str) -> i64 {
    let mut width = 0i64;
    let mut last_char: Option<char> = None;
    let mut last_width = 0;
    let mut joined = false;
    // A character after which ZWJ, VS16 or a skin tone joins: non-ASCII
    // (`1 < strlen($lastChar)`) and an emoji.
    let joinable = |c: Option<char>| c.is_some_and(|c| !c.is_ascii() && is_emoji(c));
    for c in string.chars() {
        if joined {
            joined = false;
            if !c.is_ascii() && is_emoji(c) {
                continue;
            }
        }
        let cp = c as u32;
        if (0x20..0x7F).contains(&cp) {
            width += 1;
            last_char = Some(c);
            last_width = 1;
            continue;
        }
        if cp == 0
            || cp == 0x034F
            || cp == 0x061C
            || (0x1160..=0x11FF).contains(&cp)
            || cp == 0x180E
            || (0x200B..=0x200F).contains(&cp)
            || cp == 0x2028
            || cp == 0x2029
            || (0x202A..=0x202E).contains(&cp)
            || (0x2060..=0x206F).contains(&cp)
            || (0xD7B0..=0xD7FF).contains(&cp)
            || cp == 0xFEFF
            || (0xFFF0..=0xFFFB).contains(&cp)
        {
            joined = cp == 0x200D && joinable(last_char);
            continue;
        }
        if cp < 0xA0 {
            return -1;
        }
        if cp == 0xFE0F {
            if last_char.is_some_and(is_emoji) && last_width == 1 {
                width += 1;
                last_width = 2;
            }
            continue;
        }
        if cp > 0xFFFF {
            if (0x13430..=0x1343F).contains(&cp)
                || (0x1BCA0..=0x1BCA3).contains(&cp)
                || (0x1D173..=0x1D17A).contains(&cp)
                || (0xE0000..=0xE0FFF).contains(&cp)
            {
                continue;
            }
            if (0x1F3FB..=0x1F3FF).contains(&cp) && joinable(last_char) {
                if last_width == 1 {
                    width += 1;
                    last_width = 2;
                }
                continue;
            }
        }
        if in_table(tables::ZERO, cp) {
            continue;
        }
        if in_table(tables::WIDE, cp) {
            width += 2;
            last_char = Some(c);
            last_width = 2;
            continue;
        }
        width += 1;
        last_char = Some(c);
        last_width = 1;
    }
    width
}
