//! Port of Symfony\Component\Console\Helper\FormatterHelper.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/FormatterHelper.php

use super::{helper::Helper, helper_set::HelperInterface};
use crate::formatter::output_formatter::OutputFormatter;

/// `FormatterHelper`.
#[derive(Debug, Default, Clone, Copy)]
pub struct FormatterHelper;

impl FormatterHelper {
    pub fn new() -> Self {
        Self
    }

    /// `formatSection($section, $message, $style)`.
    pub fn format_section(&self, section: &str, message: &str, style: &str) -> String {
        format!("<{style}>[{section}]</{style}> {message}")
    }

    /// `formatBlock($messages, $style, $large)`.
    pub fn format_block(&self, messages: &[&str], style: &str, large: bool) -> String {
        let mut len = 0;
        let mut lines = Vec::with_capacity(messages.len());
        for message in messages {
            let message = OutputFormatter::escape(message);
            lines.push(if large {
                format!("  {message}  ")
            } else {
                format!(" {message} ")
            });
            len = len.max(Helper::width(&message) + if large { 4 } else { 2 });
        }

        let mut messages = Vec::with_capacity(lines.len() + 2);
        if large {
            messages.push(" ".repeat(len));
        }
        for line in &lines {
            // str_repeat() of a negative count is an error in PHP; the
            // width of a line is never more than $len.
            let pad = len.saturating_sub(Helper::width(line));
            messages.push(format!("{line}{}", " ".repeat(pad)));
        }
        if large {
            messages.push(" ".repeat(len));
        }

        messages
            .iter()
            .map(|m| format!("<{style}>{m}</{style}>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// `truncate($message, $length, $suffix)`.
    pub fn truncate(&self, message: &str, length: i64, suffix: &str) -> String {
        let computed_length = length - Helper::width(suffix) as i64;
        if computed_length > Helper::width(message) as i64 {
            return message.to_owned();
        }
        format!("{}{suffix}", Helper::substr(message, 0, Some(length)))
    }
}

impl HelperInterface for FormatterHelper {
    fn get_name(&self) -> &str {
        "formatter"
    }
}
