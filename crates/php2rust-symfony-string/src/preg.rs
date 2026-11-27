//! PHP's `preg_match()`, `preg_match_all()`, `preg_split()`,
//! `preg_replace()` and `preg_replace_callback()` as symfony/string calls
//! them, on patterns compiled by php-pcre. Written from the PHP manual (no
//! php-src, PLAN.md decision 8).
//!
//! Matches are found left to right; after an empty match the next search
//! starts one character later (one code point under `/u`), as php-pcre's
//! `replace_callback()` does. PHP retries an empty match's position for a
//! non-empty match first; patterns where that differs are not among the
//! recorded cases. Named groups are refused: `match()` would have to
//! return them under their names too, which php-pcre does not report.
//!
//! Upstream: none

use php_pcre::{Groups, Pcre, PcreError};
use php_value::{PhpArray, Value};

use crate::exception::{Error, Result};

/// `PREG_PATTERN_ORDER`.
pub const PATTERN_ORDER: i64 = 1;
/// `PREG_SET_ORDER`.
pub const SET_ORDER: i64 = 2;
/// `PREG_OFFSET_CAPTURE`.
pub const OFFSET_CAPTURE: i64 = 256;
/// `PREG_UNMATCHED_AS_NULL`.
pub const UNMATCHED_AS_NULL: i64 = 512;
/// `PREG_SPLIT_NO_EMPTY`.
pub const SPLIT_NO_EMPTY: i64 = 1;
/// `PREG_SPLIT_DELIM_CAPTURE`.
pub const SPLIT_DELIM_CAPTURE: i64 = 2;
/// `PREG_SPLIT_OFFSET_CAPTURE`.
pub const SPLIT_OFFSET_CAPTURE: i64 = 4;

/// A compiled pattern and whether it carries `/u`.
pub struct Regex {
    pcre: Pcre,
    utf8: bool,
}

fn modifiers(pattern: &[u8]) -> &[u8] {
    let Some(&open) = pattern.iter().find(|b| !b.is_ascii_whitespace()) else {
        return &[];
    };
    let close = match open {
        b'(' => b')',
        b'{' => b'}',
        b'[' => b']',
        b'<' => b'>',
        other => other,
    };
    match pattern.iter().rposition(|&b| b == close) {
        Some(at) if at > 0 => &pattern[at + 1..],
        _ => &[],
    }
}

fn has_named_group(pattern: &[u8]) -> bool {
    pattern.windows(3).enumerate().any(|(i, w)| {
        let escaped = pattern[..i]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count()
            % 2
            == 1;
        !escaped
            && w[0] == b'('
            && w[1] == b'?'
            && (w[2] == b'\''
                || w[2] == b'P' && pattern.get(i + 3) == Some(&b'<')
                || w[2] == b'<' && !matches!(pattern.get(i + 3), Some(b'=' | b'!')))
    })
}

impl Regex {
    /// Compiles a delimited pattern. A pattern PHP rejects is an
    /// `InvalidArgumentException` (upstream turns the warning into one);
    /// one php-pcre does not translate is refused.
    pub fn new(pattern: &[u8], function: &str) -> Result<Self> {
        if has_named_group(pattern) {
            return Err(Error::Unsupported(
                "named groups are not supported by this port".into(),
            ));
        }
        let pcre = Pcre::new(pattern).map_err(|e| match e {
            PcreError::Compile(m) => Error::invalid(format!("{function}(): {m}")),
            PcreError::Unsupported(m) => Error::Unsupported(m),
        })?;
        Ok(Self {
            utf8: modifiers(pattern).contains(&b'u'),
            pcre,
        })
    }

    fn step(&self, subject: &[u8], at: usize) -> usize {
        if self.utf8 {
            let mut next = at + 1;
            while next < subject.len() && subject[next] & 0xC0 == 0x80 {
                next += 1;
            }
            next
        } else {
            at + 1
        }
    }

    fn check_subject(&self, subject: &[u8], function: &str) -> Result<()> {
        if self.utf8 && std::str::from_utf8(subject).is_err() {
            return Err(Error::runtime(format!(
                "{function} failed with error: Malformed UTF-8 characters, possibly incorrectly encoded"
            )));
        }
        Ok(())
    }

