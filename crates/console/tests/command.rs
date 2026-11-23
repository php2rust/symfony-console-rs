//! CommandTest, with upstream's names and assertions. The fixtures
//! (TestCommand, InvokableTestCommand, MethodBasedTestCommand and the
//! classes at the end of CommandTest.php) are handlers and `Code` values
//! written from them: their attributes become `AsCommand` values.
//!
//! Upstream: symfony/console Tests/Command/CommandTest.php Tests/Fixtures/TestCommand.php Tests/Fixtures/InvokableTestCommand.php Tests/Fixtures/MethodBasedTestCommand.php
#![allow(non_snake_case)]

use std::sync::Arc;

use php_value::{PhpArray, Value};
use symfony_console::{
    ConsoleError,
    application::Application,
    attribute::as_command::AsCommand,
    command::command::{Code, CodeFn, Command, CommandHandler, Method, SUCCESS},
    env::set_server,
    helper::{formatter_helper::FormatterHelper, helper_set::HelperInterface},
    input::{
        SuggestedValues,
        input::Input,
        input_argument::{InputArgument, OPTIONAL},
        input_definition::InputDefinition,
        input_option::{InputOption, Shortcut, VALUE_OPTIONAL},
    },
    output::{null_output::NullOutput, output::OutputInterface},
    tester::command_tester::{CommandTester, ExecuteOptions},
};

type R = Result<(), ConsoleError>;

/// Tests/Fixtures/TestCommand.php.
struct TestCommand;

impl CommandHandler for TestCommand {
    fn class(&self) -> &str {
        "TestCommand"
    }

    fn configure(&mut self, command: &mut Command) -> R {
        command
            .set_name("namespace:name")?
            .set_aliases(["name"])?
            .set_description("description")
            .set_help("help");
        Ok(())
    }

