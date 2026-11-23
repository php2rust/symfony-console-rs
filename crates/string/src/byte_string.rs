//! Port of Symfony\Component\String\ByteString.php: a binary-safe string of
//! bytes.
//!
//! `fromRandom()` reads the operating system's CSPRNG through `getrandom`,
//! as PHP's `Random\Randomizer` does (PLAN.md, decision 15);
//! `from_random_with()` takes the bytes from the caller instead.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/string ByteString.php

use php_value::Value;

use crate::abstract_string::{self as base, AbstractString, Replacement};
use crate::code_point_string::CodePointString;
use crate::exception::{Error, Result};
use crate::mb::{self, CodePage};
use crate::php::{self, PadType};
use crate::preg::Regex;
use crate::unicode_string::UnicodeString;

const ALPHABET_ALPHANUMERIC: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

const TRIM_CHARS: &[u8] = b" \t\n\r\0\x0B\x0C";

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ByteString {
    string: Vec<u8>,
    ignore_case: bool,
}

impl ByteString {
    /// `new ByteString($string)`.
    pub fn new(string: impl AsRef<[u8]>) -> Self {
        Self {
            string: string.as_ref().to_vec(),
            ignore_case: false,
        }
    }

    /// `ByteString::fromRandom($length, $alphabet)`: each byte of the
    /// result is a uniformly chosen byte of the alphabet, drawn from the
    /// operating system's CSPRNG. A failing source is `Error::Php` with
    /// `getrandom`'s text, where PHP throws `Random\RandomException`.
    pub fn from_random(length: i64, alphabet: Option<&[u8]>) -> Result<Self> {
        let mut failure = None;
        let result = Self::from_random_with(length, alphabet, &mut |buf| {
            if failure.is_none()
                && let Err(e) = getrandom::fill(buf)
            {
                failure = Some(e);
            }
        });
        match failure {
            Some(e) => Err(Error::Php(e.to_string())),
            None => result,
        }
    }

    /// `fromRandom()` with `random` filling a buffer with uniformly random
    /// bytes in place of the operating system's source.
    pub fn from_random_with(
        length: i64,
        alphabet: Option<&[u8]>,
        random: &mut dyn FnMut(&mut [u8]),
    ) -> Result<Self> {
        if length <= 0 {
            return Err(Error::invalid(format!(
                "A strictly positive length is expected, \"{length}\" given."
            )));
        }
        let alphabet = alphabet.unwrap_or(ALPHABET_ALPHANUMERIC);
        let size = alphabet.len() as u64;
        // (int) ceil(log($alphabetSize, 2.0)) in [1, 56]
        let bits = if size <= 1 {
            0
        } else {
            64 - (size - 1).leading_zeros()
        };
        if bits == 0 || bits > 56 {
            return Err(Error::invalid(
                "The length of the alphabet must in the [2^1, 2^56] range.",
            ));
        }
        let mask = (1u64 << bits) - 1;
        let mut out = Vec::with_capacity(length as usize);
        let mut buf = [0u8; 8];
        while out.len() < length as usize {
            random(&mut buf);
            let index = u64::from_le_bytes(buf) & mask;
            if index < size {
                out.push(alphabet[index as usize]);
            }
        }
        Ok(Self::new(out))
    }

    /// `isUtf8()`.
    pub fn is_utf8(&self) -> bool {
        std::str::from_utf8(&self.string).is_ok()
    }

    /// `toCodePointString($fromEncoding)`: valid UTF-8 is kept; otherwise
    /// the string is converted from `$fromEncoding` (Windows-1252 when
    /// `None`) or refused if it is not valid in it.
    pub fn to_code_point_string_from(
        &self,
        from_encoding: Option<&str>,
    ) -> Result<CodePointString> {
        if matches!(
            from_encoding,
            None | Some("utf8" | "utf-8" | "UTF8" | "UTF-8")
        ) && let Ok(s) = std::str::from_utf8(&self.string)
        {
            return CodePointString::from_bytes(s.as_bytes());
        }
        let name = from_encoding.unwrap_or("Windows-1252");
        let Some(page) = CodePage::from_name(name) else {
            return Err(Error::Unsupported(format!("encoding \"{name}\"")));
        };
        if page == CodePage::Windows1252 && !mb::detect(&self.string, page) {
            return Err(Error::invalid(format!("Invalid \"{name}\" string.")));
        }
        match mb::decode(&self.string, page) {
            Some(s) => CodePointString::from_bytes(s.as_bytes()),
            None => Err(Error::Php(
                "Cannot assign false to property Symfony\\Component\\String\\CodePointString::$string of type string".into(),
            )),
        }
    }

