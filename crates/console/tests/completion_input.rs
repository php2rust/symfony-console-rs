//! `CompletionInputTest`, with upstream's names, assertions and provider rows.
//!
//! Upstream: symfony/console Tests/Completion/CompletionInputTest.php
#![allow(non_snake_case)]

use php_value::Value;
use symfony_console::{
    ConsoleError,
    completion::completion_input::{
        CompletionInput, TYPE_ARGUMENT_VALUE, TYPE_NONE, TYPE_OPTION_NAME, TYPE_OPTION_VALUE,
    },
    input::{
        SuggestedValues,
        input_argument::{IS_ARRAY, InputArgument, OPTIONAL, REQUIRED},
        input_definition::InputDefinition,
        input_option::{InputOption, Shortcut, VALUE_NONE, VALUE_OPTIONAL, VALUE_REQUIRED},
    },
};

type R = Result<(), ConsoleError>;

fn from_tokens(tokens: &[&str], current_index: usize) -> CompletionInput {
    CompletionInput::from_tokens(
        tokens.iter().map(|t| (*t).to_owned()).collect(),
        current_index,
    )
}

fn opt(name: &str, shortcut: &str, mode: i64) -> Result<InputOption, ConsoleError> {
    InputOption::new(
        name,
        Shortcut::One(shortcut),
        Some(mode),
        "",
        Value::Null,
        SuggestedValues::default(),
    )
}

fn arg(name: &str, mode: i64, default: Value) -> Result<InputArgument, ConsoleError> {
    InputArgument::new(name, Some(mode), "", default, SuggestedValues::default())
}

type BindCase = (
    &'static str,
    CompletionInput,
    &'static str,
    Option<&'static str>,
    &'static str,
);

fn provideBindData() -> Vec<BindCase> {
    let r = Some("with-required-value");
    let o = Some("with-optional-value");
    vec![
        // option names
        (
            "optname-minimal-input",
            from_tokens(&["bin/console", "-"], 1),
            TYPE_OPTION_NAME,
            None,
            "-",
        ),
        (
            "optname-partial",
            from_tokens(&["bin/console", "--with"], 1),
            TYPE_OPTION_NAME,
            None,
            "--with",
        ),
        // option values
        (
            "optval-short",
            from_tokens(&["bin/console", "-r"], 1),
            TYPE_OPTION_VALUE,
            r,
            "",
        ),
        (
            "optval-short-partial",
            from_tokens(&["bin/console", "-rsymf"], 1),
            TYPE_OPTION_VALUE,
            r,
            "symf",
        ),
        (
            "optval-short-space",
            from_tokens(&["bin/console", "-r"], 2),
            TYPE_OPTION_VALUE,
            r,
            "",
        ),
        (
            "optval-short-space-partial",
            from_tokens(&["bin/console", "-r", "symf"], 2),
            TYPE_OPTION_VALUE,
            r,
            "symf",
        ),
        (
            "optval-short-before-arg",
            from_tokens(&["bin/console", "-r", "symfony"], 1),
            TYPE_OPTION_VALUE,
            r,
            "",
        ),
        (
            "optval-long",
            from_tokens(&["bin/console", "--with-required-value="], 1),
            TYPE_OPTION_VALUE,
            r,
            "",
        ),
        (
            "optval-long-partial",
            from_tokens(&["bin/console", "--with-required-value=symf"], 1),
            TYPE_OPTION_VALUE,
            r,
            "symf",
        ),
        (
            "optval-long-space",
            from_tokens(&["bin/console", "--with-required-value"], 2),
            TYPE_OPTION_VALUE,
            r,
            "",
        ),
        (
            "optval-long-space-partial",
            from_tokens(&["bin/console", "--with-required-value", "symf"], 2),
            TYPE_OPTION_VALUE,
            r,
            "symf",
        ),
        (
            "optval-short-optional",
            from_tokens(&["bin/console", "-o"], 1),
            TYPE_OPTION_VALUE,
            o,
            "",
        ),
        (
            "optval-short-space-optional",
            from_tokens(&["bin/console", "-o"], 2),
            TYPE_OPTION_VALUE,
            o,
            "",
        ),
        (
            "optval-long-optional",
            from_tokens(&["bin/console", "--with-optional-value="], 1),
            TYPE_OPTION_VALUE,
            o,
            "",
        ),
        (
            "optval-long-space-optional",
            from_tokens(&["bin/console", "--with-optional-value"], 2),
            TYPE_OPTION_VALUE,
            o,
            "",
        ),
        // arguments
        (
            "arg-minimal-input",
            from_tokens(&["bin/console"], 1),
            TYPE_ARGUMENT_VALUE,
            Some("required-arg"),
            "",
        ),
        (
            "arg-optional",
            from_tokens(&["bin/console", "symfony"], 2),
            TYPE_ARGUMENT_VALUE,
            Some("optional-arg"),
            "",
        ),
        (
            "arg-partial",
            from_tokens(&["bin/console", "symf"], 1),
            TYPE_ARGUMENT_VALUE,
            Some("required-arg"),
            "symf",
        ),
        (
            "arg-optional-partial",
            from_tokens(&["bin/console", "symfony", "sen"], 2),
            TYPE_ARGUMENT_VALUE,
            Some("optional-arg"),
            "sen",
        ),
        (
            "arg-after-option",
            from_tokens(&["bin/console", "--without-value"], 2),
            TYPE_ARGUMENT_VALUE,
            Some("required-arg"),
            "",
        ),
        (
            "arg-after-optional-value-option",
            from_tokens(&["bin/console", "--with-optional-value", "--"], 3),
            TYPE_ARGUMENT_VALUE,
            Some("required-arg"),
            "",
        ),
        // end of definition
        (
            "end",
            from_tokens(&["bin/console", "symfony", "sensiolabs"], 3),
            TYPE_NONE,
            None,
            "",
        ),
    ]
}