    fn execute(
        &mut self,
        _: &mut Command,
        _: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64, ConsoleError> {
        output.writeln("execute called")?;
        Ok(0)
    }

    fn interact(&mut self, _: &mut Command, _: &mut Input, output: &mut dyn OutputInterface) -> R {
        output.writeln("interact called")
    }
}

fn test_command() -> Result<Command, ConsoleError> {
    Command::with_handler(TestCommand, None)
}

fn no_options() -> ExecuteOptions {
    ExecuteOptions::default()
}

fn interactive(interactive: bool) -> ExecuteOptions {
    ExecuteOptions {
        interactive: Some(interactive),
        ..ExecuteOptions::default()
    }
}

fn success() -> CodeFn {
    Arc::new(|_: &mut Input, _: &mut dyn OutputInterface| Ok(SUCCESS))
}

/// Tests/Fixtures/InvokableTestCommand.php.
fn invokable_test_command() -> Code {
    Code::from(Method {
        class: "Symfony\\Component\\Console\\Tests\\Fixtures\\InvokableTestCommand".to_owned(),
        method: "__invoke".to_owned(),
        class_attribute: Some(AsCommand::with(
            "invokable:test",
            Some("desc"),
            &["inv-test"],
            false,
            Some("help me"),
            &["usage1", "usage2"],
        )),
        method_attribute: None,
        call: success(),
    })
}

/// Tests/Fixtures/MethodBasedTestCommand.php: `__invoke()` or a method
/// with its `#[AsCommand]`.
fn method_based_test_command(method: &str, attribute: Option<&str>) -> Code {
    Code::from(Method {
        class: "Symfony\\Component\\Console\\Tests\\Fixtures\\MethodBasedTestCommand".to_owned(),
        method: method.to_owned(),
        class_attribute: Some(AsCommand::new("app:cmd0")),
        method_attribute: attribute.map(AsCommand::new),
        call: success(),
    })
}

#[test]
fn testConstructor() -> R {
    let command = Command::new(Some("foo:bar"), None)?;
    assert_eq!(
        Some("foo:bar"),
        command.get_name(),
        "__construct() takes the command name as its first argument"
    );
    Ok(())
}

#[test]
fn testCommandNameCannotBeEmpty() -> R {
    let error = Application::default()
        .add_command(Command::new(None, None)?)
        .expect_err("LogicException");
    assert!(matches!(error, ConsoleError::Logic(_)));
    assert_eq!(
        "The command defined in \"Symfony\\Component\\Console\\Command\\Command\" cannot have an empty name.",
        error.message()
    );
    Ok(())
}

#[test]
fn testSetApplication() -> R {
    let application = Application::default();
    let mut command = test_command()?;
    command.set_application(Some(application.handle().clone()));
    assert!(
        command.get_application() == Some(application.handle()),
        "->setApplication() sets the current application"
    );
    assert_eq!(
        Some(&application.get_helper_set()),
        command.get_helper_set()
    );
    Ok(())
}

#[test]
fn testSetApplicationNull() -> R {
    let mut command = test_command()?;
    command.set_application(None);
    assert!(command.get_helper_set().is_none());
    Ok(())
}

#[test]
fn testSetGetDefinition() -> R {
    let mut command = test_command()?;
    let definition = InputDefinition::default();
    let ret: *const Command = command.set_definition(definition.clone());
    assert!(
        std::ptr::eq(ret, &command),
        "->setDefinition() implements a fluent interface"
    );
    assert_eq!(
        &definition,
        command.get_definition(),
        "->setDefinition() sets the current InputDefinition instance"
    );
    command.set_definition_items(vec![
        InputArgument::simple("foo", None, "")?.into(),
        InputOption::simple("bar", None, None, "")?.into(),
    ])?;
    assert!(
        command.get_definition().has_argument("foo"),
        "->setDefinition() also takes an array of InputArguments and InputOptions as an argument"
    );
    assert!(
        command.get_definition().has_option("bar"),
        "->setDefinition() also takes an array of InputArguments and InputOptions as an argument"
    );
    command.set_definition(InputDefinition::default());
    Ok(())
}

#[test]
fn testAddArgument() -> R {
    let mut command = test_command()?;
    let ret: *const Command =
        command.add_argument("foo", None, "", Value::Null, SuggestedValues::None)?;
    assert!(
        std::ptr::eq(ret, &command),
        "->addArgument() implements a fluent interface"
    );
    assert!(
        command.get_definition().has_argument("foo"),
        "->addArgument() adds an argument to the command"
    );
    Ok(())
}

#[test]
fn testAddArgumentFull() -> R {
    let mut command = test_command()?;
    command.add_argument(
        "foo",
        Some(OPTIONAL),
        "Description",
        Value::string("default"),
        SuggestedValues::list(["a", "b"]),
    )?;
    let argument = command.get_definition().get_argument("foo")?;
    assert_eq!("Description", argument.get_description());
    assert_eq!(&Value::string("default"), argument.get_default());
    assert!(argument.has_completion());
    Ok(())
}

#[test]
fn testAddOption() -> R {
    let mut command = test_command()?;
    let ret: *const Command = command.add_option(
        "foo",
        Shortcut::None,
        None,
        "",
        Value::Null,
        SuggestedValues::None,
    )?;
    assert!(
        std::ptr::eq(ret, &command),
        "->addOption() implements a fluent interface"
    );
    assert!(
        command.get_definition().has_option("foo"),
        "->addOption() adds an option to the command"
    );
    Ok(())
}

#[test]
fn testAddOptionFull() -> R {
    let mut command = test_command()?;
    command.add_option(
        "foo",
        Shortcut::Many(&["f"]),
        Some(VALUE_OPTIONAL),
        "Description",
        Value::string("default"),
        SuggestedValues::list(["a", "b"]),
    )?;
    let option = command.get_definition().get_option("foo")?;
    assert_eq!(Some("f"), option.get_shortcut());
    assert_eq!("Description", option.get_description());
    assert_eq!(&Value::string("default"), option.get_default());
    assert!(option.has_completion());
    Ok(())
}

#[test]
fn testSetHidden() -> R {
    let mut command = test_command()?;
    command.set_hidden(true);
    assert!(command.is_hidden());
    Ok(())
}

#[test]
fn testGetNamespaceGetNameSetName() -> R {
    let mut command = test_command()?;
    assert_eq!(
        Some("namespace:name"),
        command.get_name(),
        "->getName() returns the command name"
    );
    command.set_name("foo")?;
    assert_eq!(
        Some("foo"),
        command.get_name(),
        "->setName() sets the command name"
    );

    let ret: *const Command = command.set_name("foobar:bar")?;
    assert!(
        std::ptr::eq(ret, &command),
        "->setName() implements a fluent interface"
    );
    assert_eq!(
        Some("foobar:bar"),
        command.get_name(),
        "->setName() sets the command name"
    );
    Ok(())
}

fn provide_invalid_command_names() -> Vec<&'static str> {
    vec!["", "foo:"]
}

