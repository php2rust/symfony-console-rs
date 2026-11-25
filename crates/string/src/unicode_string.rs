//! Port of Symfony\Component\String\UnicodeString.php: a string of Unicode
//! grapheme clusters encoded as UTF-8, kept in NFC.
//!
//! Graphemes follow ICU's rules (`grapheme_*()`) through
//! `unicode-segmentation`, and NFC comes from `unicode-normalization`; both
//! match the oracle's ICU 72.1 on every recorded case (`tests/unicode.rs`).
//! `chunk()` uses PCRE2's `\X` in PHP, which splits 218 of the 4,105
//! recorded sequences differently (a lone regional indicator before an
//! extending mark, Indic conjuncts, Prepend before an emoji); and code
//! points assigned after Unicode 15.0 are classified by the crates' newer
//! data (README, "Differences").
//!
//! `ignoreCase` is `null` after `normalize()` to NFD or NFKD; the next
//! `clone` brings the string back to NFC (`__clone()`).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/string UnicodeString.php

use php_value::Value;
use unicode_normalization::is_nfc;
use unicode_segmentation::UnicodeSegmentation;

use crate::abstract_string::{self as base, AbstractString, Needle, Replacement};
use crate::abstract_unicode_string::{self as au, utf8};
use crate::code_point_string::{base_trim_prefix, base_trim_suffix};
use crate::exception::{Error, Result};
use crate::intl::{self, Form};
use crate::{mb, php};

const I32_MIN: i64 = -2_147_483_648;
const I32_MAX: i64 = 2_147_483_647;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct UnicodeString {
    string: String,
    /// `$ignoreCase`: `None` is PHP's `null` (after NFD or NFKD).
    ignore_case: Option<bool>,
}

fn nfc(s: &str) -> String {
    if s.is_empty() || is_nfc(s) {
        s.to_owned()
    } else {
        intl::normalize(s, Form::Nfc)
    }
}

impl UnicodeString {
    /// `new UnicodeString($string)`: normalized to NFC.
    pub fn new(string: &str) -> Self {
        Self {
            string: nfc(string),
            ignore_case: Some(false),
        }
    }

