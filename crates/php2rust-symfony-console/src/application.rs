//! Port of Symfony\Component\Console\Application.php: the command
//! registry (`addCommand()`, `get()`, `has()`, `find()`, `all()`,
//! namespaces and alternatives), the default definition and helper set,
//! `setDefaultCommand()`, the name and version, and `complete()`.
//!
//! Not ported yet (M7): `run()` / `doRun()`, `renderThrowable()`,
//! `configureIO()`, command loaders, the event dispatcher, the container,
//! signals and the argument resolver. The default commands are
//! registered with their names and definitions; running one is refused
//! (`command::default_commands`).
//!
//! A command keeps a shared handle (`ApplicationHandle`) to the parts of
//! its application it reads: the definition, the helper set and whether
//! the application runs a single command. Commands are shared
//! (`SharedCommand`), as `find()` returns the object it holds.
//!
//! Command names are the keys of a PHP array there; here they stay
//! strings, so a numeric name keeps its string form.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Application.php

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::Value;

use crate::{
    command::{
        command::{Code, Command},
        default_commands,
    },
    completion::{
        completion_input::{
            CompletionInput, TYPE_ARGUMENT_VALUE, TYPE_OPTION_NAME, TYPE_OPTION_VALUE,
        },
        suggestion::{CompletionSuggestions, Suggestion},
    },
    exception::{ConsoleError, Result},
    helper::{formatter_helper::FormatterHelper, helper::Helper, helper_set::HelperSet},
    input::{
        SuggestedValues,
        input_argument::{self, InputArgument},
        input_definition::{InputDefinition, Item},
        input_option::{self, InputOption, Shortcut},
    },
    php::{levenshtein, sort_strings, strnatcasecmp},
    terminal::Terminal,
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A command as the application holds it: `find()` and `get()` return
/// this same object.
#[derive(Clone)]
pub struct SharedCommand(Arc<Mutex<Command>>);

impl SharedCommand {
    pub fn new(command: Command) -> Self {
        Self(Arc::new(Mutex::new(command)))
    }

    pub fn lock(&self) -> MutexGuard<'_, Command> {
        lock(&self.0)
    }

    /// `===`.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl From<Command> for SharedCommand {
    fn from(command: Command) -> Self {
        Self::new(command)
    }
}

impl std::fmt::Debug for SharedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.lock().fmt(f)
    }
}

struct Core {
    name: String,
    version: String,
    definition: Option<InputDefinition>,
    helper_set: Option<Arc<HelperSet>>,
    default_command: String,
    single_command: bool,
}

/// The parts of an Application a command reads.
#[derive(Clone)]
pub struct ApplicationHandle(Arc<Mutex<Core>>);

impl ApplicationHandle {
    /// `getHelperSet()`.
    pub fn get_helper_set(&self) -> Arc<HelperSet> {
        let mut core = lock(&self.0);
        Arc::clone(core.helper_set.get_or_insert_with(default_helper_set))
    }

    /// `getDefinition()`: a copy (see `with_definition()` to change it).
    pub fn get_definition(&self) -> Result<InputDefinition> {
        self.with_definition(|d| Ok(d.clone()))
    }

    /// `getDefinition()` and changes made to it.
    pub fn with_definition<R>(
        &self,
        f: impl FnOnce(&mut InputDefinition) -> Result<R>,
    ) -> Result<R> {
        let mut core = lock(&self.0);
        let default_command = core.default_command.clone();
        let single_command = core.single_command;
        let definition = match &mut core.definition {
            Some(d) => d,
            slot => slot.insert(default_input_definition(&default_command)?),
        };
        if single_command {
            definition.set_arguments(Vec::new())?;
        }
        f(definition)
    }

    /// `isSingleCommand()`.
    pub fn is_single_command(&self) -> bool {
        lock(&self.0).single_command
    }

    /// `===`.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for ApplicationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let core = lock(&self.0);
        f.debug_struct("Application")
            .field("name", &core.name)
            .field("version", &core.version)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ApplicationHandle {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

/// `Application`.
pub struct Application {
    handle: ApplicationHandle,
    commands: Vec<(String, SharedCommand)>,
    initialized: bool,
    terminal: Terminal,
}

impl Default for Application {
    fn default() -> Self {
        Self::new("UNKNOWN", "UNKNOWN")
    }
}

impl Application {
    /// `new Application($name, $version)`.
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            handle: ApplicationHandle(Arc::new(Mutex::new(Core {
                name: name.to_owned(),
                version: version.to_owned(),
                definition: None,
                helper_set: None,
                default_command: "list".to_owned(),
                single_command: false,
            }))),
            commands: Vec::new(),
            initialized: false,
            terminal: Terminal,
        }
    }

