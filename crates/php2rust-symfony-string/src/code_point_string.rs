//! Port of Symfony\Component\String\CodePointString.php: a string of
//! Unicode code points encoded as UTF-8.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/string CodePointString.php

use php_value::Value;

use crate::abstract_string::{self as base, AbstractString, Needle, Replacement};
use crate::abstract_unicode_string::{self as au, text, utf8};
use crate::exception::{Error, Result};
use crate::preg::Regex;
use crate::{mb, php};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CodePointString {
    string: String,
    ignore_case: bool,
}

impl CodePointString {
    /// `new CodePointString($string)`.
    pub fn new(string: &str) -> Self {
        Self {
            string: string.to_owned(),
            ignore_case: false,
        }
    }

    /// `new CodePointString($bytes)`: invalid UTF-8 throws.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::new(utf8(bytes)?))
    }

    /// `CodePointString::fromCodePoints(...$codes)`.
    pub fn from_code_points(codes: &[i64]) -> Result<Self> {
        au::from_code_points(codes)
    }

    pub fn as_str(&self) -> &str {
        &self.string
    }

    pub fn into_string(self) -> String {
        self.string
    }

    /// `codePointsAt($offset)`: the code point at `$offset`.
    pub fn code_points_at(&self, offset: i64) -> Vec<u32> {
        let str = if offset != 0 {
            self.slice(offset, Some(1))
        } else {
            self.clone()
        };
        mb::ord(&str.string).into_iter().collect()
    }

    /// `folded($compat)`.
    #[must_use]
    pub fn folded_compat(&self, compat: bool) -> Self {
        au::folded(self, compat)
    }

    /// `normalize($form)`, `$form` a `Normalizer` constant.
    pub fn normalize(&self, form: i64) -> Result<Self> {
        au::normalize(self, form)
    }

    /// `ascii($rules)`: needs ICU's transliterators; refused.
    pub fn ascii(&self) -> Result<Self> {
        Err(Error::Unsupported(
            "ascii() needs ICU transliterators".into(),
        ))
    }
}

impl AbstractString for CodePointString {
    const UNICODE: bool = true;
    const SPLIT_LIMIT: i64 = i64::MAX;

    fn create(string: &[u8]) -> Result<Self> {
        Self::from_bytes(string)
    }

    fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    fn with_string(&self, string: Vec<u8>) -> Self {
        Self {
            string: String::from_utf8(string)
                .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
            ignore_case: false,
        }
    }

    fn ignoring_case(&self) -> bool {
        self.ignore_case
    }

    fn ignore_case(&self) -> Self {
        Self {
            string: self.string.clone(),
            ignore_case: true,
        }
    }

    fn append(&self, suffix: &[&[u8]]) -> Result<Self> {
        let string = [self.string.as_bytes(), &suffix.concat()].concat();
        Ok(Self::new(utf8(&string)?))
    }

    fn prepend(&self, prefix: &[&[u8]]) -> Result<Self> {
        let string = [prefix.concat().as_slice(), self.string.as_bytes()].concat();
        Ok(Self::new(utf8(&string)?))
    }

