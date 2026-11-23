//! ApplicationTest's tests of the command registry, with upstream's names
//! and assertions. The fixture commands (Tests/Fixtures/Foo*Command.php
//! and the others the tests load) are handlers with their names, aliases,
//! descriptions and hidden flags; `instanceof` is the handler's class.
//!
//! Upstream: symfony/console Tests/ApplicationTest.php
#![allow(non_snake_case)]

use std::sync::Arc;

use symfony_console::{
    ConsoleError,
    application::{Application, SharedCommand},
    attribute::as_command::AsCommand,
    command::command::{Code, Command, CommandHandler, Method, SUCCESS},
    env::putenv,
    helper::{formatter_helper::FormatterHelper, helper_set::HelperSet},
    input::{
        input_definition::InputDefinition,
        input_option::{InputOption, VALUE_NONE},
    },
};

type R = Result<(), ConsoleError>;

/// A fixture command: `configure()` sets these.
struct Fixture {
    class: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    hidden: bool,
}

impl CommandHandler for Fixture {
    fn class(&self) -> &str {
        self.class
    }

    fn configure(&mut self, command: &mut Command) -> R {
        command.set_name(self.name)?;
        if !self.description.is_empty() {
            command.set_description(self.description);
        }
        if !self.aliases.is_empty() {
            command.set_aliases(self.aliases.iter().copied())?;
        }
        if self.hidden {
            command.set_hidden(true);
        }
        Ok(())
    }
}

fn fixture(
    class: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    hidden: bool,
) -> Command {
    Command::with_handler(
        Fixture {
            class,
            name,
            aliases,
            description,
            hidden,
        },
        None,
    )
    .unwrap_or_else(|e| panic!("fixture {class}: {e}"))
}

fn foo() -> Command {
    fixture(
        "FooCommand",
        "foo:bar",
        &["afoobar"],
        "The foo:bar command",
        false,
    )
}
fn foo1() -> Command {
    fixture(
        "Foo1Command",
        "foo:bar1",
        &["afoobar1"],
        "The foo:bar1 command",
        false,
    )
}
fn foo2() -> Command {
    fixture(
        "Foo2Command",
        "foo1:bar",
        &["afoobar2"],
        "The foo1:bar command",
        false,
    )
}
fn foo3() -> Command {
    fixture(
        "Foo3Command",
        "foo3:bar",
        &[],
        "The foo3:bar command",
        false,
    )
}
fn foo4() -> Command {
    fixture("Foo4Command", "foo3:bar:toh", &[], "", false)
}
fn foo_same_case_uppercase() -> Command {
    fixture(
        "FooSameCaseUppercaseCommand",
        "foo:BAR",
        &[],
        "foo:BAR command",
        false,
    )
}
fn foo_same_case_lowercase() -> Command {
    fixture(
        "FooSameCaseLowercaseCommand",
        "foo:bar",
        &[],
        "foo:bar command",
        false,
    )
}
fn foobar() -> Command {
    fixture(
        "FoobarCommand",
        "foobar:foo",
        &[],
        "The foobar:foo command",
        false,
    )
}
fn bar_buc() -> Command {
    fixture("BarBucCommand", "bar:buc", &[], "", false)
}
fn foo_subnamespaced1() -> Command {
    fixture(
        "FooSubnamespaced1Command",
        "foo:bar:baz",
        &["foobarbaz"],
        "The foo:bar:baz command",
        false,
    )
}
fn foo_subnamespaced2() -> Command {
    fixture(
        "FooSubnamespaced2Command",
        "foo:go:bret",
        &["foobargo"],
        "The foo:bar:go command",
        false,
    )
}
fn test_ambiguous_command_registering() -> Command {
    fixture(
        "TestAmbiguousCommandRegistering",
        "test-ambiguous",
        &["test"],
        "The test-ambiguous command",
        false,
    )
}
fn test_ambiguous_command_registering2() -> Command {
    fixture(
        "TestAmbiguousCommandRegistering2",
        "test-ambiguous2",
        &[],
        "The test-ambiguous2 command",
        false,
    )
}
fn foo_hidden() -> Command {
    fixture("FooHiddenCommand", "foo:hidden", &["afoohidden"], "", true)
}
fn many_aliases() -> Command {
    fixture(
        "ManyAliasesCommand",
        "aliased",
        &["a", "alias", "alias2", "alias3"],
        "Aliased command",
        false,
    )
}
fn alternative() -> Command {
    fixture(
        "AlternativeCommand",
        "alternative",
        &[],
        "Aliased command 2",
        false,
    )
}