    /// Every match from `offset`, at most `limit`.
    pub fn all(&self, subject: &[u8], offset: usize, limit: Option<usize>) -> Result<Vec<Groups>> {
        let mut found = Vec::new();
        let mut pos = offset;
        while pos <= subject.len() && limit.is_none_or(|l| found.len() < l) {
            let groups = self
                .pcre
                .captures_from(subject, pos)
                .map_err(|e| Error::runtime(format!("Matching failed with error: {e}")))?;
            let Some(groups) = groups else { break };
            let Some((start, end)) = groups[0] else { break };
            pos = if end == start {
                self.step(subject, end)
            } else {
                end
            };
            found.push(groups);
        }
        Ok(found)
    }

    /// The first match from `offset`.
    pub fn first(&self, subject: &[u8], offset: usize) -> Result<Option<Groups>> {
        Ok(self.all(subject, offset, Some(1))?.into_iter().next())
    }

    /// `preg_replace_callback($pattern, $callback, $subject, $limit)`.
    pub fn replace_callback(
        &self,
        subject: &[u8],
        limit: Option<usize>,
        mut callback: impl FnMut(&[Option<&[u8]>]) -> Result<Vec<u8>>,
    ) -> Result<Vec<u8>> {
        self.check_subject(subject, "Matching")?;
        let mut out = Vec::with_capacity(subject.len());
        let mut last = 0;
        for groups in self.all(subject, 0, limit)? {
            let Some((start, end)) = groups[0] else {
                continue;
            };
            let texts: Vec<Option<&[u8]>> = groups
                .iter()
                .map(|g| g.map(|(s, e)| &subject[s..e]))
                .collect();
            out.extend_from_slice(&subject[last..start]);
            out.extend(callback(&texts)?);
            last = end;
        }
        out.extend_from_slice(&subject[last..]);
        Ok(out)
    }

    /// `preg_replace($pattern, $replacement, $subject, $limit)`: `$n`,
    /// `${n}` and `\n` (n up to 99) name groups; an unmatched or missing
    /// group is empty.
    pub fn replace(
        &self,
        subject: &[u8],
        replacement: &[u8],
        limit: Option<usize>,
    ) -> Result<Vec<u8>> {
        self.replace_callback(subject, limit, |groups| Ok(expand(replacement, groups)))
    }

    /// `preg_split($pattern, $subject, $limit, $flags)`: the pieces, each
    /// with its byte offset.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    pub fn split(&self, subject: &[u8], limit: i64, flags: i64) -> Result<Vec<(Vec<u8>, usize)>> {
        self.check_subject(subject, "Splitting")?;
        let no_empty = flags & SPLIT_NO_EMPTY != 0;
        let delim_capture = flags & SPLIT_DELIM_CAPTURE != 0;
        let limit = if limit <= 0 {
            None
        } else {
            Some(limit as usize)
        };
        let mut pieces: Vec<(Vec<u8>, usize)> = Vec::new();
        let mut last = 0;
        let mut pos = 0;
        while pos <= subject.len() {
            if limit.is_some_and(|l| pieces.len() + 1 >= l) {
                break;
            }
            let groups = self
                .pcre
                .captures_from(subject, pos)
                .map_err(|e| Error::runtime(format!("Splitting failed with error: {e}")))?;
            let Some(groups) = groups else { break };
            let Some((start, end)) = groups[0] else { break };
            if start == end && start == subject.len() {
                break;
            }
            if !(no_empty && start == last) {
                pieces.push((subject[last..start].to_vec(), last));
            }
            if delim_capture {
                let matched = groups.iter().rposition(Option::is_some).unwrap_or(0);
                for (s, e) in groups[1..=matched]
                    .iter()
                    .map(|g| g.unwrap_or((start, start)))
                {
                    if !(no_empty && s == e) {
                        pieces.push((subject[s..e].to_vec(), s));
                    }
                }
            }
            last = end;
            pos = if start == end {
                self.step(subject, end)
            } else {
                end
            };
        }
        if !(no_empty && last == subject.len()) {
            pieces.push((subject[last..].to_vec(), last));
        }
        Ok(pieces)
    }
}

