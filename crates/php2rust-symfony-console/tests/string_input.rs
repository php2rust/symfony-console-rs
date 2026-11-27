//! `StringInputTest` and `NullOutputTest`, with upstream's names, assertions
//! and provider rows.
//!
//! Upstream: symfony/console Tests/Input/StringInputTest.php Tests/Output/NullOutputTest.php
#![allow(non_snake_case)]

use php_value::Value;
use symfony_console::{
    ConsoleError,
    formatter::output_formatter::OutputFormatter,
    input::{
        SuggestedValues,
        input::{Input, escapeshellarg},
        input_definition::InputDefinition,
        input_option::{InputOption, Shortcut, VALUE_REQUIRED},
    },
    output::{
        null_output::NullOutput,
        output::{OutputInterface, VERBOSITY_NORMAL, VERBOSITY_SILENT, VERBOSITY_VERBOSE},
    },
};

type R = Result<(), ConsoleError>;

#[expect(clippy::too_many_lines, reason = "one function, as upstream's")]
fn get_tokenize_data() -> Vec<(&'static str, Vec<&'static str>, &'static str)> {
    vec![
        ("", vec![], "->tokenize() parses an empty string"),
        ("foo", vec!["foo"], "->tokenize() parses arguments"),
        (
            "  foo  bar  ",
            vec!["foo", "bar"],
            "->tokenize() ignores whitespaces between arguments",
        ),
        (
            "\"quoted\"",
            vec!["quoted"],
            "->tokenize() parses quoted arguments",
        ),
        (
            "'quoted'",
            vec!["quoted"],
            "->tokenize() parses quoted arguments",
        ),
        (
            "'a\rb\nc\td'",
            vec!["a\rb\nc\td"],
            "->tokenize() parses whitespace chars in strings",
        ),
        (
            "'a'\r'b'\n'c'\t'd'",
            vec!["a", "b", "c", "d"],
            "->tokenize() parses whitespace chars between args as spaces",
        ),
        (
            "\\\"quoted\\\"",
            vec!["\"quoted\""],
            "->tokenize() parses escaped-quoted arguments",
        ),
        (
            "\\'quoted\\'",
            vec!["'quoted'"],
            "->tokenize() parses escaped-quoted arguments",
        ),
        ("-a", vec!["-a"], "->tokenize() parses short options"),
        (
            "-azc",
            vec!["-azc"],
            "->tokenize() parses aggregated short options",
        ),
        (
            "-awithavalue",
            vec!["-awithavalue"],
            "->tokenize() parses short options with a value",
        ),
        (
            "-a\"foo bar\"",
            vec!["-afoo bar"],
            "->tokenize() parses short options with a value",
        ),
        (
            "-a\"foo bar\"\"foo bar\"",
            vec!["-afoo barfoo bar"],
            "->tokenize() parses short options with a value",
        ),
        (
            "-a'foo bar'",
            vec!["-afoo bar"],
            "->tokenize() parses short options with a value",
        ),
        (
            "-a'foo bar''foo bar'",
            vec!["-afoo barfoo bar"],
            "->tokenize() parses short options with a value",
        ),
        (
            "-a'foo bar'\"foo bar\"",
            vec!["-afoo barfoo bar"],
            "->tokenize() parses short options with a value",
        ),
        (
            "--long-option",
            vec!["--long-option"],
            "->tokenize() parses long options",
        ),
        (
            "--long-option=foo",
            vec!["--long-option=foo"],
            "->tokenize() parses long options with a value",
        ),
        (
            "--long-option=\"foo bar\"",
            vec!["--long-option=foo bar"],
            "->tokenize() parses long options with a value",
        ),
        (
            "--long-option=\"foo bar\"\"another\"",
            vec!["--long-option=foo baranother"],
            "->tokenize() parses long options with a value",
        ),
        (
            "--long-option='foo bar'",
            vec!["--long-option=foo bar"],
            "->tokenize() parses long options with a value",
        ),
        (
            "--long-option='foo bar''another'",
            vec!["--long-option=foo baranother"],
            "->tokenize() parses long options with a value",
        ),
        (
            "--long-option='foo bar'\"another\"",
            vec!["--long-option=foo baranother"],
            "->tokenize() parses long options with a value",
        ),
        (
            "foo -a -ffoo --long bar",
            vec!["foo", "-a", "-ffoo", "--long", "bar"],
            "->tokenize() parses when several arguments and options",
        ),
        (
            "--arg=\\\"'Jenny'\\''s'\\\"",
            vec!["--arg=\"Jenny's\""],
            "->tokenize() parses quoted quotes",
        ),
    ]
}

#[test]
fn testTokenize() -> R {
    for (input, tokens, message) in get_tokenize_data() {
        let input = Input::string(input)?;
        assert_eq!(tokens, input.get_raw_tokens(false)?, "{message}");
    }
    Ok(())
}

#[test]
fn testInputOptionWithGivenString() -> R {
    let definition = InputDefinition::new(vec![
        InputOption::new(
            "foo",
            Shortcut::None,
            Some(VALUE_REQUIRED),
            "",
            Value::Null,
            SuggestedValues::None,
        )?
        .into(),
    ])?;

    // call to bind
    let mut input = Input::string("--foo=bar")?;
    input.bind(definition)?;
    assert_eq!(Value::string("bar"), input.get_option("foo")?);
    Ok(())
}

#[test]
fn testToString() -> R {
    let input = Input::string("-f foo")?;
    assert_eq!("-f foo", input.to_php_string());

    let input = Input::string("-f --bar=foo \"a b c d\"")?;
    assert_eq!(
        format!("-f --bar=foo {}", escapeshellarg("a b c d")),
        input.to_php_string()
    );

    let input = Input::string("-f --bar=foo 'a b c d' 'A\nB\\'C'")?;
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

// NullOutputTest

#[test]
fn testConstructor() -> R {
    let mut output = NullOutput::new();
    // ob_start(): nothing reaches a stream, as NullOutput has none
    output.write("foo")?;
    assert!(!output.is_decorated(), "->isDecorated() returns false");
    Ok(())
}

#[test]
fn testVerbosity() {
    let mut output = NullOutput::new();
    assert_eq!(VERBOSITY_SILENT, output.get_verbosity());
    output.set_verbosity(VERBOSITY_VERBOSE);
    assert_eq!(VERBOSITY_SILENT, output.get_verbosity());
}

#[test]
fn testSetFormatter() {
    let mut output = NullOutput::new();
    let output_formatter = OutputFormatter::new(true, vec![]);
    output.set_formatter(output_formatter);
    // assertNotSame(): the formatter given is not kept
    assert!(!output.get_formatter().is_decorated());
}

#[test]
fn testSetVerbosity() {
    let mut output = NullOutput::new();
    output.set_verbosity(VERBOSITY_NORMAL);
    assert_eq!(VERBOSITY_SILENT, output.get_verbosity());
}

#[test]
fn testSetDecorated() {
    let mut output = NullOutput::new();
    output.set_decorated(true);
    assert!(!output.is_decorated());
}

#[test]
fn testIsSilent() {
    assert!(NullOutput::new().is_silent());
}

#[test]
fn testIsQuiet() {
    assert!(!NullOutput::new().is_quiet());
}

#[test]
fn testIsVerbose() {
    assert!(!NullOutput::new().is_verbose());
}

#[test]
fn testIsVeryVerbose() {
    assert!(!NullOutput::new().is_very_verbose());
}

#[test]
fn testIsDebug() {
    assert!(!NullOutput::new().is_debug());
}