fn class_of(command: &SharedCommand) -> String {
    command.lock().class().to_owned()
}

fn find_class(application: &mut Application, name: &str) -> Result<String, ConsoleError> {
    Ok(class_of(&application.find(name)?))
}

fn names(commands: &[(String, SharedCommand)]) -> Vec<&str> {
    commands.iter().map(|(n, _)| n.as_str()).collect()
}

fn add(application: &mut Application, command: Command) -> Result<SharedCommand, ConsoleError> {
    Ok(application
        .add_command(command)?
        .expect("an enabled command is added"))
}

fn find_error(application: &mut Application, name: &str) -> ConsoleError {
    match application.find(name) {
        Ok(command) => panic!("find({name:?}) returned {command:?}"),
        Err(e) => e,
    }
}

/// putenv('COLUMNS=120'), restored afterwards.
struct Columns;

impl Columns {
    fn set(value: &str) -> Self {
        putenv("COLUMNS", Some(value));
        Self
    }
}

impl Drop for Columns {
    fn drop(&mut self) {
        putenv("COLUMNS", None);
    }
}

#[test]
fn testConstructor() -> R {
    let mut application = Application::new("foo", "bar");
    assert_eq!(
        "foo",
        application.get_name(),
        "__construct() takes the application name as its first argument"
    );
    assert_eq!(
        "bar",
        application.get_version(),
        "__construct() takes the application version as its second argument"
    );
    assert_eq!(
        vec!["help", "list", "_complete", "completion"],
        names(&application.all(None)?),
        "__construct() registered the help and list commands by default"
    );
    Ok(())
}

#[test]
fn testSetGetName() {
    let mut application = Application::default();
    application.set_name("foo");
    assert_eq!(
        "foo",
        application.get_name(),
        "->setName() sets the name of the application"
    );
}

#[test]
fn testSetGetVersion() {
    let mut application = Application::default();
    application.set_version("bar");
    assert_eq!(
        "bar",
        application.get_version(),
        "->setVersion() sets the version of the application"
    );
}

#[test]
fn testGetLongVersion() {
    let application = Application::new("foo", "bar");
    assert_eq!("foo <info>bar</info>", application.get_long_version());
}

#[test]
fn testApplicationWithoutContainer() {
    let application = Application::new("foo", "bar");
    assert!(!application.get_long_version().contains("env:"));
}

#[test]
fn testHelp() {
    // Fixtures/application_gethelp.txt
    assert_eq!("Console Tool", Application::default().get_help());
}

#[test]
fn testAll() -> R {
    let mut application = Application::default();
    let commands = application.all(None)?;
    let help = commands
        .iter()
        .find(|(n, _)| n == "help")
        .map(|(_, c)| class_of(c));
    assert_eq!(
        Some("Symfony\\Component\\Console\\Command\\HelpCommand".to_owned()),
        help,
        "->all() returns the registered commands"
    );

    application.add_command(foo())?;
    let commands = application.all(Some("foo"))?;
    assert_eq!(
        1,
        commands.len(),
        "->all() takes a namespace as its first argument"
    );
    Ok(())
}

#[test]
fn testRegister() -> R {
    let mut application = Application::default();
    let command = application.register("foo")?.expect("registered");
    assert_eq!(
        Some("foo"),
        command.lock().get_name(),
        "->register() registers a new command"
    );
    Ok(())
}

