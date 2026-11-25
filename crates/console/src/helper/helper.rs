//! Port of Symfony\Component\Console\Helper\Helper.php's static functions
//! `width()`, `length()`, `substr()` and `removeDecoration()`. Not ported:
//! `formatTime()`, `formatMemory()` (no M1-B caller).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/Helper.php

use symfony_string::{AbstractString, UnicodeString};

use crate::{exception::Result, formatter::output_formatter::OutputFormatter};

pub struct Helper;

impl Helper {
    /// `width($string)`: terminal columns; each run of control characters
    /// counts as one.
    pub fn width(string: &str) -> usize {
        if string.is_empty() {
            return 0;
        }
        if string.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
            return string.len();
        }
        // preg_replace('/[\p{Cc}\x7F]++/u', '', $string, -1, $count)
        let mut clean = String::with_capacity(string.len());
        let mut runs = 0;
        let mut in_run = false;
        for c in string.chars() {
            if matches!(c, '\u{0}'..='\u{1F}' | '\u{7F}'..='\u{9F}') {
                if !in_run {
                    runs += 1;
                    in_run = true;
                }
            } else {
                in_run = false;
                clean.push(c);
            }
        }
        UnicodeString::new(&clean).width(false) + runs
    }

    /// `length($string)`: graphemes.
    pub fn length(string: &str) -> usize {
        UnicodeString::new(string).length()
    }

    /// `substr($string, $from, $length)`: by graphemes.
    pub fn substr(string: &str, from: i64, length: Option<i64>) -> String {
        UnicodeString::new(string).slice(from, length).into_string()
    }

    /// `removeDecoration($formatter, $string)`: style tags, SGR sequences
    /// and terminal hyperlinks removed.
    #[expect(clippy::similar_names, reason = "names as in the PHP source")]
    pub fn remove_decoration(formatter: &mut OutputFormatter, string: &str) -> Result<String> {
        if !string.contains('<') && !string.contains('\x1b') {
            return Ok(string.to_owned());
        }
        let decorated = formatter.is_decorated();
        formatter.set_decorated(false);
        let formatted = formatter.format(Some(string));
        formatter.set_decorated(decorated);
        let formatted = formatted?;
        Ok(remove_hyperlinks(&remove_sgr(&formatted)))
    }
}

/// `preg_replace("/\033\[[^m]*m/", '', $string)`.
fn remove_sgr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find("\x1b[") {
        match rest[at + 2..].find('m') {
            Some(m) => {
                out.push_str(&rest[..at]);
                rest = &rest[at + 2 + m + 1..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// `preg_replace('/\033]8;[^;]*;[^\033]*\033\\\\/', '', $string)`.
fn remove_hyperlinks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    'search: while let Some(at) = rest.find("\x1b]8;") {
        let body = &rest[at + 4..];
        if let Some(semi) = body.find(';')
            && let Some(esc) = body[semi + 1..].find('\x1b')
            && body[semi + 1 + esc..].starts_with("\x1b\\")
        {
            out.push_str(&rest[..at]);
            rest = &body[semi + 1 + esc + 2..];
            continue 'search;
        }
        // No match here: keep this ESC and look further on.
        out.push_str(&rest[..=at]);
        rest = &rest[at + 1..];
    }
    out.push_str(rest);
    out
}