#[test]
fn testInvalidCommandNames() -> R {
    for name in provide_invalid_command_names() {
        let error = test_command()?.set_name(name).map(|_| ()).expect_err(name);
        assert!(error.is_invalid_argument());
        assert_eq!(
            format!("Command name \"{name}\" is invalid."),
            error.message()
        );
    }
    Ok(())
}

#[test]
fn testGetSetDescription() -> R {
    let mut command = test_command()?;
    assert_eq!(
        "description",
        command.get_description(),
        "->getDescription() returns the description"
    );
    let ret: *const Command = command.set_description("description1");
    assert!(
        std::ptr::eq(ret, &command),
        "->setDescription() implements a fluent interface"
    );
    assert_eq!(
        "description1",
        command.get_description(),
        "->setDescription() sets the description"
    );
    Ok(())
}

#[test]
fn testGetSetHelp() -> R {
    let mut command = test_command()?;
    assert_eq!("help", command.get_help(), "->getHelp() returns the help");
    let ret: *const Command = command.set_help("help1");
    assert!(
        std::ptr::eq(ret, &command),
        "->setHelp() implements a fluent interface"
    );
    assert_eq!("help1", command.get_help(), "->setHelp() sets the help");
    command.set_help("");
    assert_eq!(
        "",
        command.get_help(),
        "->getHelp() does not fall back to the description"
    );
    Ok(())
}

#[test]
fn testGetProcessedHelp() -> R {
    set_server("PHP_SELF", Some("bin/console"));

    let mut command = test_command()?;
    command.set_help("The %command.name% command does... Example: %command.full_name%.");
    assert!(
        command
            .get_processed_help()
            .contains("The namespace:name command does..."),
        "->getProcessedHelp() replaces %command.name% correctly"
    );
    assert!(
        !command.get_processed_help().contains("%command.full_name%"),
        "->getProcessedHelp() replaces %command.full_name%"
    );

    let mut command = test_command()?;
    command.set_help("");
    assert!(
        command.get_processed_help().contains("description"),
        "->getProcessedHelp() falls back to the description"
    );

    let mut command = test_command()?;
    command.set_help("The %command.name% command does... Example: %command.full_name%.");
    let mut application = Application::default();
    let command = application
        .add_command(command)?
        .expect("an enabled command is added");
    application.set_default_command("namespace:name", true)?;
    let help = command.lock().get_processed_help();
    assert!(
        help.contains("The namespace:name command does..."),
        "->getProcessedHelp() replaces %command.name% correctly in single command applications"
    );
    assert!(
        !help.contains("%command.full_name%"),
        "->getProcessedHelp() replaces %command.full_name% in single command applications"
    );
    Ok(())
}

#[test]
fn testGetSetAliases() -> R {
    let mut command = test_command()?;
    let ret: *const Command = command.set_aliases(["name1"])?;
    assert!(
        std::ptr::eq(ret, &command),
        "->setAliases() implements a fluent interface"
    );
    assert_eq!(
        &["name1".to_owned()],
        command.get_aliases(),
        "->setAliases() sets the aliases"
    );
    Ok(())
}

/// The anonymous class: aliases set before calling the parent constructor.
struct AliasesBeforeConstructor;

impl CommandHandler for AliasesBeforeConstructor {
    fn before_construct(&mut self, command: &mut Command) -> R {
        // set aliases before calling parent constructor
        command.set_aliases(["existingalias"])?;
        Ok(())
    }
}

#[test]
fn testAliasesSetBeforeParentConstructorArePreserved() -> R {
    let command = Command::with_handler(AliasesBeforeConstructor, Some("foo|newalias"))?;
    assert_eq!(
        &["existingalias".to_owned(), "newalias".to_owned()],
        command.get_aliases(),
        "Aliases set before parent::__construct() must be preserved."
    );
    Ok(())
}

#[test]
fn testSetAliasesAndHiddenViaName() -> R {
    for (name, expected_name, expected_aliases, expected_hidden) in [
        (
            "name|alias1|alias2",
            "name",
            vec!["alias1", "alias2"],
            false,
        ),
        ("|alias1|alias2", "alias1", vec!["alias2"], true),
    ] {
        let command = Command::new(Some(name), None)?;
        assert_eq!(Some(expected_name), command.get_name());
        assert_eq!(expected_hidden, command.is_hidden());
        assert_eq!(expected_aliases, command.get_aliases());
    }
    Ok(())
}