#[test]
fn testAddCommand() -> R {
    let mut application = Application::default();
    let added = add(&mut application, foo())?;
    let commands = application.all(None)?;
    let registered = commands
        .iter()
        .find(|(n, _)| n == "foo:bar")
        .map(|(_, c)| c);
    assert!(
        registered.is_some_and(|c| c.ptr_eq(&added)),
        "->addCommand() registers a command"
    );

    let mut application = Application::default();
    let (first, second) = (SharedCommand::new(foo()), SharedCommand::new(foo1()));
    application.add_commands(vec![first.clone(), second.clone()])?;
    let commands = application.all(None)?;
    let get = |name: &str| {
        commands
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c.clone())
    };
    assert!(
        get("foo:bar").is_some_and(|c| c.ptr_eq(&first))
            && get("foo:bar1").is_some_and(|c| c.ptr_eq(&second)),
        "->addCommands() registers an array of commands"
    );
    Ok(())
}

#[test]
fn testAddCommandWithExtendedCommand() -> R {
    let mut application = Application::default();
    let added = add(&mut application, foo())?;
    let commands = application.all(None)?;
    assert!(
        commands
            .iter()
            .any(|(n, c)| n == "foo:bar" && c.ptr_eq(&added))
    );
    Ok(())
}

#[test]
fn testAddCommandWithInvokableCommand() -> R {
    // InvokableTestCommand
    let code = Code::from(Method {
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
        call: Arc::new(|_, _| Ok(SUCCESS)),
    });
    let mut application = Application::default();
    application.add_code(code)?;
    let command = application.get("invokable:test")?;
    let command = command.lock();
    assert_eq!(
        "Symfony\\Component\\Console\\Command\\Command",
        command.class()
    );
    assert!(
        matches!(command.get_code(), Some(Code::Method(m)) if m.class.ends_with("InvokableTestCommand"))
    );
    Ok(())
}

#[test]
fn testAddCommandThrowsExceptionOnInvalidCommand() {
    // 'a closure' (a function name such as 'strlen' has no Rust form)
    let error = Application::default()
        .add_code(Code::closure(|_, _| Ok(0)))
        .expect_err("InvalidArgumentException");
    assert!(matches!(error, ConsoleError::InvalidArgument(_)));
    assert_eq!(
        "The command must be an instance of \"Symfony\\Component\\Console\\Command\\Command\", an invokable object or a method of an object.",
        error.message()
    );

    // 'without the #[AsCommand] attribute'
    let error = Application::default()
        .add_code(Code::from(Method {
            class: "class@anonymous".to_owned(),
            method: "__invoke".to_owned(),
            class_attribute: None,
            method_attribute: None,
            call: Arc::new(|_, _| Ok(0)),
        }))
        .expect_err("LogicException");
    assert!(matches!(error, ConsoleError::Logic(_)));
    assert_eq!(
        "The command must use the \"Symfony\\Component\\Console\\Attribute\\AsCommand\" attribute.",
        error.message()
    );
}

#[test]
fn testGetInvalidCommand() {
    let error = Application::default()
        .get("foofoo")
        .expect_err("CommandNotFoundException");
    assert!(matches!(error, ConsoleError::CommandNotFound(..)));
    assert_eq!("The command \"foofoo\" does not exist.", error.message());
}

#[test]
fn testGetNamespaces() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo1())?;
    assert_eq!(vec!["foo".to_owned()], application.get_namespaces()?);
    Ok(())
}

#[test]
fn testFindNamespace() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    assert_eq!("foo", application.find_namespace("foo")?);
    assert_eq!("foo", application.find_namespace("f")?);
    application.add_command(foo2())?;
    assert_eq!("foo", application.find_namespace("foo")?);
    Ok(())
}

#[test]
fn testFindNamespaceWithSubnamespaces() -> R {
    let mut application = Application::default();
    application.add_command(foo_subnamespaced1())?;
    application.add_command(foo_subnamespaced2())?;
    assert_eq!("foo", application.find_namespace("foo")?);
    Ok(())
}