    /// `new UnicodeString($bytes)`: invalid UTF-8 throws.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::new(utf8(bytes)?))
    }

    /// `UnicodeString::fromCodePoints(...$codes)`.
    pub fn from_code_points(codes: &[i64]) -> Result<Self> {
        au::from_code_points(codes)
    }

    pub fn as_str(&self) -> &str {
        &self.string
    }

    pub fn into_string(self) -> String {
        self.string
    }

    /// The form needles are brought to: NFD after `normalize()` to a
    /// decomposed form, NFC otherwise.
    fn form(&self) -> Form {
        if self.ignore_case.is_none() {
            Form::Nfd
        } else {
            Form::Nfc
        }
    }

    fn ci(&self) -> bool {
        self.ignore_case == Some(true)
    }

    /// A needle in [`Self::form`], `None` where PHP's normalizer fails.
    fn needle(&self, needle: &[u8]) -> Option<String> {
        let needle = std::str::from_utf8(needle).ok()?;
        Some(intl::normalized(needle, self.form()))
    }

    /// `$str->string = $string` followed by NFC unless already normalized;
    /// invalid UTF-8 throws.
    fn renormalized(&self, string: &[u8]) -> Result<Self> {
        Ok(self.with_string(nfc(utf8(string)?).into_bytes()))
    }

    /// `codePointsAt($offset)`.
    pub fn code_points_at(&self, offset: i64) -> Vec<u32> {
        au::code_points_at(self, offset)
    }

    /// `folded($compat)`.
    #[must_use]
    pub fn folded_compat(&self, compat: bool) -> Self {
        au::folded(self, compat)
    }

    /// `normalize($form)`, `$form` a `Normalizer` constant.
    pub fn normalize(&self, form: i64) -> Result<Self> {
        let mut str = self.cloned();
        match Form::from_php(form) {
            Some(f @ (Form::Nfc | Form::Nfkc)) => {
                str.string = intl::normalized(&str.string, f);
            }
            Some(f @ (Form::Nfd | Form::Nfkd)) => {
                if !intl::is_normalized(&str.string, f) {
                    str.string = intl::normalize(&str.string, f);
                    str.ignore_case = None;
                }
            }
            _ => return Err(Error::invalid("Unsupported normalization form.")),
        }
        Ok(str)
    }

    /// `ascii($rules)`: needs ICU's transliterators; refused.
    pub fn ascii(&self) -> Result<Self> {
        Err(Error::Unsupported(
            "ascii() needs ICU transliterators".into(),
        ))
    }

    /// `search($haystack, $needle, &$length)`: the byte offset and length of
    /// the first occurrence, as `grapheme_strpos()` finds it, corrected where
    /// its match differs from the needle.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn search(&self, haystack: &str, needle: &str) -> Option<(usize, usize)> {
        let ci = self.ci();
        let i = intl::strpos(haystack, needle, 0, ci)?;
        let offset = intl::substr(haystack, 0, Some(i as i64)).len();
        let length = needle.len();
        let bytes = haystack.as_bytes();
        let matched = php::substr(bytes, offset as i64, Some(length as i64));
        if !ci {
            if needle.as_bytes() == matched {
                return Some((offset, length));
            }
            let previous = if i > 0 {
                intl::substr(haystack, 0, Some(i as i64 - 1)).len()
            } else {
                0
            };
            let from = (offset as i64 + 1 - length as i64)
                .min(previous as i64 + 1)
                .max(0);
            return php::strpos(bytes, needle.as_bytes(), from, false).map(|at| (at, length));
        }
        let same = php::strcasecmp_eq(needle.as_bytes(), matched)
            || std::str::from_utf8(matched).is_ok_and(|m| {
                mb::fold_simple(needle) == mb::fold_simple(m)
                    || (intl::strpos(needle, m, 0, true) == Some(0)
                        && intl::strpos(m, needle, 0, true) == Some(0))
            });
        if same {
            return Some((offset, length));
        }
        let previous = if i > 0 {
            mb::strlen(intl::substr(haystack, 0, Some(i as i64 - 1)))
        } else {
            0
        };
        let i = mb::strlen(std::str::from_utf8(&bytes[..offset]).unwrap_or_default());
        let from = (i as i64 + 1 - mb::strlen(needle) as i64)
            .min(previous as i64 + 1)
            .max(0);
        let j = mb::strpos(haystack, needle, from, true)?;
        let length = mb::substr(haystack, j as i64, Some(mb::strlen(needle) as i64)).len();
        Some((mb::substr(haystack, 0, Some(j as i64)).len(), length))
    }

    /// `grapheme_extract($this->string, $size, GRAPHEME_EXTR_MAXBYTES,
    /// $start)`, `''` for `false`.
    fn extract(&self, size: usize, start: i64) -> &str {
        intl::extract_maxbytes(&self.string, size, start).unwrap_or_default()
    }
}

