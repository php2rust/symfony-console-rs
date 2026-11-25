//! Port of Symfony\Component\Console\Formatter\OutputFormatter.php (with
//! OutputFormatterInterface.php and WrappableOutputFormatterInterface.php).
//!
//! The tag pattern `<(([a-z](?:[^\\<>]*+ | \\.)*) | /([a-z][^<>]*+)?)>`
//! (case-insensitive, bytes) and the inline-style pattern
//! `([^=]+)=([^;]+)(;|$)` are matched by hand.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Formatter/OutputFormatter.php Formatter/OutputFormatterInterface.php Formatter/WrappableOutputFormatterInterface.php

use symfony_string::{AbstractString, UnicodeString};

use super::{
    output_formatter_style::OutputFormatterStyle,
    output_formatter_style_stack::OutputFormatterStyleStack,
};
use crate::{
    exception::{ConsoleError, Result},
    helper::helper::Helper,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OutputFormatter {
    decorated: bool,
    /// Names are lowercase, in the order first set.
    styles: Vec<(String, OutputFormatterStyle)>,
    style_stack: OutputFormatterStyleStack,
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self::new(false, Vec::new())
    }
}

impl OutputFormatter {
    /// `escape($text)`: `<` and `>` escaped, and a trailing backslash.
    pub fn escape(text: &str) -> String {
        Self::escape_trailing_backslash(&text.replace('<', "\\<").replace('>', "\\>"))
    }

    /// `escapeTrailingBackslash($text)`: trailing backslashes become NUL
    /// bytes, which `format()` turns back into backslashes.
    pub fn escape_trailing_backslash(text: &str) -> String {
        if !text.ends_with('\\') {
            return text.to_owned();
        }
        let len = text.len();
        let trimmed = text.trim_end_matches('\\').replace('\0', "");
        let pad = len - trimmed.len();
        format!("{trimmed}{}", "\0".repeat(pad))
    }

    /// `new OutputFormatter($decorated, $styles)`.
    pub fn new(decorated: bool, styles: Vec<(String, OutputFormatterStyle)>) -> Self {
        let mut formatter = Self {
            decorated,
            styles: Vec::new(),
            style_stack: OutputFormatterStyleStack::default(),
        };
        let builtin = [
            ("error", Some("white"), Some("red")),
            ("info", Some("green"), None),
            ("comment", Some("yellow"), None),
            ("question", Some("black"), Some("cyan")),
        ];
        for (name, fg, bg) in builtin {
            if let Ok(style) = OutputFormatterStyle::new(fg, bg, &[]) {
                formatter.set_style(name, style);
            }
        }
        for (name, style) in styles {
            formatter.set_style(&name, style);
        }
        formatter
    }

    pub fn set_decorated(&mut self, decorated: bool) {
        self.decorated = decorated;
    }

    pub fn is_decorated(&self) -> bool {
        self.decorated
    }

