//! The commands every Application registers (`getDefaultCommands()`):
//! `help`, `list`, `_complete` and `completion`, with the names,
//! definitions, descriptions and help of HelpCommand.php, ListCommand.php,
//! CompleteCommand.php and DumpCompletionCommand.php, so that `find()`,
//! `all()` and the namespaces see what PHP's do.
//!
//! Running one is refused: they need the descriptors, the completion
//! outputs and Process, which are M7 work. Their arguments' and options'
//! suggestion closures (which read the application) are not ported either.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Command/HelpCommand.php Command/ListCommand.php Command/CompleteCommand.php Command/DumpCompletionCommand.php

use php_value::Value;

use super::command::{Command, CommandHandler};
use crate::{
    attribute::as_command::AsCommand,
    env::server,
    exception::{ConsoleError, Result},
    input::{
        SuggestedValues,
        input::Input,
        input_argument::{self, InputArgument},
        input_option::{self, InputOption, Shortcut},
    },
    output::output::OutputInterface,
};

/// `getDefaultCommands()`.
pub fn all() -> Result<Vec<Command>> {
    Ok(vec![
        Command::with_handler(HelpCommand, None)?,
        Command::with_handler(ListCommand, None)?,
        Command::with_handler(CompleteCommand, None)?,
        Command::with_handler(DumpCompletionCommand, None)?,
    ])
}

fn not_ported(command: &Command) -> ConsoleError {
    ConsoleError::Logic(format!(
        "The \"{}\" command is not available in this port of symfony/console yet.",
        command.get_name().unwrap_or_default()
    ))
}

fn option(name: &str, mode: i64, description: &str, default: Value) -> Result<InputOption> {
    InputOption::new(
        name,
        Shortcut::None,
        Some(mode),
        description,
        default,
        SuggestedValues::None,
    )
}

/// `HelpCommand`.
pub struct HelpCommand;

impl CommandHandler for HelpCommand {
    fn class(&self) -> &str {
        "Symfony\\Component\\Console\\Command\\HelpCommand"
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command.ignore_validation_errors();
        command.set_name("help")?;
        command.set_definition_items(vec![
            InputArgument::new(
                "command_name",
                Some(input_argument::OPTIONAL),
                "The command name",
                "help".into(),
                SuggestedValues::None,
            )?
            .into(),
            option(
                "format",
                input_option::VALUE_REQUIRED,
                "The output format (txt, xml, json, or md)",
                "txt".into(),
            )?
            .into(),
            option(
                "raw",
                input_option::VALUE_NONE,
                "To output raw command help",
                Value::Null,
            )?
            .into(),
        ])?;
        command.set_description("Display help for a command");
        command.set_help(concat!(
            "The <info>%command.name%</info> command displays help for a given command:\n",
            "\n",
            "  <info>%command.full_name% list</info>\n",
            "\n",
            "You can also output the help in other formats by using the <info>--format</info> option:\n",
            "\n",
            "  <info>%command.full_name% --format=xml list</info>\n",
            "\n",
            "To display the list of available commands, please use the <info>list</info> command.",
        ));
        Ok(())
    }

    fn execute(
        &mut self,
        command: &mut Command,
        _: &mut Input,
        _: &mut dyn OutputInterface,
    ) -> Result<i64> {
        Err(not_ported(command))
    }
}

/// `ListCommand`.
pub struct ListCommand;

impl CommandHandler for ListCommand {
    fn class(&self) -> &str {
        "Symfony\\Component\\Console\\Command\\ListCommand"
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command.set_name("list")?;
        command.set_definition_items(vec![
            InputArgument::new(
                "namespace",
                Some(input_argument::OPTIONAL),
                "The namespace name",
                Value::Null,
                SuggestedValues::None,
            )?
            .into(),
            option(
                "raw",
                input_option::VALUE_NONE,
                "To output raw command list",
                Value::Null,
            )?
            .into(),
            option(
                "format",
                input_option::VALUE_REQUIRED,
                "The output format (txt, xml, json, or md)",
                "txt".into(),
            )?
            .into(),
            option(
                "short",
                input_option::VALUE_NONE,
                "To skip describing commands' arguments",
                Value::Null,
            )?
            .into(),
        ])?;
        command.set_description("List commands");
        command.set_help(concat!(
            "The <info>%command.name%</info> command lists all commands:\n",
            "\n",
            "  <info>%command.full_name%</info>\n",
            "\n",
            "You can also display the commands for a specific namespace:\n",
            "\n",
            "  <info>%command.full_name% test</info>\n",
            "\n",
            "You can also output the information in other formats by using the <info>--format</info> option:\n",
            "\n",
            "  <info>%command.full_name% --format=xml</info>\n",
            "\n",
            "It's also possible to get raw list of commands (useful for embedding command runner):\n",
            "\n",
            "  <info>%command.full_name% --raw</info>",
        ));
        Ok(())
    }

    fn execute(
        &mut self,
        command: &mut Command,
        _: &mut Input,
        _: &mut dyn OutputInterface,
    ) -> Result<i64> {
        Err(not_ported(command))
    }
}

/// `CompleteCommand`, with its default completion outputs.
pub struct CompleteCommand;

