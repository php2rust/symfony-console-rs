//! Port of Symfony\Component\Console\Input\InputDefinition.php.
//!
//! PHP's arrays keyed by name are vectors of entries here, in insertion
//! order.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Input/InputDefinition.php

use php_value::{Key, PhpArray};

use super::{input_argument::InputArgument, input_option::InputOption};
use crate::exception::{ConsoleError, Result};

/// An item of `new InputDefinition([...])`.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Argument(InputArgument),
    Option(InputOption),
}

impl From<InputArgument> for Item {
    fn from(argument: InputArgument) -> Self {
        Self::Argument(argument)
    }
}

impl From<InputOption> for Item {
    fn from(option: InputOption) -> Self {
        Self::Option(option)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InputDefinition {
    arguments: Vec<InputArgument>,
    required_count: usize,
    last_array_argument: Option<String>,
    last_optional_argument: Option<String>,
    options: Vec<InputOption>,
    /// `no-<name>` → `<name>`.
    negations: Vec<(String, String)>,
    /// shortcut → option name.
    shortcuts: Vec<(String, String)>,
}

impl InputDefinition {
    pub fn new(definition: Vec<Item>) -> Result<Self> {
        let mut this = Self::default();
        this.set_definition(definition)?;
        Ok(this)
    }

    pub fn set_definition(&mut self, definition: Vec<Item>) -> Result<()> {
        let mut arguments = Vec::new();
        let mut options = Vec::new();
        for item in definition {
            match item {
                Item::Option(o) => options.push(o),
                Item::Argument(a) => arguments.push(a),
            }
        }
        self.set_arguments(arguments)?;
        self.set_options(options)
    }

    pub fn set_arguments(&mut self, arguments: Vec<InputArgument>) -> Result<()> {
        self.arguments.clear();
        self.required_count = 0;
        self.last_optional_argument = None;
        self.last_array_argument = None;
        self.add_arguments(arguments)
    }

    pub fn add_arguments(&mut self, arguments: Vec<InputArgument>) -> Result<()> {
        for argument in arguments {
            self.add_argument(argument)?;
        }
        Ok(())
    }

    pub fn add_argument(&mut self, argument: InputArgument) -> Result<()> {
        let name = argument.get_name().to_owned();
        if self.has_argument(&name) {
            return Err(ConsoleError::Logic(format!(
                "An argument with name \"{name}\" already exists."
            )));
        }
        if let Some(last) = &self.last_array_argument {
            return Err(ConsoleError::Logic(format!(
                "Cannot add a required argument \"{name}\" after an array argument \"{last}\"."
            )));
        }
        if argument.is_required()
            && let Some(last) = &self.last_optional_argument
        {
            return Err(ConsoleError::Logic(format!(
                "Cannot add a required argument \"{name}\" after an optional one \"{last}\"."
            )));
        }
        if argument.is_array() {
            self.last_array_argument = Some(name.clone());
        }
        if argument.is_required() {
            self.required_count += 1;
        } else {
            self.last_optional_argument = Some(name);
        }
        self.arguments.push(argument);
        Ok(())
    }

    /// `getArgument(string $name)`.
    pub fn get_argument(&self, name: &str) -> Result<&InputArgument> {
        self.arguments
            .iter()
            .find(|a| a.get_name() == name)
            .ok_or_else(|| {
                ConsoleError::InvalidArgument(format!("The \"{name}\" argument does not exist."))
            })
    }

    /// `getArgument(int $position)`.
    pub fn get_argument_at(&self, position: i64) -> Result<&InputArgument> {
        usize::try_from(position)
            .ok()
            .and_then(|p| self.arguments.get(p))
            .ok_or_else(|| {
                ConsoleError::InvalidArgument(format!(
                    "The \"{position}\" argument does not exist."
                ))
            })
    }

    pub fn has_argument(&self, name: &str) -> bool {
        self.arguments.iter().any(|a| a.get_name() == name)
    }

    pub fn has_argument_at(&self, position: i64) -> bool {
        usize::try_from(position).is_ok_and(|p| p < self.arguments.len())
    }

    pub fn get_arguments(&self) -> &[InputArgument] {
        &self.arguments
    }

    /// `getArgumentCount()`: `PHP_INT_MAX` after an array argument.
    pub fn get_argument_count(&self) -> i64 {
        if self.last_array_argument.is_some() {
            i64::MAX
        } else {
            self.arguments.len() as i64
        }
    }

    pub fn get_argument_required_count(&self) -> usize {
        self.required_count
    }

    pub fn get_argument_defaults(&self) -> PhpArray {
        self.arguments
            .iter()
            .map(|a| (Key::from(a.get_name()), a.get_default().clone()))
            .collect()
    }

    pub fn set_options(&mut self, options: Vec<InputOption>) -> Result<()> {
        self.options.clear();
        self.shortcuts.clear();
        self.negations.clear();
        self.add_options(options)
    }

    pub fn add_options(&mut self, options: Vec<InputOption>) -> Result<()> {
        for option in options {
            self.add_option(option)?;
        }
        Ok(())
    }

    pub fn add_option(&mut self, option: InputOption) -> Result<()> {
        let name = option.get_name().to_owned();
        if let Some(existing) = self.options.iter().find(|o| o.get_name() == name)
            && !option.equals(existing)
        {
            return Err(ConsoleError::Logic(format!(
                "An option named \"{name}\" already exists."
            )));
        }
        if self.has_negation(&name) {
            return Err(ConsoleError::Logic(format!(
                "An option named \"{name}\" already exists."
            )));
        }
        let shortcuts: Vec<String> = option
            .get_shortcut()
            .filter(|s| is_truthy(s))
            .map(|s| s.split('|').map(str::to_owned).collect())
            .unwrap_or_default();
        for shortcut in &shortcuts {
            if let Some(owner) = self.lookup(&self.shortcuts, shortcut)
                && let Some(existing) = self.options.iter().find(|o| o.get_name() == owner)
                && !option.equals(existing)
            {
                return Err(ConsoleError::Logic(format!(
                    "An option with shortcut \"{shortcut}\" already exists."
                )));
            }
        }
        let negatable = option.is_negatable();
        match self.options.iter_mut().find(|o| o.get_name() == name) {
            Some(existing) => *existing = option,
            None => self.options.push(option),
        }
        for shortcut in shortcuts {
            set(&mut self.shortcuts, shortcut, name.clone());
        }
        if negatable {
            let negated = format!("no-{name}");
            if self.has_option(&negated) {
                return Err(ConsoleError::Logic(format!(
                    "An option named \"{negated}\" already exists."
                )));
            }
            set(&mut self.negations, negated, name);
        }
        Ok(())
    }

    fn lookup<'a>(&self, map: &'a [(String, String)], key: &str) -> Option<&'a str> {
        map.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    pub fn get_option(&self, name: &str) -> Result<&InputOption> {
        self.options
            .iter()
            .find(|o| o.get_name() == name)
            .ok_or_else(|| {
                ConsoleError::InvalidArgument(format!("The \"--{name}\" option does not exist."))
            })
    }

    pub fn has_option(&self, name: &str) -> bool {
        self.options.iter().any(|o| o.get_name() == name)
    }

    pub fn get_options(&self) -> &[InputOption] {
        &self.options
    }

    pub fn has_shortcut(&self, name: &str) -> bool {
        self.lookup(&self.shortcuts, name).is_some()
    }

    pub fn has_negation(&self, name: &str) -> bool {
        self.lookup(&self.negations, name).is_some()
    }

    pub fn get_option_for_shortcut(&self, shortcut: &str) -> Result<&InputOption> {
        self.get_option(&self.shortcut_to_name(shortcut)?)
    }

    pub fn get_option_defaults(&self) -> PhpArray {
        self.options
            .iter()
            .map(|o| (Key::from(o.get_name()), o.get_default().clone()))
            .collect()
    }

    pub fn shortcut_to_name(&self, shortcut: &str) -> Result<String> {
        self.lookup(&self.shortcuts, shortcut)
            .map(str::to_owned)
            .ok_or_else(|| {
                ConsoleError::InvalidArgument(format!("The \"-{shortcut}\" option does not exist."))
            })
    }

    pub fn negation_to_name(&self, negation: &str) -> Result<String> {
        self.lookup(&self.negations, negation)
            .map(str::to_owned)
            .ok_or_else(|| {
                ConsoleError::InvalidArgument(format!(
                    "The \"--{negation}\" option does not exist."
                ))
            })
    }

    /// `getSynopsis($short)`.
    pub fn get_synopsis(&self, short: bool) -> String {
        let mut elements: Vec<String> = Vec::new();
        if short && !self.options.is_empty() {
            elements.push("[options]".to_owned());
        } else if !short {
            for option in &self.options {
                let mut value = String::new();
                if option.accept_value() {
                    let (open, close) = if option.is_value_optional() {
                        ("[", "]")
                    } else {
                        ("", "")
                    };
                    value = format!(" {open}{}{close}", option.get_name().to_uppercase());
                }
                let shortcut = match option.get_shortcut() {
                    Some(s) if is_truthy(s) => format!("-{s}|"),
                    _ => String::new(),
                };
                let negation = if option.is_negatable() {
                    format!("|--no-{}", option.get_name())
                } else {
                    String::new()
                };
                elements.push(format!(
                    "[{shortcut}--{}{value}{negation}]",
                    option.get_name()
                ));
            }
        }
        if !elements.is_empty() && !self.arguments.is_empty() {
            elements.push("[--]".to_owned());
        }
        let mut tail = String::new();
        for argument in &self.arguments {
            let mut element = format!("<{}>", argument.get_name());
            if argument.is_array() {
                element.push_str("...");
            }
            if !argument.is_required() {
                element = format!("[{element}");
                tail.push(']');
            }
            elements.push(element);
        }
        format!("{}{tail}", elements.join(" "))
    }
}

/// PHP's truthiness of a string: neither `''` nor `'0'`.
fn is_truthy(s: &str) -> bool {
    !s.is_empty() && s != "0"
}

fn set(map: &mut Vec<(String, String)>, key: String, value: String) {
    match map.iter_mut().find(|(k, _)| *k == key) {
        Some(entry) => entry.1 = value,
        None => map.push((key, value)),
    }
}
