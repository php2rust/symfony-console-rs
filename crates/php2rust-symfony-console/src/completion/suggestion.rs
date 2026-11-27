//! Port of Symfony\Component\Console\Completion\Suggestion.php and
//! CompletionSuggestions.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Completion/Suggestion.php Completion/CompletionSuggestions.php

use php_value::{PhpArray, PhpObject, Value};

use crate::input::input_option::InputOption;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    value: String,
    description: String,
}

impl Suggestion {
    pub const CLASS: &'static str = "Symfony\\Component\\Console\\Completion\\Suggestion";

    pub fn new(value: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            description: description.into(),
        }
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }
}

impl std::fmt::Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

impl PhpObject for Suggestion {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }
}

/// `CompletionSuggestions`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompletionSuggestions {
    value_suggestions: Vec<Suggestion>,
    option_suggestions: Vec<InputOption>,
}

impl CompletionSuggestions {
    /// `suggestValue(string|Suggestion $value)`; a scalar is coerced to a
    /// string, as PHP does for a `string` parameter without strict types.
    pub fn suggest_value(&mut self, value: &Value) -> &mut Self {
        let suggestion = match value {
            Value::Object(o) => o
                .downcast_ref::<Suggestion>()
                .cloned()
                .unwrap_or_else(|| Suggestion::new(o.class(), "")),
            other => Suggestion::new(
                other
                    .to_php_string()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                "",
            ),
        };
        self.value_suggestions.push(suggestion);
        self
    }

    pub fn suggest_values(&mut self, values: &[Value]) -> &mut Self {
        for value in values {
            self.suggest_value(value);
        }
        self
    }

    pub fn suggest_option(&mut self, option: InputOption) -> &mut Self {
        self.option_suggestions.push(option);
        self
    }

    pub fn suggest_options(&mut self, options: Vec<InputOption>) -> &mut Self {
        self.option_suggestions.extend(options);
        self
    }

    pub fn get_option_suggestions(&self) -> &[InputOption] {
        &self.option_suggestions
    }

    pub fn get_value_suggestions(&self) -> &[Suggestion] {
        &self.value_suggestions
    }
}