#[test]
fn testFindAmbiguousNamespace() -> R {
    let mut application = Application::default();
    application.add_command(bar_buc())?;
    application.add_command(foo())?;
    application.add_command(foo2())?;
    let error = application
        .find_namespace("f")
        .expect_err("NamespaceNotFoundException");
    assert!(matches!(error, ConsoleError::NamespaceNotFound(..)));
    assert!(error.message().contains(
        "The namespace \"f\" is ambiguous.\nDid you mean one of these?\n    foo\n    foo1"
    ));
    Ok(())
}

#[test]
fn testFindAmbiguousNamespaceSortedAlphabetically() -> R {
    let mut application = Application::default();
    for name in ["test-zzz:cmd", "test-aaa:cmd", "test-bbb:cmd"] {
        application.add_command(Command::new(Some(name), None)?)?;
    }
    let error = application
        .find_namespace("test")
        .expect_err("NamespaceNotFoundException");
    assert!(matches!(error, ConsoleError::NamespaceNotFound(..)));
    assert!(error.message().contains(
        "The namespace \"test\" is ambiguous.\nDid you mean one of these?\n    test-aaa\n    test-bbb\n    test-zzz"
    ));
    Ok(())
}

#[test]
fn testFindNonAmbiguous() -> R {
    let mut application = Application::default();
    application.add_command(test_ambiguous_command_registering())?;
    application.add_command(test_ambiguous_command_registering2())?;
    assert_eq!(
        Some("test-ambiguous".to_owned()),
        application
            .find("test")?
            .lock()
            .get_name()
            .map(str::to_owned)
    );
    Ok(())
}

#[test]
fn testFindInvalidNamespace() {
    let error = Application::default()
        .find_namespace("bar")
        .expect_err("NamespaceNotFoundException");
    assert!(matches!(error, ConsoleError::NamespaceNotFound(..)));
    assert!(
        error
            .message()
            .contains("There are no commands defined in the \"bar\" namespace.")
    );
}

#[test]
fn testFindUniqueNameButNamespaceName() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo1())?;
    application.add_command(foo2())?;
    let error = find_error(&mut application, "foo1");
    assert!(matches!(error, ConsoleError::CommandNotFound(..)));
    assert!(error.message().contains("Command \"foo1\" is not defined"));
    Ok(())
}

#[test]
fn testFind() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    assert_eq!("FooCommand", find_class(&mut application, "foo:bar")?);
    assert_eq!(
        "Symfony\\Component\\Console\\Command\\HelpCommand",
        find_class(&mut application, "h")?
    );
    assert_eq!("FooCommand", find_class(&mut application, "f:bar")?);
    assert_eq!("FooCommand", find_class(&mut application, "f:b")?);
    assert_eq!("FooCommand", find_class(&mut application, "a")?);
    Ok(())
}

#[test]
fn testFindCaseSensitiveFirst() -> R {
    let mut application = Application::default();
    application.add_command(foo_same_case_uppercase())?;
    application.add_command(foo_same_case_lowercase())?;
    assert_eq!(
        "FooSameCaseUppercaseCommand",
        find_class(&mut application, "f:B")?
    );
    assert_eq!(
        "FooSameCaseUppercaseCommand",
        find_class(&mut application, "f:BAR")?
    );
    assert_eq!(
        "FooSameCaseLowercaseCommand",
        find_class(&mut application, "f:b")?
    );
    assert_eq!(
        "FooSameCaseLowercaseCommand",
        find_class(&mut application, "f:bar")?
    );
    Ok(())
}

#[test]
fn testFindCaseInsensitiveAsFallback() -> R {
    let mut application = Application::default();
    application.add_command(foo_same_case_lowercase())?;
    assert_eq!(
        "FooSameCaseLowercaseCommand",
        find_class(&mut application, "f:b")?
    );
    assert_eq!(
        "FooSameCaseLowercaseCommand",
        find_class(&mut application, "f:B")?
    );
    assert_eq!(
        "FooSameCaseLowercaseCommand",
        find_class(&mut application, "FoO:BaR")?
    );
    Ok(())
}