#[test]
fn testGetSynopsis() -> R {
    let mut command = test_command()?;
    command.add_option(
        "foo",
        Shortcut::None,
        None,
        "",
        Value::Null,
        SuggestedValues::None,
    )?;
    command.add_argument("bar", None, "", Value::Null, SuggestedValues::None)?;
    command.add_argument("info", None, "", Value::Null, SuggestedValues::None)?;
    assert_eq!(
        "namespace:name [--foo] [--] [<bar> [<info>]]",
        command.get_synopsis(false),
        "->getSynopsis() returns the synopsis"
    );
    Ok(())
}

#[test]
fn testAddGetUsages() -> R {
    let mut command = test_command()?;
    command.add_usage("foo1");
    command.add_usage("foo2");
    assert!(
        command
            .get_usages()
            .contains(&"namespace:name foo1".to_owned())
    );
    assert!(
        command
            .get_usages()
            .contains(&"namespace:name foo2".to_owned())
    );
    Ok(())
}

#[test]
fn testGetHelper() -> R {
    let application = Application::default();
    let mut command = test_command()?;
    command.set_application(Some(application.handle().clone()));
    let formatter_helper = FormatterHelper::new();
    assert_eq!(
        formatter_helper.get_name(),
        command.get_helper("formatter")?.get_name(),
        "->getHelper() returns the correct helper"
    );
    Ok(())
}

#[test]
fn testGetHelperWithoutHelperSet() -> R {
    let error = test_command()?
        .get_helper("formatter")
        .map(|_| ())
        .expect_err("LogicException");
    assert!(matches!(error, ConsoleError::Logic(_)));
    assert!(
        error.message().contains(
            "Cannot retrieve helper \"formatter\" because there is no HelperSet defined."
        )
    );
    Ok(())
}

#[test]
fn testMergeApplicationDefinition() -> R {
    let application1 = Application::default();
    application1.with_definition(|d| {
        d.add_arguments(vec![InputArgument::simple("foo", None, "")?])?;
        d.add_options(vec![InputOption::simple("bar", None, None, "")?])
    })?;
    let mut command = test_command()?;
    command.set_application(Some(application1.handle().clone()));
    command.set_definition(InputDefinition::new(vec![
        InputArgument::simple("bar", None, "")?.into(),
        InputOption::simple("foo", None, None, "")?.into(),
    ])?);

    command.merge_application_definition(true)?;
    let definition = command.get_definition();
    assert!(definition.has_argument("foo"));
    assert!(definition.has_argument("bar"));
    assert!(definition.has_option("foo"));
    assert!(definition.has_option("bar"));

    command.merge_application_definition(true)?;
    assert_eq!(
        3,
        command.get_definition().get_argument_count(),
        "->mergeApplicationDefinition() does not try to merge twice the application arguments and options"
    );
    Ok(())
}

#[test]
fn testMergeApplicationDefinitionWithoutArgsThenWithArgsAddsArgs() -> R {
    let application1 = Application::default();
    application1.with_definition(|d| {
        d.add_arguments(vec![InputArgument::simple("foo", None, "")?])?;
        d.add_options(vec![InputOption::simple("bar", None, None, "")?])
    })?;
    let mut command = test_command()?;
    command.set_application(Some(application1.handle().clone()));
    command.set_definition(InputDefinition::new(vec![])?);

    command.merge_application_definition(false)?;
    assert!(
        command.get_definition().has_option("bar"),
        "->mergeApplicationDefinition(false) merges the application and the command options"
    );
    assert!(
        !command.get_definition().has_argument("foo"),
        "->mergeApplicationDefinition(false) does not merge the application arguments"
    );

    command.merge_application_definition(true)?;
    assert!(
        command.get_definition().has_argument("foo"),
        "->mergeApplicationDefinition(true) merges the application arguments and the command arguments"
    );

    command.merge_application_definition(true)?;
    assert_eq!(
        2,
        command.get_definition().get_argument_count(),
        "->mergeApplicationDefinition() does not try to merge twice the application arguments"
    );
    Ok(())
}

