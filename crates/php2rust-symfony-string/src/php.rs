//! PHP's native string functions as symfony/string calls them, on bytes:
//! `wordwrap()` (on `AbstractString::wordwrap()`'s ASCII mask), the case
//! functions (`strtolower()`, `ucfirst()`, `ucwords()`, ASCII-only and
//! locale-independent since PHP 8.2), `str_pad()`, `substr()`,
//! `substr_replace()`, `substr_compare()`, the `str*pos()` family, the
//! `str*cmp()` family, `str_replace()`, `str_ireplace()`, `explode()`,
//! `trim()` with its `a..z` ranges, and `str_split()`.
//!
//! Written from the PHP manual (no php-src, PLAN.md decision 8) and checked
//! against calls recorded from PHP (`tests/data/wordwrap.json`,
//! `tests/data/builtins.json`, `scripts/record.sh string`). Where PHP
//! throws a `\ValueError` (an offset outside the haystack), these return
//! `None`, which is what symfony/string turns it into.
//!
//! `wordwrap()`, from the manual: the string is wrapped at `width`
//! characters using `break`; with `cut_long_words`, words longer than
//! `width` are split, so no line is longer than `width`. As observed:
//! breaks already in the string start a new line; a line is filled
//! greedily, word by word, a space becoming the break where the next word
//! no longer fits; consecutive spaces are words of length zero.
//!
//! Upstream: none

/// `wordwrap($text, $width, $break, $cut)` for a one-byte `brk`. PHP
/// refuses `$cut` with a width of 0 (`ValueError`); callers never pass it.
pub fn wordwrap(text: &[u8], width: usize, brk: u8, cut: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + text.len() / width.max(1));
    for (index, segment) in text.split(|&b| b == brk).enumerate() {
        if index > 0 {
            out.push(brk);
        }
        wrap_segment(segment, width, brk, cut, &mut out);
    }
    out
}

fn wrap_segment(segment: &[u8], width: usize, brk: u8, cut: bool, out: &mut Vec<u8>) {
    let mut line_length = 0;
    for (index, word) in segment.split(|&b| b == b' ').enumerate() {
        if index > 0 {
            // A space that starts a line is kept: a break never makes an
            // empty line.
            if line_length == 0 || line_length + 1 + word.len() <= width {
                out.push(b' ');
                line_length += 1;
            } else {
                out.push(brk);
                line_length = 0;
            }
        }
        let mut word = word;
        if cut {
            while line_length + word.len() > width && word.len() > width - line_length.min(width) {
                let room = width - line_length;
                if room == 0 {
                    out.push(brk);
                    line_length = 0;
                    continue;
                }
                out.extend_from_slice(&word[..room]);
                out.push(brk);
                word = &word[room..];
                line_length = 0;
            }
        }
        out.extend_from_slice(word);
        line_length += word.len();
    }
}

/// `strtolower()`.
pub fn strtolower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

/// `strtoupper()`.
pub fn strtoupper(s: &[u8]) -> Vec<u8> {
    s.to_ascii_uppercase()
}

/// `ucfirst()`.
pub fn ucfirst(s: &[u8]) -> Vec<u8> {
    let mut out = s.to_vec();
    if let Some(b) = out.first_mut() {
        b.make_ascii_uppercase();
    }
    out
}

/// `lcfirst()`.
pub fn lcfirst(s: &[u8]) -> Vec<u8> {
    let mut out = s.to_vec();
    if let Some(b) = out.first_mut() {
        b.make_ascii_lowercase();
    }
    out
}

/// `ucwords($s)`: the first byte and every byte after one of the default
/// delimiters (space, tab, CR, LF, form feed, vertical tab) uppercased.
pub fn ucwords(s: &[u8]) -> Vec<u8> {
    let mut out = s.to_vec();
    let mut start = true;
    for b in &mut out {
        if start {
            b.make_ascii_uppercase();
        }
        start = matches!(*b, b' ' | b'\t' | b'\r' | b'\n' | 0x0C | 0x0B);
    }
    out
}

/// `ctype_upper($s)` in the C locale: not empty, every byte `A`-`Z`.
pub fn ctype_upper(s: &[u8]) -> bool {
    !s.is_empty() && s.iter().all(u8::is_ascii_uppercase)
}

