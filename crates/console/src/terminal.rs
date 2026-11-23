//! Port of Symfony\Component\Console\Terminal.php: the color mode and the
//! terminal's size.
//!
//! As symfony-demo-rs's console does, this behaves as Symfony does on
//! Linux on every system: without `COLUMNS` / `LINES`, the size comes from
//! `stty -a` (Windows' `ANSICON` and `mode CON` paths are not taken). The
//! static caches are per thread. Not ported: the image-protocol probes.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Terminal.php

use std::{
    cell::Cell,
    process::{Command, Stdio},
};

use crate::{env::getenv, output::ansi_color_mode::AnsiColorMode};

thread_local! {
    static COLOR_MODE: Cell<Option<AnsiColorMode>> = const { Cell::new(None) };
    /// `self::$width`, `self::$height`; the outer `Option` is "not yet read".
    static DIMENSIONS: Cell<Option<(Option<i64>, Option<i64>)>> = const { Cell::new(None) };
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Terminal;

impl Terminal {
    pub const DEFAULT_COLOR_MODE: AnsiColorMode = AnsiColorMode::Ansi4;

    pub fn get_color_mode() -> AnsiColorMode {
        if let Some(mode) = COLOR_MODE.get() {
            return mode;
        }
        let detect = |value: Option<String>| -> Option<AnsiColorMode> {
            let value = value?.to_lowercase();
            if value.contains("truecolor") {
                Some(AnsiColorMode::Ansi24)
            } else if value.contains("256color") {
                Some(AnsiColorMode::Ansi8)
            } else {
                None
            }
        };
        let mode = detect(getenv("COLORTERM"))
            .or_else(|| detect(getenv("TERM")))
            .unwrap_or(Self::DEFAULT_COLOR_MODE);
        Self::set_color_mode(Some(mode));
        mode
    }

    pub fn set_color_mode(mode: Option<AnsiColorMode>) {
        COLOR_MODE.set(mode);
    }

    pub fn get_width(&self) -> i64 {
        if let Some(width) = getenv("COLUMNS") {
            return int_cast(&width);
        }
        dimensions().0.filter(|&w| w != 0).unwrap_or(80)
    }

    pub fn get_height(&self) -> i64 {
        if let Some(height) = getenv("LINES") {
            return int_cast(&height);
        }
        dimensions().1.filter(|&h| h != 0).unwrap_or(50)
    }
}

/// `(int) trim($value)`: the leading integer, 0 if none.
fn int_cast(value: &str) -> i64 {
    php_value::numeric::string_to_int(value.trim().as_bytes())
}

fn dimensions() -> (Option<i64>, Option<i64>) {
    if let Some(dimensions) = DIMENSIONS.get() {
        return dimensions;
    }
    let dimensions = stty_columns()
        .map(|s| parse_stty(&s))
        .unwrap_or((None, None));
    DIMENSIONS.set(Some(dimensions));
    dimensions
}

/// `readFromProcess(['stty', '-a'])`: stdout, with stdin inherited.
fn stty_columns() -> Option<String> {
    let output = Command::new("stty")
        .arg("-a")
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `/rows.(\d+);.columns.(\d+);/is`, else `/;.(\d+).rows;.(\d+).columns/is`.
fn parse_stty(text: &str) -> (Option<i64>, Option<i64>) {
    let lower = text.to_ascii_lowercase();
    let b = lower.as_bytes();
    let digits = |at: usize| -> Option<(i64, usize)> {
        let n = b[at.min(b.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        (n > 0).then(|| (int_cast(&lower[at..at + n]), at + n))
    };
    // rows.(\d+);.columns.(\d+);
    for (at, _) in lower.match_indices("rows") {
        let Some((rows, k)) = digits(at + 5) else {
            continue;
        };
        if b.get(k) != Some(&b';') || !lower.get(k + 2..).is_some_and(|s| s.starts_with("columns"))
        {
            continue;
        }
        if let Some((columns, k)) = digits(k + 2 + 7 + 1)
            && b.get(k) == Some(&b';')
        {
            return (Some(columns), Some(rows));
        }
    }
    // ;.(\d+).rows;.(\d+).columns
    for (at, _) in lower.match_indices(';') {
        let Some((rows, k)) = digits(at + 2) else {
            continue;
        };
        if !lower.get(k + 1..).is_some_and(|s| s.starts_with("rows;")) {
            continue;
        }
        if let Some((columns, k)) = digits(k + 1 + 5 + 1)
            && lower.get(k + 1..).is_some_and(|s| s.starts_with("columns"))
        {
            return (Some(columns), Some(rows));
        }
    }
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::parse_stty;

    #[test]
    fn reads_both_stty_layouts() {
        assert_eq!(
            parse_stty("speed 38400 baud; rows 24; columns 80; line = 0;"),
            (Some(80), Some(24))
        );
        assert_eq!(
            parse_stty("speed 9600 baud; 40 rows; 132 columns;"),
            (Some(132), Some(40))
        );
        assert_eq!(
            parse_stty("stty: 'standard input': Inappropriate ioctl"),
            (None, None)
        );
    }
}