#[test]
fn testBind() -> R {
    for (name, mut input, expected_type, expected_name, expected_value) in provideBindData() {
        let definition = InputDefinition::new(vec![
            opt("with-required-value", "r", VALUE_REQUIRED)?.into(),
            opt("with-optional-value", "o", VALUE_OPTIONAL)?.into(),
            opt("without-value", "n", VALUE_NONE)?.into(),
            arg("required-arg", REQUIRED, Value::Null)?.into(),
            arg("optional-arg", OPTIONAL, Value::Null)?.into(),
        ])?;

        input.bind(definition)?;

        assert_eq!(
            expected_type,
            input.get_completion_type(),
            "Unexpected type ({name})"
        );
        assert_eq!(
            expected_name,
            input.get_completion_name(),
            "Unexpected name ({name})"
        );
        assert_eq!(
            expected_value,
            input.get_completion_value(),
            "Unexpected value ({name})"
        );
    }
    Ok(())
}

#[test]
fn testBindWithLastArrayArgument() -> R {
    // provideBindWithLastArrayArgumentData; assertEquals(null, '') holds,
    // so a null expected value is ''
    let cases = vec![
        (from_tokens(&["bin/console"], 1), ""),
        (
            from_tokens(&["bin/console", "symfony", "sensiolabs"], 3),
            "",
        ),
        (from_tokens(&["bin/console", "symfony", "sen"], 2), "sen"),
    ];
    for (mut input, expected_value) in cases {
        let definition = InputDefinition::new(vec![
            arg("list-arg", IS_ARRAY | REQUIRED, Value::Null)?.into(),
        ])?;

        input.bind(definition)?;

        assert_eq!(
            TYPE_ARGUMENT_VALUE,
            input.get_completion_type(),
            "Unexpected type"
        );
        assert_eq!(
            Some("list-arg"),
            input.get_completion_name(),
            "Unexpected name"
        );
        assert_eq!(
            expected_value,
            input.get_completion_value(),
            "Unexpected value"
        );
    }
    Ok(())
}

#[test]
fn testBindArgumentWithDefault() -> R {
    let definition = InputDefinition::new(vec![
        arg("arg-with-default", OPTIONAL, Value::string("default"))?.into(),
    ])?;

    let mut input = from_tokens(&["bin/console"], 1);
    input.bind(definition)?;

    assert_eq!(
        TYPE_ARGUMENT_VALUE,
        input.get_completion_type(),
        "Unexpected type"
    );
    assert_eq!(
        Some("arg-with-default"),
        input.get_completion_name(),
        "Unexpected name"
    );
    assert_eq!("", input.get_completion_value(), "Unexpected value");
    Ok(())
}

#[test]
fn testFromString() {
    // provideFromStringData
    let cases: Vec<(&str, Vec<&str>)> = vec![
        (
            "bin/console cache:clear",
            vec!["bin/console", "cache:clear"],
        ),
        (
            "bin/console --env prod",
            vec!["bin/console", "--env", "prod"],
        ),
        ("bin/console --env=prod", vec!["bin/console", "--env=prod"]),
        ("bin/console -eprod", vec!["bin/console", "-eprod"]),
        (
            "bin/console cache:clear \"multi word string\"",
            vec!["bin/console", "cache:clear", "\"multi word string\""],
        ),
        (
            "bin/console cache:clear 'multi word string'",
            vec!["bin/console", "cache:clear", "'multi word string'"],
        ),
    ];
    for (input_str, expected_tokens) in cases {
        let input = CompletionInput::from_string(input_str, 1);
        // ReflectionClass's `tokens` is what tokens() returns
        assert_eq!(expected_tokens, input.tokens(), "{input_str}");
    }
}

#[test]
fn testToString() {
    let input = from_tokens(&["foo", "bar", "baz"], 0);
    assert_eq!("foo| bar baz", input.to_php_string());

    let input = from_tokens(&["foo", "bar", "baz"], 1);
    assert_eq!("foo bar| baz", input.to_php_string());

    let input = from_tokens(&["foo", "bar", "baz"], 2);
    assert_eq!("foo bar baz|", input.to_php_string());

    let input = from_tokens(&["foo", "bar", "baz"], 11);
    assert_eq!("foo bar baz |", input.to_php_string());
}