/// The side `str_pad()` pads on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadType {
    Left,
    Right,
    Both,
}

/// `str_pad($s, $length, $pad, $type)`. `None` for an empty pad where
/// padding is needed, which PHP refuses with a `ValueError`.
pub fn str_pad(s: &[u8], length: i64, pad: &[u8], kind: PadType) -> Option<Vec<u8>> {
    let free = usize::try_from(length).unwrap_or(0).saturating_sub(s.len());
    if free == 0 {
        return Some(s.to_vec());
    }
    if pad.is_empty() {
        return None;
    }
    let (left, right) = match kind {
        PadType::Left => (free, 0),
        PadType::Right => (0, free),
        PadType::Both => (free / 2, free - free / 2),
    };
    let padding = |n: usize| pad.iter().copied().cycle().take(n);
    let mut out = Vec::with_capacity(s.len() + free);
    out.extend(padding(left));
    out.extend_from_slice(s);
    out.extend(padding(right));
    Some(out)
}

/// The range `substr($s, $start, $length)` returns, for a string of `len`
/// units: a negative start counts from the end; a negative length leaves
/// that many units off the end; out of range gives an empty range.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn substr_range(len: usize, start: i64, length: Option<i64>) -> (usize, usize) {
    let len_i = len as i64;
    let from = if start < 0 {
        (len_i.saturating_add(start)).max(0)
    } else {
        start.min(len_i)
    };
    let to = match length {
        None => len_i,
        Some(l) if l < 0 => (len_i.saturating_add(l)).max(0),
        Some(l) => from.saturating_add(l).min(len_i),
    };
    if to <= from {
        return (from as usize, from as usize);
    }
    (from as usize, to as usize)
}

/// `substr($s, $start, $length)`.
pub fn substr(s: &[u8], start: i64, length: Option<i64>) -> &[u8] {
    let (from, to) = substr_range(s.len(), start, length);
    &s[from..to]
}

/// `substr_replace($s, $replacement, $start, $length)`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn substr_replace(s: &[u8], replacement: &[u8], start: i64, length: Option<i64>) -> Vec<u8> {
    let len = s.len() as i64;
    let from = if start < 0 {
        (len.saturating_add(start)).max(0)
    } else {
        start.min(len)
    };
    let to = match length {
        None => len,
        Some(l) if l < 0 => (len.saturating_add(l)).max(from),
        Some(l) => from.saturating_add(l).min(len),
    };
    let (from, to) = (from as usize, (to as usize).max(from as usize));
    let mut out = Vec::with_capacity(s.len() + replacement.len());
    out.extend_from_slice(&s[..from]);
    out.extend_from_slice(replacement);
    out.extend_from_slice(&s[to..]);
    out
}

/// `strrev()`.
pub fn strrev(s: &[u8]) -> Vec<u8> {
    s.iter().rev().copied().collect()
}

fn fold(s: &[u8], ci: bool) -> std::borrow::Cow<'_, [u8]> {
    if ci {
        std::borrow::Cow::Owned(s.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

pub(crate) fn find_from(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    if n.len() > h.len() || from > h.len() - n.len() {
        return None;
    }
    (from..=h.len() - n.len()).find(|&i| &h[i..i + n.len()] == n)
}

/// `strpos($h, $n, $offset)` (`stripos()` with `ci`). `None` for no match
/// and for an offset outside the haystack (a `ValueError` in PHP).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn strpos(h: &[u8], n: &[u8], offset: i64, ci: bool) -> Option<usize> {
    let len = h.len() as i64;
    let offset = if offset < 0 { offset + len } else { offset };
    if offset < 0 || offset > len {
        return None;
    }
    find_from(&fold(h, ci), &fold(n, ci), offset as usize)
}

/// The lowest and highest start `strrpos()` and `mb_strrpos()` accept, in
/// units: a non-negative offset is the lowest start; a negative one counts
/// from the end and bounds the start from above. `None` when PHP throws a
/// `ValueError` (the offset is outside the haystack).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn rpos_bounds(len: usize, needle_len: usize, offset: i64) -> Option<(usize, usize)> {
    let len_i = len as i64;
    if offset > len_i || offset < -len_i {
        return None;
    }
    if needle_len > len {
        return Some((1, 0));
    }
    let last = len - needle_len;
    if offset >= 0 {
        Some((offset as usize, last))
    } else {
        Some((0, last.min((len_i + offset) as usize)))
    }
}

