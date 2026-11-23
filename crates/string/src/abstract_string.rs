//! Port of Symfony\Component\String\AbstractString.php and
//! TruncateMode.php.
//!
//! The abstract class is the trait [`AbstractString`]: its concrete methods
//! are the trait's provided methods, its abstract ones the required
//! methods [`ByteString`](crate::ByteString),
//! [`CodePointString`](crate::CodePointString) and
//! [`UnicodeString`](crate::UnicodeString) implement. `string|iterable`
//! arguments are a [`Needle`]; a callable `$to` is a [`Replacement`].
//! Strings are bytes (`&[u8]`, `impl AsRef<[u8]>`), as in PHP: a
//! ByteString holds any bytes, the Unicode classes refuse invalid UTF-8.
//!
//! PHP's `clone` resets `ignoreCase` (`__clone()`), so every method returns
//! a case-sensitive string except [`AbstractString::ignore_case`].
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/string AbstractString.php TruncateMode.php

use php_value::{Key, PhpArray, Value};

use crate::byte_string::ByteString;
use crate::code_point_string::CodePointString;
use crate::exception::{Error, Result};
use crate::mb::{self, CodePage};
use crate::preg::{self, Regex};
use crate::unicode_string::UnicodeString;
use crate::{php, preg::SPLIT_OFFSET_CAPTURE};

/// A `string|iterable` argument: one string, or several tried in turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Needle {
    One(Vec<u8>),
    Many(Vec<Vec<u8>>),
}

impl From<&str> for Needle {
    fn from(s: &str) -> Self {
        Self::One(s.as_bytes().to_vec())
    }
}

impl From<String> for Needle {
    fn from(s: String) -> Self {
        Self::One(s.into_bytes())
    }
}

impl From<&[u8]> for Needle {
    fn from(s: &[u8]) -> Self {
        Self::One(s.to_vec())
    }
}

impl From<Vec<u8>> for Needle {
    fn from(s: Vec<u8>) -> Self {
        Self::One(s)
    }
}