    fn camel(&self) -> Result<Self> {
        au::camel(self)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn chunk(&self, length: i64) -> Result<Vec<Self>> {
        if length < 1 {
            return Err(Error::invalid(
                "The chunk length must be greater than zero.",
            ));
        }
        if self.string.is_empty() {
            return Ok(Vec::new());
        }
        if self.string.len() as i64 <= length {
            return Ok(vec![self.cloned()]);
        }
        let chars: Vec<char> = self.string.chars().collect();
        Ok(chars
            .chunks(length as usize)
            .map(|c| Self::new(&c.iter().collect::<String>()))
            .collect())
    }

    fn ends_with_str(&self, suffix: &[u8]) -> bool {
        let Ok(suffix) = std::str::from_utf8(suffix) else {
            return false;
        };
        if suffix.is_empty() {
            return false;
        }
        if self.ignore_case {
            // preg_match('{'.preg_quote($suffix).'$}iuD')
            let pattern = [
                b"{".as_slice(),
                &php::preg_quote(suffix.as_bytes()),
                b"$}iuD",
            ]
            .concat();
            return Regex::new(&pattern, "preg_match")
                .and_then(|r| r.first(self.string.as_bytes(), 0))
                .is_ok_and(|m| m.is_some());
        }
        self.string.ends_with(suffix)
    }

    fn equals_to_str(&self, string: &[u8]) -> bool {
        if !string.is_empty() && self.ignore_case {
            return string.len() == self.string.len()
                && std::str::from_utf8(string)
                    .is_ok_and(|s| mb::strpos(&self.string, s, 0, true) == Some(0));
        }
        string == self.string.as_bytes()
    }

    fn folded(&self) -> Self {
        au::folded(self, true)
    }

    fn index_of_str(&self, needle: &[u8], offset: i64) -> Option<usize> {
        let needle = std::str::from_utf8(needle).ok()?;
        if needle.is_empty() {
            return None;
        }
        mb::strpos(&self.string, needle, offset, self.ignore_case)
    }

    fn index_of_last_str(&self, needle: &[u8], offset: i64) -> Option<usize> {
        let needle = std::str::from_utf8(needle).ok()?;
        if needle.is_empty() {
            return None;
        }
        mb::strrpos(&self.string, needle, offset, self.ignore_case)
    }

    fn join(&self, strings: &[&[u8]], last_glue: Option<&[u8]>) -> Result<Self> {
        au::join(self, strings, last_glue)
    }

    fn length(&self) -> usize {
        mb::strlen(&self.string)
    }

    fn lower(&self) -> Self {
        au::lower(self)
    }

    fn matches(&self, regexp: &[u8], flags: i64, offset: i64) -> Result<Value> {
        base::match_regex(self, regexp, flags, offset)
    }

    fn pad_both(&self, length: i64, pad: &[u8]) -> Result<Self> {
        au::pad(self, length, pad, php::PadType::Both)
    }

    fn pad_end(&self, length: i64, pad: &[u8]) -> Result<Self> {
        au::pad(self, length, pad, php::PadType::Right)
    }

    fn pad_start(&self, length: i64, pad: &[u8]) -> Result<Self> {
        au::pad(self, length, pad, php::PadType::Left)
    }

    fn replace(&self, from: &[u8], to: &[u8]) -> Result<Self> {
        if from.is_empty() || std::str::from_utf8(from).is_err() {
            return Ok(self.cloned());
        }
        au::check_replacement(to)?;
        if self.ignore_case {
            let pattern = [b"{".as_slice(), &php::preg_quote(from), b"}iuD"].concat();
            let pieces =
                Regex::new(&pattern, "preg_split")?.split(self.string.as_bytes(), -1, 0)?;
            let joined = pieces
                .into_iter()
                .map(|(p, _)| p)
                .collect::<Vec<_>>()
                .join(to);
            return Ok(self.with_string(joined));
        }
        Ok(self.with_string(php::str_replace(from, to, self.string.as_bytes(), false)))
    }

    fn replace_matches(&self, from_regexp: &[u8], to: Replacement<'_>) -> Result<Self> {
        if let Replacement::Text(t) = &to {
            au::check_replacement(t)?;
        }
        base::replace_regex(
            self.string.as_bytes(),
            from_regexp,
            self.ignore_case,
            true,
            to,
        )
        .map(|s| self.with_string(s))
    }

    fn reverse(&self) -> Self {
        au::reverse(self)
    }

    fn slice(&self, start: i64, length: Option<i64>) -> Self {
        self.with_string(mb::substr(&self.string, start, length).as_bytes().to_vec())
    }

    fn snake(&self) -> Result<Self> {
        au::snake(self)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn splice(&self, replacement: &[u8], start: i64, length: Option<i64>) -> Result<Self> {
        utf8(replacement)?;
        let start = if start != 0 {
            mb::substr(&self.string, 0, Some(start)).len() as i64
        } else {
            0
        };
        let length = match length {
            Some(l) if l != 0 => {
                let rest = self.string.get(start as usize..).unwrap_or_default();
                Some(mb::substr(rest, 0, Some(l)).len() as i64)
            }
            other => other,
        };
        let out = php::substr_replace(self.string.as_bytes(), replacement, start, length);
        Ok(Self::new(utf8(&out)?))
    }

    fn split_str(&self, delimiter: &[u8], limit: i64) -> Result<Vec<Self>> {
        if std::str::from_utf8(delimiter).is_err() {
            return Err(Error::invalid(
                "Split delimiter is not a valid UTF-8 string.",
            ));
        }
        let chunks: Vec<Vec<u8>> = if self.ignore_case {
            let pattern = [b"{".as_slice(), &php::preg_quote(delimiter), b"}iuD"].concat();
            Regex::new(&pattern, "preg_split")?
                .split(self.string.as_bytes(), limit, 0)?
                .into_iter()
                .map(|(s, _)| s)
                .collect()
        } else {
            php::explode(delimiter, self.string.as_bytes(), limit)
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect()
        };
        Ok(chunks.into_iter().map(|c| self.with_string(c)).collect())
    }

    fn starts_with_str(&self, prefix: &[u8]) -> bool {
        let Ok(prefix) = std::str::from_utf8(prefix) else {
            return false;
        };
        if prefix.is_empty() {
            return false;
        }
        if self.ignore_case {
            return mb::strpos(&self.string, prefix, 0, true) == Some(0);
        }
        self.string.starts_with(prefix)
    }

    fn title(&self, all_words: bool) -> Result<Self> {
        au::title(self, all_words)
    }

    fn trim(&self, chars: Option<&[u8]>) -> Result<Self> {
        au::trim(self, chars, true, true)
    }

    fn trim_end(&self, chars: Option<&[u8]>) -> Result<Self> {
        au::trim(self, chars, false, true)
    }

    fn trim_start(&self, chars: Option<&[u8]>) -> Result<Self> {
        au::trim(self, chars, true, false)
    }

    fn trim_prefix(&self, prefix: impl Into<Needle>) -> Result<Self> {
        let prefix = prefix.into();
        if !self.ignore_case {
            return base_trim_prefix(self, prefix);
        }
        au::trim_affix_ignore_case(self, prefix, false)
    }

    fn trim_suffix(&self, suffix: impl Into<Needle>) -> Result<Self> {
        let suffix = suffix.into();
        if !self.ignore_case {
            return base_trim_suffix(self, suffix);
        }
        au::trim_affix_ignore_case(self, suffix, true)
    }

    fn upper(&self) -> Self {
        au::upper(self)
    }

    fn width(&self, ignore_ansi_decoration: bool) -> usize {
        au::width(text(self), ignore_ansi_decoration)
    }
}

/// `AbstractString::trimPrefix()`, which the Unicode classes call as
/// `parent::trimPrefix()`.
pub(crate) fn base_trim_prefix<S: AbstractString>(s: &S, prefix: Needle) -> Result<S> {
    Ok(match prefix {
        Needle::Many(ps) => {
            for p in &ps {
                let t = s.trim_prefix(p.as_slice())?;
                if t.as_bytes() != s.as_bytes() {
                    return Ok(t);
                }
            }
            s.cloned()
        }
        Needle::One(p) => {
            let b = s.as_bytes();
            if !p.is_empty()
                && b.len() >= p.len()
                && php::substr_compare_eq(b, &p, 0, Some(p.len()), s.ignoring_case()) == Some(true)
            {
                s.with_string(b[p.len()..].to_vec())
            } else {
                s.cloned()
            }
        }
    })
}

/// `AbstractString::trimSuffix()` as `parent::trimSuffix()`.
#[expect(
    clippy::cast_possible_wrap,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
pub(crate) fn base_trim_suffix<S: AbstractString>(s: &S, suffix: Needle) -> Result<S> {
    Ok(match suffix {
        Needle::Many(xs) => {
            for x in &xs {
                let t = s.trim_suffix(x.as_slice())?;
                if t.as_bytes() != s.as_bytes() {
                    return Ok(t);
                }
            }
            s.cloned()
        }
        Needle::One(x) => {
            let b = s.as_bytes();
            if !x.is_empty()
                && b.len() >= x.len()
                && php::substr_compare_eq(b, &x, -(x.len() as i64), None, s.ignoring_case())
                    == Some(true)
            {
                s.with_string(b[..b.len() - x.len()].to_vec())
            } else {
                s.cloned()
            }
        }
    })
}

impl std::fmt::Display for CodePointString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string)
    }
}