    /// What a command's `getApplication()` returns.
    pub fn handle(&self) -> &ApplicationHandle {
        &self.handle
    }

    /// `setHelperSet($helperSet)`.
    pub fn set_helper_set(&mut self, helper_set: Arc<HelperSet>) {
        lock(&self.handle.0).helper_set = Some(helper_set);
    }

    /// `getHelperSet()`.
    pub fn get_helper_set(&self) -> Arc<HelperSet> {
        self.handle.get_helper_set()
    }

    /// `setDefinition($definition)`.
    pub fn set_definition(&mut self, definition: InputDefinition) {
        lock(&self.handle.0).definition = Some(definition);
    }

    /// `getDefinition()`: a copy.
    pub fn get_definition(&self) -> Result<InputDefinition> {
        self.handle.get_definition()
    }

    /// `getDefinition()` and changes made to it.
    pub fn with_definition<R>(
        &self,
        f: impl FnOnce(&mut InputDefinition) -> Result<R>,
    ) -> Result<R> {
        self.handle.with_definition(f)
    }

    /// `complete($input, $suggestions)`.
    pub fn complete(
        &mut self,
        input: &CompletionInput,
        suggestions: &mut CompletionSuggestions,
    ) -> Result<()> {
        if input.get_completion_type() == TYPE_ARGUMENT_VALUE
            && input.get_completion_name() == Some("command")
        {
            for (name, command) in self.all(None)? {
                let command = command.lock();
                // skip hidden commands and aliased commands as they already get added below
                if command.is_hidden() || command.get_name() != Some(name.as_str()) {
                    continue;
                }
                let description = command.get_description();
                suggestions.suggest_value(&Value::Object(Arc::new(Suggestion::new(
                    command.get_name().unwrap_or_default(),
                    description,
                ))));
                for alias in command.get_aliases() {
                    suggestions.suggest_value(&Value::Object(Arc::new(Suggestion::new(
                        alias.as_str(),
                        description,
                    ))));
                }
            }
            return Ok(());
        }

        if input.get_completion_type() == TYPE_OPTION_NAME {
            suggestions.suggest_options(self.get_definition()?.get_options().to_vec());
        }

        let definition = self.get_definition()?;
        let name = input.get_completion_name().unwrap_or_default();
        if input.get_completion_type() == TYPE_OPTION_VALUE && definition.has_option(name) {
            definition.get_option(name)?.complete(input, suggestions)?;
        }
        Ok(())
    }

    /// `getHelp()`.
    pub fn get_help(&self) -> String {
        self.get_long_version()
    }

    /// `getName()`.
    pub fn get_name(&self) -> String {
        lock(&self.handle.0).name.clone()
    }

    /// `setName($name)`.
    pub fn set_name(&mut self, name: &str) {
        name.clone_into(&mut lock(&self.handle.0).name);
    }

    /// `getVersion()`.
    pub fn get_version(&self) -> String {
        lock(&self.handle.0).version.clone()
    }

    /// `setVersion($version)`.
    pub fn set_version(&mut self, version: &str) {
        version.clone_into(&mut lock(&self.handle.0).version);
    }

    /// `getLongVersion()` (no container, so no environment part).
    pub fn get_long_version(&self) -> String {
        let (name, version) = (self.get_name(), self.get_version());
        if name == "UNKNOWN" {
            "Console Tool".to_owned()
        } else if version == "UNKNOWN" {
            name
        } else {
            format!("{name} <info>{version}</info>")
        }
    }

    /// `register($name)`.
    pub fn register(&mut self, name: &str) -> Result<Option<SharedCommand>> {
        self.add_command(Command::new(Some(name), None)?)
    }

    /// `addCommands($commands)`.
    pub fn add_commands(&mut self, commands: Vec<SharedCommand>) -> Result<()> {
        for command in commands {
            self.add_command(command)?;
        }
        Ok(())
    }

    /// `addCommand($command)` with a callable: `new Command(null, $code)`.
    pub fn add_code(&mut self, code: Code) -> Result<Option<SharedCommand>> {
        self.add_command(Command::new(None, Some(code))?)
    }

    /// `addCommand($command)`.
    pub fn add_command(
        &mut self,
        command: impl Into<SharedCommand>,
    ) -> Result<Option<SharedCommand>> {
        self.init()?;
        self.add_command_initialized(command.into())
    }

