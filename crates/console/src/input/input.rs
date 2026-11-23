//! Port of Symfony\Component\Console\Input\Input.php, with ArrayInput.php,
//! ArgvInput.php and InputInterface.php.
//!
//! PHP's subclasses are the `Source` of one `Input`: an array of
//! parameters (`ArrayInput`) or command-line tokens (`ArgvInput`, and
//! `CompletionInput`, which ignores parse errors). Options and arguments
//! are PHP arrays (`PhpArray`), so keys behave as PHP's do. Tokens are
//! bytes in PHP: values cut from them (`-ofoo`, `-abc`) keep the bytes.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Input/Input.php Input/ArrayInput.php Input/ArgvInput.php Input/InputInterface.php

use std::collections::VecDeque;

use php_value::{Key, PhpArray, PhpString, Value};

use super::input_definition::InputDefinition;
use crate::exception::{ConsoleError, Result};

#[derive(Debug, Clone, PartialEq)]
enum Source {
    /// `ArrayInput`'s parameters.
    Array(PhpArray),
    /// `ArgvInput`'s tokens (the application name already removed).
    Argv { tokens: Vec<String>, lenient: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    definition: InputDefinition,
    options: PhpArray,
    arguments: PhpArray,
    interactive: bool,
    source: Source,
}

impl Input {
    /// `new ArrayInput($parameters, $definition)`.
    pub fn array(parameters: PhpArray, definition: Option<InputDefinition>) -> Result<Self> {
        Self::construct(Source::Array(parameters), definition)
    }

    /// `new ArgvInput($argv, $definition)`: the first token, the
    /// application's name, is dropped.
    pub fn argv(argv: &[&str], definition: Option<InputDefinition>) -> Result<Self> {
        let tokens = argv.iter().skip(1).map(|t| (*t).to_owned()).collect();
        Self::construct(
            Source::Argv {
                tokens,
                lenient: false,
            },
            definition,
        )
    }

    /// `new ArgvInput(null, $definition)`: the tokens come from
    /// `$_SERVER['argv']`. Tokens are `String`s here, so an argument that
    /// is not UTF-8 is refused (PHP would keep its bytes).
    pub fn argv_from_server(definition: Option<InputDefinition>) -> Result<Self> {
        let argv = crate::env::server_argv().map_err(|arg| {
            ConsoleError::Runtime(format!(
                "Argument values expected to be UTF-8, got {arg:?}."
            ))
        })?;
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        Self::argv(&argv, definition)
    }

    /// `new ArgvInput([])` with `setTokens($tokens)` (StringInput).
    pub(crate) fn from_tokens(tokens: Vec<String>) -> Self {
        Self {
            definition: InputDefinition::default(),
            options: PhpArray::new(),
            arguments: PhpArray::new(),
            interactive: true,
            source: Source::Argv {
                tokens,
                lenient: false,
            },
        }
    }

    /// `CompletionInput`'s base: tokens as given, parse errors ignored.
    pub(crate) fn completion(tokens: Vec<String>) -> Self {
        Self {
            definition: InputDefinition::default(),
            options: PhpArray::new(),
            arguments: PhpArray::new(),
            interactive: true,
            source: Source::Argv {
                tokens,
                lenient: true,
            },
        }
    }

    fn construct(source: Source, definition: Option<InputDefinition>) -> Result<Self> {
        let mut input = Self {
            definition: InputDefinition::default(),
            options: PhpArray::new(),
            arguments: PhpArray::new(),
            interactive: true,
            source,
        };
        if let Some(definition) = definition {
            input.bind(definition)?;
            input.validate()?;
        }
        Ok(input)
    }

    pub fn definition(&self) -> &InputDefinition {
        &self.definition
    }

    /// `bind($definition)`.
    pub fn bind(&mut self, definition: InputDefinition) -> Result<()> {
        self.arguments = PhpArray::new();
        self.options = PhpArray::new();
        self.definition = definition;
        self.parse()
    }

