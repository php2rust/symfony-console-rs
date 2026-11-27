//! `Symfony\Component\Console\Input`.
//!
//! Upstream: none

use std::sync::Arc;

use php_value::Value;

use crate::{
    completion::completion_input::CompletionInput,
    exception::{ConsoleError, Result},
};

#[allow(clippy::module_inception)]
pub mod input;
pub mod input_argument;
pub mod input_definition;
pub mod input_option;
pub mod string_input;

/// A closure giving suggested values; it must return an array.
pub type SuggestionClosure = Arc<dyn Fn(&CompletionInput) -> Value + Send + Sync>;

/// `array|\Closure $suggestedValues`.
#[derive(Clone, Default)]
pub enum SuggestedValues {
    #[default]
    None,
    List(Vec<Value>),
    Closure(SuggestionClosure),
}

impl SuggestedValues {
    pub fn list<S: Into<Value>>(values: impl IntoIterator<Item = S>) -> Self {
        let values: Vec<Value> = values.into_iter().map(Into::into).collect();
        if values.is_empty() {
            Self::None
        } else {
            Self::List(values)
        }
    }

    /// `[] === $this->suggestedValues`: a closure is never empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::List(values) => values.is_empty(),
            Self::Closure(_) => false,
        }
    }

    /// The values, calling the closure; it must return an array.
    pub(crate) fn resolve(
        &self,
        input: &CompletionInput,
        what: impl FnOnce() -> String,
    ) -> Result<Vec<Value>> {
        match self {
            Self::None => Ok(Vec::new()),
            Self::List(values) => Ok(values.clone()),
            Self::Closure(closure) => match closure(input) {
                Value::Array(array) => Ok(array.values().cloned().collect()),
                other => Err(ConsoleError::Logic(format!(
                    "Closure for {} must return an array. Got \"{}\".",
                    what(),
                    other.debug_type()
                ))),
            },
        }
    }
}

impl std::fmt::Debug for SuggestedValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("[]"),
            Self::List(values) => values.fmt(f),
            Self::Closure(_) => f.write_str("Closure"),
        }
    }
}

/// PHPUnit's `assertEquals` on two closures compares the objects; here a
/// closure equals only itself.
impl PartialEq for SuggestedValues {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Closure(a), Self::Closure(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