    fn add_command_initialized(&mut self, shared: SharedCommand) -> Result<Option<SharedCommand>> {
        let (name, aliases) = {
            let mut command = shared.lock();
            command.set_application(Some(self.handle.clone()));

            if !command.is_enabled() {
                command.set_application(None);
                return Ok(None);
            }

            // Will throw if the command is not correctly initialized.
            command.get_definition();

            let name = match command.get_name() {
                Some(name) if !name.is_empty() && name != "0" => name.to_owned(),
                _ => {
                    return Err(ConsoleError::Logic(format!(
                        "The command defined in \"{}\" cannot have an empty name.",
                        command.class()
                    )));
                }
            };
            (name, command.get_aliases().to_vec())
        };

        self.set(name, shared.clone());
        for alias in aliases {
            self.set(alias, shared.clone());
        }
        Ok(Some(shared))
    }

    /// `$this->commands[$name] = $command`: an existing key keeps its place.
    fn set(&mut self, name: String, command: SharedCommand) {
        match self.commands.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = command,
            None => self.commands.push((name, command)),
        }
    }

    fn lookup(&self, name: &str) -> Option<&SharedCommand> {
        self.commands
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c)
    }

    /// `get($name)`.
    pub fn get(&mut self, name: &str) -> Result<SharedCommand> {
        self.init()?;
        match self.lookup(name) {
            Some(command) => Ok(command.clone()),
            None => Err(ConsoleError::CommandNotFound(
                format!("The command \"{name}\" does not exist."),
                Vec::new(),
            )),
        }
    }

    /// `has($name)`.
    pub fn has(&mut self, name: &str) -> Result<bool> {
        self.init()?;
        Ok(self.lookup(name).is_some())
    }

    /// `getNamespaces()`.
    pub fn get_namespaces(&mut self) -> Result<Vec<String>> {
        let mut namespaces: Vec<String> = Vec::new();
        for (_, command) in self.all(None)? {
            let command = command.lock();
            if command.is_hidden() {
                continue;
            }
            let names = std::iter::once(command.get_name().unwrap_or_default().to_owned())
                .chain(command.get_aliases().iter().cloned());
            for name in names {
                for namespace in extract_all_namespaces(&name) {
                    // array_unique(array_filter(...)): "" and "0" dropped
                    if !namespace.is_empty() && namespace != "0" && !namespaces.contains(&namespace)
                    {
                        namespaces.push(namespace);
                    }
                }
            }
        }
        Ok(namespaces)
    }

    /// `findNamespace($namespace)`.
    pub fn find_namespace(&mut self, namespace: &str) -> Result<String> {
        let all_namespaces = self.get_namespaces()?;
        let mut namespaces: Vec<String> = all_namespaces
            .iter()
            .filter(|n| abbreviation_matches(namespace, n, false, false))
            .cloned()
            .collect();

        if namespaces.is_empty() {
            let mut message =
                format!("There are no commands defined in the \"{namespace}\" namespace.");
            let alternatives = find_alternatives(namespace, &all_namespaces);
            if !alternatives.is_empty() {
                if alternatives.len() == 1 {
                    message.push_str("\n\nDid you mean this?\n    ");
                } else {
                    message.push_str("\n\nDid you mean one of these?\n    ");
                }
                message.push_str(&alternatives.join("\n    "));
            }
            return Err(ConsoleError::NamespaceNotFound(message, alternatives));
        }

        let exact = namespaces.iter().any(|n| n == namespace);
        if namespaces.len() > 1 && !exact {
            sort_strings(&mut namespaces);
            return Err(ConsoleError::NamespaceNotFound(
                format!(
                    "The namespace \"{namespace}\" is ambiguous.\nDid you mean one of these?\n{}.",
                    abbreviation_suggestions(&namespaces)
                ),
                namespaces,
            ));
        }

        Ok(if exact {
            namespace.to_owned()
        } else {
            namespaces.swap_remove(0)
        })
    }

    /// `find($name)`.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    #[expect(clippy::too_many_lines, reason = "one function, as upstream's")]
    pub fn find(&mut self, name: &str) -> Result<SharedCommand> {
        self.init()?;

        let registered: Vec<SharedCommand> = self.commands.iter().map(|(_, c)| c.clone()).collect();
        for command in registered {
            let aliases = command.lock().get_aliases().to_vec();
            for alias in aliases {
                if self.lookup(&alias).is_none() {
                    self.set(alias, command.clone());
                }
            }
        }

        if let Some(command) = self.lookup(name) {
            return Ok(command.clone());
        }

        let all_commands: Vec<String> = self.commands.iter().map(|(n, _)| n.clone()).collect();
        let mut commands: Vec<String> = all_commands
            .iter()
            .filter(|c| abbreviation_matches(name, c, false, false))
            .cloned()
            .collect();
        if commands.is_empty() {
            commands = all_commands
                .iter()
                .filter(|c| abbreviation_matches(name, c, true, false))
                .cloned()
                .collect();
        }

        sort_strings(&mut commands);

        // if no commands matched or we just matched namespaces
        if commands.is_empty()
            || !commands
                .iter()
                .any(|c| abbreviation_matches(name, c, true, true))
        {
            if let Some(pos) = name.rfind(':') {
                // check if a namespace exists and contains commands
                self.find_namespace(&name[..pos])?;
            }

            let mut message = format!("Command \"{name}\" is not defined.");

            let mut alternatives = find_alternatives(name, &all_commands);
            if !alternatives.is_empty() {
                // remove hidden commands
                alternatives.retain(|n| self.lookup(n).is_some_and(|c| !c.lock().is_hidden()));
                if !alternatives.is_empty() {
                    let _ = write!(
                        message,
                        "\n\nDid you mean {}?\n    {}",
                        if alternatives.len() == 1 {
                            "this"
                        } else {
                            "one of these"
                        },
                        alternatives.join("\n    ")
                    );
                }
            }

            return Err(ConsoleError::CommandNotFound(message, alternatives));
        }

        let command_name = |app: &Self, name_or_alias: &str| {
            app.lookup(name_or_alias)
                .and_then(|c| c.lock().get_name().map(str::to_owned))
                .unwrap_or_default()
        };

        // filter out aliases for commands which are already on the list
        if commands.len() > 1 {
            let snapshot = commands.clone();
            let mut kept: Vec<String> = Vec::new();
            for name_or_alias in &snapshot {
                let name = command_name(self, name_or_alias);
                let keep = name == *name_or_alias || !snapshot.contains(&name);
                if keep && !kept.contains(name_or_alias) {
                    kept.push(name_or_alias.clone());
                }
            }
            commands = kept;
        }

        // check whether all commands left are aliases to the same one
        if commands.len() > 1 {
            let mut unique: Vec<String> = Vec::new();
            for name_or_alias in &commands {
                let name = command_name(self, name_or_alias);
                if !unique.contains(&name) {
                    unique.push(name);
                }
            }
            if unique.len() == 1 {
                commands = unique;
            }
        }

        if commands.len() > 1 {
            sort_strings(&mut commands);
            let usable_width = self.terminal.get_width() - 10;
            let max_len = commands.iter().map(|c| Helper::width(c)).max().unwrap_or(0);
            let mut abbrevs: Vec<String> = Vec::new();
            let mut visible: Vec<String> = Vec::new();
            for cmd in &commands {
                let Some(command) = self.lookup(cmd) else {
                    continue;
                };
                let command = command.lock();
                if command.is_hidden() {
                    continue;
                }
                visible.push(cmd.clone());
                // str_pad() pads to a length in bytes
                let pad = max_len.saturating_sub(cmd.len());
                let abbrev = format!("{cmd}{} {}", " ".repeat(pad), command.get_description());
                abbrevs.push(if Helper::width(&abbrev) as i64 > usable_width {
                    format!("{}...", Helper::substr(&abbrev, 0, Some(usable_width - 3)))
                } else {
                    abbrev
                });
            }
            commands = visible;

            if commands.len() > 1 {
                return Err(ConsoleError::CommandNotFound(
                    format!(
                        "Command \"{name}\" is ambiguous.\nDid you mean one of these?\n{}",
                        abbreviation_suggestions(&abbrevs)
                    ),
                    commands,
                ));
            }
        }

        let command = match commands.first() {
            Some(first) => Some(self.get(first)?),
            None => None,
        };
        match command {
            Some(command) if !command.lock().is_hidden() => Ok(command),
            _ => Err(ConsoleError::CommandNotFound(
                format!("The command \"{name}\" does not exist."),
                Vec::new(),
            )),
        }
    }

    /// `all($namespace)`: names (and aliases) with their commands.
    pub fn all(&mut self, namespace: Option<&str>) -> Result<Vec<(String, SharedCommand)>> {
        self.init()?;
        let Some(namespace) = namespace else {
            return Ok(self.commands.clone());
        };
        let limit = namespace.matches(':').count() + 1;
        Ok(self
            .commands
            .iter()
            .filter(|(name, _)| extract_namespace(name, Some(limit)) == namespace)
            .cloned()
            .collect())
    }

    /// `getAbbreviations($names)`.
    pub fn get_abbreviations(names: &[&str]) -> Vec<(String, Vec<String>)> {
        let mut abbrevs: Vec<(String, Vec<String>)> = Vec::new();
        for name in names {
            for len in (1..=name.len()).rev() {
                // substr() cuts bytes
                let abbrev = String::from_utf8_lossy(&name.as_bytes()[..len]).into_owned();
                match abbrevs.iter_mut().find(|(a, _)| *a == abbrev) {
                    Some((_, list)) => list.push((*name).to_owned()),
                    None => abbrevs.push((abbrev, vec![(*name).to_owned()])),
                }
            }
        }
        abbrevs
    }

    /// `extractNamespace($name, $limit)`.
    pub fn extract_namespace(&self, name: &str, limit: Option<usize>) -> String {
        extract_namespace(name, limit)
    }

    /// `setDefaultCommand($commandName, $isSingleCommand)`.
    pub fn set_default_command(
        &mut self,
        command_name: &str,
        is_single_command: bool,
    ) -> Result<&mut Self> {
        let default = command_name
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or_default()
            .to_owned();
        lock(&self.handle.0).default_command = default;

        if is_single_command {
            // Ensure the command exist
            self.find(command_name)?;
            lock(&self.handle.0).single_command = true;
        }
        Ok(self)
    }

    /// The default command's name (PHP's private `$defaultCommand`).
    pub fn default_command(&self) -> String {
        lock(&self.handle.0).default_command.clone()
    }

    /// `isSingleCommand()`.
    pub fn is_single_command(&self) -> bool {
        self.handle.is_single_command()
    }

    fn init(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        for command in default_commands::all()? {
            self.add_command_initialized(command.into())?;
        }
        Ok(())
    }
}

