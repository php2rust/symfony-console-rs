//! `ArgvInputTest`, with upstream's names, assertions and provider rows
//! (transcribed: they build objects).
//!
//! Upstream: symfony/console Tests/Input/ArgvInputTest.php
#![allow(non_snake_case)]

use php_value::{Key, PhpArray, Value};
use symfony_console::{
    ConsoleError, env,
    input::{
        SuggestedValues,
        input::{Input, escapeshellarg},
        input_argument::{IS_ARRAY, InputArgument, OPTIONAL, REQUIRED},
        input_definition::{InputDefinition, Item},
        input_option::{
            InputOption, Shortcut, VALUE_IS_ARRAY, VALUE_NEGATABLE, VALUE_NONE, VALUE_OPTIONAL,
            VALUE_REQUIRED,
        },
    },
};

type R = Result<(), ConsoleError>;

/// An `unparseProvider` row: definition, argv, option names, expected.
type UnparseCase = (
    Option<InputDefinition>,
    Vec<String>,
    Option<Vec<&'static str>>,
    Vec<String>,
);

fn params(entries: Vec<(&str, Value)>) -> PhpArray {
    entries
        .into_iter()
        .map(|(k, v)| (Key::from(k), v))
        .collect()
}

fn s(text: &str) -> Value {
    Value::string(text)
}

fn list(values: &[&str]) -> Value {
    Value::Array(PhpArray::list(values.iter().map(|v| Value::string(*v))))
}

fn def(items: Vec<Item>) -> Result<InputDefinition, ConsoleError> {
    InputDefinition::new(items)
}

fn arg(name: &str, mode: Option<i64>) -> Result<Item, ConsoleError> {
    Ok(InputArgument::new(name, mode, "", Value::Null, SuggestedValues::default())?.into())
}

fn opt(name: &str, shortcut: Option<&str>, mode: Option<i64>) -> Result<Item, ConsoleError> {
    opt_default(name, shortcut, mode, Value::Null)
}

fn opt_default(
    name: &str,
    shortcut: Option<&str>,
    mode: Option<i64>,
    default: Value,
) -> Result<Item, ConsoleError> {
    let shortcut = shortcut.map_or(Shortcut::None, Shortcut::One);
    Ok(InputOption::new(
        name,
        shortcut,
        mode,
        "",
        default,
        SuggestedValues::default(),
    )?
    .into())
}

fn argv(tokens: &[&str]) -> Result<Input, ConsoleError> {
    Input::argv(tokens, None)
}

/// `expectException(\RuntimeException::class)` and
/// `expectExceptionMessage()`, which matches a substring.
fn expect_runtime<T: std::fmt::Debug>(result: Result<T, ConsoleError>, message: &str) {
    match result {
        Ok(v) => panic!("expected an exception, got {v:?}"),
        Err(e) => {
            assert!(e.is_runtime(), "{e:?}");
            assert!(
                e.message().contains(message),
                "{:?} does not contain {message:?}",
                e.message()
            );
        }
    }
}

#[test]
fn testConstructor() -> R {
    env::set_server_argv(Some(vec!["cli.php".to_owned(), "foo".to_owned()]));
    let input = Input::argv_from_server(None);
    env::set_server_argv(None);
    // ReflectionObject's `tokens` is what getRawTokens() returns
    assert_eq!(
        vec!["foo"],
        input?.get_raw_tokens(false)?,
        "__construct() automatically get its input from the argv server variable"
    );
    Ok(())
}

#[test]
fn testParseArguments() -> R {
    let mut input = argv(&["cli.php", "foo"])?;
    input.bind(def(vec![arg("name", None)?])?)?;
    assert_eq!(
        params(vec![("name", s("foo"))]),
        input.get_arguments(),
        "->parse() parses required arguments"
    );
    input.bind(def(vec![arg("name", None)?])?)?;
    assert_eq!(
        params(vec![("name", s("foo"))]),
        input.get_arguments(),
        "->parse() is stateless"
    );
    Ok(())
}

type OptionsCase = (Vec<&'static str>, Vec<Item>, PhpArray, &'static str);