impl CommandHandler for CompleteCommand {
    fn class(&self) -> &str {
        "Symfony\\Component\\Console\\Command\\CompleteCommand"
    }

    fn attribute(&self) -> Option<AsCommand> {
        Some(AsCommand::with(
            "|_complete",
            Some("Internal command to provide shell completion suggestions"),
            &[],
            false,
            None,
            &[],
        ))
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        let required = Some(input_option::VALUE_REQUIRED);
        command.add_option(
            "shell",
            Shortcut::One("s"),
            required,
            "The shell type (\"bash\", \"fish\", \"zsh\")",
            Value::Null,
            SuggestedValues::None,
        )?;
        command.add_option(
            "input",
            Shortcut::One("i"),
            Some(input_option::VALUE_REQUIRED | input_option::VALUE_IS_ARRAY),
            "An array of input tokens (e.g. COMP_WORDS or argv)",
            Value::Null,
            SuggestedValues::None,
        )?;
        command.add_option(
            "current",
            Shortcut::One("c"),
            required,
            "The index of the \"input\" array that the cursor is in (e.g. COMP_CWORD)",
            Value::Null,
            SuggestedValues::None,
        )?;
        command.add_option(
            "api-version",
            Shortcut::One("a"),
            required,
            "The API version of the completion script",
            Value::Null,
            SuggestedValues::None,
        )?;
        command.add_option(
            "symfony",
            Shortcut::One("S"),
            required,
            "deprecated",
            Value::Null,
            SuggestedValues::None,
        )?;
        Ok(())
    }

    fn execute(
        &mut self,
        command: &mut Command,
        _: &mut Input,
        _: &mut dyn OutputInterface,
    ) -> Result<i64> {
        Err(not_ported(command))
    }
}

/// `DumpCompletionCommand`. Its shells are the `completion.*` files that
/// symfony/console ships in `Resources/`.
pub struct DumpCompletionCommand;

const SUPPORTED_SHELLS: [&str; 3] = ["bash", "fish", "zsh"];

use php_value::path::basename_str as basename;

impl CommandHandler for DumpCompletionCommand {
    fn class(&self) -> &str {
        "Symfony\\Component\\Console\\Command\\DumpCompletionCommand"
    }

    fn attribute(&self) -> Option<AsCommand> {
        Some(AsCommand::with(
            "completion",
            Some("Dump the shell completion script"),
            &[],
            false,
            None,
            &[],
        ))
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        let full_command = server("PHP_SELF").unwrap_or_default();
        let command_name = basename(&full_command).to_owned();
        let full_command = std::fs::canonicalize(&full_command)
            .ok()
            .and_then(|p| p.to_str().map(str::to_owned))
            .filter(|p| !p.is_empty())
            .unwrap_or(full_command);

        let shell = basename(&server("SHELL").unwrap_or_default()).to_owned();
        let (rc_file, completion_file) = match shell.as_str() {
            "fish" => (
                "~/.config/fish/config.fish".to_owned(),
                format!("/etc/fish/completions/{command_name}.fish"),
            ),
            "zsh" => ("~/.zshrc".to_owned(), format!("$fpath[1]/_{command_name}")),
            _ => (
                "~/.bashrc".to_owned(),
                format!("/etc/bash_completion.d/{command_name}"),
            ),
        };
        let supported_shells = SUPPORTED_SHELLS.join(", ");

        command.set_help(&format!(
            "The <info>%command.name%</> command dumps the shell completion script required\n\
             to use shell autocompletion (currently, {supported_shells} completion are supported).\n\
             \n\
             <comment>Static installation\n\
             -------------------</>\n\
             \n\
             Dump the script to a global completion file and restart your shell:\n\
             \n\
             \x20   <info>%command.full_name% {shell} | sudo tee {completion_file}</>\n\
             \n\
             Or dump the script to a local file and source it:\n\
             \n\
             \x20   <info>%command.full_name% {shell} > completion.sh</>\n\
             \n\
             \x20   <comment># source the file whenever you use the project</>\n\
             \x20   <info>source completion.sh</>\n\
             \n\
             \x20   <comment># or add this line at the end of your \"{rc_file}\" file:</>\n\
             \x20   <info>source /path/to/completion.sh</>\n\
             \n\
             <comment>Dynamic installation\n\
             --------------------</>\n\
             \n\
             Add this to the end of your shell configuration file (e.g. <info>\"{rc_file}\"</>):\n\
             \n\
             \x20   <info>eval \"$({full_command} completion {shell})\"</>"
        ));
        command.add_argument(
            "shell",
            Some(input_argument::OPTIONAL),
            "The shell type (e.g. \"bash\"), the value of the \"$SHELL\" env var will be used if this is not given",
            Value::Null,
            SuggestedValues::list(SUPPORTED_SHELLS),
        )?;
        command.add_option(
            "debug",
            Shortcut::None,
            Some(input_option::VALUE_NONE),
            "Tail the completion debug log",
            Value::Null,
            SuggestedValues::None,
        )?;
        Ok(())
    }

    fn execute(
        &mut self,
        command: &mut Command,
        _: &mut Input,
        _: &mut dyn OutputInterface,
    ) -> Result<i64> {
        Err(not_ported(command))
    }
}
