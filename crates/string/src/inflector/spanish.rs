//! Port of Symfony\Component\String\Inflector\SpanishInflector.php
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! The patterns are upstream's, run by php-pcre without `/u`: `/i` folds
//! ASCII letters only and `\w` is an ASCII word byte.
//!
//! Upstream: symfony/string Inflector/SpanishInflector.php

use std::sync::LazyLock;

use super::{Inflector, Rules, apply, compile};
use crate::{exception::Result, preg::Regex};

/// The rules for pluralize, in order.
///
/// See <https://www.spanishdict.com/guide/spanish-plural-noun-forms> and
/// <https://www.rae.es/gram%C3%A1tica/morfolog%C3%ADa/la-formaci%C3%B3n-del-plural-plurales-en-s-y-plurales-en-es-reglas-generales>
const PLURALIZE_REGEXP: &[(&str, &str)] = &[
    // Specials sí, no
    ("/(sí|no)$/i", "\\1es"),
    // Words ending with vowel must use -s (RAE 3.2a, 3.2c)
    ("/(a|e|i|o|u|á|é|í|ó|ú)$/i", "\\1s"),
    // Word ending in s or x and the previous letter is accented (RAE 3.2n)
    ("/ás$/i", "ases"),
    ("/és$/i", "eses"),
    ("/ís$/i", "ises"),
    ("/ós$/i", "oses"),
    ("/ús$/i", "uses"),
    // Words ending in -ión must changed to -iones
    ("/ión$/i", "\\1iones"),
    // Words ending in some consonants must use -es (RAE 3.2k)
    ("/(l|r|n|d|j|s|x|ch|y)$/i", "\\1es"),
    // Word ending in z, must changed to ces
    ("/(z)$/i", "ces"),
];

/// The rules for singularize, in order.
const SINGULARIZE_REGEXP: &[(&str, &str)] = &[
    // Specials sí, no
    ("/(sí|no)es$/i", "\\1"),
    // Words ending in -ión must changed to -iones
    ("/iones$/i", "\\1ión"),
    // Word ending in z, must changed to ces
    ("/ces$/i", "z"),
    // Word ending in s or x and the previous letter is accented (RAE 3.2n)
    ("/(\\w)ases$/i", "\\1ás"),
    ("/eses$/i", "és"),
    ("/ises$/i", "ís"),
    ("/(\\w{2,})oses$/i", "\\1ós"),
    ("/(\\w)uses$/i", "\\1ús"),
    // Words ending in some consonants and -es, must be the consonants
    ("/(l|r|n|d|j|s|x|ch|y)e?s$/i", "\\1"),
    // Words ended with vowel and s, must be vowel
    ("/(a|e|i|o|u|á|é|ó|í|ú)s$/i", "\\1"),
];

const UNINFLECTED_RULES: &[&str] = &[
    // Words ending with pies (RAE 3.2n)
    "/.*(piés)$/i",
];

const UNINFLECTED: &str = "/^(lunes|martes|miércoles|jueves|viernes|análisis|torax|yo|pies)$/i";

static PLURALIZE: LazyLock<Rules> = LazyLock::new(|| compile(PLURALIZE_REGEXP));
static SINGULARIZE: LazyLock<Rules> = LazyLock::new(|| compile(SINGULARIZE_REGEXP));
static UNINFLECTED_RES: LazyLock<Result<Vec<Regex>>> = LazyLock::new(|| {
    UNINFLECTED_RULES
        .iter()
        .chain([&UNINFLECTED])
        .map(|pattern| Regex::new(pattern.as_bytes(), "preg_match"))
        .collect()
});

/// `Symfony\Component\String\Inflector\SpanishInflector`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpanishInflector;

impl SpanishInflector {
    pub fn new() -> Self {
        Self
    }

    /// The rules, then the word list.
    fn is_inflected_word(word: &[u8]) -> Result<bool> {
        for regex in UNINFLECTED_RES.as_ref().map_err(Clone::clone)? {
            if regex.first(word, 0)?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Inflector for SpanishInflector {
    fn singularize(&self, plural: &[u8]) -> Result<Vec<Vec<u8>>> {
        if Self::is_inflected_word(plural)? {
            return Ok(vec![plural.to_vec()]);
        }
        apply(&SINGULARIZE, plural, plural.to_vec())
    }

    fn pluralize(&self, singular: &[u8]) -> Result<Vec<Vec<u8>>> {
        if Self::is_inflected_word(singular)? {
            return Ok(vec![singular.to_vec()]);
        }
        apply(&PLURALIZE, singular, [singular, b"s"].concat())
    }
}