fn expand(replacement: &[u8], groups: &[Option<&[u8]>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    let digits_at = |j: usize| -> (usize, usize) {
        let mut n = 0;
        let mut k = j;
        while k < replacement.len() && k < j + 2 && replacement[k].is_ascii_digit() {
            n = n * 10 + usize::from(replacement[k] - b'0');
            k += 1;
        }
        (n, k)
    };
    while i < replacement.len() {
        let c = replacement[i];
        let next = replacement.get(i + 1).copied();
        if (c == b'$' || c == b'\\') && next.is_some_and(|b| b.is_ascii_digit()) {
            let (n, k) = digits_at(i + 1);
            if let Some(Some(g)) = groups.get(n) {
                out.extend_from_slice(g);
            }
            i = k;
        } else if c == b'$'
            && next == Some(b'{')
            && replacement.get(i + 2).is_some_and(u8::is_ascii_digit)
        {
            let (n, k) = digits_at(i + 2);
            if replacement.get(k) == Some(&b'}') {
                if let Some(Some(g)) = groups.get(n) {
                    out.extend_from_slice(g);
                }
                i = k + 1;
            } else {
                out.push(c);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

#[expect(
    clippy::cast_possible_wrap,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
fn group_value(subject: &[u8], group: Option<(usize, usize)>, offset_capture: bool) -> Value {
    let text = match group {
        Some((s, e)) => Value::string(subject[s..e].to_vec()),
        None => Value::Null,
    };
    if !offset_capture {
        return text;
    }
    let mut pair = PhpArray::new();
    pair.insert(0, text);
    pair.insert(1, Value::Int(group.map_or(-1, |(s, _)| s as i64)));
    Value::Array(pair)
}

#[expect(
    clippy::cast_possible_wrap,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
fn groups_array(subject: &[u8], groups: &Groups, offset_capture: bool) -> PhpArray {
    let mut array = PhpArray::new();
    for (i, g) in groups.iter().enumerate() {
        array.insert(i as i64, group_value(subject, *g, offset_capture));
    }
    array
}

/// `preg_match()` or, with `PREG_PATTERN_ORDER` or `PREG_SET_ORDER` in
/// `flags`, `preg_match_all()`, always with `PREG_UNMATCHED_AS_NULL`
/// (symfony/string adds it): the `$matches` array.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub fn matches(regex: &Regex, subject: &[u8], flags: i64, offset: i64) -> Result<Value> {
    regex.check_subject(subject, "Matching")?;
    let offset_capture = flags & OFFSET_CAPTURE != 0;
    let len = subject.len() as i64;
    let offset = if offset < 0 {
        (len + offset).max(0)
    } else {
        offset
    };
    if offset > len {
        return Err(Error::runtime(
            "Matching failed with error: Internal error".to_owned(),
        ));
    }
    let offset = offset as usize;
    if flags & (PATTERN_ORDER | SET_ORDER) == 0 {
        return Ok(Value::Array(match regex.first(subject, offset)? {
            Some(groups) => groups_array(subject, &groups, offset_capture),
            None => PhpArray::new(),
        }));
    }
    let found = regex.all(subject, offset, None)?;
    let mut out = PhpArray::new();
    if flags & SET_ORDER != 0 {
        for (i, groups) in found.iter().enumerate() {
            out.insert(
                i as i64,
                Value::Array(groups_array(subject, groups, offset_capture)),
            );
        }
        return Ok(Value::Array(out));
    }
    let count = regex
        .pcre
        .group_count()
        .max(found.first().map_or(1, Vec::len));
    for g in 0..count {
        let mut column = PhpArray::new();
        for (i, groups) in found.iter().enumerate() {
            column.insert(
                i as i64,
                group_value(subject, groups.get(g).copied().flatten(), offset_capture),
            );
        }
        out.insert(g as i64, Value::Array(column));
    }
    Ok(Value::Array(out))
}