    pub fn set_style(&mut self, name: &str, style: OutputFormatterStyle) {
        let name = name.to_lowercase();
        match self.styles.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = style,
            None => self.styles.push((name, style)),
        }
    }

    pub fn has_style(&self, name: &str) -> bool {
        let name = name.to_lowercase();
        self.styles.iter().any(|(n, _)| *n == name)
    }

    pub fn get_style(&mut self, name: &str) -> Result<&mut OutputFormatterStyle> {
        let lower = name.to_lowercase();
        self.styles
            .iter_mut()
            .find(|(n, _)| *n == lower)
            .map(|(_, s)| s)
            .ok_or_else(|| ConsoleError::InvalidArgument(format!("Undefined style: \"{name}\".")))
    }

    pub fn get_style_stack(&mut self) -> &mut OutputFormatterStyleStack {
        &mut self.style_stack
    }

    /// `format($message)`.
    pub fn format(&mut self, message: Option<&str>) -> Result<String> {
        self.format_and_wrap(message, 0)
    }

    /// `formatAndWrap($message, $width)`.
    pub fn format_and_wrap(&mut self, message: Option<&str>, width: usize) -> Result<String> {
        let Some(message) = message else {
            return Ok(String::new());
        };
        let bytes = message.as_bytes();
        let mut offset = 0;
        let mut output = String::new();
        let mut current_line_length = 0;
        for tag in find_tags(bytes) {
            if tag.start != 0 && bytes[tag.start - 1] == b'\\' {
                continue;
            }
            let text = &message[offset..tag.start];
            let styled = self.apply_current_style(text, &output, width, &mut current_line_length);
            output.push_str(&styled);
            offset = tag.end;
            let whole = &message[tag.start..tag.end];
            let name = &message[tag.name.0..tag.name.1];
            if !tag.open && name.is_empty() {
                // </>
                self.style_stack.pop(None)?;
            } else {
                match self.create_style_from_string(name)? {
                    None => {
                        let styled = self.apply_current_style(
                            whole,
                            &output,
                            width,
                            &mut current_line_length,
                        );
                        output.push_str(&styled);
                    }
                    Some(style) if tag.open => self.style_stack.push(style),
                    Some(style) => {
                        self.style_stack.pop(Some(&style))?;
                    }
                }
            }
        }
        let styled =
            self.apply_current_style(&message[offset..], &output, width, &mut current_line_length);
        output.push_str(&styled);
        Ok(unescape(&output))
    }

    /// `createStyleFromString($string)`: a defined style (exact name), or
    /// an inline style; `None` when it is neither. An invalid color or
    /// option throws.
    fn create_style_from_string(&self, string: &str) -> Result<Option<OutputFormatterStyle>> {
        if let Some((_, style)) = self.styles.iter().find(|(n, _)| n == string) {
            return Ok(Some(style.clone()));
        }
        let matches = inline_style_matches(string);
        if matches.is_empty() {
            return Ok(None);
        }
        let mut style = OutputFormatterStyle::empty();
        for (key, value) in matches {
            match key.to_lowercase().as_str() {
                "fg" => style.set_foreground(Some(&value.to_lowercase()))?,
                "bg" => style.set_background(Some(&value.to_lowercase()))?,
                "href" => style.set_href(&value.replace("\\<", "<").replace("\\>", ">")),
                "options" => {
                    for option in value
                        .to_lowercase()
                        .split([',', ';'])
                        .filter(|o| !o.is_empty())
                    {
                        style.set_option(option)?;
                    }
                }
                _ => return Ok(None),
            }
        }
        Ok(Some(style))
    }

    /// `applyCurrentStyle($text, $current, $width, &$currentLineLength)`.
    fn apply_current_style(
        &self,
        text: &str,
        current: &str,
        width: usize,
        current_line_length: &mut usize,
    ) -> String {
        if text.is_empty() {
            return String::new();
        }
        let style = self.style_stack.get_current();
        if width == 0 {
            return if self.decorated {
                style.apply(text)
            } else {
                text.to_owned()
            };
        }
        let mut text = text.to_owned();
        if *current_line_length == 0 && !current.is_empty() {
            text = text
                .trim_start_matches([' ', '\t', '\n', '\r', '\0', '\x0B'])
                .to_owned();
        }
        let prefix;
        if *current_line_length != 0 {
            let (first, rest) = match text.split_once('\n') {
                Some((first, rest)) => (first.to_owned(), Some(rest.to_owned())),
                None => (text.clone(), None),
            };
            let room = width as i64 - *current_line_length as i64;
            prefix = format!("{}\n", Helper::substr(&first, 0, Some(room)));
            text = Helper::substr(&first, room, None);
            if let Some(rest) = rest {
                // $prefix may contain the full first line in which the \n
                // is already a part of $prefix.
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&rest);
            }
        } else {
            prefix = String::new();
        }
        let trailing_newline = text.ends_with('\n');
        let mut text = format!("{prefix}{}", add_line_breaks(&text, width));
        text = format!(
            "{}{}",
            text.trim_end_matches('\n'),
            if trailing_newline { "\n" } else { "" }
        );
        if *current_line_length == 0 && !current.is_empty() && !current.ends_with('\n') {
            text = format!("\n{text}");
        }
        let lines: Vec<&str> = text.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            let length = Helper::length(line);
            *current_line_length = if i == 0 {
                *current_line_length + length
            } else {
                length
            };
            if width <= *current_line_length {
                *current_line_length = 0;
            }
        }
        if self.decorated {
            return lines
                .iter()
                .map(|l| style.apply(l))
                .collect::<Vec<_>>()
                .join("\n");
        }
        lines.join("\n")
    }
}

