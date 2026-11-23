//! Port of Symfony\Component\Console\Color.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Color.php

use crate::{
    exception::{ConsoleError, Result},
    terminal::Terminal,
};

const COLORS: [(&str, u8); 9] = [
    ("black", 0),
    ("red", 1),
    ("green", 2),
    ("yellow", 3),
    ("blue", 4),
    ("magenta", 5),
    ("cyan", 6),
    ("white", 7),
    ("default", 9),
];

const BRIGHT_COLORS: [(&str, u8); 8] = [
    ("gray", 0),
    ("bright-red", 1),
    ("bright-green", 2),
    ("bright-yellow", 3),
    ("bright-blue", 4),
    ("bright-magenta", 5),
    ("bright-cyan", 6),
    ("bright-white", 7),
];

/// `AVAILABLE_OPTIONS`: name, set code, unset code.
const AVAILABLE_OPTIONS: [(&str, u8, u8); 5] = [
    ("bold", 1, 22),
    ("underscore", 4, 24),
    ("blink", 5, 25),
    ("reverse", 7, 27),
    ("conceal", 8, 28),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color {
    foreground: String,
    background: String,
    /// In the order first given, without repeats (PHP keys them by name).
    options: Vec<(&'static str, u8, u8)>,
}

impl Color {
    pub fn new(foreground: &str, background: &str, options: &[String]) -> Result<Self> {
        let foreground = parse_color(foreground, false)?;
        let background = parse_color(background, true)?;
        let mut chosen: Vec<(&'static str, u8, u8)> = Vec::new();
        for option in options {
            let Some(&found) = AVAILABLE_OPTIONS.iter().find(|(name, ..)| name == option) else {
                let names: Vec<&str> = AVAILABLE_OPTIONS.iter().map(|(n, ..)| *n).collect();
                return Err(ConsoleError::InvalidArgument(format!(
                    "Invalid option specified: \"{option}\". Expected one of ({}).",
                    names.join(", ")
                )));
            };
            if !chosen.iter().any(|(name, ..)| *name == found.0) {
                chosen.push(found);
            }
        }
        Ok(Self {
            foreground,
            background,
            options: chosen,
        })
    }

    pub fn apply(&self, text: &str) -> String {
        format!("{}{text}{}", self.set(), self.unset())
    }

    pub fn set(&self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if !self.foreground.is_empty() {
            codes.push(self.foreground.clone());
        }
        if !self.background.is_empty() {
            codes.push(self.background.clone());
        }
        codes.extend(self.options.iter().map(|(_, set, _)| set.to_string()));
        if codes.is_empty() {
            return String::new();
        }
        format!("\x1b[{}m", codes.join(";"))
    }

    pub fn unset(&self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if !self.foreground.is_empty() {
            codes.push("39".to_owned());
        }
        if !self.background.is_empty() {
            codes.push("49".to_owned());
        }
        codes.extend(self.options.iter().map(|(_, _, unset)| unset.to_string()));
        if codes.is_empty() {
            return String::new();
        }
        format!("\x1b[{}m", codes.join(";"))
    }
}

fn parse_color(color: &str, background: bool) -> Result<String> {
    if color.is_empty() {
        return Ok(String::new());
    }
    if color.starts_with('#') {
        let code = Terminal::get_color_mode().convert_from_hex_to_ansi_color_code(color)?;
        return Ok(format!("{}{code}", if background { '4' } else { '3' }));
    }
    if let Some((_, code)) = COLORS.iter().find(|(name, _)| *name == color) {
        return Ok(format!("{}{code}", if background { '4' } else { '3' }));
    }
    if let Some((_, code)) = BRIGHT_COLORS.iter().find(|(name, _)| *name == color) {
        return Ok(format!("{}{code}", if background { "10" } else { "9" }));
    }
    let names: Vec<&str> = COLORS
        .iter()
        .map(|(n, _)| *n)
        .chain(BRIGHT_COLORS.iter().map(|(n, _)| *n))
        .collect();
    Err(ConsoleError::InvalidArgument(format!(
        "Invalid \"{color}\" color; expected one of ({}).",
        names.join(", ")
    )))
}