#[test]
fn testFindCaseInsensitiveSuggestions() -> R {
    let mut application = Application::default();
    application.add_command(foo_same_case_lowercase())?;
    application.add_command(foo_same_case_uppercase())?;
    let error = find_error(&mut application, "FoO:BaR");
    assert!(matches!(error, ConsoleError::CommandNotFound(..)));
    assert!(error.message().contains("Command \"FoO:BaR\" is ambiguous"));
    Ok(())
}

#[test]
fn testFindSingleWithAmbiguousAliases() -> R {
    let mut application = Application::default();
    application.add_command(many_aliases())?;
    application.add_command(alternative())?;
    for name in ["a", "alias", "aliased", "ali"] {
        assert_eq!(
            "ManyAliasesCommand",
            find_class(&mut application, name)?,
            "{name}"
        );
    }
    Ok(())
}

fn provide_ambiguous_abbreviations() -> Vec<(&'static str, &'static str)> {
    vec![
        ("f", "Command \"f\" is not defined."),
        (
            "a",
            "Command \"a\" is ambiguous.\nDid you mean one of these?\n    afoobar  The foo:bar command\n    afoobar1 The foo:bar1 command\n    afoobar2 The foo1:bar command",
        ),
        (
            "foo:b",
            "Command \"foo:b\" is ambiguous.\nDid you mean one of these?\n    foo1:bar The foo1:bar command\n    foo:bar  The foo:bar command\n    foo:bar1 The foo:bar1 command",
        ),
    ]
}

#[test]
fn testFindWithAmbiguousAbbreviations() -> R {
    let _columns = Columns::set("120");
    for (abbreviation, expected_exception_message) in provide_ambiguous_abbreviations() {
        let mut application = Application::default();
        application.add_command(foo())?;
        application.add_command(foo1())?;
        application.add_command(foo2())?;
        let error = find_error(&mut application, abbreviation);
        assert!(matches!(error, ConsoleError::CommandNotFound(..)));
        assert!(
            error.message().contains(expected_exception_message),
            "{abbreviation}: {}",
            error.message()
        );
    }
    Ok(())
}

#[test]
fn testFindAmbiguousCommandsSortedAlphabetically() -> R {
    let _columns = Columns::set("120");
    let mut application = Application::default();
    // Register commands in non-alphabetical order
    for name in ["test:zzz", "test:aaa", "test:bbb"] {
        application.add_command(Command::new(Some(name), None)?)?;
    }
    let error = find_error(&mut application, "test:");
    assert!(matches!(error, ConsoleError::CommandNotFound(..)));
    let message = error.message();
    let (a, b, z) = (
        message.find("test:aaa"),
        message.find("test:bbb"),
        message.find("test:zzz"),
    );
    assert!(a < b && b < z && a.is_some(), "{message}");
    Ok(())
}

#[test]
fn testFindWithAmbiguousAbbreviationsFindsCommandIfAlternativesAreHidden() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo_hidden())?;
    assert_eq!("FooCommand", find_class(&mut application, "foo:")?);
    Ok(())
}

#[test]
fn testFindCommandEqualNamespace() -> R {
    let mut application = Application::default();
    application.add_command(foo3())?;
    application.add_command(foo4())?;
    assert_eq!("Foo3Command", find_class(&mut application, "foo3:bar")?);
    assert_eq!("Foo4Command", find_class(&mut application, "foo3:bar:toh")?);
    Ok(())
}

#[test]
fn testFindCommandWithAmbiguousNamespacesButUniqueName() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foobar())?;
    assert_eq!("FoobarCommand", find_class(&mut application, "f:f")?);
    Ok(())
}

#[test]
fn testFindCommandWithMissingNamespace() -> R {
    let mut application = Application::default();
    application.add_command(foo4())?;
    assert_eq!("Foo4Command", find_class(&mut application, "f::t")?);
    Ok(())
}