/// `b($text)->toUnicodeString()->wordwrap($width, "\n", true)`.
fn add_line_breaks(text: &str, width: usize) -> String {
    UnicodeString::new(text)
        .wordwrap(i64::try_from(width).unwrap_or(i64::MAX), "\n", true)
        .map_or_else(|_| text.to_owned(), UnicodeString::into_string)
}

/// `strtr($output, ["\0" => '\\', '\\<' => '<', '\\>' => '>'])`.
fn unescape(output: &str) -> String {
    let mut out = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\0' => out.push('\\'),
            '\\' if matches!(chars.peek(), Some('<' | '>')) => {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            c => out.push(c),
        }
    }
    out
}

struct Tag {
    start: usize,
    end: usize,
    open: bool,
    /// The name's byte range: group 2 for an opening tag, group 3 (maybe
    /// empty) for a closing one.
    name: (usize, usize),
}

/// `preg_match_all("#<(($openTagRegex) | /($closeTagRegex)?)>#ix")`.
fn find_tags(b: &[u8]) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut at = 0;
    while at < b.len() {
        if b[at] != b'<' {
            at += 1;
            continue;
        }
        let tag = if b.get(at + 1) == Some(&b'/') {
            close_tag(b, at)
        } else {
            open_tag(b, at)
        };
        match tag {
            Some(tag) => {
                at = tag.end;
                tags.push(tag);
            }
            None => at += 1,
        }
    }
    tags
}

/// `[a-z](?:[^\\<>]*+ | \\.)*` then `>`.
fn open_tag(b: &[u8], at: usize) -> Option<Tag> {
    let first = at + 1;
    if !b.get(first)?.is_ascii_alphabetic() {
        return None;
    }
    let mut k = first + 1;
    loop {
        match b.get(k) {
            Some(b'>') => {
                return Some(Tag {
                    start: at,
                    end: k + 1,
                    open: true,
                    name: (first, k),
                });
            }
            Some(b'<') | None => return None,
            Some(b'\\') => match b.get(k + 1) {
                Some(b'\n') | None => return None,
                Some(_) => k += 2,
            },
            Some(_) => k += 1,
        }
    }
}

/// `/([a-z][^<>]*+)?` then `>`.
fn close_tag(b: &[u8], at: usize) -> Option<Tag> {
    let first = at + 2;
    if b.get(first) == Some(&b'>') {
        return Some(Tag {
            start: at,
            end: first + 1,
            open: false,
            name: (first, first),
        });
    }
    if !b.get(first)?.is_ascii_alphabetic() {
        return None;
    }
    let end = first
        + b[first..]
            .iter()
            .take_while(|c| !matches!(c, b'<' | b'>'))
            .count();
    (b.get(end) == Some(&b'>')).then_some(Tag {
        start: at,
        end: end + 1,
        open: false,
        name: (first, end),
    })
}

/// `preg_match_all('/([^=]+)=([^;]+)(;|$)/', $string, PREG_SET_ORDER)`:
/// key and value of each match.
fn inline_style_matches(s: &str) -> Vec<(String, String)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < b.len() {
        // [^=]+ cannot pass an '=', so the key runs to the next '='.
        let Some(eq) = b[at..].iter().position(|&c| c == b'=').map(|p| at + p) else {
            break;
        };
        if eq == at {
            at += 1;
            continue;
        }
        // [^;]+ runs to the next ';' or the end; then ';' or $ must follow.
        let value_end = eq + 1 + b[eq + 1..].iter().take_while(|&&c| c != b';').count();
        if value_end == eq + 1 {
            // An empty value: retry from the next position.
            at += 1;
            continue;
        }
        out.push((s[at..eq].to_owned(), s[eq + 1..value_end].to_owned()));
        at = if value_end < b.len() {
            value_end + 1
        } else {
            value_end
        };
    }
    out
}
