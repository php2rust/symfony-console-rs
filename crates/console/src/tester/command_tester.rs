//! Port of Symfony\Component\Console\Tester\CommandTester.php and the
//! parts of Tester/TesterTrait.php it uses with `execute()`.
//!
//! The output is a `StreamOutput` over memory (`php://memory`). Not ported
//! (M7): `setInputs()` and the input stream (they feed `QuestionHelper`),
//! `capture_stderr_separately` (`ConsoleOutput`), `run()` with
//! `ExecutionResult`, and the PHPUnit constraints.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Tester/CommandTester.php Tester/TesterTrait.php

use php_value::{Key, PhpArray, Value};

use crate::{
    application::SharedCommand,
    command::command::{Code, Command},
    exception::{ConsoleError, Result},
    formatter::output_formatter::OutputFormatter,
    input::input::Input,
    output::output::{OutputInterface, StreamOutput, VERBOSITY_NORMAL},
};

/// `execute()`'s options.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecuteOptions {
    /// `interactive`.
    pub interactive: Option<bool>,
    /// `decorated`.
    pub decorated: Option<bool>,
    /// `verbosity`.
    pub verbosity: Option<i64>,
}

/// `CommandTester`.
pub struct CommandTester {
    command: SharedCommand,
    interactive: Option<bool>,
    decorated: bool,
    verbosity: i64,
    output: Option<StreamOutput<Vec<u8>>>,
    input: Option<Input>,
    status_code: Option<i64>,
}

impl CommandTester {
    /// `new CommandTester($command)`.
    pub fn new(command: impl Into<SharedCommand>) -> Self {
        Self {
            command: command.into(),
            interactive: None,
            decorated: false,
            verbosity: VERBOSITY_NORMAL,
            output: None,
            input: None,
            status_code: None,
        }
    }

    /// `new CommandTester($callable)`: `new Command(null, $callable)`.
    pub fn from_code(code: Code) -> Result<Self> {
        Ok(Self::new(Command::new(None, Some(code))?))
    }

    pub fn command(&self) -> &SharedCommand {
        &self.command
    }

    /// `setInteractive($interactive)`.
    pub fn set_interactive(&mut self, interactive: bool) {
        self.interactive = Some(interactive);
    }

    /// `setDecorated($decorated)`.
    pub fn set_decorated(&mut self, decorated: bool) {
        self.decorated = decorated;
    }

    /// `setVerbosity($level)`.
    pub fn set_verbosity(&mut self, level: i64) {
        self.verbosity = level;
    }

    /// `execute($input, $options)`.
    pub fn execute(&mut self, input: PhpArray, options: ExecuteOptions) -> Result<i64> {
        let mut input = self.create_input(input, options.interactive.or(self.interactive))?;

        // initOutput(): a StreamOutput over php://memory, then
        // setDecorated() and setVerbosity() when given.
        let mut output = StreamOutput::new(
            Vec::new(),
            false,
            VERBOSITY_NORMAL,
            None,
            Some(OutputFormatter::default()),
        );
        output.set_decorated(options.decorated.unwrap_or(self.decorated));
        if let Some(verbosity) = options.verbosity {
            output.set_verbosity(verbosity);
        }

        let result = self.command.lock().run(&mut input, &mut output);
        self.output = Some(output);
        self.input = Some(input);
        let status = result?;
        self.status_code = Some(status);
        Ok(status)
    }

    /// `createInput($input, [], $interactive)`.
    fn create_input(&self, mut parameters: PhpArray, interactive: Option<bool>) -> Result<Input> {
        let (name, app_has_command) = {
            let command = self.command.lock();
            let app_has_command = match command.get_application() {
                Some(app) => app.with_definition(|d| Ok(d.has_argument("command")))?,
                None => false,
            };
            (command.get_name().map(str::to_owned), app_has_command)
        };
        // isset($input['command']): a null value counts as unset
        let has_command = parameters
            .get(&Key::from("command"))
            .is_some_and(|v| !v.is_null());
        if !has_command && app_has_command {
            let mut merged = PhpArray::new();
            merged.insert("command", name.map_or(Value::Null, Value::from));
            // array_merge(): string keys overwrite, integer keys renumber
            for (key, value) in parameters.iter() {
                match key {
                    Key::Int(_) => {
                        let _ = merged.push(value.clone());
                    }
                    Key::Str(_) => merged.insert(key.clone(), value.clone()),
                }
            }
            parameters = merged;
        }

        let mut input = Input::array(parameters, None)?;
        if let Some(interactive) = interactive {
            input.set_interactive(interactive);
        }
        Ok(input)
    }

    /// `getDisplay($normalize)`. `PHP_EOL` is `"\n"`, so normalizing
    /// changes nothing.
    pub fn get_display(&mut self, _normalize: bool) -> Result<String> {
        let output = self.output.as_mut().ok_or_else(|| ConsoleError::Other {
            class: "RuntimeException".to_owned(),
            message:
                "Output not initialized, did you execute the command before requesting the display?"
                    .to_owned(),
        })?;
        Ok(String::from_utf8_lossy(output.get_stream()).into_owned())
    }

    /// `getInput()`.
    pub fn get_input(&self) -> Option<&Input> {
        self.input.as_ref()
    }

    /// `getOutput()`.
    pub fn get_output(&mut self) -> Option<&mut StreamOutput<Vec<u8>>> {
        self.output.as_mut()
    }

    /// `getStatusCode()`.
    pub fn get_status_code(&self) -> Result<i64> {
        self.status_code.ok_or_else(|| ConsoleError::Other {
            class: "RuntimeException".to_owned(),
            message: "Status code not initialized, did you execute the command before requesting the status code?".to_owned(),
        })
    }
}
