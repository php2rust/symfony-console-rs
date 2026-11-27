//! Port of Symfony\Component\Console\Formatter\OutputFormatterStyleStack.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Formatter/OutputFormatterStyleStack.php

use super::output_formatter_style::OutputFormatterStyle;
use crate::exception::{ConsoleError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct OutputFormatterStyleStack {
    styles: Vec<OutputFormatterStyle>,
    empty_style: OutputFormatterStyle,
}

impl Default for OutputFormatterStyleStack {
    fn default() -> Self {
        Self::new(None)
    }
}

impl OutputFormatterStyleStack {
    pub fn new(empty_style: Option<OutputFormatterStyle>) -> Self {
        Self {
            styles: Vec::new(),
            empty_style: empty_style.unwrap_or_else(OutputFormatterStyle::empty),
        }
    }

    pub fn reset(&mut self) {
        self.styles.clear();
    }

    pub fn push(&mut self, style: OutputFormatterStyle) {
        self.styles.push(style);
    }

    /// `pop($style)`: the last style, or the last one that applies as
    /// `$style` does, dropping those above it.
    pub fn pop(&mut self, style: Option<&OutputFormatterStyle>) -> Result<OutputFormatterStyle> {
        if self.styles.is_empty() {
            return Ok(self.empty_style.clone());
        }
        let Some(style) = style else {
            return Ok(self
                .styles
                .pop()
                .unwrap_or_else(|| self.empty_style.clone()));
        };
        let wanted = style.apply("");
        if let Some(index) = self.styles.iter().rposition(|s| s.apply("") == wanted) {
            let found = self.styles[index].clone();
            self.styles.truncate(index);
            return Ok(found);
        }
        Err(ConsoleError::InvalidArgument(
            "Incorrectly nested style tag found.".to_owned(),
        ))
    }

    pub fn get_current(&self) -> &OutputFormatterStyle {
        self.styles.last().unwrap_or(&self.empty_style)
    }

    pub fn set_empty_style(&mut self, style: OutputFormatterStyle) -> &mut Self {
        self.empty_style = style;
        self
    }

    pub fn get_empty_style(&self) -> &OutputFormatterStyle {
        &self.empty_style
    }
}
