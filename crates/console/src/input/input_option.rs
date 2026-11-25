//! Port of Symfony\Component\Console\Input\InputOption.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Input/InputOption.php

use php_value::Value;
use symfony_deprecation_contracts::trigger_deprecation;

use super::SuggestedValues;
use crate::{
    completion::{completion_input::CompletionInput, suggestion::CompletionSuggestions},
    exception::{ConsoleError, Result},
};

pub const VALUE_NONE: i64 = 1;
pub const VALUE_REQUIRED: i64 = 2;
pub const VALUE_OPTIONAL: i64 = 4;
pub const VALUE_IS_ARRAY: i64 = 8;
pub const VALUE_NEGATABLE: i64 = 16;

/// The shortcut argument: none, a string, or an array of strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shortcut<'a> {
    None,
    One(&'a str),
    Many(&'a [&'a str]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputOption {
    name: String,
    shortcut: Option<String>,
    mode: i64,
    description: String,
    default: Value,
    suggested_values: SuggestedValues,
}

impl InputOption {
    /// `new InputOption($name, $shortcut, $mode, $description, $default, $suggestedValues)`.
    #[expect(clippy::needless_pass_by_value, reason = "upstream's signature")]
    pub fn new(
        name: &str,
        shortcut: Shortcut<'_>,
        mode: Option<i64>,
        description: &str,
        default: Value,
        suggested_values: SuggestedValues,
    ) -> Result<Self> {
        let name = name.strip_prefix("--").unwrap_or(name);
        // `!$name`: '' and '0' are empty.
        if name.is_empty() || name == "0" {
            return Err(ConsoleError::InvalidArgument(
                "An option name cannot be empty.".to_owned(),
            ));
        }
        let shortcut = match shortcut {
            Shortcut::None | Shortcut::One("") | Shortcut::Many([]) => None,
            Shortcut::One(s) => Some(s.to_owned()),
            Shortcut::Many(list) => Some(list.join("|")),
        };
        let shortcut = match shortcut {
            None => None,
            Some(s) => {
                // preg_split('{(\|)-?}', ltrim($shortcut, '-')), empty parts dropped
                let parts: Vec<&str> = s
                    .trim_start_matches('-')
                    .split('|')
                    .enumerate()
                    .map(|(i, p)| {
                        if i > 0 {
                            p.strip_prefix('-').unwrap_or(p)
                        } else {
                            p
                        }
                    })
                    .filter(|p| !p.is_empty())
                    .collect();
                let joined = parts.join("|");
                if joined.is_empty() {
                    return Err(ConsoleError::InvalidArgument(
                        "An option shortcut cannot be empty.".to_owned(),
                    ));
                }
                Some(joined)
            }
        };
        let mode = mode.unwrap_or(0);
        // If not explicitly marked as required or optional, we assume the value accepts no input
        let mode =
            if VALUE_REQUIRED == VALUE_REQUIRED & mode || VALUE_OPTIONAL == VALUE_OPTIONAL & mode {
                mode
            } else {
                VALUE_NONE | mode
            };
        if !(1..VALUE_NEGATABLE << 1).contains(&mode) {
            return Err(ConsoleError::InvalidArgument(format!(
                "Option mode \"{mode}\" is not valid."
            )));
        }
        let kinds = mode & (VALUE_NONE | VALUE_REQUIRED | VALUE_OPTIONAL);
        if ![VALUE_NONE, VALUE_REQUIRED, VALUE_OPTIONAL].contains(&kinds) {
            trigger_deprecation(
                b"symfony/console",
                b"8.1",
                format!("Option \"{name}\" mode should be either none, required or optional.")
                    .as_bytes(),
            );
        }
        let mut option = Self {
            name: name.to_owned(),
            shortcut,
            mode,
            description: description.to_owned(),
            default: Value::Null,
            suggested_values,
        };
        if !option.suggested_values.is_empty() && !option.accept_value() {
            return Err(ConsoleError::Logic(
                "Cannot set suggested values if the option does not accept a value.".to_owned(),
            ));
        }
        if option.is_array() && !option.accept_value() {
            return Err(ConsoleError::InvalidArgument(
                "Impossible to have an option mode VALUE_IS_ARRAY if the option does not accept a value.".to_owned(),
            ));
        }
        if option.is_negatable() && option.accept_value() {
            return Err(ConsoleError::InvalidArgument(
                "Impossible to have an option mode VALUE_NEGATABLE if the option also accepts a value.".to_owned(),
            ));
        }
        option.set_default(default)?;
        Ok(option)
    }

    /// `new InputOption($name, $shortcut, $mode, $description)`.
    pub fn simple(
        name: &str,
        shortcut: Option<&str>,
        mode: Option<i64>,
        description: &str,
    ) -> Result<Self> {
        let shortcut = shortcut.map_or(Shortcut::None, Shortcut::One);
        Self::new(
            name,
            shortcut,
            mode,
            description,
            Value::Null,
            SuggestedValues::default(),
        )
    }

    pub fn get_shortcut(&self) -> Option<&str> {
        self.shortcut.as_deref()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn accept_value(&self) -> bool {
        self.is_value_required() || self.is_value_optional()
    }

    pub fn is_value_required(&self) -> bool {
        VALUE_REQUIRED == VALUE_REQUIRED & self.mode
    }

    pub fn is_value_optional(&self) -> bool {
        VALUE_OPTIONAL == VALUE_OPTIONAL & self.mode
    }

    pub fn is_array(&self) -> bool {
        VALUE_IS_ARRAY == VALUE_IS_ARRAY & self.mode
    }

    pub fn is_negatable(&self) -> bool {
        VALUE_NEGATABLE == VALUE_NEGATABLE & self.mode
    }

    pub fn set_default(&mut self, default: Value) -> Result<()> {
        if VALUE_NONE == VALUE_NONE & self.mode && !self.is_negatable() && !default.is_null() {
            return Err(ConsoleError::Logic(
                "Cannot set a default value when using InputOption::VALUE_NONE mode.".to_owned(),
            ));
        }
        if self.is_negatable() && !default.is_null() && !matches!(default, Value::Bool(_)) {
            return Err(ConsoleError::Logic(
                "A default value for a negatable option must be a boolean or null.".to_owned(),
            ));
        }
        let default = if self.is_array() {
            match default {
                Value::Null => Value::Array(php_value::PhpArray::default()),
                Value::Array(a) => Value::Array(a),
                _ => {
                    return Err(ConsoleError::Logic(
                        "A default value for an array option must be an array.".to_owned(),
                    ));
                }
            }
        } else {
            default
        };
        self.default = if self.accept_value() || self.is_negatable() {
            default
        } else {
            Value::Bool(false)
        };
        Ok(())
    }

    pub fn get_default(&self) -> &Value {
        &self.default
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn has_completion(&self) -> bool {
        !self.suggested_values.is_empty()
    }

    pub fn complete(
        &self,
        input: &CompletionInput,
        suggestions: &mut CompletionSuggestions,
    ) -> Result<()> {
        let values = self
            .suggested_values
            .resolve(input, || format!("option \"{}\"", self.name))?;
        if !values.is_empty() {
            suggestions.suggest_values(&values);
        }
        Ok(())
    }

    /// `equals($option)`: same name, shortcut, default (`===`) and mode flags.
    pub fn equals(&self, option: &InputOption) -> bool {
        option.name == self.name
            && option.shortcut == self.shortcut
            && option.default.identical(&self.default)
            && option.is_negatable() == self.is_negatable()
            && option.is_array() == self.is_array()
            && option.is_value_required() == self.is_value_required()
            && option.is_value_optional() == self.is_value_optional()
    }
}
