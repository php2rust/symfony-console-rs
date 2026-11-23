//! Port of Symfony\Component\Console\Command\Command.php, with the parts
//! of InvokableCommand.php a plain callable uses.
//!
//! PHP's subclass is a `CommandHandler`: the methods a subclass overrides
//! (`configure()`, `execute()`, `interact()`, `initialize()`,
//! `isEnabled()`, `complete()`), its class name and its `#[AsCommand]`
//! attribute, stated as a value instead of reflected. A handler method
//! receives the `Command` as PHP's `$this`.
//!
//! A callable (`new Command($name, $code)`, `setCode()`) is a `Code`: a
//! closure, or an object's method with the attributes found on it and
//! its class. Not ported (M7, with the rest of InvokableCommand):
//! parameters resolved from `#[Argument]`, `#[Option]`, `#[MapInput]`
//! and the interactive attributes, an invokable subclass of Command, and
//! signals. PHP binds an unbound closure to the command (`$this`); a Rust
//! closure captures what it uses instead.
//!
//! The process title is kept (`get_process_title()`) but the process is
//! not renamed: PHP's `cli_set_process_title()` has no safe Rust
//! equivalent.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Command/Command.php Command/InvokableCommand.php

use std::sync::Arc;

use crate::{
    application::ApplicationHandle,
    attribute::as_command::AsCommand,
    completion::{
        completion_input::{CompletionInput, TYPE_ARGUMENT_VALUE, TYPE_OPTION_VALUE},
        suggestion::CompletionSuggestions,
    },
    env::server,
    exception::{ConsoleError, Result},
    helper::helper_set::{HelperInterface, HelperSet},
    input::{
        SuggestedValues,
        input::Input,
        input_argument::InputArgument,
        input_definition::{InputDefinition, Item},
        input_option::{InputOption, Shortcut},
    },
    output::output::OutputInterface,
};

pub const SUCCESS: i64 = 0;
pub const FAILURE: i64 = 1;
pub const INVALID: i64 = 2;

/// `Command::class`.
pub const COMMAND_CLASS: &str = "Symfony\\Component\\Console\\Command\\Command";
const AS_COMMAND_CLASS: &str = "Symfony\\Component\\Console\\Attribute\\AsCommand";

/// A callable taking the input and the output and returning the exit code.
pub type CodeFn = Arc<dyn Fn(&mut Input, &mut dyn OutputInterface) -> Result<i64> + Send + Sync>;

/// A method of an object given as a command's code: `$object` (its
/// `__invoke()`) or `$object->method(...)`.
#[derive(Clone)]
pub struct Method {
    /// The object's class.
    pub class: String,
    /// The method's name; `__invoke` for an invokable object.
    pub method: String,
    /// `#[AsCommand]` on the class.
    pub class_attribute: Option<AsCommand>,
    /// `#[AsCommand]` on the method.
    pub method_attribute: Option<AsCommand>,
    pub call: CodeFn,
}

/// `callable $code`.
#[derive(Clone)]
pub enum Code {
    /// A closure (`function (...) {}`, `fn (...) =>`, static or not).
    Closure(CodeFn),
    /// A method of an object.
    Method(Box<Method>),
}

impl From<Method> for Code {
    fn from(method: Method) -> Self {
        Self::Method(Box::new(method))
    }
}

impl Code {
    pub fn closure(
        f: impl Fn(&mut Input, &mut dyn OutputInterface) -> Result<i64> + Send + Sync + 'static,
    ) -> Self {
        Self::Closure(Arc::new(f))
    }

    fn call(&self) -> &CodeFn {
        match self {
            Self::Closure(f) => f,
            Self::Method(m) => &m.call,
        }
    }
}

impl std::fmt::Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closure(_) => f.write_str("Closure"),
            Self::Method(m) => write!(f, "{}::{}()", m.class, m.method),
        }
    }
}

/// The methods a subclass of Command overrides.
pub trait CommandHandler: std::any::Any + Send {
    /// `static::class`.
    fn class(&self) -> &str {
        COMMAND_CLASS
    }