#[test]
fn testRunInteractive() -> R {
    let mut tester = CommandTester::new(test_command()?);
    tester.execute(PhpArray::new(), interactive(true))?;
    assert_eq!(
        "interact called\nexecute called\n",
        tester.get_display(false)?,
        "->run() calls the interact() method if the input is interactive"
    );
    Ok(())
}

#[test]
fn testInvokableCommand() -> R {
    let command = Command::new(None, Some(invokable_test_command()))?;
    assert_eq!(Some("invokable:test"), command.get_name());
    assert_eq!(&["inv-test".to_owned()], command.get_aliases());
    assert_eq!(
        &[
            "invokable:test usage1".to_owned(),
            "invokable:test usage2".to_owned()
        ],
        command.get_usages()
    );
    assert_eq!("desc", command.get_description());
    assert_eq!("help me", command.get_help());

    let mut tester = CommandTester::from_code(invokable_test_command())?;
    assert_eq!(SUCCESS, tester.execute(PhpArray::new(), no_options())?);
    Ok(())
}

#[test]
fn testRunNonInteractive() -> R {
    let mut tester = CommandTester::new(test_command()?);
    tester.execute(PhpArray::new(), interactive(false))?;
    assert_eq!(
        "execute called\n",
        tester.get_display(false)?,
        "->run() does not call the interact() method if the input is not interactive"
    );
    Ok(())
}

#[test]
fn testExecuteMethodNeedsToBeOverridden() -> R {
    let error = Command::new(Some("foo"), None)?
        .run(&mut Input::string("")?, &mut NullOutput::new())
        .expect_err("LogicException");
    assert!(matches!(error, ConsoleError::Logic(_)));
    assert_eq!(
        "You must override the execute() method in the concrete command class.",
        error.message()
    );
    Ok(())
}

#[test]
fn testRunWithInvalidOption() -> R {
    let mut tester = CommandTester::new(test_command()?);
    let mut input = PhpArray::new();
    input.insert("--bar", Value::Bool(true));
    let error = tester
        .execute(input, no_options())
        .expect_err("InvalidOptionException");
    assert!(matches!(error, ConsoleError::InvalidOption(_)));
    assert_eq!("The \"--bar\" option does not exist.", error.message());
    Ok(())
}

#[test]
fn testRunWithApplication() -> R {
    let mut command = test_command()?;
    command.set_application(Some(Application::default().handle().clone()));
    let exit_code = command.run(&mut Input::string("")?, &mut NullOutput::new())?;
    assert_eq!(0, exit_code, "->run() returns an integer exit code");
    Ok(())
}

#[test]
fn testRunReturnsAlwaysInteger() -> R {
    let mut command = test_command()?;
    assert_eq!(
        0,
        command.run(&mut Input::string("")?, &mut NullOutput::new())?
    );
    Ok(())
}

#[test]
fn testRunWithProcessTitle() -> R {
    let mut command = test_command()?;
    command.set_application(Some(Application::default().handle().clone()));
    command.set_process_title("foo");
    assert_eq!(
        0,
        command.run(&mut Input::string("")?, &mut NullOutput::new())?
    );
    // cli_get_process_title(): the process is not renamed here; the
    // title is kept (crate README, Differences).
    assert_eq!(Some("foo"), command.get_process_title());
    Ok(())
}

#[test]
fn testSetCode() -> R {
    let mut command = test_command()?;
    let ret: *const Command = command.set_code(Code::closure(|_, output| {
        output.writeln("from the code...")?;
        Ok(0)
    }));
    assert!(
        std::ptr::eq(ret, &command),
        "->setCode() implements a fluent interface"
    );
    let mut tester = CommandTester::new(command);
    tester.execute(PhpArray::new(), no_options())?;
    assert_eq!(
        "interact called\nfrom the code...\n",
        tester.get_display(false)?
    );
    Ok(())
}

#[test]
fn testSetCodeWithNonClosureCallable() -> R {
    // $this->callableMethodCommand(...)
    let mut command = test_command()?;
    let ret: *const Command = command.set_code(Code::closure(|_, output| {
        output.writeln("from the code...")?;
        Ok(0)
    }));
    assert!(
        std::ptr::eq(ret, &command),
        "->setCode() implements a fluent interface"
    );
    let mut tester = CommandTester::new(command);
    tester.execute(PhpArray::new(), no_options())?;
    assert_eq!(
        "interact called\nfrom the code...\n",
        tester.get_display(false)?
    );
    Ok(())
}