    /// `toUnicodeString($fromEncoding)`.
    pub fn to_unicode_string_from(&self, from_encoding: Option<&str>) -> Result<UnicodeString> {
        UnicodeString::from_bytes(self.to_code_point_string_from(from_encoding)?.as_bytes())
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.string
    }

    fn trim_with(&self, chars: Option<&[u8]>, left: bool, right: bool) -> Self {
        self.with_string(php::trim(
            &self.string,
            chars.unwrap_or(TRIM_CHARS),
            left,
            right,
        ))
    }

    fn pad(&self, length: i64, pad: &[u8], kind: PadType) -> Result<Self> {
        php::str_pad(&self.string, length, pad, kind)
            .map(|s| self.with_string(s))
            .ok_or_else(|| {
                Error::Php("str_pad(): Argument #3 ($pad_string) must be a non-empty string".into())
            })
    }
}

impl AbstractString for ByteString {
    const UNICODE: bool = false;
    const SPLIT_LIMIT: i64 = i64::MAX;

    fn create(string: &[u8]) -> Result<Self> {
        Ok(Self::new(string))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.string
    }

    fn with_string(&self, string: Vec<u8>) -> Self {
        Self {
            string,
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

    fn bytes_at(&self, offset: i64) -> Vec<u8> {
        // $this->string[$offset] ?? '': a negative offset counts from the end
        let len = self.string.len() as i64;
        let at = if offset < 0 { offset + len } else { offset };
        if at < 0 || at >= len {
            return Vec::new();
        }
        vec![self.string[at as usize]]
    }

    fn append(&self, suffix: &[&[u8]]) -> Result<Self> {
        Ok(self.with_string([self.string.as_slice(), &suffix.concat()].concat()))
    }

    fn prepend(&self, prefix: &[&[u8]]) -> Result<Self> {
        Ok(self.with_string([prefix.concat().as_slice(), &self.string].concat()))
    }

    fn camel(&self) -> Result<Self> {
        // preg_replace('/[^a-zA-Z0-9\x7f-\xff]++/', ' ', ...) then trim()
        let mut words = Vec::with_capacity(self.string.len());
        let mut in_run = false;
        for &b in &self.string {
            if b.is_ascii_alphanumeric() || b >= 0x7F {
                words.push(b);
                in_run = false;
            } else if !in_run {
                words.push(b' ');
                in_run = true;
            }
        }
        let words = php::trim(&words, TRIM_CHARS, true, true);
        let upper = php::ucwords(&words);
        let mut parts: Vec<Vec<u8>> = upper.split(|&b| b == b' ').map(<[u8]>::to_vec).collect();
        // a leading uppercase letter followed by another one is kept
        let leading_caps =
            words.len() >= 2 && words[0].is_ascii_uppercase() && words[1].is_ascii_uppercase();
        if !(leading_caps || (parts[0].len() != 1 && php::ctype_upper(&parts[0]))) {
            parts[0] = php::lcfirst(&parts[0]);
        }
        Ok(self.with_string(parts.concat()))
    }

    fn chunk(&self, length: i64) -> Result<Vec<Self>> {
        if length < 1 {
            return Err(Error::invalid(
                "The chunk length must be greater than zero.",
            ));
        }
        if self.string.is_empty() {
            return Ok(Vec::new());
        }
        Ok(php::str_split(&self.string, length as usize)
            .into_iter()
            .map(|c| self.with_string(c.to_vec()))
            .collect())
    }

    fn ends_with_str(&self, suffix: &[u8]) -> bool {
        !suffix.is_empty()
            && self.string.len() >= suffix.len()
            && php::substr_compare_eq(
                &self.string,
                suffix,
                -(suffix.len() as i64),
                None,
                self.ignore_case,
            ) == Some(true)
    }

    fn equals_to_str(&self, string: &[u8]) -> bool {
        if !string.is_empty() && self.ignore_case {
            return php::strcasecmp_eq(string, &self.string);
        }
        string == self.string.as_slice()
    }

    fn folded(&self) -> Self {
        self.with_string(php::strtolower(&self.string))
    }

    fn index_of_str(&self, needle: &[u8], offset: i64) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        php::strpos(&self.string, needle, offset, self.ignore_case)
    }

    fn index_of_last_str(&self, needle: &[u8], offset: i64) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        php::strrpos(&self.string, needle, offset, self.ignore_case)
    }

    fn join(&self, strings: &[&[u8]], last_glue: Option<&[u8]>) -> Result<Self> {
        Ok(self.with_string(join_bytes(&self.string, strings, last_glue)))
    }

    fn length(&self) -> usize {
        self.string.len()
    }

    fn lower(&self) -> Self {
        self.with_string(php::strtolower(&self.string))
    }

    fn matches(&self, regexp: &[u8], flags: i64, offset: i64) -> Result<Value> {
        base::match_regex(self, regexp, flags, offset)
    }

