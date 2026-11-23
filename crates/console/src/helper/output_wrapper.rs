//! Port of Symfony\Component\Console\Helper\OutputWrapper.php: wraps
//! text to a width without cutting style tags (or, by default, URLs).
//!
//! Upstream's pattern is written here for fancy-regex (PLAN.md decision 5:
//! it needs an atomic group and a lookbehind), with the same flags: `i`,
//! `m` and `u` (text is UTF-8 here, so the non-`u` branch for malformed
//! text never applies). `preg_replace()`'s scan is reproduced: after an
//! empty match the scan moves on by one character, and the pattern can
//! only match empty at the end of the text. Checked against recorded
//! `wrap()` calls (`tests/data/builtins.json`).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/OutputWrapper.php

use std::{cell::RefCell, collections::HashMap};

use fancy_regex::Regex;

use crate::exception::{ConsoleError, Result};

const TAG_OPEN_REGEX_SEGMENT: &str = r"[a-z](?:[^\\<>]*+|\\.)*";
const TAG_CLOSE_REGEX_SEGMENT: &str = r"[a-z][^<>]*+";
const URL_PATTERN: &str = r"https?://\S+";

/// `OutputWrapper`.
#[derive(Debug, Default, Clone, Copy)]
pub struct OutputWrapper {
    allow_cut_urls: bool,
}

thread_local! {
    static PATTERNS: RefCell<HashMap<(bool, i64), Regex>> = RefCell::new(HashMap::new());
}

impl OutputWrapper {
    /// `new OutputWrapper($allowCutUrls)`.
    pub fn new(allow_cut_urls: bool) -> Self {
        Self { allow_cut_urls }
    }

    fn pattern(&self, width: i64) -> Result<Regex> {
        let key = (self.allow_cut_urls, width);
        if let Some(regex) = PATTERNS.with(|p| p.borrow().get(&key).cloned()) {
            return Ok(regex);
        }
        let tag_pattern =
            format!("<(?:(?:{TAG_OPEN_REGEX_SEGMENT})|/(?:{TAG_CLOSE_REGEX_SEGMENT})?)>");
        let mut blocks = vec![tag_pattern.as_str()];
        if !self.allow_cut_urls {
            blocks.push(URL_PATTERN);
        }
        blocks.push(".");
        // "{1,$width}": with a negative width PCRE reads it as literal text
        let limit = if width < 0 {
            format!(r"\{{1,{width}\}}")
        } else {
            format!("{{1,{width}}}")
        };
        let row = format!("(?:{}){limit}", blocks.join("|"));
        let pattern = format!(
            r"(?im)(?:((?>({row})((?<=[^\S\r\n])[^\S\r\n]?|(?=\r?\n)|$|[^\S\r\n]))|({row}))(?:\r?\n)?|(?:\r?\n|$))"
        );
        let regex = Regex::new(&pattern)
            .map_err(|e| ConsoleError::Logic(format!("OutputWrapper pattern: {e}")))?;
        PATTERNS.with(|p| p.borrow_mut().insert(key, regex.clone()));
        Ok(regex)
    }

    /// `wrap($text, $width, $break)`.
    pub fn wrap(&self, text: &str, width: i64, break_: &str) -> Result<String> {
        if width == 0 {
            return Ok(text.to_owned());
        }
        let regex = self.pattern(width)?;

        // preg_replace($pattern, '\1'.$break, $text)
        let mut output = String::with_capacity(text.len() + text.len() / 8);
        let mut copied = 0;
        let mut at = 0;
        while at <= text.len() {
            let Some(captures) = regex
                .captures_from_pos(text, at)
                .map_err(|e| ConsoleError::Runtime(format!("OutputWrapper: {e}")))?
            else {
                break;
            };
            let Some(whole) = captures.get(0) else {
                break;
            };
            output.push_str(&text[copied..whole.start()]);
            if let Some(group) = captures.get(1) {
                output.push_str(group.as_str());
            }
            output.push_str(break_);
            copied = whole.end();
            if whole.end() > whole.start() {
                at = whole.end();
            } else {
                // an empty match: PCRE retries a non-empty match here,
                // which this pattern cannot make, then moves on
                match text[whole.end()..].chars().next() {
                    Some(c) => at = whole.end() + c.len_utf8(),
                    None => break,
                }
            }
        }
        output.push_str(&text[copied..]);

        // rtrim($output, $break): the break's characters, from the end
        let trimmed = output.trim_end_matches(|c| break_.contains(c));
        Ok(trimmed.replace(&format!(" {break_}"), break_))
    }
}