#[test]
fn testMethodCommandNameIsPrefixedWithTheClassLevelName() -> R {
    let command = Command::new(None, Some(method_based_test_command("cmd1", Some("cmd1"))))?;
    assert_eq!(Some("app:cmd0:cmd1"), command.get_name());

    let command = Command::new(None, Some(method_based_test_command("__invoke", None)))?;
    assert_eq!(Some("app:cmd0"), command.get_name());
    Ok(())
}

#[test]
fn testMethodCommandsOfAHiddenClassLevelNameAreHidden() -> R {
    // HiddenGroupCommands::one()
    let code = Code::from(Method {
        class: "Symfony\\Component\\Console\\Tests\\Command\\HiddenGroupCommands".to_owned(),
        method: "one".to_owned(),
        class_attribute: Some(AsCommand::with("hidden-group", None, &[], true, None, &[])),
        method_attribute: Some(AsCommand::new("one")),
        call: success(),
    });
    let command = Command::new(None, Some(code))?;
    assert_eq!(Some("hidden-group:one"), command.get_name());
    assert!(command.is_hidden());
    Ok(())
}

#[test]
fn testTheAttributeCannotBeOnBothTheClassAndItsInvokeMethod() {
    // AttributeOnBothCommand
    let code = Code::from(Method {
        class: "Symfony\\Component\\Console\\Tests\\Command\\AttributeOnBothCommand".to_owned(),
        method: "__invoke".to_owned(),
        class_attribute: Some(AsCommand::new("both")),
        method_attribute: Some(AsCommand::new("both-invoke")),
        call: success(),
    });
    let error = Command::new(None, Some(code)).expect_err("LogicException");
    assert!(matches!(error, ConsoleError::Logic(_)));
    assert_eq!(
        "The \"Symfony\\Component\\Console\\Tests\\Command\\AttributeOnBothCommand\" class and its \"__invoke()\" method cannot both have the \"Symfony\\Component\\Console\\Attribute\\AsCommand\" attribute.",
        error.message()
    );
}

#[test]
fn testMethodCommandNameMustNotRepeatTheClassLevelName() {
    // RepeatedPrefixCommands::one()
    let code = Code::from(Method {
        class: "Symfony\\Component\\Console\\Tests\\Command\\RepeatedPrefixCommands".to_owned(),
        method: "one".to_owned(),
        class_attribute: Some(AsCommand::new("group")),
        method_attribute: Some(AsCommand::new("group:one")),
        call: success(),
    });
    let error = Command::new(None, Some(code)).expect_err("LogicException");
    assert!(matches!(error, ConsoleError::Logic(_)));
    assert_eq!(
        "The name \"group:one\" of the command \"Symfony\\Component\\Console\\Tests\\Command\\RepeatedPrefixCommands::one()\" repeats the class-level name \"group\": method-level names are relative to it, use \"one\" instead.",
        error.message()
    );
}

/// `Php8Command`.
struct Php8Command;

impl CommandHandler for Php8Command {
    fn class(&self) -> &str {
        "Symfony\\Component\\Console\\Tests\\Command\\Php8Command"
    }

    fn attribute(&self) -> Option<AsCommand> {
        Some(AsCommand::with(
            "foo",
            Some("desc"),
            &["f"],
            true,
            Some("help"),
            &["usage1", "usage2"],
        ))
    }
}

#[test]
fn testCommandAttribute() -> R {
    let command = Command::with_handler(Php8Command, None)?;
    assert_eq!(Some("foo"), command.get_name());
    assert_eq!("desc", command.get_description());
    assert_eq!("help", command.get_help());
    assert_eq!(2, command.get_usages().len());
    assert!(command.get_usages()[0].contains("usage1"));
    assert!(command.is_hidden());
    assert_eq!(&["f".to_owned()], command.get_aliases());
    // Standard commands don't have code.
    assert!(command.get_code().is_none());
    Ok(())
}

#[test]
fn testDefaultCommand() -> R {
    let mut apl = Application::default();
    apl.set_default_command("foo", false)?;
    assert_eq!("foo", apl.default_command());

    apl.set_default_command("foo2", false)?;
    assert_eq!("foo2", apl.default_command());
    Ok(())
}
