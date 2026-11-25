//! Port of Symfony\Component\Console\Formatter\OutputFormatterStyle.php
//! (and OutputFormatterStyleInterface.php).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Formatter/OutputFormatterStyle.php Formatter/OutputFormatterStyleInterface.php

use std::cell::Cell;

use crate::{
    color::Color,
    env::{getenv, server},
    exception::Result,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OutputFormatterStyle {
    color: Color,
    foreground: String,
    background: String,
    options: Vec<String>,
    href: Option<String>,
    /// `$this->handlesHrefGracefully`, decided on the first `apply()`.
    handles_href_gracefully: Cell<Option<bool>>,
}

impl OutputFormatterStyle {
    /// `new OutputFormatterStyle($foreground, $background, $options)`.
    pub fn new(
        foreground: Option<&str>,
        background: Option<&str>,
        options: &[&str],
    ) -> Result<Self> {
        let foreground = foreground.unwrap_or_default().to_owned();
        let background = background.unwrap_or_default().to_owned();
        let options: Vec<String> = options.iter().map(|o| (*o).to_owned()).collect();
        Ok(Self {
            color: Color::new(&foreground, &background, &options)?,
            foreground,
            background,
            options,
            href: None,
            handles_href_gracefully: Cell::new(None),
        })
    }

    /// `new OutputFormatterStyle()`.
    pub fn empty() -> Self {
        Self {
            color: Color::new("", "", &[]).unwrap_or_else(|_| unreachable!("no color is valid")),
            foreground: String::new(),
            background: String::new(),
            options: Vec::new(),
            href: None,
            handles_href_gracefully: Cell::new(None),
        }
    }

    /// `setForeground($color)`: the name is kept even when the color is
    /// invalid and the exception leaves the previous color in place.
    pub fn set_foreground(&mut self, color: Option<&str>) -> Result<()> {
        color.unwrap_or_default().clone_into(&mut self.foreground);
        self.color = Color::new(&self.foreground, &self.background, &self.options)?;
        Ok(())
    }

    pub fn set_background(&mut self, color: Option<&str>) -> Result<()> {
        color.unwrap_or_default().clone_into(&mut self.background);
        self.color = Color::new(&self.foreground, &self.background, &self.options)?;
        Ok(())
    }

    pub fn set_href(&mut self, url: &str) {
        self.href = Some(url.to_owned());
    }

    /// `setOption($option)`: appended before the new Color is built, so an
    /// invalid option stays in the list after the exception.
    pub fn set_option(&mut self, option: &str) -> Result<()> {
        self.options.push(option.to_owned());
        self.color = Color::new(&self.foreground, &self.background, &self.options)?;
        Ok(())
    }

    /// `unsetOption($option)`: the first equal entry is removed.
    pub fn unset_option(&mut self, option: &str) -> Result<()> {
        if let Some(at) = self.options.iter().position(|o| o == option) {
            self.options.remove(at);
        }
        self.color = Color::new(&self.foreground, &self.background, &self.options)?;
        Ok(())
    }

    pub fn set_options(&mut self, options: &[&str]) -> Result<()> {
        self.options = options.iter().map(|o| (*o).to_owned()).collect();
        self.color = Color::new(&self.foreground, &self.background, &self.options)?;
        Ok(())
    }

    pub fn apply(&self, text: &str) -> String {
        let graceful = if let Some(graceful) = self.handles_href_gracefully.get() {
            graceful
        } else {
            let konsole = getenv("KONSOLE_VERSION");
            let graceful = getenv("TERMINAL_EMULATOR").as_deref() != Some("JetBrains-JediTerm")
                && konsole
                    .as_deref()
                    .is_none_or(|v| v.is_empty() || v == "0" || php_int(v) > 201_100)
                && server("IDEA_INITIAL_DIRECTORY").is_none();
            self.handles_href_gracefully.set(Some(graceful));
            graceful
        };
        match &self.href {
            Some(href) if graceful => self
                .color
                .apply(&format!("\x1b]8;;{href}\x1b\\{text}\x1b]8;;\x1b\\")),
            _ => self.color.apply(text),
        }
    }
}

/// `(int) $string`.
fn php_int(value: &str) -> i64 {
    php_value::numeric::string_to_int(value.as_bytes())
}