/// `getDefaultInputDefinition()`.
fn default_input_definition(default_command: &str) -> Result<InputDefinition> {
    let flag = |name: &str, shortcut: Shortcut<'_>, description: &str| -> Result<Item> {
        Ok(InputOption::new(
            name,
            shortcut,
            Some(input_option::VALUE_NONE),
            description,
            Value::Null,
            SuggestedValues::None,
        )?
        .into())
    };
    InputDefinition::new(vec![
        InputArgument::new(
            "command",
            Some(input_argument::REQUIRED),
            "The command to execute",
            Value::Null,
            SuggestedValues::None,
        )?
        .into(),
        flag(
            "--help",
            Shortcut::One("-h"),
            &format!(
                "Display help for the given command. When no command is given display help for the <info>{default_command}</info> command"
            ),
        )?,
        flag("--silent", Shortcut::None, "Do not output any message")?,
        flag(
            "--quiet",
            Shortcut::One("-q"),
            "Only errors are displayed. All other output is suppressed",
        )?,
        flag(
            "--verbose",
            Shortcut::One("-v|vv|vvv"),
            "Increase the verbosity of messages: 1 for normal output, 2 for more verbose output and 3 for debug",
        )?,
        flag(
            "--version",
            Shortcut::One("-V"),
            "Display this application version",
        )?,
        InputOption::new(
            "--ansi",
            Shortcut::One(""),
            Some(input_option::VALUE_NEGATABLE),
            "Force (or disable --no-ansi) ANSI output",
            Value::Null,
            SuggestedValues::None,
        )?
        .into(),
        flag(
            "--no-interaction",
            Shortcut::One("-n"),
            "Do not ask any interactive question",
        )?,
    ])
}