/// `strrpos($h, $n, $offset)` (`strripos()` with `ci`).
pub fn strrpos(h: &[u8], n: &[u8], offset: i64, ci: bool) -> Option<usize> {
    let (lo, hi) = rpos_bounds(h.len(), n.len(), offset)?;
    if lo > hi {
        return None;
    }
    let (h, n) = (fold(h, ci), fold(n, ci));
    (lo..=hi).rev().find(|&i| h[i..i + n.len()] == *n)
}

/// `strcasecmp($a, $b) === 0`.
pub fn strcasecmp_eq(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// `strncmp($a, $b, $n) === 0` (`strncasecmp()` with `ci`).
pub fn strncmp_eq(a: &[u8], b: &[u8], n: usize, ci: bool) -> bool {
    let (a, b) = (&a[..a.len().min(n)], &b[..b.len().min(n)]);
    if ci {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// `substr_compare($main, $str, $offset, $length, $ci) === 0`. `None` for
/// an offset outside `$main` (a `ValueError`).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn substr_compare_eq(
    main: &[u8],
    s: &[u8],
    offset: i64,
    length: Option<usize>,
    ci: bool,
) -> Option<bool> {
    let len = main.len() as i64;
    let offset = if offset < 0 {
        (len + offset).max(0)
    } else {
        offset
    };
    if offset > len {
        return None;
    }
    let rest = &main[offset as usize..];
    let n = length.unwrap_or_else(|| s.len().max(rest.len()));
    Some(strncmp_eq(rest, s, n, ci))
}

/// `str_replace($from, $to, $s)` (`str_ireplace()` with `ci`): every
/// occurrence, left to right, without overlap.
pub fn str_replace(from: &[u8], to: &[u8], s: &[u8], ci: bool) -> Vec<u8> {
    if from.is_empty() || from.len() > s.len() {
        return s.to_vec();
    }
    let (fs, ff) = (fold(s, ci), fold(from, ci));
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if i + ff.len() <= s.len() && fs[i..i + ff.len()] == *ff {
            out.extend_from_slice(to);
            i += ff.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

/// `explode($delimiter, $s, $limit)` for a positive limit and a non-empty
/// delimiter.
#[expect(
    clippy::cast_possible_wrap,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn explode<'a>(delimiter: &[u8], s: &'a [u8], limit: i64) -> Vec<&'a [u8]> {
    let mut out = Vec::new();
    let mut rest = s;
    while (out.len() as i64) < limit - 1 {
        match find_from(rest, delimiter, 0) {
            Some(at) => {
                out.push(&rest[..at]);
                rest = &rest[at + delimiter.len()..];
            }
            None => break,
        }
    }
    out.push(rest);
    out
}

/// The bytes `trim()`'s character list names: each byte, and `x..y` as the
/// range from `x` to `y` when `y >= x`.
pub fn charmask(chars: &[u8]) -> [bool; 256] {
    let mut mask = [false; 256];
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if i + 3 < chars.len() && chars[i + 1] == b'.' && chars[i + 2] == b'.' && chars[i + 3] >= c
        {
            for b in c..=chars[i + 3] {
                mask[b as usize] = true;
            }
            i += 4;
            continue;
        }
        mask[c as usize] = true;
        i += 1;
    }
    mask
}

/// `trim($s, $chars)` (`ltrim()` with `right` false, `rtrim()` with `left`
/// false).
pub fn trim(s: &[u8], chars: &[u8], left: bool, right: bool) -> Vec<u8> {
    let mask = charmask(chars);
    let mut from = 0;
    let mut to = s.len();
    if left {
        while from < to && mask[s[from] as usize] {
            from += 1;
        }
    }
    if right {
        while to > from && mask[s[to - 1] as usize] {
            to -= 1;
        }
    }
    s[from..to].to_vec()
}

/// `str_split($s, $length)` for a non-empty string.
pub fn str_split(s: &[u8], length: usize) -> Vec<&[u8]> {
    s.chunks(length.max(1)).collect()
}

/// `preg_quote($s)`, without a delimiter.
pub fn preg_quote(s: &[u8]) -> Vec<u8> {
    php_pcre::preg_quote(s, None)
}