impl From<&[&str]> for Needle {
    fn from(s: &[&str]) -> Self {
        Self::Many(s.iter().map(|n| n.as_bytes().to_vec()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for Needle {
    fn from(s: [&str; N]) -> Self {
        Self::Many(s.iter().map(|n| n.as_bytes().to_vec()).collect())
    }
}

impl From<Vec<Vec<u8>>> for Needle {
    fn from(s: Vec<Vec<u8>>) -> Self {
        Self::Many(s)
    }
}

impl<T: AbstractString> From<&T> for Needle {
    fn from(s: &T) -> Self {
        Self::One(s.as_bytes().to_vec())
    }
}

impl Needle {
    fn list(&self) -> Vec<&[u8]> {
        match self {
            Self::One(n) => vec![n.as_slice()],
            Self::Many(ns) => ns.iter().map(Vec::as_slice).collect(),
        }
    }
}

/// A `replaceMatches()` callback: given the groups (unmatched ones
/// `None`), the replacement.
pub type Callback<'a> = &'a mut dyn FnMut(&[Option<&[u8]>]) -> Vec<u8>;

/// `replaceMatches()`'s `$to`: a replacement string with `$n` references,
/// or a callback.
pub enum Replacement<'a> {
    Text(&'a [u8]),
    Callback(Callback<'a>),
}

/// `Symfony\Component\String\TruncateMode`; `truncate()`'s `bool` `$cut`
/// converts with `From<bool>` (`true` cuts like `Char`, `false` like
/// `WordAfter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateMode {
    /// Will cut exactly at given length.
    Char,
    /// Returns up to the previous word.
    WordBefore,
    /// Returns up to the next word.
    WordAfter,
}

impl From<bool> for TruncateMode {
    fn from(cut: bool) -> Self {
        if cut { Self::Char } else { Self::WordAfter }
    }
}

/// An element of what `wrap()` returns.
#[derive(Debug, Clone, PartialEq)]
pub enum Wrapped<S> {
    String(S),
    Array(Vec<(Key, Wrapped<S>)>),
    Other(Value),
}

/// `split()`'s chunks with `PREG_SPLIT_OFFSET_CAPTURE`: each with its byte
/// offset.
pub type OffsetChunks<S> = Vec<(S, i64)>;

const WORD_BREAKS: [&str; 4] = [" ", "\r", "\n", "\t"];

pub trait AbstractString: Sized + Clone + PartialEq + std::fmt::Debug {
    /// `new static($string)`.
    fn create(string: &[u8]) -> Result<Self>;

    /// The string's bytes (`$this->string`).
    fn as_bytes(&self) -> &[u8];

    /// `$str = clone $this; $str->string = $string;`.
    fn with_string(&self, string: Vec<u8>) -> Self;

    /// `clone $this`.
    fn cloned(&self) -> Self {
        self.with_string(self.as_bytes().to_vec())
    }

    /// Whether `$this->ignoreCase` is true.
    fn ignoring_case(&self) -> bool;

    /// `ignoreCase()`.
    fn ignore_case(&self) -> Self;

    /// Whether `$this instanceof AbstractUnicodeString`.
    const UNICODE: bool;

    /// `append(...$suffix)`.
    fn append(&self, suffix: &[&[u8]]) -> Result<Self>;

    /// `prepend(...$prefix)`.
    fn prepend(&self, prefix: &[&[u8]]) -> Result<Self>;

    /// `bytesAt($offset)`.
    fn bytes_at(&self, offset: i64) -> Vec<u8> {
        self.slice(offset, Some(1)).as_bytes().to_vec()
    }

    /// `camel()`.
    fn camel(&self) -> Result<Self>;

    /// `chunk($length)`.
    fn chunk(&self, length: i64) -> Result<Vec<Self>>;

    /// `endsWith($suffix)` for one string.
    fn ends_with_str(&self, suffix: &[u8]) -> bool;

    /// `equalsTo($string)` for one string.
    fn equals_to_str(&self, string: &[u8]) -> bool;

    /// `folded()`.
    fn folded(&self) -> Self;

    /// `indexOf($needle, $offset)` for one string.
    fn index_of_str(&self, needle: &[u8], offset: i64) -> Option<usize>;

    /// `indexOfLast($needle, $offset)` for one string.
    fn index_of_last_str(&self, needle: &[u8], offset: i64) -> Option<usize>;

    /// `join($strings, $lastGlue)`.
    fn join(&self, strings: &[&[u8]], last_glue: Option<&[u8]>) -> Result<Self>;

    /// `length()`.
    fn length(&self) -> usize;

    /// `lower()`.
    fn lower(&self) -> Self;

    /// `match($regexp, $flags, $offset)`: the `$matches` array.
    fn matches(&self, regexp: &[u8], flags: i64, offset: i64) -> Result<Value>;

    /// `padBoth($length, $padStr)`.
    fn pad_both(&self, length: i64, pad: &[u8]) -> Result<Self>;

    /// `padEnd($length, $padStr)`.
    fn pad_end(&self, length: i64, pad: &[u8]) -> Result<Self>;

    /// `padStart($length, $padStr)`.
    fn pad_start(&self, length: i64, pad: &[u8]) -> Result<Self>;

    /// `replace($from, $to)`.
    fn replace(&self, from: &[u8], to: &[u8]) -> Result<Self>;

    /// `replaceMatches($fromRegexp, $to)`.
    fn replace_matches(&self, from_regexp: &[u8], to: Replacement<'_>) -> Result<Self>;

    /// `reverse()`.
    fn reverse(&self) -> Self;

    /// `slice($start, $length)`.
    fn slice(&self, start: i64, length: Option<i64>) -> Self;

    /// `snake()`.
    fn snake(&self) -> Result<Self>;

    /// `splice($replacement, $start, $length)`.
    fn splice(&self, replacement: &[u8], start: i64, length: Option<i64>) -> Result<Self>;

    /// `split($delimiter, $limit)` without flags, after the checks every
    /// class makes.
    fn split_str(&self, delimiter: &[u8], limit: i64) -> Result<Vec<Self>>;

    /// The `$limit` a class's `split()` uses for `null`.
    const SPLIT_LIMIT: i64;

    /// `startsWith($prefix)` for one string.
    fn starts_with_str(&self, prefix: &[u8]) -> bool;

    /// `title($allWords)`.
    fn title(&self, all_words: bool) -> Result<Self>;

    /// `trim($chars)`, upstream's default list for `None`.
    fn trim(&self, chars: Option<&[u8]>) -> Result<Self>;

    /// `trimEnd($chars)`.
    fn trim_end(&self, chars: Option<&[u8]>) -> Result<Self>;

    /// `trimStart($chars)`.
    fn trim_start(&self, chars: Option<&[u8]>) -> Result<Self>;

    /// `upper()`.
    fn upper(&self) -> Self;

    /// `width($ignoreAnsiDecoration)`.
    fn width(&self, ignore_ansi_decoration: bool) -> usize;

    /// `unwrap($values)`: the strings of `wrap()`'s result.
    fn unwrap(values: &[(Key, Wrapped<Self>)]) -> PhpArray {
        let mut out = PhpArray::new();
        for (k, v) in values {
            let v = match v {
                Wrapped::String(s) => Value::string(s.as_bytes().to_vec()),
                Wrapped::Array(a) => Value::Array(Self::unwrap(a)),
                Wrapped::Other(o) => o.clone(),
            };
            out.insert(k.clone(), v);
        }
        out
    }

    /// `wrap($values)`: strings become instances, string keys are
    /// normalized like their values, arrays are wrapped recursively.
    fn wrap(values: &PhpArray) -> Result<Vec<(Key, Wrapped<Self>)>> {
        let mut out = Vec::with_capacity(values.len());
        for (k, v) in values.iter() {
            let key = match k {
                Key::Str(s) if !s.as_bytes().is_empty() => {
                    Key::from_bytes(Self::create(s.as_bytes())?.as_bytes())
                }
                other => other.clone(),
            };
            let value = match v {
                Value::String(s) => Wrapped::String(Self::create(s.as_bytes())?),
                Value::Array(a) => Wrapped::Array(Self::wrap(a)?),
                other => Wrapped::Other(other.clone()),
            };
            // array_combine(): a later duplicate key wins, at the first
            // key's place
            match out.iter_mut().find(|(existing, _)| *existing == key) {
                Some(slot) => slot.1 = value,
                None => out.push((key, value)),
            }
        }
        Ok(out)
    }

    /// `after($needle, $includeNeedle, $offset)`.
    fn after(&self, needle: impl Into<Needle>, include_needle: bool, offset: i64) -> Self {
        let needle = needle.into();
        let mut found: Option<(usize, &[u8])> = None;
        for n in needle.list() {
            if let Some(j) = self.index_of_str(n, offset)
                && found.is_none_or(|(i, _)| j < i)
            {
                found = Some((j, n));
            }
        }
        let Some((mut i, n)) = found else {
            return self.cloned();
        };
        if !include_needle {
            i += self.with_string(n.to_vec()).length();
        }
        self.slice(i as i64, None)
    }

    /// `afterLast($needle, $includeNeedle, $offset)`.
    fn after_last(&self, needle: impl Into<Needle>, include_needle: bool, offset: i64) -> Self {
        let needle = needle.into();
        let mut offset = offset;
        let mut found: Option<(usize, &[u8])> = None;
        for n in needle.list() {
            if let Some(j) = self.index_of_last_str(n, offset)
                && found.is_none_or(|(i, _)| j >= i)
            {
                offset = j as i64;
                found = Some((j, n));
            }
        }
        let Some((mut i, n)) = found else {
            return self.cloned();
        };
        if !include_needle {
            i += self.with_string(n.to_vec()).length();
        }
        self.slice(i as i64, None)
    }

    /// `before($needle, $includeNeedle, $offset)`.
    fn before(&self, needle: impl Into<Needle>, include_needle: bool, offset: i64) -> Self {
        let needle = needle.into();
        let mut found: Option<(usize, &[u8])> = None;
        for n in needle.list() {
            if let Some(j) = self.index_of_str(n, offset)
                && found.is_none_or(|(i, _)| j < i)
            {
                found = Some((j, n));
            }
        }
        let Some((mut i, n)) = found else {
            return self.cloned();
        };
        if include_needle {
            i += self.with_string(n.to_vec()).length();
        }
        self.slice(0, Some(i as i64))
    }

    /// `beforeLast($needle, $includeNeedle, $offset)`.
    fn before_last(&self, needle: impl Into<Needle>, include_needle: bool, offset: i64) -> Self {
        let needle = needle.into();
        let mut offset = offset;
        let mut found: Option<(usize, &[u8])> = None;
        for n in needle.list() {
            if let Some(j) = self.index_of_last_str(n, offset)
                && found.is_none_or(|(i, _)| j >= i)
            {
                offset = j as i64;
                found = Some((j, n));
            }
        }
        let Some((mut i, n)) = found else {
            return self.cloned();
        };
        if include_needle {
            i += self.with_string(n.to_vec()).length();
        }
        self.slice(0, Some(i as i64))
    }

    /// `collapseWhitespace()`: every run of spaces, tabs, CRs, LFs and form
    /// feeds becomes one space, then those are trimmed.
    fn collapse_whitespace(&self) -> Self {
        let ws = |b: u8| matches!(b, b' ' | b'\n' | b'\r' | b'\t' | 0x0C);
        let mut out = Vec::with_capacity(self.as_bytes().len());
        let mut in_run = false;
        for &b in self.as_bytes() {
            if ws(b) {
                if !in_run {
                    out.push(b' ');
                }
                in_run = true;
            } else {
                out.push(b);
                in_run = false;
            }
        }
        self.with_string(php::trim(&out, b" \n\r\t\x0C", true, true))
    }

    /// `containsAny($needle)`.
    fn contains_any(&self, needle: impl Into<Needle>) -> bool {
        self.index_of(needle, 0).is_some()
    }

    /// `endsWith($suffix)`.
    fn ends_with(&self, suffix: impl Into<Needle>) -> bool {
        suffix
            .into()
            .list()
            .into_iter()
            .any(|s| self.ends_with_str(s))
    }

    /// `ensureEnd($suffix)`.
    fn ensure_end(&self, suffix: impl AsRef<[u8]>) -> Result<Self> {
        let suffix = suffix.as_ref();
        if !self.ends_with_str(suffix) {
            return self.append(&[suffix]);
        }
        let quoted = php::preg_quote(suffix);
        let mut regex = b"{(".to_vec();
        regex.extend_from_slice(&quoted);
        regex.extend_from_slice(b")(?:");
        regex.extend_from_slice(&quoted);
        regex.extend_from_slice(b")++$}D");
        if self.ignoring_case() {
            regex.push(b'i');
        }
        self.replace_matches(&regex, Replacement::Text(b"$1"))
    }

    /// `ensureStart($prefix)`.
    fn ensure_start(&self, prefix: impl AsRef<[u8]>) -> Result<Self> {
        let prefix = Self::create(prefix.as_ref())?;
        if !self.starts_with_str(prefix.as_bytes()) {
            return self.prepend(&[prefix.as_bytes()]);
        }
        let mut str = self.cloned();
        let prefix_len = prefix.length();
        let mut i = prefix_len;
        while self.index_of_str(prefix.as_bytes(), i as i64) == Some(i) {
            str = str.slice(prefix_len as i64, None);
            i += prefix_len;
        }
        Ok(str)
    }

    /// `equalsTo($string)`.
    fn equals_to(&self, string: impl Into<Needle>) -> bool {
        string
            .into()
            .list()
            .into_iter()
            .any(|s| self.equals_to_str(s))
    }

    /// `indexOf($needle, $offset)`.
    fn index_of(&self, needle: impl Into<Needle>, offset: i64) -> Option<usize> {
        match needle.into() {
            Needle::One(n) => self.index_of_str(&n, offset),
            Needle::Many(ns) => ns.iter().filter_map(|n| self.index_of_str(n, offset)).min(),
        }
    }

    /// `indexOfLast($needle, $offset)`.
    fn index_of_last(&self, needle: impl Into<Needle>, offset: i64) -> Option<usize> {
        match needle.into() {
            Needle::One(n) => self.index_of_last_str(&n, offset),
            Needle::Many(ns) => {
                let mut offset = offset;
                let mut i = None;
                for n in &ns {
                    if let Some(j) = self.index_of_last_str(n, offset)
                        && i.is_none_or(|i| j >= i)
                    {
                        i = Some(j);
                        offset = j as i64;
                    }
                }
                i
            }
        }
    }

    /// `isEmpty()`.
    fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// `jsonSerialize()`.
    fn json_serialize(&self) -> &[u8] {
        self.as_bytes()
    }

    /// `kebab()`.
    fn kebab(&self) -> Result<Self> {
        self.snake()?.replace(b"_", b"-")
    }

    /// `pascal()`.
    fn pascal(&self) -> Result<Self> {
        self.camel()?.title(false)
    }

    /// `repeat($multiplier)`.
    fn repeat(&self, multiplier: i64) -> Result<Self> {
        let Ok(times) = usize::try_from(multiplier) else {
            return Err(Error::invalid(format!(
                "Multiplier must be positive, {multiplier} given."
            )));
        };
        Ok(self.with_string(self.as_bytes().repeat(times)))
    }

    /// The checks every class's `split()` makes before splitting.
    fn split_checks(&self, delimiter: &[u8], limit: Option<i64>) -> Result<i64> {
        let limit = limit.unwrap_or(Self::SPLIT_LIMIT);
        if limit < 1 {
            return Err(Error::invalid("Split limit must be a positive integer."));
        }
        if delimiter.is_empty() {
            return Err(Error::invalid("Split delimiter is empty."));
        }
        Ok(limit)
    }

    /// `split($delimiter, $limit, $flags)`. With flags, `$delimiter` is a
    /// pattern; `PREG_SPLIT_OFFSET_CAPTURE` needs
    /// [`AbstractString::split_offsets`].
    fn split(
        &self,
        delimiter: impl AsRef<[u8]>,
        limit: Option<i64>,
        flags: Option<i64>,
    ) -> Result<Vec<Self>> {
        let delimiter = delimiter.as_ref();
        let limit = self.split_checks(delimiter, limit)?;
        match flags {
            None => self.split_str(delimiter, limit),
            Some(flags) if flags & SPLIT_OFFSET_CAPTURE != 0 => Err(Error::Unsupported(
                "PREG_SPLIT_OFFSET_CAPTURE returns offsets: use split_offsets()".into(),
            )),
            Some(flags) => Ok(self
                .preg_split(delimiter, limit, flags)?
                .into_iter()
                .map(|(s, _)| s)
                .collect()),
        }
    }

    /// `split($delimiter, $limit, $flags | PREG_SPLIT_OFFSET_CAPTURE)`.
    fn split_offsets(
        &self,
        delimiter: impl AsRef<[u8]>,
        limit: Option<i64>,
        flags: i64,
    ) -> Result<OffsetChunks<Self>> {
        let delimiter = delimiter.as_ref();
        let limit = self.split_checks(delimiter, limit)?;
        self.preg_split(delimiter, limit, flags | SPLIT_OFFSET_CAPTURE)
    }

    /// AbstractString::split() with flags: `preg_split()`.
    fn preg_split(&self, delimiter: &[u8], limit: i64, flags: i64) -> Result<OffsetChunks<Self>> {
        let mut pattern = delimiter.to_vec();
        if Self::UNICODE {
            pattern.push(b'u');
        }
        if self.ignoring_case() {
            pattern.push(b'i');
        }
        let regex = Regex::new(&pattern, "preg_split")?;
        Ok(regex
            .split(self.as_bytes(), limit, flags)?
            .into_iter()
            .map(|(s, at)| (self.with_string(s), at as i64))
            .collect())
    }

    /// `startsWith($prefix)`.
    fn starts_with(&self, prefix: impl Into<Needle>) -> bool {
        prefix
            .into()
            .list()
            .into_iter()
            .any(|p| self.starts_with_str(p))
    }

    /// `toByteString($toEncoding)`: UTF-8 is kept; Windows-1252 through
    /// mbstring (a character it lacks becomes `?`); windows-1250 through
    /// the `iconv()` fallback, which fails on such a character (a PHP
    /// `TypeError` upstream). Other encodings are refused.
    fn to_byte_string(&self, to_encoding: Option<&str>) -> Result<ByteString> {
        let to = match to_encoding {
            None | Some("utf8" | "utf-8" | "UTF8" | "UTF-8") => {
                return Ok(ByteString::new(self.as_bytes()));
            }
            Some(other) => other,
        };
        let Some(page) = CodePage::from_name(to) else {
            return Err(Error::Unsupported(format!("encoding \"{to}\"")));
        };
        let text = String::from_utf8_lossy(self.as_bytes());
        match mb::encode(&text, page) {
            Some(bytes) => Ok(ByteString::new(&bytes)),
            None => Err(Error::Php(
                "Cannot assign false to property Symfony\\Component\\String\\ByteString::$string of type string".into(),
            )),
        }
    }

    /// `toCodePointString()`.
    fn to_code_point_string(&self) -> Result<CodePointString> {
        CodePointString::from_bytes(self.as_bytes())
    }

    /// `toString()`.
    fn to_php_string(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// `toUnicodeString()`.
    fn to_unicode_string(&self) -> Result<UnicodeString> {
        UnicodeString::from_bytes(self.as_bytes())
    }

    /// `trimPrefix($prefix)`.
    fn trim_prefix(&self, prefix: impl Into<Needle>) -> Result<Self> {
        Ok(match prefix.into() {
            Needle::Many(ps) => {
                for p in &ps {
                    let t = self.trim_prefix(p.as_slice())?;
                    if t.as_bytes() != self.as_bytes() {
                        return Ok(t);
                    }
                }
                self.cloned()
            }
            Needle::One(p) => {
                let s = self.as_bytes();
                if !p.is_empty()
                    && s.len() >= p.len()
                    && php::substr_compare_eq(s, &p, 0, Some(p.len()), self.ignoring_case())
                        == Some(true)
                {
                    self.with_string(s[p.len()..].to_vec())
                } else {
                    self.cloned()
                }
            }
        })
    }

    /// `trimSuffix($suffix)`.
    fn trim_suffix(&self, suffix: impl Into<Needle>) -> Result<Self> {
        Ok(match suffix.into() {
            Needle::Many(ss) => {
                for x in &ss {
                    let t = self.trim_suffix(x.as_slice())?;
                    if t.as_bytes() != self.as_bytes() {
                        return Ok(t);
                    }
                }
                self.cloned()
            }
            Needle::One(x) => {
                let s = self.as_bytes();
                if !x.is_empty()
                    && s.len() >= x.len()
                    && php::substr_compare_eq(s, &x, -(x.len() as i64), None, self.ignoring_case())
                        == Some(true)
                {
                    self.with_string(s[..s.len() - x.len()].to_vec())
                } else {
                    self.cloned()
                }
            }
        })
    }

    /// `truncate($length, $ellipsis, $cut)`.
    fn truncate(
        &self,
        length: i64,
        ellipsis: impl AsRef<[u8]>,
        cut: impl Into<TruncateMode>,
    ) -> Result<Self> {
        let ellipsis = ellipsis.as_ref();
        let cut = cut.into();
        if self.length() as i64 <= length {
            return Ok(self.cloned());
        }
        let mut ellipsis_length = if ellipsis.is_empty() {
            0
        } else {
            Self::create(ellipsis)?.length() as i64
        };
        if length < ellipsis_length {
            ellipsis_length = 0;
        }
        let desired_length = length;
        let mut length = length;
        let from = (if length == 0 { 1 } else { length }) - 1;
        if cut == TruncateMode::WordAfter {
            let Some(at) = self.index_of(WORD_BREAKS, from) else {
                return Ok(self.cloned());
            };
            length = at as i64 + ellipsis_length;
        } else if cut == TruncateMode::WordBefore && self.index_of(WORD_BREAKS, from).is_some() {
            length += ellipsis_length;
        }
        let mut str = self.slice(0, Some(length - ellipsis_length));
        if cut == TruncateMode::WordBefore {
            if ellipsis_length == 0
                && self.index_of(WORD_BREAKS, length).map(|i| i as i64) == Some(desired_length)
            {
                return Ok(str);
            }
            str = str.before_last(WORD_BREAKS, false, 0);
        }
        if ellipsis_length != 0 {
            str.trim_end(None)?.append(&[ellipsis])
        } else {
            Ok(str)
        }
    }

    /// `wordwrap($width, $break, $cut)`: PHP's `wordwrap()` run on a mask
    /// of the string's characters (`chunk()`), so no character is split.
    fn wordwrap(&self, width: i64, brk: impl AsRef<[u8]>, cut: bool) -> Result<Self> {
        let brk = brk.as_ref();
        let lines = if brk.is_empty() {
            vec![self.cloned()]
        } else {
            self.split(brk, None, None)?
        };
        if lines.len() == 1 && lines[0].as_bytes().is_empty() {
            return Ok(lines[0].clone());
        }
        let mut chars: Vec<Vec<u8>> = Vec::new();
        let mut mask = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                chars.push(brk.to_vec());
                mask.push(b'#');
            }
            for c in line.chunk(1)? {
                mask.push(if c.as_bytes() == b" " { b' ' } else { b'?' });
                chars.push(c.as_bytes().to_vec());
            }
        }
        // wordwrap() does not recognize a break at the very end of its
        // input and counts it as part of the last line.
        let kept = mask.len() - mask.iter().rev().take_while(|&&b| b == b'#').count();
        let width = usize::try_from(width).unwrap_or(0);
        let mut wrapped = php::wordwrap(&mask[..kept], width, b'#', cut);
        wrapped.extend_from_slice(&mask[kept..]);

        let mut string = Vec::new();
        let mut j = 0;
        let mut i: i64 = -1;
        for b in wrapped
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == b'#')
            .map(|(b, _)| b as i64)
        {
            i += 1;
            while i < b {
                if let Some(c) = chars.get(j) {
                    string.extend_from_slice(c);
                }
                j += 1;
                i += 1;
            }
            if chars.get(j).is_some_and(|c| c == brk || c == b" ") {
                j += 1;
            }
            string.extend_from_slice(brk);
        }
        for c in chars.iter().skip(j) {
            string.extend_from_slice(c);
        }
        Ok(self.with_string(string))
    }

    /// `__serialize()`.
    fn serialize(&self) -> PhpArray {
        let mut a = PhpArray::new();
        a.insert("string", Value::string(self.as_bytes().to_vec()));
        a
    }
}

/// `match()` shared by the classes: `$regexp` (with `i` under
/// `ignoreCase()`, `u` for the Unicode classes), `PREG_UNMATCHED_AS_NULL`.
pub(crate) fn match_regex<S: AbstractString>(
    s: &S,
    regexp: &[u8],
    flags: i64,
    offset: i64,
) -> Result<Value> {
    let mut pattern = regexp.to_vec();
    if s.ignoring_case() {
        pattern.push(b'i');
    }
    if S::UNICODE {
        pattern.push(b'u');
    }
    let function = if flags & (preg::PATTERN_ORDER | preg::SET_ORDER) != 0 {
        "preg_match_all"
    } else {
        "preg_match"
    };
    let regex = Regex::new(&pattern, function)?;
    preg::matches(
        &regex,
        s.as_bytes(),
        flags | preg::UNMATCHED_AS_NULL,
        offset,
    )
}

/// `replaceMatches()`'s `preg_replace()` / `preg_replace_callback()`.
pub(crate) fn replace_regex(
    subject: &[u8],
    from_regexp: &[u8],
    ignore_case: bool,
    unicode: bool,
    to: Replacement<'_>,
) -> Result<Vec<u8>> {
    let mut pattern = from_regexp.to_vec();
    if ignore_case {
        pattern.push(b'i');
    }
    if unicode {
        pattern.push(b'u');
    }
    match to {
        Replacement::Text(t) => Regex::new(&pattern, "preg_replace")?.replace(subject, t, None),
        Replacement::Callback(f) => {
            let regex = Regex::new(&pattern, "preg_replace_callback")?;
            regex.replace_callback(subject, None, |groups| {
                let out = f(groups);
                if unicode && !out.is_empty() && std::str::from_utf8(&out).is_err() {
                    return Err(Error::invalid(
                        "Replace callback must return a valid UTF-8 string.",
                    ));
                }
                Ok(out)
            })
        }
    }
}