impl AbstractString for UnicodeString {
    const UNICODE: bool = true;
    const SPLIT_LIMIT: i64 = I32_MAX;

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
            ignore_case: Some(false),
        }
    }

    fn cloned(&self) -> Self {
        Self {
            string: if self.ignore_case.is_none() {
                nfc(&self.string)
            } else {
                self.string.clone()
            },
            ignore_case: Some(false),
        }
    }

    fn ignoring_case(&self) -> bool {
        self.ci()
    }

    fn ignore_case(&self) -> Self {
        let mut str = self.cloned();
        str.ignore_case = Some(true);
        str
    }

    fn append(&self, suffix: &[&[u8]]) -> Result<Self> {
        self.renormalized(&[self.string.as_bytes(), &suffix.concat()].concat())
    }

    fn prepend(&self, prefix: &[&[u8]]) -> Result<Self> {
        self.renormalized(&[prefix.concat().as_slice(), self.string.as_bytes()].concat())
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
        let graphemes = pcre_graphemes(&self.string);
        Ok(graphemes
            .chunks(length as usize)
            .map(|chunk| self.with_string(chunk.concat().into_bytes()))
            .collect())
    }

    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn ends_with_str(&self, suffix: &[u8]) -> bool {
        let Some(suffix) = self.needle(suffix) else {
            return false;
        };
        if suffix.is_empty() {
            return false;
        }
        let start = self.string.len() as i64 - suffix.len() as i64;
        let grapheme = self.extract(suffix.len(), start);
        if self.ci() {
            return mb::strpos(grapheme, &suffix, 0, true) == Some(0);
        }
        suffix == grapheme
    }

    fn equals_to_str(&self, string: &[u8]) -> bool {
        let Some(string) = self.needle(string) else {
            return false;
        };
        if !string.is_empty() && self.ci() {
            return string.len() == self.string.len()
                && mb::strpos(&self.string, &string, 0, true) == Some(0);
        }
        string == self.string
    }

    fn folded(&self) -> Self {
        au::folded(self, true)
    }

    fn index_of_str(&self, needle: &[u8], offset: i64) -> Option<usize> {
        let needle = self.needle(needle)?;
        if needle.is_empty() {
            return None;
        }
        intl::strpos(&self.string, &needle, offset, self.ci())
    }

    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn index_of_last_str(&self, needle: &[u8], offset: i64) -> Option<usize> {
        let needle = self.needle(needle)?;
        if needle.is_empty() {
            return None;
        }
        let mut string: &str = &self.string;
        let mut offset = offset;
        if offset < 0 {
            // workaround https://bugs.php.net/74264
            offset += intl::strlen(&needle) as i64;
            if offset < 0 {
                string = intl::substr(string, 0, Some(offset.max(I32_MIN)));
            }
            offset = 0;
        }
        intl::strrpos(string, &needle, offset, self.ci())
    }

    fn join(&self, strings: &[&[u8]], last_glue: Option<&[u8]>) -> Result<Self> {
        let str = au::join(self, strings, last_glue)?;
        Ok(self.with_string(nfc(&str.string).into_bytes()))
    }

    fn length(&self) -> usize {
        intl::strlen(&self.string)
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
        let Some(from) = std::str::from_utf8(from).ok().map(nfc) else {
            return Ok(self.cloned());
        };
        if from.is_empty() {
            return Ok(self.cloned());
        }
        let mut tail: &str = &self.string;
        let mut result = Vec::new();
        while !tail.is_empty() {
            let Some((i, length)) = self.search(tail, &from) else {
                break;
            };
            result.extend_from_slice(&tail.as_bytes()[..i]);
            result.extend_from_slice(to);
            tail = tail.get(i + length..).unwrap_or_default();
        }
        result.extend_from_slice(tail.as_bytes());
        self.renormalized(&result)
    }

    fn replace_matches(&self, from_regexp: &[u8], to: Replacement<'_>) -> Result<Self> {
        if let Replacement::Text(t) = &to {
            au::check_replacement(t)?;
        }
        let out = base::replace_regex(self.string.as_bytes(), from_regexp, self.ci(), true, to)?;
        self.renormalized(&out)
    }

    fn reverse(&self) -> Self {
        au::reverse(self)
    }

    fn slice(&self, start: i64, length: Option<i64>) -> Self {
        // grapheme_substr() accepts only 32-bit offsets and lengths
        let start = start.clamp(I32_MIN, I32_MAX);
        let length = length.unwrap_or(I32_MAX).clamp(I32_MIN, I32_MAX);
        self.with_string(
            intl::substr(&self.string, start, Some(length))
                .as_bytes()
                .to_vec(),
        )
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
        let start = if start != 0 {
            intl::substr(&self.string, 0, Some(start.clamp(I32_MIN, I32_MAX))).len() as i64
        } else {
            0
        };
        let length = match length {
            Some(l) if l != 0 => {
                let rest = self.string.get(start as usize..).unwrap_or_default();
                Some(intl::substr(rest, 0, Some(l.clamp(I32_MIN, I32_MAX))).len() as i64)
            }
            other => other,
        };
        let out = php::substr_replace(
            self.string.as_bytes(),
            replacement,
            start,
            Some(length.unwrap_or(I32_MAX)),
        );
        self.renormalized(&out)
    }

    fn split_str(&self, delimiter: &[u8], limit: i64) -> Result<Vec<Self>> {
        let Ok(delimiter) = std::str::from_utf8(delimiter).map(nfc) else {
            return Err(Error::invalid(
                "Split delimiter is not a valid UTF-8 string.",
            ));
        };
        let mut tail: &str = &self.string;
        let mut chunks = Vec::new();
        let mut limit = limit;
        while limit > 1 {
            let Some((i, length)) = self.search(tail, &delimiter) else {
                break;
            };
            chunks.push(self.with_string(tail.as_bytes()[..i].to_vec()));
            tail = tail.get(i + length..).unwrap_or_default();
            limit -= 1;
        }
        chunks.push(self.with_string(tail.as_bytes().to_vec()));
        Ok(chunks)
    }

    fn starts_with_str(&self, prefix: &[u8]) -> bool {
        let Some(prefix) = self.needle(prefix) else {
            return false;
        };
        if prefix.is_empty() {
            return false;
        }
        let grapheme = self.extract(prefix.len(), 0);
        if self.ci() {
            return mb::strpos(grapheme, &prefix, 0, true) == Some(0);
        }
        prefix == grapheme
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
        let prefix = match prefix.into() {
            Needle::One(p) => Needle::One(match std::str::from_utf8(&p) {
                Ok(s) => nfc(s).into_bytes(),
                Err(_) => p,
            }),
            many @ Needle::Many(_) => many,
        };
        if !self.ci() {
            return base_trim_prefix(self, prefix);
        }
        au::trim_affix_ignore_case(self, prefix, false)
    }

    fn trim_suffix(&self, suffix: impl Into<Needle>) -> Result<Self> {
        let suffix = match suffix.into() {
            Needle::One(x) => Needle::One(match std::str::from_utf8(&x) {
                Ok(s) => nfc(s).into_bytes(),
                Err(_) => x,
            }),
            many @ Needle::Many(_) => many,
        };
        if !self.ci() {
            return base_trim_suffix(self, suffix);
        }
        au::trim_affix_ignore_case(self, suffix, true)
    }

    fn upper(&self) -> Self {
        au::upper(self)
    }

    fn width(&self, ignore_ansi_decoration: bool) -> usize {
        au::width(&self.string, ignore_ansi_decoration)
    }
}