/// `getDefaultHelperSet()`: the formatter helper. PHP's set also has
/// `DebugFormatterHelper`, `ProcessHelper` and `QuestionHelper` (M7).
fn default_helper_set() -> Arc<HelperSet> {
    Arc::new(HelperSet::new(vec![(None, Arc::new(FormatterHelper))]))
}

/// `getAbbreviationSuggestions($abbrevs)`.
fn abbreviation_suggestions(abbrevs: &[String]) -> String {
    format!("    {}", abbrevs.join("\n    "))
}

/// `explode(':', $name, -1)`: the parts before the last colon.
fn namespace_parts(name: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = name.split(':').collect();
    parts.pop();
    parts
}

/// `extractNamespace($name, $limit)`.
fn extract_namespace(name: &str, limit: Option<usize>) -> String {
    let parts = namespace_parts(name);
    let parts = match limit {
        Some(limit) => &parts[..limit.min(parts.len())],
        None => &parts[..],
    };
    parts.join(":")
}

/// `extractAllNamespaces($name)`.
fn extract_all_namespaces(name: &str) -> Vec<String> {
    let mut namespaces: Vec<String> = Vec::new();
    for part in namespace_parts(name) {
        let namespace = match namespaces.last() {
            Some(last) => format!("{last}:{part}"),
            None => part.to_owned(),
        };
        namespaces.push(namespace);
    }
    namespaces
}

