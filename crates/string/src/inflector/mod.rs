//! Port of Symfony\Component\String\Inflector\InflectorInterface.php
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Words are bytes, as PHP's strings are: the English inflector compares
//! ASCII letters, the French and Spanish ones match byte patterns (no
//! `/u`), so other bytes pass through unchanged.
//!
//! Upstream: symfony/string Inflector/InflectorInterface.php

mod english;
mod french;
mod spanish;

pub use english::EnglishInflector;
pub use french::FrenchInflector;
pub use spanish::SpanishInflector;

use crate::{exception::Result, preg::Regex};

/// `InflectorInterface`. The French and Spanish inflectors return an
/// error only where php-pcre fails a match PHP would answer (none of the
/// recorded words).
pub trait Inflector {
    /// The singular forms of a word; several when it cannot tell.
    fn singularize(&self, plural: &[u8]) -> Result<Vec<Vec<u8>>>;

    /// The plural forms of a word; several when it cannot tell.
    fn pluralize(&self, singular: &[u8]) -> Result<Vec<Vec<u8>>>;
}

/// A rule list: `[regexp, replacement]` pairs, compiled once.
type Rules = Result<Vec<(Regex, &'static [u8])>>;

fn compile(rules: &[(&str, &'static str)]) -> Rules {
    rules
        .iter()
        .map(|(pattern, replacement)| {
            Ok((
                Regex::new(pattern.as_bytes(), "preg_match")?,
                replacement.as_bytes(),
            ))
        })
        .collect()
}

/// The French and Spanish loop: the first rule that matches is applied
/// with `preg_replace()`; none matching gives `$default`.
fn apply(rules: &Rules, word: &[u8], default: Vec<u8>) -> Result<Vec<Vec<u8>>> {
    let rules = rules.as_ref().map_err(Clone::clone)?;
    for (regex, replacement) in rules {
        if regex.first(word, 0)?.is_some() {
            return Ok(vec![regex.replace(word, replacement, None)?]);
        }
    }
    Ok(vec![default])
}