#[test]
fn testFindAlternativeExceptionMessageSingle() -> R {
    for name in ["foo3:barr", "fooo3:bar"] {
        let mut application = Application::default();
        application.add_command(foo3())?;
        let error = find_error(&mut application, name);
        assert!(error.alternatives().is_some());
        assert!(error.message().contains("Did you mean this"), "{name}");
    }
    Ok(())
}

#[test]
fn testFindAlternativeExceptionMessageMultiple() -> R {
    let _columns = Columns::set("120");
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo1())?;
    application.add_command(foo2())?;

    // Command + plural
    let e = find_error(&mut application, "foo:baR");
    assert!(matches!(e, ConsoleError::CommandNotFound(..)));
    assert!(e.message().contains("Did you mean one of these"));
    assert!(e.message().contains("foo1:bar"));
    assert!(e.message().contains("foo:bar"));

    // Namespace + plural
    let e = find_error(&mut application, "foo2:bar");
    assert!(e.alternatives().is_some());
    assert!(e.message().contains("Did you mean one of these"));
    assert!(e.message().contains("foo1"));

    application.add_command(foo3())?;
    application.add_command(foo4())?;

    // Subnamespace + plural
    let e = find_error(&mut application, "foo3:");
    assert!(e.alternatives().is_some());
    assert!(e.message().contains("foo3:bar"));
    assert!(e.message().contains("foo3:bar:toh"));
    Ok(())
}

#[test]
fn testFindAlternativeCommands() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo1())?;
    application.add_command(foo2())?;

    let command_name = "Unknown command";
    let e = find_error(&mut application, command_name);
    assert!(matches!(e, ConsoleError::CommandNotFound(..)));
    assert_eq!(Some(&[][..]), e.alternatives());
    assert_eq!(
        format!("Command \"{command_name}\" is not defined."),
        e.message()
    );

    // Test if "bar1" command throw a "CommandNotFoundException" and does not contain
    // "foo:bar" as alternative because "bar1" is too far from "foo:bar"
    let command_name = "bar1";
    let e = find_error(&mut application, command_name);
    assert!(matches!(e, ConsoleError::CommandNotFound(..)));
    assert_eq!(
        Some(&["afoobar1".to_owned(), "foo:bar1".to_owned()][..]),
        e.alternatives()
    );
    let message = e.message();
    assert!(message.contains("Command \"bar1\" is not defined."));
    assert!(message.contains("afoobar1"));
    assert!(message.contains("foo:bar1"));
    // /foo:bar(?!1)/
    assert!(
        !message
            .match_indices("foo:bar")
            .any(|(i, _)| !message[i + 7..].starts_with('1'))
    );
    Ok(())
}

#[test]
fn testFindAlternativeNamespace() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo1())?;
    application.add_command(foo2())?;
    application.add_command(foo3())?;

    let e = find_error(&mut application, "Unknown-namespace:Unknown-command");
    assert!(e.alternatives().is_some());
    assert_eq!(Some(&[][..]), e.alternatives());
    assert_eq!(
        "There are no commands defined in the \"Unknown-namespace\" namespace.",
        e.message()
    );

    let e = find_error(&mut application, "foo2:command");
    assert!(matches!(e, ConsoleError::NamespaceNotFound(..)));
    let alternatives = e.alternatives().unwrap_or_default();
    assert_eq!(3, alternatives.len());
    for namespace in ["foo", "foo1", "foo3"] {
        assert!(alternatives.contains(&namespace.to_owned()));
        assert!(e.message().contains(namespace));
    }
    assert!(
        e.message()
            .contains("There are no commands defined in the \"foo2\" namespace.")
    );
    Ok(())
}