/// `preg_match('{^'.$expr.'}', $candidate)` where `$expr` is
/// `implode('[^:]*:', array_map('preg_quote', explode(':', $name))).'[^:]*'`:
/// each colon-separated part of the name starts the candidate's part at
/// the same place. With `full` (`$` added), the candidate has no more
/// parts than the name. `caseless` is `/i` (ASCII, no `/u`).
fn abbreviation_matches(name: &str, candidate: &str, caseless: bool, full: bool) -> bool {
    let wanted: Vec<&str> = name.split(':').collect();
    let parts: Vec<&str> = candidate.split(':').collect();
    if parts.len() < wanted.len() || (full && parts.len() != wanted.len()) {
        return false;
    }
    wanted.iter().zip(&parts).all(|(w, p)| {
        let (w, p) = (w.as_bytes(), p.as_bytes());
        p.len() >= w.len()
            && if caseless {
                p[..w.len()].eq_ignore_ascii_case(w)
            } else {
                &p[..w.len()] == w
            }
    })
}

/// `findAlternatives($name, $collection)`.
#[expect(
    clippy::cast_precision_loss,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
fn find_alternatives(name: &str, collection: &[String]) -> Vec<String> {
    const THRESHOLD: f64 = 1e3;
    fn entry<'a>(alternatives: &'a mut [(String, f64)], name: &str) -> Option<&'a mut f64> {
        alternatives
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, lev)| lev)
    }

    let mut alternatives: Vec<(String, f64)> = Vec::new();

    let collection_parts: Vec<(&String, Vec<&str>)> = collection
        .iter()
        .map(|item| (item, item.split(':').collect()))
        .collect();

    for (i, subname) in name.split(':').enumerate() {
        for (collection_name, parts) in &collection_parts {
            let exists = entry(&mut alternatives, collection_name).is_some();
            let Some(part) = parts.get(i) else {
                if let Some(lev) = entry(&mut alternatives, collection_name) {
                    *lev += THRESHOLD;
                }
                continue;
            };

            let lev = levenshtein(subname.as_bytes(), part.as_bytes()) as f64;
            if lev <= subname.len() as f64 / 3.0 || (!subname.is_empty() && part.contains(subname))
            {
                match entry(&mut alternatives, collection_name) {
                    Some(existing) => *existing += lev,
                    None => alternatives.push(((*collection_name).clone(), lev)),
                }
            } else if exists && let Some(existing) = entry(&mut alternatives, collection_name) {
                *existing += THRESHOLD;
            }
        }
    }

    for item in collection {
        let lev = levenshtein(name.as_bytes(), item.as_bytes()) as f64;
        if lev <= name.len() as f64 / 3.0 || item.contains(name) {
            match entry(&mut alternatives, item) {
                Some(existing) => *existing -= lev,
                None => alternatives.push((item.clone(), lev)),
            }
        }
    }

    alternatives.retain(|(_, lev)| *lev < 2.0 * THRESHOLD);
    alternatives.sort_by(|(a, _), (b, _)| strnatcasecmp(a.as_bytes(), b.as_bytes()));
    alternatives.into_iter().map(|(n, _)| n).collect()
}

#[cfg(test)]
mod tests {
    use super::abbreviation_matches;

    #[test]
    fn abbreviations() {
        assert!(abbreviation_matches("f", "foo:bar", false, false));
        assert!(abbreviation_matches("f:b", "foo:bar", false, false));
        assert!(abbreviation_matches("f:b", "foo:bar:baz", false, false));
        assert!(!abbreviation_matches("f:b", "foo:bar:baz", false, true));
        assert!(!abbreviation_matches("foo:bar", "foo", false, false));
        assert!(abbreviation_matches("FOO", "foo", true, true));
        assert!(!abbreviation_matches("FOO", "foo", false, false));
        assert!(abbreviation_matches("", "foo", false, false));
    }
}