impl std::fmt::Display for UnicodeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string)
    }
}

/// Extended grapheme clusters as PCRE2 10.44's `\X` finds them, which
/// `chunk()` (and through it `wordwrap()`) uses: ICU's clusters, split where
/// the oracle shows PCRE2 breaks and ICU does not (`tests/unicode.rs`):
/// before a character joined only by the Indic conjunct rule (`GB9c`, which
/// PCRE2 10.44 lacks), after a regional indicator before anything but
/// another regional indicator, and between a Prepend character and an
/// `Extended_Pictographic` one.
pub fn pcre_graphemes(string: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for cluster in string.graphemes(true) {
        let mut start = 0;
        let mut previous: Option<char> = None;
        for (at, c) in cluster.char_indices() {
            if let Some(p) = previous
                && pcre_breaks_between(p, c)
            {
                out.push(&cluster[start..at]);
                start = at;
            }
            previous = Some(c);
        }
        out.push(&cluster[start..]);
    }
    out
}

fn is_regional_indicator(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

/// Whether `c` extends any base (GB9, GB9a): "a" followed by it is one cluster.
fn extends(c: char) -> bool {
    let mut text = String::from("a");
    text.push(c);
    text.graphemes(true).count() == 1
}

/// Whether `c` is Prepend (GB9b): it followed by "a" is one cluster.
fn is_prepend(c: char) -> bool {
    let mut text = String::from(c);
    text.push('a');
    text.graphemes(true).count() == 1
}

/// Inside one ICU cluster, whether PCRE2's `\X` breaks between `p` and `c`.
fn pcre_breaks_between(p: char, c: char) -> bool {
    if is_regional_indicator(p) {
        return !is_regional_indicator(c);
    }
    let pictographic = crate::abstract_unicode_string::in_table(crate::tables::EXTPICT, c as u32);
    if is_prepend(p) && pictographic {
        return true;
    }
    // After an extending character (a virama, ZWJ or other mark), a joined
    // character that does not extend, is not an emoji after ZWJ (GB11) and
    // not a regional indicator was joined by GB9c alone.
    extends(p) && !extends(c) && (p != '\u{200D}' || !pictographic) && !is_regional_indicator(c)
}