    /// `#[AsCommand]` on the class.
    fn attribute(&self) -> Option<AsCommand> {
        None
    }

    /// What the subclass's constructor does before `parent::__construct()`.
    fn before_construct(&mut self, _command: &mut Command) -> Result<()> {
        Ok(())
    }

    /// `configure()`.
    fn configure(&mut self, _command: &mut Command) -> Result<()> {
        Ok(())
    }

    /// `isEnabled()`.
    fn is_enabled(&self, _command: &Command) -> bool {
        true
    }

    /// `initialize($input, $output)`.
    fn initialize(
        &mut self,
        _command: &mut Command,
        _input: &mut Input,
        _output: &mut dyn OutputInterface,
    ) -> Result<()> {
        Ok(())
    }

    /// `interact($input, $output)`.
    fn interact(
        &mut self,
        _command: &mut Command,
        _input: &mut Input,
        _output: &mut dyn OutputInterface,
    ) -> Result<()> {
        Ok(())
    }

    /// `execute($input, $output)`.
    fn execute(
        &mut self,
        _command: &mut Command,
        _input: &mut Input,
        _output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        Err(ConsoleError::Logic(
            "You must override the execute() method in the concrete command class.".to_owned(),
        ))
    }

    /// `complete($input, $suggestions)`.
    fn complete(
        &mut self,
        command: &mut Command,
        input: &CompletionInput,
        suggestions: &mut CompletionSuggestions,
    ) -> Result<()> {
        command.complete_from_definition(input, suggestions)
    }
}

/// `new Command()` itself: no method overridden.
#[derive(Debug, Default, Clone, Copy)]
pub struct BaseCommand;

impl CommandHandler for BaseCommand {}

/// `Command`.
pub struct Command {
    handler: Option<Box<dyn CommandHandler>>,
    application: Option<ApplicationHandle>,
    name: Option<String>,
    process_title: Option<String>,
    aliases: Vec<String>,
    definition: InputDefinition,
    hidden: bool,
    help: String,
    description: String,
    full_definition: Option<InputDefinition>,
    ignore_validation_errors: bool,
    code: Option<Code>,
    synopsis_short: Option<String>,
    synopsis_long: Option<String>,
    usages: Vec<String>,
    helper_set: Option<Arc<HelperSet>>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("class", &self.class())
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .finish_non_exhaustive()
    }
}

impl Command {
    /// `new Command($name, $code)`.
    pub fn new(name: Option<&str>, code: Option<Code>) -> Result<Self> {
        Self::construct(Box::new(BaseCommand), name, code)
    }

    /// `new MyCommand($name)` for a subclass: its constructor calls
    /// `parent::__construct($name)`.
    pub fn with_handler(
        handler: impl CommandHandler + 'static,
        name: Option<&str>,
    ) -> Result<Self> {
        Self::construct(Box::new(handler), name, None)
    }

    fn construct(
        handler: Box<dyn CommandHandler>,
        name: Option<&str>,
        code: Option<Code>,
    ) -> Result<Self> {
        let mut command = Self {
            handler: Some(handler),
            application: None,
            name: None,
            process_title: None,
            aliases: Vec::new(),
            definition: InputDefinition::default(),
            hidden: false,
            help: String::new(),
            description: String::new(),
            full_definition: None,
            ignore_validation_errors: false,
            code: None,
            synopsis_short: None,
            synopsis_long: None,
            usages: Vec::new(),
            helper_set: None,
        };
        command.with_handler_mut(|h, c| h.before_construct(c))?;

        let attribute = command.get_command_attribute(code.as_ref())?;

        if let Some(code) = code {
            command.set_code(code);
        }

        let mut name = name
            .map(str::to_owned)
            .or_else(|| attribute.as_ref().map(|a| a.name.clone()));
        if let Some(full) = name.clone() {
            let mut aliases: std::collections::VecDeque<String> =
                full.split('|').map(str::to_owned).collect();
            name = aliases.pop_front();
            if name.as_deref() == Some("") {
                command.set_hidden(true);
                name = aliases.pop_front();
            }
            // we must not overwrite existing aliases, combine new ones with existing ones
            let mut combined: Vec<String> = Vec::new();
            for alias in command.aliases.iter().cloned().chain(aliases) {
                if !combined.contains(&alias) {
                    combined.push(alias);
                }
            }
            command.set_aliases(combined)?;
        }

        if let Some(name) = name {
            command.set_name(&name)?;
        }

        if command.description.is_empty() {
            let description = attribute
                .as_ref()
                .and_then(|a| a.description.clone())
                .unwrap_or_default();
            command.set_description(&description);
        }

        if command.help.is_empty() {
            let help = attribute
                .as_ref()
                .and_then(|a| a.help.clone())
                .unwrap_or_default();
            command.set_help(&help);
        }

        for usage in attribute.map(|a| a.usages).unwrap_or_default() {
            command.add_usage(&usage);
        }

        command.with_handler_mut(|h, c| h.configure(c))?;
        Ok(command)
    }