    fn pad_both(&self, length: i64, pad: &[u8]) -> Result<Self> {
        self.pad(length, pad, PadType::Both)
    }

    fn pad_end(&self, length: i64, pad: &[u8]) -> Result<Self> {
        self.pad(length, pad, PadType::Right)
    }

    fn pad_start(&self, length: i64, pad: &[u8]) -> Result<Self> {
        self.pad(length, pad, PadType::Left)
    }

    fn replace(&self, from: &[u8], to: &[u8]) -> Result<Self> {
        if from.is_empty() {
            return Ok(self.cloned());
        }
        Ok(self.with_string(php::str_replace(from, to, &self.string, self.ignore_case)))
    }

    fn replace_matches(&self, from_regexp: &[u8], to: Replacement<'_>) -> Result<Self> {
        base::replace_regex(&self.string, from_regexp, self.ignore_case, false, to)
            .map(|s| self.with_string(s))
    }

    fn reverse(&self) -> Self {
        self.with_string(php::strrev(&self.string))
    }

    fn slice(&self, start: i64, length: Option<i64>) -> Self {
        self.with_string(php::substr(&self.string, start, length).to_vec())
    }

    fn snake(&self) -> Result<Self> {
        let camel = self.camel()?;
        // preg_replace(['/([A-Z]+)([A-Z][a-z])/', '/([a-z\d])([A-Z])/'], '\1_\2', ...)
        let first = Regex::new(b"/([A-Z]+)([A-Z][a-z])/", "preg_replace")?.replace(
            camel.as_bytes(),
            b"\\1_\\2",
            None,
        )?;
        let second = Regex::new(b"/([a-z\\d])([A-Z])/", "preg_replace")?
            .replace(&first, b"\\1_\\2", None)?;
        Ok(self.with_string(php::strtolower(&second)))
    }

    fn splice(&self, replacement: &[u8], start: i64, length: Option<i64>) -> Result<Self> {
        Ok(self.with_string(php::substr_replace(
            &self.string,
            replacement,
            start,
            length,
        )))
    }

    fn split_str(&self, delimiter: &[u8], limit: i64) -> Result<Vec<Self>> {
        let chunks: Vec<Vec<u8>> = if self.ignore_case {
            let mut pattern = b"{".to_vec();
            pattern.extend(php::preg_quote(delimiter));
            pattern.extend_from_slice(b"}iD");
            Regex::new(&pattern, "preg_split")?
                .split(&self.string, limit, 0)?
                .into_iter()
                .map(|(s, _)| s)
                .collect()
        } else {
            php::explode(delimiter, &self.string, limit)
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect()
        };
        Ok(chunks.into_iter().map(|c| self.with_string(c)).collect())
    }

    fn starts_with_str(&self, prefix: &[u8]) -> bool {
        !prefix.is_empty() && php::strncmp_eq(&self.string, prefix, prefix.len(), self.ignore_case)
    }

    fn title(&self, all_words: bool) -> Result<Self> {
        Ok(self.with_string(if all_words {
            php::ucwords(&self.string)
        } else {
            php::ucfirst(&self.string)
        }))
    }

    fn trim(&self, chars: Option<&[u8]>) -> Result<Self> {
        Ok(self.trim_with(chars, true, true))
    }

    fn trim_end(&self, chars: Option<&[u8]>) -> Result<Self> {
        Ok(self.trim_with(chars, false, true))
    }

    fn trim_start(&self, chars: Option<&[u8]>) -> Result<Self> {
        Ok(self.trim_with(chars, true, false))
    }

    fn upper(&self) -> Self {
        self.with_string(php::strtoupper(&self.string))
    }

    fn width(&self, ignore_ansi_decoration: bool) -> usize {
        // non-UTF-8 bytes count as '?'
        let text = match std::str::from_utf8(&self.string) {
            Ok(s) => s.to_owned(),
            Err(_) => self
                .string
                .iter()
                .map(|&b| if b >= 0x80 { '?' } else { b as char })
                .collect(),
        };
        CodePointString::from_bytes(text.as_bytes())
            .map(|s| s.width(ignore_ansi_decoration))
            .unwrap_or(0)
    }
}

/// `implode($glue, $strings).$tail`, `$tail` being `$lastGlue` and the last
/// string when there are two or more.
pub(crate) fn join_bytes(glue: &[u8], strings: &[&[u8]], last_glue: Option<&[u8]>) -> Vec<u8> {
    match last_glue {
        Some(last) if strings.len() > 1 => {
            let (tail, head) = strings
                .split_last()
                .map_or((&[][..], &[][..]), |(t, h)| (*t, h));
            [head.join(glue).as_slice(), last, tail].concat()
        }
        _ => strings.join(glue),
    }
}

impl std::fmt::Display for ByteString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.string))
    }
}
