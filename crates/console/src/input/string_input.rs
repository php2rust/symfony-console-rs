//! Port of Symfony\Component\Console\Input\StringInput.php: `new
//! StringInput($input)` is an argv input whose tokens are cut from a
//! string.
//!
//! The tokenizer's four patterns are matched by hand, on bytes as PCRE
//! does without `/u`: `\s` is space, `\t`, `\n`, `\v`, `\f` and `\r`; `.`
//! is any byte but `\n`. `stripcslashes()` is written from PHP's manual
//! and checked against recorded calls. A token that is not UTF-8 once
//! unescaped (`\xff`) is refused, as `argv_from_server()` refuses one
//! (PLAN.md decision 11).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Input/StringInput.php

use super::input::Input;
use crate::exception::{ConsoleError, Result};

impl Input {
    /// `new StringInput($input)`.
    pub fn string(input: &str) -> Result<Self> {
        let tokens = tokenize(input.as_bytes())?
            .into_iter()
            .map(|t| {
                String::from_utf8(t).map_err(|e| {
                    ConsoleError::Runtime(format!(
                        "Argument values expected to be UTF-8, got {:?}.",
                        String::from_utf8_lossy(e.as_bytes())
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::from_tokens(tokens))
    }
}

/// `/\s/` without `/u`.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// `tokenize($input)`.
pub fn tokenize(input: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut tokens = Vec::new();
    let mut token: Option<Vec<u8>> = None;
    let mut cursor = 0;
    while cursor < input.len() {
        if input[cursor] == b'\\' {
            cursor += 1;
            let t = token.get_or_insert_with(Vec::new);
            if let Some(&b) = input.get(cursor) {
                t.push(b);
            }
            cursor += 1;
            continue;
        }

        let consumed = if is_space(input[cursor]) {
            // /\s+/A
            if let Some(t) = token.take() {
                tokens.push(t);
            }
            input[cursor..].iter().take_while(|&&b| is_space(b)).count()
        } else if let Some((run, equals, quoted_end)) = unquoted_then_quoted(input, cursor) {
            // /([^="'\s]+?)(=?)((?:QUOTED)+)/A
            let t = token.get_or_insert_with(Vec::new);
            t.extend_from_slice(&input[cursor..run]);
            if equals {
                t.push(b'=');
            }
            let start = run + usize::from(equals);
            let inner = &input[start + 1..quoted_end - 1];
            let joined = str_replace_all(inner, &[b"\"'", b"'\"", b"''", b"\"\""]);
            t.extend_from_slice(&stripcslashes(&joined));
            quoted_end - cursor
        } else if let Some(end) = quoted(input, cursor) {
            // /QUOTED/A
            token
                .get_or_insert_with(Vec::new)
                .extend_from_slice(&stripcslashes(&input[cursor + 1..end - 1]));
            end - cursor
        } else if input[cursor] != b'\\' && !is_space(input[cursor]) {
            // /([^\s\\]+?)/A: a lazy match is one byte
            token.get_or_insert_with(Vec::new).push(input[cursor]);
            1
        } else {
            // should never happen
            let near = &input[cursor..input.len().min(cursor + 10)];
            return Err(ConsoleError::InvalidArgument(format!(
                "Unable to parse input near \"... {} ...\".",
                String::from_utf8_lossy(near)
            )));
        };
        cursor += consumed;
    }
    if let Some(t) = token {
        tokens.push(t);
    }
    Ok(tokens)
}

/// `([^="'\s]+?)(=?)((?:QUOTED)+)` at `at`: the end of the run, whether
/// an `=` follows it, and the end of the quoted strings. The run cannot
/// contain a quote, so the lazy run always extends to the first `=`,
/// quote or space.
fn unquoted_then_quoted(input: &[u8], at: usize) -> Option<(usize, bool, usize)> {
    let run = at
        + input[at..]
            .iter()
            .take_while(|&&b| !matches!(b, b'=' | b'"' | b'\'') && !is_space(b))
            .count();
    if run == at {
        return None;
    }
    let try_from = |start: usize| {
        let mut end = quoted(input, start)?;
        while let Some(next) = quoted(input, end) {
            end = next;
        }
        Some(end)
    };
    if input.get(run) == Some(&b'=')
        && let Some(end) = try_from(run + 1)
    {
        return Some((run, true, end));
    }
    try_from(run).map(|end| (run, false, end))
}

/// `"([^"\\]*(?:\\.[^"\\]*)*)"|'([^'\\]*(?:\\.[^'\\]*)*)'` at `at`: the
/// end of the quoted string. `\\.` does not take a newline.
fn quoted(input: &[u8], at: usize) -> Option<usize> {
    let quote = *input.get(at)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut i = at + 1;
    loop {
        match *input.get(i)? {
            b if b == quote => return Some(i + 1),
            b'\\' => {
                if *input.get(i + 1)? == b'\n' {
                    return None;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
}

/// `str_replace($searches, '', $subject)`: each search in turn, over the
/// whole result of the one before.
fn str_replace_all(subject: &[u8], searches: &[&[u8]]) -> Vec<u8> {
    let mut out = subject.to_vec();
    for search in searches {
        let mut next = Vec::with_capacity(out.len());
        let mut i = 0;
        while i < out.len() {
            if out[i..].starts_with(search) {
                i += search.len();
            } else {
                next.push(out[i]);
                i += 1;
            }
        }
        out = next;
    }
    out
}

/// `stripcslashes($string)`: C escapes `\a \b \f \n \r \t \v`, octal
/// `\0`-`\777` (up to three digits, the byte taken modulo 256), `\x` with
/// one or two hex digits; any other escaped byte stands for itself, and a
/// final lone backslash is kept (observed; the manual does not say).
pub fn stripcslashes(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] != b'\\' {
            out.push(s[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&c) = s.get(i) else {
            // a final lone backslash is kept
            out.push(b'\\');
            break;
        };
        match c {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'a' => out.push(0x07),
            b'v' => out.push(0x0B),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0C),
            b'x' if s.get(i + 1).is_some_and(u8::is_ascii_hexdigit) => {
                let digits = s[i + 1..]
                    .iter()
                    .take(2)
                    .take_while(|b| b.is_ascii_hexdigit())
                    .count();
                let value = s[i + 1..i + 1 + digits]
                    .iter()
                    .fold(0u32, |v, &d| v * 16 + hex_value(d));
                out.push(value as u8);
                i += digits;
            }
            b'0'..=b'7' => {
                let digits = s[i..]
                    .iter()
                    .take(3)
                    .take_while(|b| (b'0'..=b'7').contains(b))
                    .count();
                let value = s[i..i + digits]
                    .iter()
                    .fold(0u32, |v, &d| v * 8 + u32::from(d - b'0'));
                out.push((value & 0xFF) as u8);
                i += digits - 1;
            }
            other => out.push(other),
        }
        i += 1;
    }
    out
}

fn hex_value(d: u8) -> u32 {
    match d {
        b'0'..=b'9' => u32::from(d - b'0'),
        b'a'..=b'f' => u32::from(d - b'a' + 10),
        _ => u32::from(d - b'A' + 10),
    }
}