fn provideOptions() -> Result<Vec<OptionsCase>, ConsoleError> {
    let t = Value::Bool(true);
    Ok(vec![
        (
            vec!["cli.php", "--foo"],
            vec![opt("foo", None, None)?],
            params(vec![("foo", t.clone())]),
            "->parse() parses long options without a value",
        ),
        (
            vec!["cli.php", "--foo=bar"],
            vec![opt("foo", Some("f"), Some(VALUE_REQUIRED))?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses long options with a required value (with a = separator)",
        ),
        (
            vec!["cli.php", "--foo", "bar"],
            vec![opt("foo", Some("f"), Some(VALUE_REQUIRED))?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses long options with a required value (with a space separator)",
        ),
        (
            vec!["cli.php", "--foo="],
            vec![opt("foo", Some("f"), Some(VALUE_OPTIONAL))?],
            params(vec![("foo", s(""))]),
            "->parse() parses long options with optional value which is empty (with a = separator) as empty string",
        ),
        (
            vec!["cli.php", "--foo=", "bar"],
            vec![
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
                arg("name", Some(REQUIRED))?,
            ],
            params(vec![("foo", s(""))]),
            "->parse() parses long options with optional value without value specified or an empty string (with a = separator) followed by an argument as empty string",
        ),
        (
            vec!["cli.php", "bar", "--foo"],
            vec![
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
                arg("name", Some(REQUIRED))?,
            ],
            params(vec![("foo", Value::Null)]),
            "->parse() parses long options with optional value which is empty (with a = separator) preceded by an argument",
        ),
        (
            vec!["cli.php", "--foo", "", "bar"],
            vec![
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
                arg("name", Some(REQUIRED))?,
            ],
            params(vec![("foo", s(""))]),
            "->parse() parses long options with optional value which is empty as empty string even followed by an argument",
        ),
        (
            vec!["cli.php", "--foo"],
            vec![opt("foo", Some("f"), Some(VALUE_OPTIONAL))?],
            params(vec![("foo", Value::Null)]),
            "->parse() parses long options with optional value specified with no separator and no value as null",
        ),
        (
            vec!["cli.php", "-f"],
            vec![opt("foo", Some("f"), None)?],
            params(vec![("foo", t.clone())]),
            "->parse() parses short options without a value",
        ),
        (
            vec!["cli.php", "-fbar"],
            vec![opt("foo", Some("f"), Some(VALUE_REQUIRED))?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses short options with a required value (with no separator)",
        ),
        (
            vec!["cli.php", "-f", "bar"],
            vec![opt("foo", Some("f"), Some(VALUE_REQUIRED))?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses short options with a required value (with a space separator)",
        ),
        (
            vec!["cli.php", "-f", ""],
            vec![opt("foo", Some("f"), Some(VALUE_OPTIONAL))?],
            params(vec![("foo", s(""))]),
            "->parse() parses short options with an optional empty value",
        ),
        (
            vec!["cli.php", "-f", "", "foo"],
            vec![
                arg("name", None)?,
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
            ],
            params(vec![("foo", s(""))]),
            "->parse() parses short options with an optional empty value followed by an argument",
        ),
        (
            vec!["cli.php", "-f", "", "-b"],
            vec![
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
                opt("bar", Some("b"), None)?,
            ],
            params(vec![("foo", s("")), ("bar", t.clone())]),
            "->parse() parses short options with an optional empty value followed by an option",
        ),
        (
            vec!["cli.php", "-f", "-b", "foo"],
            vec![
                arg("name", None)?,
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
                opt("bar", Some("b"), None)?,
            ],
            params(vec![("foo", Value::Null), ("bar", t.clone())]),
            "->parse() parses short options with an optional value which is not present",
        ),
        (
            vec!["cli.php", "-fb"],
            vec![opt("foo", Some("f"), None)?, opt("bar", Some("b"), None)?],
            params(vec![("foo", t.clone()), ("bar", t.clone())]),
            "->parse() parses short options when they are aggregated as a single one",
        ),
        (
            vec!["cli.php", "-fb", "bar"],
            vec![
                opt("foo", Some("f"), None)?,
                opt("bar", Some("b"), Some(VALUE_REQUIRED))?,
            ],
            params(vec![("foo", t.clone()), ("bar", s("bar"))]),
            "->parse() parses short options when they are aggregated as a single one and the last one has a required value",
        ),
        (
            vec!["cli.php", "-fb", "bar"],
            vec![
                opt("foo", Some("f"), None)?,
                opt("bar", Some("b"), Some(VALUE_OPTIONAL))?,
            ],
            params(vec![("foo", t.clone()), ("bar", s("bar"))]),
            "->parse() parses short options when they are aggregated as a single one and the last one has an optional value",
        ),
        (
            vec!["cli.php", "-fbbar"],
            vec![
                opt("foo", Some("f"), None)?,
                opt("bar", Some("b"), Some(VALUE_OPTIONAL))?,
            ],
            params(vec![("foo", t.clone()), ("bar", s("bar"))]),
            "->parse() parses short options when they are aggregated as a single one and the last one has an optional value with no separator",
        ),
        (
            vec!["cli.php", "-fbbar"],
            vec![
                opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
                opt("bar", Some("b"), Some(VALUE_OPTIONAL))?,
            ],
            params(vec![("foo", s("bbar")), ("bar", Value::Null)]),
            "->parse() parses short options when they are aggregated as a single one and one of them takes a value",
        ),
    ])
}

fn provideNegatableOptions() -> Result<Vec<OptionsCase>, ConsoleError> {
    let message = "->parse() parses long options without a value";
    let negatable = || opt("foo", None, Some(VALUE_NEGATABLE));
    let none_negatable = || opt("foo", None, Some(VALUE_NONE | VALUE_NEGATABLE));
    Ok(vec![
        (
            vec!["cli.php", "--foo"],
            vec![negatable()?],
            params(vec![("foo", Value::Bool(true))]),
            message,
        ),
        (
            vec!["cli.php", "--foo"],
            vec![none_negatable()?],
            params(vec![("foo", Value::Bool(true))]),
            message,
        ),
        (
            vec!["cli.php", "--no-foo"],
            vec![negatable()?],
            params(vec![("foo", Value::Bool(false))]),
            message,
        ),
        (
            vec!["cli.php", "--no-foo"],
            vec![none_negatable()?],
            params(vec![("foo", Value::Bool(false))]),
            message,
        ),
        (
            vec!["cli.php"],
            vec![negatable()?],
            params(vec![("foo", Value::Null)]),
            message,
        ),
        (
            vec!["cli.php"],
            vec![none_negatable()?],
            params(vec![("foo", Value::Null)]),
            message,
        ),
        (
            vec!["cli.php"],
            vec![opt_default(
                "foo",
                None,
                Some(VALUE_NEGATABLE),
                Value::Bool(false),
            )?],
            params(vec![("foo", Value::Bool(false))]),
            message,
        ),
        (
            vec!["cli.php"],
            vec![opt_default(
                "foo",
                None,
                Some(VALUE_NONE | VALUE_NEGATABLE),
                Value::Bool(false),
            )?],
            params(vec![("foo", Value::Bool(false))]),
            message,
        ),
    ])
}

fn check_options(cases: Vec<OptionsCase>) -> R {
    for (tokens, options, expected, message) in cases {
        let mut input = argv(&tokens)?;
        input.bind(def(options)?)?;
        assert_eq!(expected, input.get_options(), "{message} {tokens:?}");
    }
    Ok(())
}

#[test]
fn testParseOptions() -> R {
    check_options(provideOptions()?)
}

#[test]
fn testParseOptionsNegatable() -> R {
    check_options(provideNegatableOptions()?)
}

type InvalidCase = (Vec<&'static str>, InputDefinition, &'static str);

/// Upstream's last row, `['cli.php', ['array']]` ("Argument values expected
/// to be all scalars, got "array"."), cannot be written: tokens are strings.
fn provideInvalidInput() -> Result<Vec<InvalidCase>, ConsoleError> {
    let none_f = || def(vec![opt("foo", Some("f"), Some(VALUE_NONE))?]);
    let required_f = || def(vec![opt("foo", Some("f"), Some(VALUE_REQUIRED))?]);
    Ok(vec![
        (
            vec!["cli.php", "--foo"],
            required_f()?,
            "The \"--foo\" option requires a value.",
        ),
        (
            vec!["cli.php", "-f"],
            required_f()?,
            "The \"--foo\" option requires a value.",
        ),
        (
            vec!["cli.php", "-ffoo"],
            none_f()?,
            "The \"-o\" option does not exist.",
        ),
        (
            vec!["cli.php", "--foo=bar"],
            none_f()?,
            "The \"--foo\" option does not accept a value.",
        ),
        (
            vec!["cli.php", "foo", "bar"],
            InputDefinition::default(),
            "No arguments expected, got \"foo\".",
        ),
        (
            vec!["cli.php", "foo", "bar"],
            def(vec![arg("number", None)?])?,
            "Too many arguments, expected arguments \"number\".",
        ),
        (
            vec!["cli.php", "foo", "bar", "zzz"],
            def(vec![arg("number", None)?, arg("county", None)?])?,
            "Too many arguments, expected arguments \"number\" \"county\".",
        ),
        (
            vec!["cli.php", "--foo"],
            InputDefinition::default(),
            "The \"--foo\" option does not exist.",
        ),
        (
            vec!["cli.php", "-f"],
            InputDefinition::default(),
            "The \"-f\" option does not exist.",
        ),
        (
            vec!["cli.php", "-1"],
            def(vec![arg("number", None)?])?,
            "The \"-1\" option does not exist.",
        ),
        (
            vec!["cli.php", "-fЩ"],
            none_f()?,
            "The \"-Щ\" option does not exist.",
        ),
        (
            vec!["cli.php", "acme:foo", "bar"],
            def(vec![arg("command", Some(REQUIRED))?])?,
            "No arguments expected for \"acme:foo\" command, got \"bar\"",
        ),
        (
            vec!["cli.php", "acme:foo", "bar"],
            def(vec![arg("name", Some(REQUIRED))?])?,
            "Too many arguments, expected arguments \"name\".",
        ),
    ])
}

fn provideInvalidNegatableInput() -> Result<Vec<InvalidCase>, ConsoleError> {
    let message = "The \"--no-foo\" option does not accept a value.";
    let negatable = || def(vec![opt("foo", Some("f"), Some(VALUE_NEGATABLE))?]);
    let none_negatable = || {
        def(vec![opt(
            "foo",
            Some("f"),
            Some(VALUE_NONE | VALUE_NEGATABLE),
        )?])
    };
    Ok(vec![
        (vec!["cli.php", "--no-foo=bar"], negatable()?, message),
        (vec!["cli.php", "--no-foo="], negatable()?, message),
        (vec!["cli.php", "--no-foo=bar"], none_negatable()?, message),
        (vec!["cli.php", "--no-foo="], none_negatable()?, message),
    ])
}

fn check_invalid(cases: Vec<InvalidCase>) -> R {
    for (tokens, definition, message) in cases {
        expect_runtime(argv(&tokens)?.bind(definition), message);
    }
    Ok(())
}

#[test]
fn testInvalidInput() -> R {
    check_invalid(provideInvalidInput()?)
}

#[test]
fn testInvalidInputNegatable() -> R {
    check_invalid(provideInvalidNegatableInput()?)
}

#[test]
fn testParseArrayArgument() -> R {
    let mut input = argv(&["cli.php", "foo", "bar", "baz", "bat"])?;
    input.bind(def(vec![arg("name", Some(IS_ARRAY))?])?)?;
    assert_eq!(
        params(vec![("name", list(&["foo", "bar", "baz", "bat"]))]),
        input.get_arguments(),
        "->parse() parses array arguments"
    );
    Ok(())
}

#[test]
fn testParseArrayOption() -> R {
    let name = || opt("name", None, Some(VALUE_OPTIONAL | VALUE_IS_ARRAY));

    let mut input = argv(&["cli.php", "--name=foo", "--name=bar", "--name=baz"])?;
    input.bind(def(vec![name()?])?)?;
    assert_eq!(
        params(vec![("name", list(&["foo", "bar", "baz"]))]),
        input.get_options(),
        "->parse() parses array options (\"--option=value\" syntax)"
    );

    let mut input = argv(&["cli.php", "--name", "foo", "--name", "bar", "--name", "baz"])?;
    input.bind(def(vec![name()?])?)?;
    assert_eq!(
        params(vec![("name", list(&["foo", "bar", "baz"]))]),
        input.get_options(),
        "->parse() parses array options (\"--option value\" syntax)"
    );

    let mut input = argv(&["cli.php", "--name=foo", "--name=bar", "--name="])?;
    input.bind(def(vec![name()?])?)?;
    assert_eq!(
        params(vec![("name", list(&["foo", "bar", ""]))]),
        input.get_options(),
        "->parse() parses empty array options as null (\"--option=value\" syntax)"
    );

    let mut input = argv(&[
        "cli.php",
        "--name",
        "foo",
        "--name",
        "bar",
        "--name",
        "--anotherOption",
    ])?;
    input.bind(def(vec![
        name()?,
        opt("anotherOption", None, Some(VALUE_NONE))?,
    ])?)?;
    assert_eq!(
        params(vec![
            (
                "name",
                Value::Array(PhpArray::list([s("foo"), s("bar"), Value::Null]))
            ),
            ("anotherOption", Value::Bool(true)),
        ]),
        input.get_options(),
        "->parse() parses empty array options (\"--option value\" syntax)"
    );
    Ok(())
}

#[test]
fn testParseNegativeNumberAfterDoubleDash() -> R {
    let mut input = argv(&["cli.php", "--", "-1"])?;
    input.bind(def(vec![arg("number", None)?])?)?;
    assert_eq!(
        params(vec![("number", s("-1"))]),
        input.get_arguments(),
        "->parse() parses arguments with leading dashes as arguments after having encountered a double-dash sequence"
    );

    let mut input = argv(&["cli.php", "-f", "bar", "--", "-1"])?;
    input.bind(def(vec![
        arg("number", None)?,
        opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
    ])?)?;
    assert_eq!(
        params(vec![("foo", s("bar"))]),
        input.get_options(),
        "->parse() parses arguments with leading dashes as options before having encountered a double-dash sequence"
    );
    assert_eq!(
        params(vec![("number", s("-1"))]),
        input.get_arguments(),
        "->parse() parses arguments with leading dashes as arguments after having encountered a double-dash sequence"
    );
    Ok(())
}

#[test]
fn testParseEmptyStringArgument() -> R {
    let mut input = argv(&["cli.php", "-f", "bar", ""])?;
    input.bind(def(vec![
        arg("empty", None)?,
        opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
    ])?)?;
    assert_eq!(
        params(vec![("empty", s(""))]),
        input.get_arguments(),
        "->parse() parses empty string arguments"
    );
    Ok(())
}

#[test]
fn testGetFirstArgument() -> R {
    let input = argv(&["cli.php", "-fbbar"])?;
    assert_eq!(
        None,
        input.get_first_argument()?,
        "->getFirstArgument() returns null when there is no arguments"
    );

    let input = argv(&["cli.php", "-fbbar", "foo"])?;
    assert_eq!(
        Some(s("foo")),
        input.get_first_argument()?,
        "->getFirstArgument() returns the first argument from the raw input"
    );

    let mut input = argv(&["cli.php", "--foo", "fooval", "bar"])?;
    input.bind(def(vec![
        opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
        arg("arg", None)?,
    ])?)?;
    assert_eq!(Some(s("bar")), input.get_first_argument()?);

    let mut input = argv(&["cli.php", "-bf", "fooval", "argval"])?;
    input.bind(def(vec![
        opt("bar", Some("b"), Some(VALUE_NONE))?,
        opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
        arg("arg", None)?,
    ])?)?;
    assert_eq!(Some(s("argval")), input.get_first_argument()?);
    Ok(())
}

#[test]
fn testHasParameterOption() -> R {
    let input = argv(&["cli.php", "-f", "foo"])?;
    assert!(
        input.has_parameter_option(&["-f"], false),
        "->hasParameterOption() returns true if the given short option is in the raw input"
    );

    let input = argv(&["cli.php", "-etest"])?;
    assert!(
        input.has_parameter_option(&["-e"], false),
        "->hasParameterOption() returns true if the given short option is in the raw input"
    );
    assert!(
        !input.has_parameter_option(&["-s"], false),
        "->hasParameterOption() returns true if the given short option is in the raw input"
    );

    let input = argv(&["cli.php", "--foo", "foo"])?;
    assert!(
        input.has_parameter_option(&["--foo"], false),
        "->hasParameterOption() returns true if the given short option is in the raw input"
    );

    let input = argv(&["cli.php", "foo"])?;
    assert!(
        !input.has_parameter_option(&["--foo"], false),
        "->hasParameterOption() returns false if the given short option is not in the raw input"
    );

    let input = argv(&["cli.php", "--foo=bar"])?;
    assert!(
        input.has_parameter_option(&["--foo"], false),
        "->hasParameterOption() returns true if the given option with provided value is in the raw input"
    );
    Ok(())
}

#[test]
fn testHasParameterOptionOnlyOptions() -> R {
    let input = argv(&["cli.php", "-f", "foo"])?;
    assert!(
        input.has_parameter_option(&["-f"], true),
        "->hasParameterOption() returns true if the given short option is in the raw input"
    );

    let input = argv(&["cli.php", "--foo", "--", "foo"])?;
    assert!(
        input.has_parameter_option(&["--foo"], true),
        "->hasParameterOption() returns true if the given long option is in the raw input"
    );

    let input = argv(&["cli.php", "--foo=bar", "foo"])?;
    assert!(
        input.has_parameter_option(&["--foo"], true),
        "->hasParameterOption() returns true if the given long option with provided value is in the raw input"
    );

    let input = argv(&["cli.php", "--", "--foo"])?;
    assert!(
        !input.has_parameter_option(&["--foo"], true),
        "->hasParameterOption() returns false if the given option is in the raw input but after an end of options signal"
    );
    Ok(())
}

#[test]
fn testHasParameterOptionEdgeCasesAndLimitations() -> R {
    let message =
        "->hasParameterOption() returns true if the given short option is in the raw input";
    let input = argv(&["cli.php", "-fh"])?;
    // hasParameterOption does not know if the previous short option, -f,
    // takes a value or not. If -f takes a value, then -fh does NOT include
    // -h; Otherwise it does. Since we do not know which short options take
    // values, hasParameterOption does not support this use-case.
    assert!(!input.has_parameter_option(&["-h"], false), "{message}");
    // hasParameterOption does detect that `-fh` contains `-f`, since
    // `-f` is the first short option in the set.
    assert!(input.has_parameter_option(&["-f"], false), "{message}");
    // The test below happens to pass, although it might make more sense
    // to disallow it, and require the use of
    // $input->hasParameterOption('-f') && $input->hasParameterOption('-h')
    // instead.
    assert!(input.has_parameter_option(&["-fh"], false), "{message}");
    // In theory, if -fh is supported, then -hf should also work.
    // However, this is not supported.
    assert!(!input.has_parameter_option(&["-hf"], false), "{message}");

    let input = argv(&["cli.php", "-f", "-h"])?;
    // If hasParameterOption('-fh') is supported for 'cli.php -fh', then
    // one might also expect that it should also be supported for
    // 'cli.php -f -h'. However, this is not supported.
    assert!(!input.has_parameter_option(&["-fh"], false), "{message}");
    Ok(())
}

#[test]
fn testNoWarningOnInvalidParameterOption() -> R {
    let input = argv(&["cli.php", "-edev"])?;

    assert!(input.has_parameter_option(&["-e", ""], false));
    // No warning thrown
    assert!(!input.has_parameter_option(&["-m", ""], false));

    assert_eq!(
        s("dev"),
        input.get_parameter_option(&["-e", ""], Value::Bool(false), false)
    );
    // No warning thrown
    assert_eq!(
        Value::Bool(false),
        input.get_parameter_option(&["-m", ""], Value::Bool(false), false)
    );
    Ok(())
}

#[test]
fn testToString() -> R {
    let input = argv(&["cli.php", "-f", "foo"])?;
    assert_eq!("-f foo", input.to_php_string());

    let input = argv(&["cli.php", "-f", "--bar=foo", "a b c d", "A\nB'C"])?;
    assert_eq!(
        format!(
            "-f --bar=foo {} {}",
            escapeshellarg("a b c d"),
            escapeshellarg("A\nB'C")
        ),
        input.to_php_string()
    );
    Ok(())
}

/// A `provideGetParameterOptionValues` row.
type ParameterOptionCase = (
    Vec<&'static str>,
    Vec<&'static str>,
    &'static str,
    bool,
    &'static str,
);

#[test]
fn testGetParameterOptionEqualSign() -> R {
    // provideGetParameterOptionValues: argv, key, default, onlyParams, expected
    let cases: Vec<ParameterOptionCase> = vec![
        (
            vec!["app/console", "foo:bar"],
            vec!["-e"],
            "default",
            false,
            "default",
        ),
        (
            vec!["app/console", "foo:bar", "-e", "dev"],
            vec!["-e"],
            "default",
            false,
            "dev",
        ),
        (
            vec!["app/console", "foo:bar", "--env=dev"],
            vec!["--env"],
            "default",
            false,
            "dev",
        ),
        (
            vec!["app/console", "foo:bar", "-e", "dev"],
            vec!["-e", "--env"],
            "default",
            false,
            "dev",
        ),
        (
            vec!["app/console", "foo:bar", "--env=dev"],
            vec!["-e", "--env"],
            "default",
            false,
            "dev",
        ),
        (
            vec!["app/console", "foo:bar", "--env=dev", "--en=1"],
            vec!["--en"],
            "default",
            false,
            "1",
        ),
        (
            vec!["app/console", "foo:bar", "--env=dev", "", "--en=1"],
            vec!["--en"],
            "default",
            false,
            "1",
        ),
        (
            vec!["app/console", "foo:bar", "--env", "val"],
            vec!["--env"],
            "default",
            false,
            "val",
        ),
        (
            vec!["app/console", "foo:bar", "--env", "val", "--dummy"],
            vec!["--env"],
            "default",
            false,
            "val",
        ),
        (
            vec!["app/console", "foo:bar", "--", "--env=dev"],
            vec!["--env"],
            "default",
            false,
            "dev",
        ),
        (
            vec!["app/console", "foo:bar", "--", "--env=dev"],
            vec!["--env"],
            "default",
            true,
            "default",
        ),
    ];
    for (tokens, key, default, only_params, expected) in cases {
        let input = argv(&tokens)?;
        assert_eq!(
            s(expected),
            input.get_parameter_option(&key, s(default), only_params),
            "->getParameterOption() returns the expected value {tokens:?}"
        );
    }
    Ok(())
}

#[test]
fn testParseSingleDashAsArgument() -> R {
    let mut input = argv(&["cli.php", "-"])?;
    input.bind(def(vec![arg("file", None)?])?)?;
    assert_eq!(
        params(vec![("file", s("-"))]),
        input.get_arguments(),
        "->parse() parses single dash as an argument"
    );
    Ok(())
}

fn optional_foo_and_name(mode: i64) -> Result<InputDefinition, ConsoleError> {
    def(vec![
        opt("foo", Some("f"), Some(VALUE_OPTIONAL))?,
        arg("name", Some(mode))?,
    ])
}

#[test]
fn testParseOptionWithValueOptionalGivenEmptyAndRequiredArgument() -> R {
    let mut input = argv(&["cli.php", "--foo=", "bar"])?;
    input.bind(optional_foo_and_name(REQUIRED)?)?;
    assert_eq!(
        params(vec![("foo", s(""))]),
        input.get_options(),
        "->parse() parses optional options with empty value as null"
    );
    assert_eq!(
        params(vec![("name", s("bar"))]),
        input.get_arguments(),
        "->parse() parses required arguments"
    );

    let mut input = argv(&["cli.php", "--foo=0", "bar"])?;
    input.bind(optional_foo_and_name(REQUIRED)?)?;
    assert_eq!(
        params(vec![("foo", s("0"))]),
        input.get_options(),
        "->parse() parses optional options with empty value as null"
    );
    assert_eq!(
        params(vec![("name", s("bar"))]),
        input.get_arguments(),
        "->parse() parses required arguments"
    );
    Ok(())
}

#[test]
fn testParseOptionWithValueOptionalGivenEmptyAndOptionalArgument() -> R {
    let mut input = argv(&["cli.php", "--foo=", "bar"])?;
    input.bind(optional_foo_and_name(OPTIONAL)?)?;
    assert_eq!(
        params(vec![("foo", s(""))]),
        input.get_options(),
        "->parse() parses optional options with empty value as null"
    );
    assert_eq!(
        params(vec![("name", s("bar"))]),
        input.get_arguments(),
        "->parse() parses optional arguments"
    );

    let mut input = argv(&["cli.php", "--foo=0", "bar"])?;
    input.bind(optional_foo_and_name(OPTIONAL)?)?;
    assert_eq!(
        params(vec![("foo", s("0"))]),
        input.get_options(),
        "->parse() parses optional options with empty value as null"
    );
    assert_eq!(
        params(vec![("name", s("bar"))]),
        input.get_arguments(),
        "->parse() parses optional arguments"
    );
    Ok(())
}

#[test]
fn testGetRawTokensFalse() -> R {
    let input = argv(&["cli.php", "--foo", "bar"])?;
    assert_eq!(vec!["--foo", "bar"], input.get_raw_tokens(false)?);
    Ok(())
}

#[test]
fn testGetRawTokensTrue() -> R {
    // provideGetRawTokensTrueTests
    let cases: Vec<(Vec<&str>, Vec<&str>)> = vec![
        (vec!["app/console", "foo:bar"], vec![]),
        (
            vec!["app/console", "foo:bar", "--env=prod"],
            vec!["--env=prod"],
        ),
        (
            vec!["app/console", "foo:bar", "--env", "prod"],
            vec!["--env", "prod"],
        ),
        (
            vec!["app/console", "--no-ansi", "foo:bar", "--env", "prod"],
            vec!["--env", "prod"],
        ),
        (
            vec!["app/console", "--no-ansi", "foo:bar", "--env", "prod"],
            vec!["--env", "prod"],
        ),
        (
            vec!["app/console", "--no-ansi", "foo:bar", "argument"],
            vec!["argument"],
        ),
        (
            vec!["app/console", "--no-ansi", "foo:bar", "foo:bar"],
            vec!["foo:bar"],
        ),
        (
            vec!["app/console", "--no-ansi", "foo:bar", "--", "argument"],
            vec!["--", "argument"],
        ),
    ];
    for (tokens, expected) in cases {
        let input = argv(&tokens)?;
        assert_eq!(expected, input.get_raw_tokens(true)?, "{tokens:?}");
    }
    Ok(())
}

fn complete_input_definition() -> Result<InputDefinition, ConsoleError> {
    def(vec![
        InputArgument::new(
            "requiredArgWithoutDefaultValue",
            Some(REQUIRED),
            "Argument without a default value",
            Value::Null,
            SuggestedValues::default(),
        )?
        .into(),
        InputArgument::new(
            "optionalArgWithDefaultValue",
            Some(OPTIONAL),
            "Argument with a default value",
            s("argDefaultValue"),
            SuggestedValues::default(),
        )?
        .into(),
        InputOption::new(
            "optWithoutDefaultValue",
            Shortcut::None,
            Some(VALUE_REQUIRED),
            "Option without a default value",
            Value::Null,
            SuggestedValues::default(),
        )?
        .into(),
        InputOption::new(
            "optWithDefaultValue",
            Shortcut::None,
            Some(VALUE_REQUIRED),
            "Option with a default value",
            s("optDefaultValue"),
            SuggestedValues::default(),
        )?
        .into(),
    ])
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_owned()).collect()
}

#[test]
fn testUnparse() -> R {
    // $createSingleOptionScenario
    let single =
        |option: Item, input: &[&str], expected: &[&str]| -> Result<UnparseCase, ConsoleError> {
            let mut tokens = strings(&["appName"]);
            tokens.extend(strings(input));
            Ok((Some(def(vec![option])?), tokens, None, strings(expected)))
        };
    // $createEscapeOptionTokenScenario
    let escape = |value: &str, expected: &str| -> Result<UnparseCase, ConsoleError> {
        Ok((
            Some(def(vec![opt("opt", None, Some(VALUE_REQUIRED))?])?),
            vec!["appName".to_owned(), format!("--opt={value}")],
            None,
            vec![format!("--opt={expected}")],
        ))
    };
    let all_four = || {
        strings(&[
            "argValue",
            "argDefaultValue",
            "--optWithoutDefaultValue=optValue",
            "--optWithDefaultValue=optDefaultValue",
        ])
    };
    let two_options = || {
        strings(&[
            "--optWithoutDefaultValue=optValue1",
            "--optWithDefaultValue=optValue2",
        ])
    };
    let cases: Vec<(&str, UnparseCase)> = vec![
        (
            "empty input and empty definition",
            (Some(InputDefinition::default()), vec![], None, vec![]),
        ),
        (
            "empty input and definition with default values: ignore default values",
            (
                Some(def(vec![
                    InputArgument::new(
                        "argWithDefaultValue",
                        Some(OPTIONAL),
                        "Argument with a default value",
                        s("arg1DefaultValue"),
                        SuggestedValues::default(),
                    )?
                    .into(),
                    InputOption::new(
                        "optWithDefaultValue",
                        Shortcut::None,
                        Some(VALUE_REQUIRED),
                        "Option with a default value",
                        s("opt1DefaultValue"),
                        SuggestedValues::default(),
                    )?
                    .into(),
                ])?),
                vec![],
                None,
                vec![],
            ),
        ),
        (
            "arguments & options: returns all passed options but ignore default values",
            (
                Some(complete_input_definition()?),
                strings(&["argValue", "--optWithoutDefaultValue=optValue"]),
                None,
                strings(&["--optWithoutDefaultValue=optValue"]),
            ),
        ),
        (
            "arguments & options; explicitly pass the default values: the default values are returned",
            (
                Some(complete_input_definition()?),
                all_four(),
                None,
                strings(&[
                    "--optWithoutDefaultValue=optValue",
                    "--optWithDefaultValue=optDefaultValue",
                ]),
            ),
        ),
        (
            "arguments & options; no input definition: nothing returned",
            (None, all_four(), None, vec![]),
        ),
        (
            "arguments & options; parsing an argument name instead of an option name: that option is ignored",
            (
                Some(complete_input_definition()?),
                strings(&["argValue"]),
                Some(vec!["requiredArgWithoutDefaultValue"]),
                vec![],
            ),
        ),
        (
            "arguments & options; non passed option: it is ignored",
            (
                Some(complete_input_definition()?),
                strings(&["argValue"]),
                Some(vec!["optWithDefaultValue"]),
                vec![],
            ),
        ),
        (
            "arguments & options; requesting a specific option",
            (
                Some(complete_input_definition()?),
                two_options(),
                Some(vec!["optWithDefaultValue"]),
                strings(&["--optWithDefaultValue=optValue2"]),
            ),
        ),
        (
            "arguments & options; requesting no options",
            (
                Some(complete_input_definition()?),
                two_options(),
                Some(vec![]),
                vec![],
            ),
        ),
        (
            "option without value",
            single(opt("opt", None, Some(VALUE_NONE))?, &["--opt"], &["--opt"])?,
        ),
        (
            "option without value by shortcut",
            single(
                opt("opt", Some("o"), Some(VALUE_NONE))?,
                &["-o"],
                &["--opt"],
            )?,
        ),
        (
            "option with value required",
            single(
                opt("opt", None, Some(VALUE_REQUIRED))?,
                &["--opt=foo"],
                &["--opt=foo"],
            )?,
        ),
        (
            "option with non string value (bool)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED))?,
                &["--opt=1"],
                &["--opt=1"],
            )?,
        ),
        (
            "option with non string value (int)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED))?,
                &["--opt=20"],
                &["--opt=20"],
            )?,
        ),
        (
            "option with non string value (float)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED))?,
                &["--opt=5.3"],
                &["--opt=5.3"],
            )?,
        ),
        (
            "option with non string value (array of strings)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED | VALUE_IS_ARRAY))?,
                &["--opt=v1", "--opt=v2", "--opt=v3 --opt=v4"],
                &["--opt=v1", "--opt=v2", "--opt=v3 --opt=v4"],
            )?,
        ),
        (
            "negatable option (positive)",
            single(
                opt("opt", None, Some(VALUE_NEGATABLE))?,
                &["--opt"],
                &["--opt"],
            )?,
        ),
        (
            "negatable option (negative)",
            single(
                opt("opt", None, Some(VALUE_NEGATABLE))?,
                &["--no-opt"],
                &["--no-opt"],
            )?,
        ),
        ("escape token; string token", escape("foo", "foo")?),
        (
            "escape token; escaped string token",
            escape("\"foo\"", "\"foo\"")?,
        ),
        (
            "escape token; escaped string token with both types of quotes",
            escape("\"o_id in('20')\"", "\"o_id in('20')\"")?,
        ),
        (
            "escape token; string token with spaces",
            escape("a b c d", "a b c d")?,
        ),
        (
            "escape token; string token with line return",
            escape("A\nB'C", "A\nB'C")?,
        ),
    ];
    for (name, (definition, tokens, parsed_options, expected)) in cases {
        let tokens: Vec<&str> = tokens.iter().map(String::as_str).collect();
        let mut input = argv(&tokens)?;
        if let Some(definition) = definition {
            input.bind(definition)?;
        }
        let actual = input.unparse(parsed_options.as_deref())?;
        assert_eq!(expected, actual, "{name}");
    }
    Ok(())
}
