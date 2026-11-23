//! Port of Symfony\Component\Console\Completion\CompletionInput.php: an
//! ArgvInput whose parse errors are ignored, and what is being completed.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Completion/CompletionInput.php

use php_value::{Key, Value};

use crate::{
    exception::{ConsoleError, Result},
    input::{
        input::{Input, php_text},
        input_definition::InputDefinition,
        input_option::InputOption,
    },
};

pub const TYPE_ARGUMENT_VALUE: &str = "argument_value";
pub const TYPE_OPTION_VALUE: &str = "option_value";
pub const TYPE_OPTION_NAME: &str = "option_name";
pub const TYPE_NONE: &str = "none";

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionInput {
    input: Input,
    /// All tokens, the application's name included.
    tokens: Vec<String>,
    current_index: usize,
    completion_type: &'static str,
    completion_name: Option<String>,
    completion_value: String,
}

impl CompletionInput {
    /// `fromString($inputStr, $currentIndex)`: tokens split on whitespace,
    /// a token may be quoted (the quotes are kept).
    pub fn from_string(input: &str, current_index: usize) -> Self {
        Self::from_tokens(split_tokens(input), current_index)
    }

    /// `fromTokens($tokens, $currentIndex)`.
    pub fn from_tokens(tokens: Vec<String>, current_index: usize) -> Self {
        Self {
            input: Input::completion(tokens.iter().skip(1).cloned().collect()),
            tokens,
            current_index,
            completion_type: TYPE_NONE,
            completion_name: None,
            completion_value: String::new(),
        }
    }

    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    /// `bind($definition)`.
    pub fn bind(&mut self, definition: InputDefinition) -> Result<()> {
        self.input.bind(definition)?;
        let relevant = self.relevant_token()?.to_owned();
        if relevant.starts_with('-') {
            // the current token is an input option: complete either option name or option value
            let (option_token, option_value) =
                relevant.split_once('=').unwrap_or((relevant.as_str(), ""));
            let option = self.option_from_token(option_token);
            if option.is_none() && !self.is_cursor_free()? {
                self.completion_type = TYPE_OPTION_NAME;
                self.completion_value = relevant.clone();
                return Ok(());
            }
            if let Some(option) = option.filter(InputOption::accept_value) {
                self.completion_type = TYPE_OPTION_VALUE;
                self.completion_name = Some(option.get_name().to_owned());
                self.completion_value = if !option_value.is_empty() && option_value != "0" {
                    option_value.to_owned()
                } else if !option_token.starts_with("--") {
                    option_token.get(2..).unwrap_or_default().to_owned()
                } else {
                    String::new()
                };
                return Ok(());
            }
        }
        let previous = self
            .current_index
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .cloned();
        if let Some(previous) = previous
            && previous.starts_with('-')
            && !previous.trim_matches('-').is_empty()
        {
            // check if previous option accepted a value
            if let Some(option) = self
                .option_from_token(&previous)
                .filter(InputOption::accept_value)
            {
                self.completion_type = TYPE_OPTION_VALUE;
                self.completion_name = Some(option.get_name().to_owned());
                self.completion_value = relevant;
                return Ok(());
            }
        }
        // complete argument value
        self.completion_type = TYPE_ARGUMENT_VALUE;
        let mut argument_name: Option<String> = None;
        let names: Vec<String> = self
            .input
            .definition()
            .get_arguments()
            .iter()
            .map(|a| a.get_name().to_owned())
            .collect();
        for name in names {
            argument_name = Some(name.clone());
            let value = self
                .input
                .get_raw_arguments()
                .get(&Key::from(name.as_str()))
                .cloned();
            let Some(value) = value.filter(|v| !v.is_null()) else {
                break;
            };
            self.completion_name = Some(name);
            self.completion_value = match &value {
                Value::Array(list) => list.last().map(|(_, v)| php_text(v)).unwrap_or_default(),
                other => php_text(other),
            };
        }
        if self.current_index >= self.tokens.len() {
            let set = argument_name.as_deref().is_some_and(|n| {
                self.input
                    .get_raw_arguments()
                    .get(&Key::from(n))
                    .is_some_and(|v| !v.is_null())
            });
            let is_array = argument_name
                .as_deref()
                .and_then(|n| self.input.definition().get_argument(n).ok())
                .is_some_and(|a| a.is_array());
            if !set || is_array {
                self.completion_name = argument_name;
            } else {
                // we've reached the end
                self.completion_type = TYPE_NONE;
                self.completion_name = None;
            }
            self.completion_value = String::new();
        }
        Ok(())
    }