    fn parse(&mut self) -> Result<()> {
        match self.source.clone() {
            Source::Array(parameters) => self.parse_array(&parameters),
            Source::Argv { tokens, lenient } => {
                let mut parsed: VecDeque<String> = tokens.into();
                let mut parse_options = true;
                while let Some(token) = parsed.pop_front() {
                    match self.parse_token(&token, parse_options, &mut parsed) {
                        Ok(next) => parse_options = next,
                        // CompletionInput: completed input is almost never valid
                        Err(ConsoleError::Runtime(_)) if lenient => {}
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
        }
    }

    /// `validate()`.
    pub fn validate(&self) -> Result<()> {
        let missing: Vec<&str> = self
            .definition
            .get_arguments()
            .iter()
            .filter(|a| a.is_required() && !self.arguments.contains_key(&Key::from(a.get_name())))
            .map(|a| a.get_name())
            .collect();
        if !missing.is_empty() {
            return Err(ConsoleError::Runtime(format!(
                "Not enough arguments (missing: \"{}\").",
                missing.join(", ")
            )));
        }
        Ok(())
    }

    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    pub fn set_interactive(&mut self, interactive: bool) {
        self.interactive = interactive;
    }

    /// `getArguments()`: `array_merge($defaults, $arguments)`.
    pub fn get_arguments(&self) -> PhpArray {
        array_merge(&self.definition.get_argument_defaults(), &self.arguments)
    }

    pub fn get_argument(&self, name: &str) -> Result<Value> {
        if !self.definition.has_argument(name) {
            return Err(ConsoleError::InvalidArgument(format!(
                "The \"{name}\" argument does not exist."
            )));
        }
        match self.arguments.get(&Key::from(name)) {
            Some(value) if !value.is_null() => Ok(value.clone()),
            _ => Ok(self.definition.get_argument(name)?.get_default().clone()),
        }
    }

    pub fn set_argument(&mut self, name: &str, value: Value) -> Result<()> {
        if !self.definition.has_argument(name) {
            return Err(ConsoleError::InvalidArgument(format!(
                "The \"{name}\" argument does not exist."
            )));
        }
        self.arguments.insert(name, value);
        Ok(())
    }

    pub fn has_argument(&self, name: &str) -> bool {
        self.definition.has_argument(name)
    }

    /// `getOptions()`: `array_merge($defaults, $options)`.
    pub fn get_options(&self) -> PhpArray {
        array_merge(&self.definition.get_option_defaults(), &self.options)
    }

    pub fn get_option(&self, name: &str) -> Result<Value> {
        if self.definition.has_negation(name) {
            let value = self.get_option(&self.definition.negation_to_name(name)?)?;
            if value.is_null() {
                return Ok(value);
            }
            return Ok(Value::Bool(!truthy(&value)));
        }
        if !self.definition.has_option(name) {
            return Err(ConsoleError::InvalidArgument(format!(
                "The \"{name}\" option does not exist."
            )));
        }
        match self.options.get(&Key::from(name)) {
            Some(value) => Ok(value.clone()),
            None => Ok(self.definition.get_option(name)?.get_default().clone()),
        }
    }

    pub fn set_option(&mut self, name: &str, value: Value) -> Result<()> {
        if self.definition.has_negation(name) {
            let target = self.definition.negation_to_name(name)?;
            self.options.insert(target, Value::Bool(!truthy(&value)));
            return Ok(());
        }
        if !self.definition.has_option(name) {
            return Err(ConsoleError::InvalidArgument(format!(
                "The \"{name}\" option does not exist."
            )));
        }
        self.options.insert(name, value);
        Ok(())
    }

    pub fn has_option(&self, name: &str) -> bool {
        self.definition.has_option(name) || self.definition.has_negation(name)
    }

    /// `escapeToken($token)`.
    pub fn escape_token(token: &str) -> String {
        let word = !token.is_empty()
            && token
                .chars()
                .all(|c| c == '-' || c == '_' || c.is_ascii_alphanumeric());
        if word {
            token.to_owned()
        } else {
            escapeshellarg(token)
        }
    }

    pub fn get_raw_arguments(&self) -> &PhpArray {
        &self.arguments
    }

    pub fn get_raw_options(&self) -> &PhpArray {
        &self.options
    }

    /// `unparse($optionNames)`.
    pub fn unparse(&self, option_names: Option<&[&str]>) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for (key, parsed) in self.options.iter() {
            let name = key_text(key);
            if option_names.is_some_and(|names| !names.contains(&name.as_str())) {
                continue;
            }
            let option = self.definition.get_option(&name)?;
            if option.is_negatable() {
                out.push(format!(
                    "--{}{name}",
                    if truthy(parsed) { "" } else { "no-" }
                ));
            } else if !option.accept_value() {
                out.push(format!("--{name}"));
            } else if option.is_array() {
                for item in parsed
                    .as_array()
                    .map(|a| a.values().cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
                {
                    out.push(format!("--{name}={}", php_text(&item)));
                }
            } else {
                out.push(format!("--{name}={}", php_text(parsed)));
            }
        }
        Ok(out)
    }

    // ArrayInput / ArgvInput

    /// `getFirstArgument()`.
    pub fn get_first_argument(&self) -> Result<Option<Value>> {
        match &self.source {
            Source::Array(parameters) => {
                for (key, value) in parameters.iter() {
                    if let Key::Str(k) = key
                        && k.as_bytes().first() == Some(&b'-')
                    {
                        continue;
                    }
                    return Ok(Some(value.clone()));
                }
                Ok(None)
            }
            Source::Argv { tokens, .. } => {
                let mut is_option = false;
                for (i, token) in tokens.iter().enumerate() {
                    if token.starts_with('-') {
                        if token.contains('=') || i + 1 >= tokens.len() {
                            continue;
                        }
                        // A long option's name is everything after "--"; for
                        // a short option set, only the last one can take a
                        // value separated by a space.
                        let b = token.as_bytes();
                        let name = if b.get(1) == Some(&b'-') {
                            String::from_utf8_lossy(&b[2..]).into_owned()
                        } else {
                            String::from_utf8_lossy(&b[b.len() - 1..]).into_owned()
                        };
                        // isset(): a null value counts as unset
                        let has = |n: &str| {
                            self.options
                                .get(&Key::from(n))
                                .is_some_and(|v| !v.is_null())
                        };
                        if !has(&name) && !self.definition.has_shortcut(&name) {
                            // noop
                        } else {
                            let resolved = if has(&name) {
                                name.clone()
                            } else {
                                self.definition.shortcut_to_name(&name)?
                            };
                            if has(&resolved)
                                && self.options.get(&Key::from(resolved.as_str())).is_some_and(
                                    |v| {
                                        v.as_string().is_some_and(|s| {
                                            s.as_bytes() == tokens[i + 1].as_bytes()
                                        })
                                    },
                                )
                            {
                                is_option = true;
                            }
                        }
                        continue;
                    }
                    if is_option {
                        is_option = false;
                        continue;
                    }
                    return Ok(Some(Value::string(token.as_str())));
                }
                Ok(None)
            }
        }
    }

    /// `hasParameterOption($values, $onlyParams)`.
    pub fn has_parameter_option(&self, values: &[&str], only_params: bool) -> bool {
        match &self.source {
            Source::Array(parameters) => {
                for (key, value) in parameters.iter() {
                    let v = match key {
                        Key::Int(_) => value.clone(),
                        Key::Str(s) => Value::String(s.clone()),
                    };
                    if only_params && v.as_string().is_some_and(|s| s.as_bytes() == b"--") {
                        return false;
                    }
                    if values
                        .iter()
                        .any(|candidate| loose_equals_str(&v, candidate))
                    {
                        return true;
                    }
                }
                false
            }
            Source::Argv { tokens, .. } => {
                for token in tokens {
                    if only_params && token == "--" {
                        return false;
                    }
                    for value in values {
                        let leading = if value.starts_with("--") {
                            format!("{value}=")
                        } else {
                            (*value).to_owned()
                        };
                        if token == value || (!leading.is_empty() && token.starts_with(&leading)) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    /// `getParameterOption($values, $default, $onlyParams)`.
    pub fn get_parameter_option(
        &self,
        values: &[&str],
        default: Value,
        only_params: bool,
    ) -> Value {
        match &self.source {
            Source::Array(parameters) => {
                for (key, value) in parameters.iter() {
                    let is_dashdash =
                        |v: &Value| v.as_string().is_some_and(|s| s.as_bytes() == b"--");
                    if only_params
                        && (matches!(key, Key::Str(s) if s.as_bytes() == b"--")
                            || (matches!(key, Key::Int(_)) && is_dashdash(value)))
                    {
                        return default;
                    }
                    match key {
                        Key::Int(_) => {
                            if values.iter().any(|c| loose_equals_str(value, c)) {
                                return Value::Bool(true);
                            }
                        }
                        Key::Str(s) => {
                            if values
                                .iter()
                                .any(|c| loose_equals_str(&Value::String(s.clone()), c))
                            {
                                return value.clone();
                            }
                        }
                    }
                }
                default
            }
            Source::Argv { tokens, .. } => {
                let mut tokens: VecDeque<&String> = tokens.iter().collect();
                while let Some(token) = tokens.pop_front() {
                    if only_params && token == "--" {
                        return default;
                    }
                    for value in values {
                        if token == value {
                            return tokens
                                .pop_front()
                                .map_or(Value::Null, |t| Value::string(t.as_str()));
                        }
                        let leading = if value.starts_with("--") {
                            format!("{value}=")
                        } else {
                            (*value).to_owned()
                        };
                        if !leading.is_empty() && token.starts_with(&leading) {
                            return Value::String(PhpString::from(
                                &token.as_bytes()[leading.len()..],
                            ));
                        }
                    }
                }
                default
            }
        }
    }

    /// `ArgvInput::getRawTokens($strip)`.
    pub fn get_raw_tokens(&self, strip: bool) -> Result<Vec<String>> {
        let Source::Argv { tokens, .. } = &self.source else {
            return Ok(Vec::new());
        };
        if !strip {
            return Ok(tokens.clone());
        }
        let first = self
            .get_first_argument()?
            .and_then(|v| v.as_string().map(|s| s.to_string_lossy().into_owned()));
        let mut parameters = Vec::new();
        let mut keep = false;
        for value in tokens {
            if !keep && Some(value) == first.as_ref() {
                keep = true;
                continue;
            }
            if keep {
                parameters.push(value.clone());
            }
        }
        Ok(parameters)
    }

    /// `__toString()`.
    pub fn to_php_string(&self) -> String {
        match &self.source {
            Source::Array(parameters) => {
                let mut params: Vec<String> = Vec::new();
                for (key, value) in parameters.iter() {
                    if let Key::Str(k) = key
                        && k.as_bytes().first() == Some(&b'-')
                    {
                        let param = k.to_string_lossy().into_owned();
                        let glue = if k.as_bytes().get(1) == Some(&b'-') {
                            "="
                        } else {
                            " "
                        };
                        let one = |v: &Value| {
                            let text = php_text(v);
                            if loose_equals_str(v, "") {
                                param.clone()
                            } else {
                                format!("{param}{glue}{}", Self::escape_token(&text))
                            }
                        };
                        match value.as_array() {
                            Some(list) => params.extend(list.values().map(one)),
                            None => params.push(one(value)),
                        }
                    } else {
                        match value.as_array() {
                            Some(list) => params.push(
                                list.values()
                                    .map(|v| Self::escape_token(&php_text(v)))
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            ),
                            None => params.push(Self::escape_token(&php_text(value))),
                        }
                    }
                }
                params.join(" ")
            }
            Source::Argv { tokens, .. } => tokens
                .iter()
                .map(|token| {
                    // {^(-[^=]+=)(.+)}: `.` stops at a newline, and what
                    // follows it is dropped
                    if token.starts_with('-')
                        && let Some(eq) = token.find('=')
                        && eq > 1
                    {
                        let value = token[eq + 1..].split('\n').next().unwrap_or_default();
                        if !value.is_empty() {
                            return format!("{}{}", &token[..=eq], Self::escape_token(value));
                        }
                    }
                    if !token.is_empty() && !token.starts_with('-') {
                        return Self::escape_token(token);
                    }
                    token.clone()
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    // ArrayInput::parse()

    fn parse_array(&mut self, parameters: &PhpArray) -> Result<()> {
        for (key, value) in parameters.iter() {
            let name = key_text(key);
            if matches!(key, Key::Str(_)) && name == "--" {
                return Ok(());
            }
            if let Some(long) = name.strip_prefix("--") {
                self.array_add_long_option(long, value.clone())?;
            } else if let Some(short) = name.strip_prefix('-') {
                if !self.definition.has_shortcut(short) {
                    return Err(ConsoleError::InvalidOption(format!(
                        "The \"-{short}\" option does not exist."
                    )));
                }
                let long = self
                    .definition
                    .get_option_for_shortcut(short)?
                    .get_name()
                    .to_owned();
                self.array_add_long_option(&long, value.clone())?;
            } else {
                let exists = match key {
                    Key::Int(position) => self.definition.has_argument_at(*position),
                    Key::Str(_) => self.definition.has_argument(&name),
                };
                if !exists {
                    return Err(ConsoleError::InvalidArgument(format!(
                        "The \"{name}\" argument does not exist."
                    )));
                }
                self.arguments.insert(key.clone(), value.clone());
            }
        }
        Ok(())
    }

    fn array_add_long_option(&mut self, name: &str, value: Value) -> Result<()> {
        if !self.definition.has_option(name) {
            if !self.definition.has_negation(name) {
                return Err(ConsoleError::InvalidOption(format!(
                    "The \"--{name}\" option does not exist."
                )));
            }
            let option_name = self.definition.negation_to_name(name)?;
            self.options.insert(option_name, Value::Bool(false));
            return Ok(());
        }
        let option = self.definition.get_option(name)?;
        let mut value = value;
        if value.is_null() {
            if option.is_value_required() {
                return Err(ConsoleError::InvalidOption(format!(
                    "The \"--{name}\" option requires a value."
                )));
            }
            if !option.is_value_optional() {
                value = Value::Bool(true);
            }
        }
        self.options.insert(name, value);
        Ok(())
    }

    // ArgvInput::parse()

    fn parse_token(
        &mut self,
        token: &str,
        parse_options: bool,
        parsed: &mut VecDeque<String>,
    ) -> Result<bool> {
        if parse_options && token.is_empty() {
            self.parse_argument(token)?;
        } else if parse_options && token == "--" {
            return Ok(false);
        } else if parse_options && token.starts_with("--") {
            self.parse_long_option(token, parsed)?;
        } else if parse_options && token.starts_with('-') && token != "-" {
            self.parse_short_option(token, parsed)?;
        } else {
            self.parse_argument(token)?;
        }
        Ok(parse_options)
    }

    fn parse_short_option(&mut self, token: &str, parsed: &mut VecDeque<String>) -> Result<()> {
        let name = &token.as_bytes()[1..];
        if name.len() > 1 {
            let first = std::str::from_utf8(&name[..1]).ok();
            let accepts = match first {
                Some(f) if self.definition.has_shortcut(f) => {
                    self.definition.get_option_for_shortcut(f)?.accept_value()
                }
                _ => false,
            };
            if let (true, Some(f)) = (accepts, first) {
                // an option with a value (with no space)
                self.add_short_option(f, Some(PhpString::from(&name[1..])), parsed)
            } else {
                self.parse_short_option_set(name, parsed)
            }
        } else {
            self.add_short_option(&String::from_utf8_lossy(name), None, parsed)
        }
    }

    fn parse_short_option_set(&mut self, name: &[u8], parsed: &mut VecDeque<String>) -> Result<()> {
        let len = name.len();
        for i in 0..len {
            let shortcut = std::str::from_utf8(&name[i..=i])
                .ok()
                .filter(|s| self.definition.has_shortcut(s));
            let Some(shortcut) = shortcut else {
                // mb_substr($name, $i, 1): the byte index used as a character index
                let text = String::from_utf8_lossy(name);
                let shown: String = text.chars().nth(i).map(String::from).unwrap_or_default();
                return Err(ConsoleError::Runtime(format!(
                    "The \"-{shown}\" option does not exist."
                )));
            };
            let option = self.definition.get_option_for_shortcut(shortcut)?;
            let option_name = option.get_name().to_owned();
            if option.accept_value() {
                let value = if i == len - 1 {
                    None
                } else {
                    Some(PhpString::from(&name[i + 1..]))
                };
                self.add_long_option(&option_name, value, parsed)?;
                break;
            }
            self.add_long_option(&option_name, None, parsed)?;
        }
        Ok(())
    }

    fn parse_long_option(&mut self, token: &str, parsed: &mut VecDeque<String>) -> Result<()> {
        let name = &token[2..];
        if let Some(pos) = name.find('=') {
            let value = &name[pos + 1..];
            if value.is_empty() {
                parsed.push_front(value.to_owned());
            }
            self.add_long_option(&name[..pos], Some(PhpString::from(value)), parsed)
        } else {
            self.add_long_option(name, None, parsed)
        }
    }

    fn parse_argument(&mut self, token: &str) -> Result<()> {
        let c = self.arguments.len() as i64;
        if self.definition.has_argument_at(c) {
            // if input is expecting another argument, add it
            let argument = self.definition.get_argument_at(c)?;
            let value = if argument.is_array() {
                Value::Array(PhpArray::list([Value::string(token)]))
            } else {
                Value::string(token)
            };
            let name = argument.get_name().to_owned();
            self.arguments.insert(name, value);
        } else if self.definition.has_argument_at(c - 1)
            && self.definition.get_argument_at(c - 1)?.is_array()
        {
            // if last argument isArray(), append token to last argument
            let name = self
                .definition
                .get_argument_at(c - 1)?
                .get_name()
                .to_owned();
            append(&mut self.arguments, &name, Value::string(token));
        } else {
            // unexpected argument
            let mut all: Vec<&str> = self
                .definition
                .get_arguments()
                .iter()
                .map(|a| a.get_name())
                .collect();
            let mut command_name: Option<String> = None;
            if all.first() == Some(&"command") {
                command_name = self
                    .arguments
                    .get(&Key::from("command"))
                    .filter(|v| truthy(v))
                    .map(php_text);
                all.remove(0);
            }
            let message = if !all.is_empty() {
                match &command_name {
                    Some(command) => format!(
                        "Too many arguments to \"{command}\" command, expected arguments \"{}\".",
                        all.join("\" \"")
                    ),
                    None => format!(
                        "Too many arguments, expected arguments \"{}\".",
                        all.join("\" \"")
                    ),
                }
            } else if let Some(command) = &command_name {
                format!("No arguments expected for \"{command}\" command, got \"{token}\".")
            } else {
                format!("No arguments expected, got \"{token}\".")
            };
            return Err(ConsoleError::Runtime(message));
        }
        Ok(())
    }

    fn add_short_option(
        &mut self,
        shortcut: &str,
        value: Option<PhpString>,
        parsed: &mut VecDeque<String>,
    ) -> Result<()> {
        if !self.definition.has_shortcut(shortcut) {
            return Err(ConsoleError::Runtime(format!(
                "The \"-{shortcut}\" option does not exist."
            )));
        }
        let name = self
            .definition
            .get_option_for_shortcut(shortcut)?
            .get_name()
            .to_owned();
        self.add_long_option(&name, value, parsed)
    }

    fn add_long_option(
        &mut self,
        name: &str,
        value: Option<PhpString>,
        parsed: &mut VecDeque<String>,
    ) -> Result<()> {
        if !self.definition.has_option(name) {
            if !self.definition.has_negation(name) {
                return Err(ConsoleError::Runtime(format!(
                    "The \"--{name}\" option does not exist."
                )));
            }
            let option_name = self.definition.negation_to_name(name)?;
            if value.is_some() {
                return Err(ConsoleError::Runtime(format!(
                    "The \"--{name}\" option does not accept a value."
                )));
            }
            self.options.insert(option_name, Value::Bool(false));
            return Ok(());
        }
        let option = self.definition.get_option(name)?.clone();
        if value.is_some() && !option.accept_value() {
            return Err(ConsoleError::Runtime(format!(
                "The \"--{name}\" option does not accept a value."
            )));
        }
        let mut value: Value = value.map_or(Value::Null, Value::String);
        let empty = value.is_null() || value.as_string().is_some_and(|s| s.is_empty());
        if empty && option.accept_value() && !parsed.is_empty() {
            // if option accepts an optional or mandatory argument
            // let's see if there is one provided
            if let Some(next) = parsed.pop_front() {
                if (!next.is_empty() && !next.starts_with('-')) || next.is_empty() {
                    value = Value::string(next.as_str());
                } else {
                    parsed.push_front(next);
                }
            }
        }
        if value.is_null() {
            if option.is_value_required() {
                return Err(ConsoleError::Runtime(format!(
                    "The \"--{name}\" option requires a value."
                )));
            }
            if !option.is_array() && !option.is_value_optional() {
                value = Value::Bool(true);
            }
        }
        if option.is_array() {
            append(&mut self.options, name, value);
        } else {
            self.options.insert(name, value);
        }
        Ok(())
    }
}

/// `$array[$name][] = $value`.
fn append(array: &mut PhpArray, name: &str, value: Value) {
    let key = Key::from(name);
    let mut list = match array.get(&key) {
        Some(Value::Array(list)) => list.clone(),
        _ => PhpArray::new(),
    };
    let _ = list.push(value);
    array.insert(key, Value::Array(list));
}

/// `array_merge($a, $b)`: string keys are overwritten in place or appended,
/// integer keys renumbered.
pub(crate) fn array_merge(a: &PhpArray, b: &PhpArray) -> PhpArray {
    let mut out = PhpArray::new();
    for array in [a, b] {
        for (key, value) in array.iter() {
            match key {
                Key::Int(_) => {
                    let _ = out.push(value.clone());
                }
                Key::Str(_) => out.insert(key.clone(), value.clone()),
            }
        }
    }
    out
}

fn key_text(key: &Key) -> String {
    match key {
        Key::Int(i) => i.to_string(),
        Key::Str(s) => s.to_string_lossy().into_owned(),
    }
}

/// `(string) $value` for what Console prints; arrays read `Array`.
pub(crate) fn php_text(value: &Value) -> String {
    match value {
        Value::Array(_) => "Array".to_owned(),
        other => other
            .to_php_string()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// PHP's truthiness.
pub(crate) fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::String(s) => !(s.is_empty() || s.as_bytes() == b"0"),
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

/// `$value == $string` (PHP 8): a numeric string against a number or
/// another numeric string compares numerically; a bool compares
/// truthiness; null equals `''`; otherwise strings compare as bytes.
pub(crate) fn loose_equals_str(value: &Value, string: &str) -> bool {
    use php_value::numeric::{is_numeric, string_to_float};
    let s = string.as_bytes();
    match value {
        Value::Null => s.is_empty(),
        Value::Bool(b) => *b == truthy(&Value::string(string)),
        Value::Int(i) => {
            if is_numeric(s) {
                string_to_float(s) == *i as f64
            } else {
                i.to_string().as_bytes() == s
            }
        }
        Value::Float(f) => {
            if is_numeric(s) {
                string_to_float(s) == *f
            } else {
                php_text(value).as_bytes() == s
            }
        }
        Value::String(v) => {
            if is_numeric(v.as_bytes()) && is_numeric(s) {
                string_to_float(v.as_bytes()) == string_to_float(s)
            } else {
                v.as_bytes() == s
            }
        }
        Value::Array(_) | Value::Object(_) => false,
    }
}

/// `escapeshellarg()` on Linux: single quotes, with each `'` written `'\''`.
pub fn escapeshellarg(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}
