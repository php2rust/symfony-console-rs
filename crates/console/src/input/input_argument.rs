//! Port of Symfony\Component\Console\Input\InputArgument.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Input/InputArgument.php

use php_value::Value;
use symfony_deprecation_contracts::trigger_deprecation;

use super::SuggestedValues;
use crate::{
    completion::{completion_input::CompletionInput, suggestion::CompletionSuggestions},
    exception::{ConsoleError, Result},
};

pub const REQUIRED: i64 = 1;
pub const OPTIONAL: i64 = 2;
pub const IS_ARRAY: i64 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct InputArgument {
    name: String,
    mode: i64,
    description: String,
    default: Value,
    suggested_values: SuggestedValues,
}

impl InputArgument {
    /// `new InputArgument($name, $mode, $description, $default, $suggestedValues)`.
    pub fn new(
        name: &str,
        mode: Option<i64>,
        description: &str,
        default: Value,
        suggested_values: SuggestedValues,
    ) -> Result<Self> {
        let mode = mode.unwrap_or(0);
        // If not explicitly marked as required, we assume the value to be optional
        let mode = if REQUIRED == REQUIRED & mode {
            mode
        } else {
            OPTIONAL | mode
        };
        if !(1..IS_ARRAY << 1).contains(&mode) {
            return Err(ConsoleError::InvalidArgument(format!(
                "Argument mode \"{mode}\" is not valid."
            )));
        }
        if (REQUIRED | OPTIONAL) == (REQUIRED | OPTIONAL) & mode {
            trigger_deprecation(
                b"symfony/console",
                b"8.1",
                format!("Argument \"{name}\" mode should specify either required or optional.")
                    .as_bytes(),
            );
        }
        let mut argument = Self {
            name: name.to_owned(),
            mode,
            description: description.to_owned(),
            default: Value::Null,
            suggested_values,
        };
        argument.set_default(default)?;
        Ok(argument)
    }

    /// `new InputArgument($name, $mode, $description)`.
    pub fn simple(name: &str, mode: Option<i64>, description: &str) -> Result<Self> {
        Self::new(
            name,
            mode,
            description,
            Value::Null,
            SuggestedValues::default(),
        )
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_required(&self) -> bool {
        REQUIRED == REQUIRED & self.mode
    }

    pub fn is_array(&self) -> bool {
        IS_ARRAY == IS_ARRAY & self.mode
    }

    pub fn set_default(&mut self, default: Value) -> Result<()> {
        if self.is_required() && !default.is_null() {
            return Err(ConsoleError::Logic(
                "Cannot set a default value except for InputArgument::OPTIONAL mode.".to_owned(),
            ));
        }
        let default = if self.is_array() {
            match default {
                Value::Null => Value::Array(php_value::PhpArray::default()),
                Value::Array(a) => Value::Array(a),
                _ => {
                    return Err(ConsoleError::Logic(
                        "A default value for an array argument must be an array.".to_owned(),
                    ));
                }
            }
        } else {
            default
        };
        self.default = default;
        Ok(())
    }

    pub fn get_default(&self) -> &Value {
        &self.default
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
            .resolve(input, || format!("argument \"{}\"", self.name))?;
        if !values.is_empty() {
            suggestions.suggest_values(&values);
        }
        Ok(())
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }
}