    pub fn get_completion_type(&self) -> &'static str {
        self.completion_type
    }

    pub fn get_completion_name(&self) -> Option<&str> {
        self.completion_name.as_deref()
    }

    pub fn get_completion_value(&self) -> &str {
        &self.completion_value
    }

    pub fn must_suggest_option_values_for(&self, option_name: &str) -> bool {
        self.completion_type == TYPE_OPTION_VALUE
            && self.completion_name.as_deref() == Some(option_name)
    }

    pub fn must_suggest_argument_values_for(&self, argument_name: &str) -> bool {
        self.completion_type == TYPE_ARGUMENT_VALUE
            && self.completion_name.as_deref() == Some(argument_name)
    }

    fn option_from_token(&self, option_token: &str) -> Option<InputOption> {
        let name = option_token.trim_start_matches('-');
        if name.is_empty() || name == "0" {
            return None;
        }
        let definition = self.input.definition();
        if option_token.as_bytes().get(1) == Some(&b'-') {
            // long option name
            return definition.get_option(name).ok().cloned();
        }
        // short option name
        let first = std::str::from_utf8(&name.as_bytes()[..1]).ok()?;
        definition.get_option_for_shortcut(first).ok().cloned()
    }

    fn relevant_token(&self) -> Result<&str> {
        let index = if self.is_cursor_free()? {
            self.current_index.checked_sub(1)
        } else {
            Some(self.current_index)
        };
        Ok(index
            .and_then(|i| self.tokens.get(i))
            .map_or("", String::as_str))
    }

    fn is_cursor_free(&self) -> Result<bool> {
        if self.current_index > self.tokens.len() {
            return Err(ConsoleError::Other {
                class: "LogicException".to_owned(),
                message:
                    "Current index is invalid, it must be the number of input tokens or one more."
                        .to_owned(),
            });
        }
        Ok(self.current_index >= self.tokens.len())
    }

    /// `__toString()`: the tokens, with `|` at the cursor.
    pub fn to_php_string(&self) -> String {
        let mut s = String::new();
        for (i, token) in self.tokens.iter().enumerate() {
            s.push_str(token);
            if self.current_index == i {
                s.push('|');
            }
            s.push(' ');
        }
        if !self.tokens.is_empty() && self.current_index > self.tokens.len() - 1 {
            s.push('|');
        }
        s.trim_end_matches([' ', '\t', '\n', '\r', '\0', '\x0B'])
            .to_owned()
    }
}

/// `preg_match_all('/(?<=^|\s)([\'"]?)(.+?)(?<!\\\\)\1(?=$|\s)/', $inputStr)`:
/// the whole matches.
fn split_tokens(input: &str) -> Vec<String> {
    let b = input.as_bytes();
    let is_space = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C);
    // `$` matches at the end or before a final newline
    let at_end = |i: usize| i == b.len() || (i + 1 == b.len() && b[i] == b'\n');
    let mut tokens = Vec::new();
    let mut start = 0;
    while start < b.len() {
        if start > 0 && !is_space(b[start - 1]) {
            start += 1;
            continue;
        }
        let mut found = None;
        // ([\'"]?) tries the quote first, then none
        let quotes: Vec<Option<u8>> = match b[start] {
            q @ (b'\'' | b'"') => vec![Some(q), None],
            _ => vec![None],
        };
        'quote: for quote in &quotes {
            let content = start + usize::from(quote.is_some());
            // (.+?): at least one character, no newline; the shortest that fits
            let mut end = content + 1;
            while end <= b.len() {
                if b[end - 1] == b'\n' {
                    break;
                }
                let close = match quote {
                    Some(q) => (b.get(end) == Some(q)).then_some(end + 1),
                    None => Some(end),
                };
                if let Some(after) = close
                    && b[end - 1] != b'\\'
                    && (at_end(after) || b.get(after).is_some_and(|&c| is_space(c)))
                {
                    found = Some(after);
                    break 'quote;
                }
                end += 1;
            }
        }
        match found {
            Some(end) => {
                tokens.push(String::from_utf8_lossy(&b[start..end]).into_owned());
                start = end.max(start + 1);
            }
            None => start += 1,
        }
    }
    tokens
}