    /// Calls the handler with the command as `$this`. A handler calling
    /// back into its own command's handler methods finds none.
    fn with_handler_mut<R>(
        &mut self,
        f: impl FnOnce(&mut dyn CommandHandler, &mut Self) -> Result<R>,
    ) -> Result<R> {
        let mut handler = self.handler.take().unwrap_or_else(|| Box::new(BaseCommand));
        let result = f(handler.as_mut(), self);
        self.handler = Some(handler);
        result
    }

    /// `static::class`, as `get_debug_type($command)` names it.
    pub fn class(&self) -> &str {
        self.handler.as_ref().map_or(COMMAND_CLASS, |h| h.class())
    }

    /// The handler as its concrete type, to reach a subclass's own state.
    pub fn handler<T: CommandHandler + 'static>(&self) -> Option<&T> {
        let handler: &dyn CommandHandler = self.handler.as_deref()?;
        (handler as &dyn std::any::Any).downcast_ref()
    }

    /// `ignoreValidationErrors()`.
    pub fn ignore_validation_errors(&mut self) {
        self.ignore_validation_errors = true;
    }

    /// `setApplication($application)`.
    pub fn set_application(&mut self, application: Option<ApplicationHandle>) {
        match &application {
            Some(app) => self.set_helper_set(app.get_helper_set()),
            None => self.helper_set = None,
        }
        self.application = application;
        self.full_definition = None;
    }

    /// `setHelperSet($helperSet)`.
    pub fn set_helper_set(&mut self, helper_set: Arc<HelperSet>) {
        self.helper_set = Some(helper_set);
    }

    /// `getHelperSet()`.
    pub fn get_helper_set(&self) -> Option<&Arc<HelperSet>> {
        self.helper_set.as_ref()
    }

    /// `getApplication()`.
    pub fn get_application(&self) -> Option<&ApplicationHandle> {
        self.application.as_ref()
    }

    /// `isEnabled()`.
    pub fn is_enabled(&self) -> bool {
        self.handler.as_ref().is_none_or(|h| h.is_enabled(self))
    }

    /// `run($input, $output)`.
    pub fn run(&mut self, input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64> {
        // add the application arguments and options
        self.merge_application_definition(true)?;

        // bind the input against the command specific arguments/options
        if let Err(e) = input.bind(self.get_definition().clone())
            && (!e.is_console_exception() || !self.ignore_validation_errors)
        {
            return Err(e);
        }

        // The command name argument is often omitted when a command is executed directly with its run() method,
        // and it may hold an abbreviation or an alias when the command was resolved from one (e.g. Application::find()).
        // Normalize it to the command's actual name so it can be relied on afterwards, since it's required by the
        // application, and so argument resolution during interact() below can already rely on it.
        if input.has_argument("command")
            && let Some(name) = self.name.clone()
        {
            input.set_argument("command", name.into())?;
        }

        self.with_handler_mut(|h, c| h.initialize(c, input, output))?;

        // cli_set_process_title(): the title is kept, the process is not
        // renamed (module documentation).

        if input.is_interactive() {
            self.with_handler_mut(|h, c| h.interact(c, input, output))?;
        }

        input.validate()?;

        if let Some(code) = self.code.clone() {
            return (code.call())(input, output);
        }

        self.with_handler_mut(|h, c| h.execute(c, input, output))
    }

    /// `complete($input, $suggestions)`.
    pub fn complete(
        &mut self,
        input: &CompletionInput,
        suggestions: &mut CompletionSuggestions,
    ) -> Result<()> {
        self.with_handler_mut(|h, c| h.complete(c, input, suggestions))
    }

    /// Command's own `complete()`: what the argument or option being
    /// completed suggests.
    pub fn complete_from_definition(
        &self,
        input: &CompletionInput,
        suggestions: &mut CompletionSuggestions,
    ) -> Result<()> {
        let definition = self.get_definition();
        let name = input.get_completion_name().unwrap_or_default();
        if input.get_completion_type() == TYPE_OPTION_VALUE && definition.has_option(name) {
            definition.get_option(name)?.complete(input, suggestions)?;
        } else if input.get_completion_type() == TYPE_ARGUMENT_VALUE
            && definition.has_argument(name)
        {
            definition
                .get_argument(name)?
                .complete(input, suggestions)?;
        }
        Ok(())
    }

    /// `getCode()`.
    pub fn get_code(&self) -> Option<&Code> {
        self.code.as_ref()
    }

    /// `setCode($code)`.
    pub fn set_code(&mut self, code: Code) -> &mut Self {
        self.code = Some(code);
        self
    }

    /// `mergeApplicationDefinition($mergeArgs)`.
    pub fn merge_application_definition(&mut self, merge_args: bool) -> Result<()> {
        let Some(application) = &self.application else {
            return Ok(());
        };
        let app_definition = application.get_definition()?;

        let mut full = InputDefinition::default();
        full.set_options(self.definition.get_options().to_vec())?;
        full.add_options(app_definition.get_options().to_vec())?;

        if merge_args {
            full.set_arguments(app_definition.get_arguments().to_vec())?;
            full.add_arguments(self.definition.get_arguments().to_vec())?;
        } else {
            full.set_arguments(self.definition.get_arguments().to_vec())?;
        }
        self.full_definition = Some(full);
        Ok(())
    }

    /// `setDefinition($definition)` with an InputDefinition.
    pub fn set_definition(&mut self, definition: InputDefinition) -> &mut Self {
        self.definition = definition;
        self.full_definition = None;
        self
    }

    /// `setDefinition($definition)` with an array of arguments and options.
    pub fn set_definition_items(&mut self, items: Vec<Item>) -> Result<&mut Self> {
        self.definition.set_definition(items)?;
        self.full_definition = None;
        Ok(self)
    }

    /// `getDefinition()`.
    pub fn get_definition(&self) -> &InputDefinition {
        self.full_definition
            .as_ref()
            .unwrap_or_else(|| self.get_native_definition())
    }

    /// `getNativeDefinition()`.
    pub fn get_native_definition(&self) -> &InputDefinition {
        &self.definition
    }

    /// `addArgument($name, $mode, $description, $default, $suggestedValues)`.
    pub fn add_argument(
        &mut self,
        name: &str,
        mode: Option<i64>,
        description: &str,
        default: php_value::Value,
        suggested_values: SuggestedValues,
    ) -> Result<&mut Self> {
        let argument = InputArgument::new(name, mode, description, default, suggested_values)?;
        self.definition.add_argument(argument.clone())?;
        if let Some(full) = &mut self.full_definition {
            full.add_argument(argument)?;
        }
        Ok(self)
    }

    /// `addOption($name, $shortcut, $mode, $description, $default, $suggestedValues)`.
    pub fn add_option(
        &mut self,
        name: &str,
        shortcut: Shortcut<'_>,
        mode: Option<i64>,
        description: &str,
        default: php_value::Value,
        suggested_values: SuggestedValues,
    ) -> Result<&mut Self> {
        let option =
            InputOption::new(name, shortcut, mode, description, default, suggested_values)?;
        self.definition.add_option(option.clone())?;
        if let Some(full) = &mut self.full_definition {
            full.add_option(option)?;
        }
        Ok(self)
    }

    /// `setName($name)`.
    pub fn set_name(&mut self, name: &str) -> Result<&mut Self> {
        validate_name(name)?;
        self.name = Some(name.to_owned());
        Ok(self)
    }

    /// `setProcessTitle($title)`.
    pub fn set_process_title(&mut self, title: &str) -> &mut Self {
        self.process_title = Some(title.to_owned());
        self
    }

    /// The title `setProcessTitle()` gave.
    pub fn get_process_title(&self) -> Option<&str> {
        self.process_title.as_deref()
    }

    /// `getName()`.
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// `setHidden($hidden)`.
    pub fn set_hidden(&mut self, hidden: bool) -> &mut Self {
        self.hidden = hidden;
        self
    }

    /// `isHidden()`.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// `setDescription($description)`.
    pub fn set_description(&mut self, description: &str) -> &mut Self {
        self.description = description.to_owned();
        self
    }

    /// `getDescription()`.
    pub fn get_description(&self) -> &str {
        &self.description
    }

    /// `setHelp($help)`.
    pub fn set_help(&mut self, help: &str) -> &mut Self {
        self.help = help.to_owned();
        self
    }

    /// `getHelp()`.
    pub fn get_help(&self) -> &str {
        &self.help
    }

    /// `getProcessedHelp()`: `%command.name%` and `%command.full_name%`
    /// replaced. `$_SERVER['PHP_SELF']` comes from the `env` overlay.
    pub fn get_processed_help(&self) -> String {
        let name = self.name.clone().unwrap_or_default();
        let is_single_command = self
            .application
            .as_ref()
            .is_some_and(ApplicationHandle::is_single_command);
        let php_self = server("PHP_SELF").unwrap_or_default();
        let full_name = if is_single_command {
            php_self
        } else {
            format!("{php_self} {name}")
        };
        let text = if self.help.is_empty() {
            &self.description
        } else {
            &self.help
        };
        // str_replace() with arrays: each search over the result of the last.
        text.replace("%command.name%", &name)
            .replace("%command.full_name%", &full_name)
    }

    /// `setAliases($aliases)`.
    pub fn set_aliases<S: AsRef<str>>(
        &mut self,
        aliases: impl IntoIterator<Item = S>,
    ) -> Result<&mut Self> {
        let mut list = Vec::new();
        for alias in aliases {
            validate_name(alias.as_ref())?;
            list.push(alias.as_ref().to_owned());
        }
        self.aliases = list;
        Ok(self)
    }

    /// `getAliases()`.
    pub fn get_aliases(&self) -> &[String] {
        &self.aliases
    }

    /// `getSynopsis($short)`: computed once for each form, then kept.
    pub fn get_synopsis(&mut self, short: bool) -> String {
        let name = self.name.clone().unwrap_or_default();
        let definition = &self.definition;
        let slot = if short {
            &mut self.synopsis_short
        } else {
            &mut self.synopsis_long
        };
        slot.get_or_insert_with(|| {
            format!("{name} {}", definition.get_synopsis(short))
                .trim_matches(TRIM)
                .to_owned()
        })
        .clone()
    }

    /// `addUsage($usage)`.
    pub fn add_usage(&mut self, usage: &str) -> &mut Self {
        let name = self.name.clone().unwrap_or_default();
        let usage = if usage.starts_with(&name) {
            usage.to_owned()
        } else {
            format!("{name} {usage}")
        };
        self.usages.push(usage);
        self
    }

    /// `getUsages()`.
    pub fn get_usages(&self) -> &[String] {
        &self.usages
    }

    /// `getHelper($name)`.
    pub fn get_helper(&self, name: &str) -> Result<Arc<dyn HelperInterface>> {
        let Some(helper_set) = &self.helper_set else {
            return Err(ConsoleError::Logic(format!(
                "Cannot retrieve helper \"{name}\" because there is no HelperSet defined. Did you forget to add your command to the application or to set the application on the command using the setApplication() method? You can also set the HelperSet directly using the setHelperSet() method."
            )));
        };
        helper_set.get(name)
    }

    /// `getCommandAttribute($code)`.
    fn get_command_attribute(&self, code: Option<&Code>) -> Result<Option<AsCommand>> {
        let Some(code) = code else {
            return Ok(self.handler.as_ref().and_then(|h| h.attribute()));
        };
        let Code::Method(method) = code else {
            return Err(ConsoleError::InvalidArgument(format!(
                "The command must be an instance of \"{COMMAND_CLASS}\", an invokable object or a method of an object."
            )));
        };

        let mut attribute = method.method_attribute.clone();

        if method.method == "__invoke" {
            if attribute.is_some() && method.class_attribute.is_some() {
                return Err(ConsoleError::Logic(format!(
                    "The \"{}\" class and its \"__invoke()\" method cannot both have the \"{AS_COMMAND_CLASS}\" attribute.",
                    method.class
                )));
            }
            if attribute.is_none() {
                attribute = method.class_attribute.clone();
            }
        } else if let Some(attr) = &mut attribute
            && let Some(prefix) = method
                .class_attribute
                .as_ref()
                .map(|a| a.name.as_str())
                .filter(|p| truthy(p))
        {
            // the class-level name prefixes the names declared on methods
            let hidden = prefix.starts_with('|');
            let prefix = prefix
                .trim_start_matches('|')
                .split('|')
                .next()
                .unwrap_or_default();
            if truthy(prefix) {
                let mut names: Vec<&str> = attr.name.split('|').collect();
                if hidden && !names[0].is_empty() {
                    // the method commands of a hidden class-level command are hidden too
                    names.insert(0, "");
                }
                let mut prefixed = Vec::with_capacity(names.len());
                for name in names {
                    if name.is_empty() {
                        prefixed.push(String::new());
                        continue;
                    }
                    if let Some(rest) = name.strip_prefix(&format!("{prefix}:")) {
                        return Err(ConsoleError::Logic(format!(
                            "The name \"{name}\" of the command \"{}::{}()\" repeats the class-level name \"{prefix}\": method-level names are relative to it, use \"{rest}\" instead.",
                            method.class, method.method
                        )));
                    }
                    prefixed.push(format!("{prefix}:{name}"));
                }
                attr.name = prefixed.join("|");
            }
        }

        match attribute {
            Some(attribute) => Ok(Some(attribute)),
            None => Err(ConsoleError::Logic(format!(
                "The command must use the \"{AS_COMMAND_CLASS}\" attribute."
            ))),
        }
    }
}

/// `trim()`'s default characters.
const TRIM: &[char] = &[' ', '\t', '\n', '\r', '\0', '\x0B'];

/// A string's truthiness: `""` and `"0"` are false.
fn truthy(s: &str) -> bool {
    !s.is_empty() && s != "0"
}

/// `validateName($name)`: `/^[^\:]++(\:[^\:]++)*$/`, non-empty parts
/// separated by single colons. A newline is a part's character, and the
/// possessive runs take it, so `$` matching before a final newline
/// changes nothing.
fn validate_name(name: &str) -> Result<()> {
    if name.split(':').all(|part| !part.is_empty()) {
        Ok(())
    } else {
        Err(ConsoleError::InvalidArgument(format!(
            "Command name \"{name}\" is invalid."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::validate_name;

    #[test]
    fn name_pattern() {
        for valid in ["a", "a:b", "a:b:c", "a\n", "a:\n", "\n", "a b"] {
            assert!(validate_name(valid).is_ok(), "{valid:?}");
        }
        for invalid in ["", ":", "a:", ":a", "a::b", "a:\n:"] {
            assert!(validate_name(invalid).is_err(), "{invalid:?}");
        }
    }
}