#[test]
fn testFindAlternativesOutput() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo1())?;
    application.add_command(foo2())?;
    application.add_command(foo3())?;
    application.add_command(foo_hidden())?;

    let expected_alternatives = [
        "afoobar", "afoobar1", "afoobar2", "foo1:bar", "foo3:bar", "foo:bar", "foo:bar1",
    ];
    let e = find_error(&mut application, "foo");
    assert!(matches!(e, ConsoleError::CommandNotFound(..)));
    assert_eq!(
        Some(expected_alternatives.map(str::to_owned).as_slice()),
        e.alternatives()
    );
    assert!(
        e.message()
            .starts_with("Command \"foo\" is not defined.\n\nDid you mean one of these?\n")
    );
    Ok(())
}

#[test]
fn testFindWithDoubleColonInNameThrowsException() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo4())?;
    let e = find_error(&mut application, "foo::bar");
    assert!(matches!(e, ConsoleError::CommandNotFound(..)));
    assert!(e.message().contains("Command \"foo::bar\" is not defined."));
    Ok(())
}

#[test]
fn testFindHiddenWithExactName() -> R {
    let mut application = Application::default();
    application.add_command(foo_hidden())?;
    assert_eq!(
        "FooHiddenCommand",
        find_class(&mut application, "foo:hidden")?
    );
    assert_eq!(
        "FooHiddenCommand",
        find_class(&mut application, "afoohidden")?
    );
    Ok(())
}

#[test]
fn testFindAmbiguousCommandsIfAllAlternativesAreHidden() -> R {
    let mut application = Application::default();
    application.add_command(foo())?;
    application.add_command(foo_hidden())?;
    assert_eq!("FooCommand", find_class(&mut application, "foo:")?);
    Ok(())
}

#[test]
fn testFindAmbiguousHiddenCommands() -> R {
    let mut application = Application::default();
    application.add_command(Command::new(Some("test:foo"), None)?)?;
    application.add_command(Command::new(Some("test:foobar"), None)?)?;
    application.get("test:foo")?.lock().set_hidden(true);
    application.get("test:foobar")?.lock().set_hidden(true);
    let e = find_error(&mut application, "t:f");
    assert!(matches!(e, ConsoleError::CommandNotFound(..)));
    assert!(e.message().contains("The command \"t:f\" does not exist."));
    Ok(())
}

#[test]
fn testGetDefaultHelperSetReturnsDefaultValues() {
    let application = Application::default();
    assert!(application.get_helper_set().has("formatter"));
}

#[test]
fn testAddingSingleHelperSetOverwritesDefaultValues() {
    let mut application = Application::default();
    application.set_helper_set(Arc::new(HelperSet::new(vec![(
        None,
        Arc::new(FormatterHelper),
    )])));
    let helper_set = application.get_helper_set();
    assert!(helper_set.has("formatter"));
    // no other default helper set should be returned
    assert!(!helper_set.has("dialog"));
    assert!(!helper_set.has("progress"));
}

#[test]
fn testGetDefaultInputDefinitionReturnsDefaultValues() -> R {
    let input_definition = Application::default().get_definition()?;
    assert!(input_definition.has_argument("command"));
    for option in ["help", "quiet", "verbose", "version", "ansi"] {
        assert!(input_definition.has_option(option), "{option}");
    }
    assert!(input_definition.has_negation("no-ansi"));
    assert!(!input_definition.has_option("no-ansi"));
    assert!(input_definition.has_option("no-interaction"));
    Ok(())
}

#[test]
fn testSettingCustomInputDefinitionOverwritesDefaultValues() -> R {
    let mut application = Application::default();
    application.set_definition(InputDefinition::new(vec![
        InputOption::simple(
            "--custom",
            Some("-c"),
            Some(VALUE_NONE),
            "Set the custom input definition.",
        )?
        .into(),
    ])?);
    let input_definition = application.get_definition()?;
    // check whether the default arguments and options are not returned any more
    assert!(!input_definition.has_argument("command"));
    for option in [
        "help",
        "quiet",
        "verbose",
        "version",
        "ansi",
        "no-interaction",
    ] {
        assert!(!input_definition.has_option(option), "{option}");
    }
    assert!(!input_definition.has_negation("no-ansi"));
    assert!(input_definition.has_option("custom"));
    Ok(())
}
