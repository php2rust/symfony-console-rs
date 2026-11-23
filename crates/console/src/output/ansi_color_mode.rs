//! Port of Symfony\Component\Console\Output\AnsiColorMode.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Output/AnsiColorMode.php

use crate::exception::{ConsoleError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColorMode {
    /// Classical 4-bit ANSI colors: `\e[3Xm` / `\e[4Xm`.
    Ansi4,
    /// 8-bit, 256 colors: `\e[38;5;Xm` / `\e[48;5;Xm`.
    Ansi8,
    /// 24-bit "true color": `\e[38;2;R;G;Bm` / `\e[48;2;R;G;Bm`.
    Ansi24,
}

impl AnsiColorMode {
    /// The enum case's name, as `$this->name` gives it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ansi4 => "Ansi4",
            Self::Ansi8 => "Ansi8",
            Self::Ansi24 => "Ansi24",
        }
    }

    /// `convertFromHexToAnsiColorCode($hexColor)`: the code after the
    /// leading `3` (foreground) or `4` (background).
    pub fn convert_from_hex_to_ansi_color_code(self, hex_color: &str) -> Result<String> {
        let mut hex: String = hex_color.replace('#', "");
        if hex.len() == 3 {
            let b = hex.as_bytes();
            hex = [b[0], b[0], b[1], b[1], b[2], b[2]]
                .iter()
                .map(|&c| c as char)
                .collect();
        }
        if hex.len() != 6 {
            return Err(ConsoleError::InvalidArgument(format!(
                "Invalid \"#{hex}\" color."
            )));
        }
        let color = hexdec(&hex);
        let (r, g, b) = ((color >> 16) & 255, (color >> 8) & 255, color & 255);
        Ok(match self {
            Self::Ansi4 => self.convert_from_rgb(r, g, b)?.to_string(),
            Self::Ansi8 => format!("8;5;{}", self.convert_from_rgb(r, g, b)?),
            Self::Ansi24 => format!("8;2;{r};{g};{b}"),
        })
    }

    fn convert_from_rgb(self, r: i64, g: i64, b: i64) -> Result<i64> {
        match self {
            Self::Ansi4 => Ok(degrade_to_ansi4(r, g, b)),
            Self::Ansi8 => Ok(degrade_to_ansi8(r, g, b)),
            Self::Ansi24 => Err(ConsoleError::InvalidArgument(format!(
                "RGB cannot be converted to {}.",
                self.name()
            ))),
        }
    }
}

/// `hexdec()`: hexadecimal digits are read, anything else is ignored.
fn hexdec(text: &str) -> i64 {
    text.chars()
        .filter_map(|c| c.to_digit(16))
        .fold(0i64, |n, d| {
            n.saturating_mul(16).saturating_add(i64::from(d))
        })
}

/// PHP's `round()` for these quotients: none of them is ever half-way
/// between two integers, so rounding to nearest is exact.
fn round(x: f64) -> i64 {
    x.round() as i64
}

fn degrade_to_ansi4(r: i64, g: i64, b: i64) -> i64 {
    (round(b as f64 / 255.0) << 2) | (round(g as f64 / 255.0) << 1) | round(r as f64 / 255.0)
}

fn degrade_to_ansi8(r: i64, g: i64, b: i64) -> i64 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return round((r - 8) as f64 / 247.0 * 24.0) + 232;
    }
    16 + 36 * round(r as f64 / 255.0 * 5.0)
        + 6 * round(g as f64 / 255.0 * 5.0)
        + round(b as f64 / 255.0 * 5.0)
}
