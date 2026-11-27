//! Port of Symfony\Component\Console\Tester\CommandCompletionTester.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Tester/CommandCompletionTester.php

use crate::{
    application::SharedCommand,
    completion::{completion_input::CompletionInput, suggestion::CompletionSuggestions},
    exception::Result,
};

/// `CommandCompletionTester`.
pub struct CommandCompletionTester {
    command: SharedCommand,
}

impl CommandCompletionTester {
    pub fn new(command: impl Into<SharedCommand>) -> Self {
        Self {
            command: command.into(),
        }
    }

    /// `complete($input)`: the suggested option names (`--name`) and
    /// values, as strings.
    pub fn complete(&self, input: &[&str]) -> Result<Vec<String>> {
        let current_index = input.len();
        let mut tokens: Vec<String> = input.iter().map(|t| (*t).to_owned()).collect();
        if tokens.last().is_some_and(String::is_empty) {
            tokens.pop();
        }
        let mut command = self.command.lock();
        tokens.insert(0, command.get_name().unwrap_or_default().to_owned());

        let mut completion_input = CompletionInput::from_tokens(tokens, current_index);
        completion_input.bind(command.get_definition().clone())?;
        let mut suggestions = CompletionSuggestions::default();

        command.complete(&completion_input, &mut suggestions)?;

        let mut out: Vec<String> = suggestions
            .get_option_suggestions()
            .iter()
            .map(|o| format!("--{}", o.get_name()))
            .collect();
        out.extend(
            suggestions
                .get_value_suggestions()
                .iter()
                .map(ToString::to_string),
        );
        Ok(out)
    }
}
