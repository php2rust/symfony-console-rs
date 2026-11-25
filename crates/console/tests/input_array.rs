//! `InputTest` and `ArrayInputTest`, with upstream's names, assertions and
//! provider rows (transcribed: they build objects).
//!
//! Upstream: symfony/console Tests/Input/InputTest.php Tests/Input/ArrayInputTest.php
#![allow(non_snake_case)]

use php_value::{Key, PhpArray, Value};
use symfony_console::{
    ConsoleError,
    input::{
        SuggestedValues,
        input::{Input, escapeshellarg},
        input_argument::{InputArgument, OPTIONAL, REQUIRED},
        input_definition::{InputDefinition, Item},
        input_option::{
            InputOption, Shortcut, VALUE_IS_ARRAY, VALUE_NEGATABLE, VALUE_NONE, VALUE_OPTIONAL,
            VALUE_REQUIRED,
        },
    },
};

type R = Result<(), ConsoleError>;

/// A `unparseProvider` row: definition, parameters, option names, expected.
type UnparseCase = (
    Option<InputDefinition>,
    PhpArray,
    Option<Vec<&'static str>>,
    Vec<&'static str>,
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

fn arg(name: &str, mode: Option<i64>, default: Value) -> Result<Item, ConsoleError> {
    Ok(InputArgument::new(name, mode, "", default, SuggestedValues::default())?.into())
}

fn opt(
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

fn expect<T: std::fmt::Debug>(
    result: Result<T, ConsoleError>,
    check: fn(&ConsoleError) -> bool,
    message: &str,
) {
    match result {
        Ok(v) => panic!("expected an exception, got {v:?}"),
        Err(e) => {
            assert!(check(&e), "{e:?}");
            assert_eq!(message, e.message());
        }
    }
}

fn invalid(e: &ConsoleError) -> bool {
    e.is_invalid_argument()
}

fn runtime(e: &ConsoleError) -> bool {
    e.is_runtime()
}

// InputTest

#[test]
fn testConstructor() -> R {
    let input = Input::array(
        params(vec![("name", s("foo"))]),
        Some(def(vec![arg("name", None, Value::Null)?])?),
    )?;
    assert_eq!(s("foo"), input.get_argument("name")?);
    Ok(())
}

#[test]
fn testOptions() -> R {
    let mut input = Input::array(
        params(vec![("--name", s("foo"))]),
        Some(def(vec![opt("name", None, None, Value::Null)?])?),
    )?;
    assert_eq!(s("foo"), input.get_option("name")?);
    input.set_option("name", s("bar"))?;
    assert_eq!(s("bar"), input.get_option("name")?);
    assert_eq!(params(vec![("name", s("bar"))]), input.get_options());

    let two = || -> Result<InputDefinition, ConsoleError> {
        def(vec![
            opt("name", None, None, Value::Null)?,
            opt("bar", Some(""), Some(VALUE_OPTIONAL), s("default"))?,
        ])
    };
    let input = Input::array(params(vec![("--name", s("foo"))]), Some(two()?))?;
    assert_eq!(s("default"), input.get_option("bar")?);
    assert_eq!(
        params(vec![("name", s("foo")), ("bar", s("default"))]),
        input.get_options()
    );

    let input = Input::array(
        params(vec![("--name", s("foo")), ("--bar", s(""))]),
        Some(two()?),
    )?;
    assert_eq!(s(""), input.get_option("bar")?);
    assert_eq!(
        params(vec![("name", s("foo")), ("bar", s(""))]),
        input.get_options()
    );

    let input = Input::array(
        params(vec![("--name", s("foo")), ("--bar", Value::Null)]),
        Some(two()?),
    )?;
    assert!(input.get_option("bar")?.is_null());
    assert_eq!(
        params(vec![("name", s("foo")), ("bar", Value::Null)]),
        input.get_options()
    );
    assert_eq!(
        &params(vec![("name", s("foo")), ("bar", Value::Null)]),
        input.get_raw_options()
    );

    let negatable = || def(vec![opt("name", None, Some(VALUE_NEGATABLE), Value::Null)?]);
    let input = Input::array(params(vec![("--name", Value::Null)]), Some(negatable()?))?;
    assert!(input.has_option("name"));
    assert!(input.has_option("no-name"));
    assert_eq!(Value::Bool(true), input.get_option("name")?);
    assert_eq!(Value::Bool(false), input.get_option("no-name")?);

    let input = Input::array(params(vec![("--no-name", Value::Null)]), Some(negatable()?))?;
    assert_eq!(Value::Bool(false), input.get_option("name")?);
    assert_eq!(Value::Bool(true), input.get_option("no-name")?);

    let input = Input::array(PhpArray::new(), Some(negatable()?))?;
    assert!(input.get_option("name")?.is_null());
    assert!(input.get_option("no-name")?.is_null());
    Ok(())
}

fn name_and_bar() -> Result<InputDefinition, ConsoleError> {
    def(vec![
        opt("name", None, None, Value::Null)?,
        opt("bar", Some(""), Some(VALUE_OPTIONAL), s("default"))?,
    ])
}

#[test]
fn testSetInvalidOption() -> R {
    let mut input = Input::array(params(vec![("--name", s("foo"))]), Some(name_and_bar()?))?;
    expect(
        input.set_option("foo", s("bar")),
        invalid,
        "The \"foo\" option does not exist.",
    );
    Ok(())
}

#[test]
fn testGetInvalidOption() -> R {
    let input = Input::array(params(vec![("--name", s("foo"))]), Some(name_and_bar()?))?;
    expect(
        input.get_option("foo"),
        invalid,
        "The \"foo\" option does not exist.",
    );
    Ok(())
}

fn name_and_bar_arguments() -> Result<InputDefinition, ConsoleError> {
    def(vec![
        arg("name", None, Value::Null)?,
        arg("bar", Some(OPTIONAL), s("default"))?,
    ])
}

#[test]
fn testArguments() -> R {
    let mut input = Input::array(
        params(vec![("name", s("foo"))]),
        Some(def(vec![arg("name", None, Value::Null)?])?),
    )?;
    assert_eq!(s("foo"), input.get_argument("name")?);
    input.set_argument("name", s("bar"))?;
    assert_eq!(s("bar"), input.get_argument("name")?);
    assert_eq!(params(vec![("name", s("bar"))]), input.get_arguments());
    assert_eq!(&params(vec![("name", s("bar"))]), input.get_raw_arguments());

    let input = Input::array(
        params(vec![("name", s("foo"))]),
        Some(name_and_bar_arguments()?),
    )?;
    assert_eq!(s("default"), input.get_argument("bar")?);
    assert_eq!(
        params(vec![("name", s("foo")), ("bar", s("default"))]),
        input.get_arguments()
    );
    assert_eq!(&params(vec![("name", s("foo"))]), input.get_raw_arguments());
    Ok(())
}

#[test]
fn testSetInvalidArgument() -> R {
    let mut input = Input::array(
        params(vec![("name", s("foo"))]),
        Some(name_and_bar_arguments()?),
    )?;
    expect(
        input.set_argument("foo", s("bar")),
        invalid,
        "The \"foo\" argument does not exist.",
    );
    Ok(())
}

#[test]
fn testGetInvalidArgument() -> R {
    let input = Input::array(
        params(vec![("name", s("foo"))]),
        Some(name_and_bar_arguments()?),
    )?;
    expect(
        input.get_argument("foo"),
        invalid,
        "The \"foo\" argument does not exist.",
    );
    Ok(())
}

#[test]
fn testValidateWithMissingArguments() -> R {
    let mut input = Input::array(PhpArray::new(), None)?;
    input.bind(def(vec![arg("name", Some(REQUIRED), Value::Null)?])?)?;
    expect(
        input.validate(),
        runtime,
        "Not enough arguments (missing: \"name\").",
    );
    Ok(())
}

#[test]
fn testValidateWithMissingRequiredArguments() -> R {
    let mut input = Input::array(params(vec![("bar", s("baz"))]), None)?;
    input.bind(def(vec![
        arg("name", Some(REQUIRED), Value::Null)?,
        arg("bar", Some(OPTIONAL), Value::Null)?,
    ])?)?;
    expect(
        input.validate(),
        runtime,
        "Not enough arguments (missing: \"name\").",
    );
    Ok(())
}

#[test]
fn testValidate() -> R {
    let mut input = Input::array(params(vec![("name", s("foo"))]), None)?;
    input.bind(def(vec![arg("name", Some(REQUIRED), Value::Null)?])?)?;
    assert!(input.validate().is_ok());
    Ok(())
}

#[test]
fn testSetGetInteractive() -> R {
    let mut input = Input::array(PhpArray::new(), None)?;
    assert!(input.is_interactive());
    input.set_interactive(false);
    assert!(!input.is_interactive());
    Ok(())
}

// ArrayInputTest

#[test]
fn testGetFirstArgument() -> R {
    assert_eq!(
        None,
        Input::array(PhpArray::new(), None)?.get_first_argument()?
    );
    assert_eq!(
        Some(s("Fabien")),
        Input::array(params(vec![("name", s("Fabien"))]), None)?.get_first_argument()?
    );
    assert_eq!(
        Some(s("Fabien")),
        Input::array(
            params(vec![("--foo", s("bar")), ("name", s("Fabien"))]),
            None
        )?
        .get_first_argument()?
    );
    Ok(())
}

#[test]
fn testHasParameterOption() -> R {
    let input = Input::array(
        params(vec![("name", s("Fabien")), ("--foo", s("bar"))]),
        None,
    )?;
    assert!(input.has_parameter_option(&["--foo"], false));
    assert!(!input.has_parameter_option(&["--bar"], false));
    let input = Input::array(PhpArray::list([s("--foo")]), None)?;
    assert!(input.has_parameter_option(&["--foo"], false));
    let input = Input::array(PhpArray::list([s("--foo"), s("--"), s("--bar")]), None)?;
    assert!(input.has_parameter_option(&["--bar"], false));
    assert!(!input.has_parameter_option(&["--bar"], true));
    Ok(())
}

#[test]
fn testGetParameterOption() -> R {
    let input = Input::array(
        params(vec![("name", s("Fabien")), ("--foo", s("bar"))]),
        None,
    )?;
    assert_eq!(
        s("bar"),
        input.get_parameter_option(&["--foo"], Value::Bool(false), false)
    );
    assert_eq!(
        s("default"),
        input.get_parameter_option(&["--bar"], s("default"), false)
    );
    let mut p = PhpArray::list([s("Fabien")]);
    p.insert("--foo", s("bar"));
    let input = Input::array(p, None)?;
    assert_eq!(
        s("bar"),
        input.get_parameter_option(&["--foo"], Value::Bool(false), false)
    );
    let mut p = PhpArray::list([s("--foo"), s("--")]);
    p.insert("--bar", s("woop"));
    let input = Input::array(p, None)?;
    assert_eq!(
        s("woop"),
        input.get_parameter_option(&["--bar"], Value::Bool(false), false)
    );
    assert_eq!(
        s("default"),
        input.get_parameter_option(&["--bar"], s("default"), true)
    );
    Ok(())
}

#[test]
fn testParseArguments() -> R {
    let input = Input::array(
        params(vec![("name", s("foo"))]),
        Some(def(vec![arg("name", None, Value::Null)?])?),
    )?;
    assert_eq!(params(vec![("name", s("foo"))]), input.get_arguments());
    Ok(())
}

#[test]
fn testParseOptions() -> R {
    let optional_f = || opt("foo", Some("f"), Some(VALUE_OPTIONAL), s("default"));
    let cases: Vec<(PhpArray, Vec<Item>, PhpArray, &str)> = vec![
        (
            params(vec![("--foo", s("bar"))]),
            vec![opt("foo", None, None, Value::Null)?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses long options",
        ),
        (
            params(vec![("--foo", s("bar"))]),
            vec![optional_f()?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses long options with a default value",
        ),
        (
            PhpArray::new(),
            vec![optional_f()?],
            params(vec![("foo", s("default"))]),
            "->parse() uses the default value for long options with value optional which are not passed",
        ),
        (
            params(vec![("--foo", Value::Null)]),
            vec![optional_f()?],
            params(vec![("foo", Value::Null)]),
            "->parse() parses long options with a default value",
        ),
        (
            params(vec![("-f", s("bar"))]),
            vec![opt("foo", Some("f"), None, Value::Null)?],
            params(vec![("foo", s("bar"))]),
            "->parse() parses short options",
        ),
        (
            params(vec![("--", Value::Null), ("-f", s("bar"))]),
            vec![optional_f()?],
            params(vec![("foo", s("default"))]),
            "->parse() does not parse opts after an end of options signal",
        ),
        (
            params(vec![("--", Value::Null)]),
            vec![],
            PhpArray::new(),
            "->parse() does not choke on end of options signal",
        ),
    ];
    for (parameters, options, expected, message) in cases {
        let input = Input::array(parameters, Some(def(options)?))?;
        assert_eq!(expected, input.get_options(), "{message}");
    }
    Ok(())
}

#[test]
fn testParseInvalidInput() -> R {
    let cases: Vec<(PhpArray, InputDefinition, &str)> = vec![
        (
            params(vec![("foo", s("foo"))]),
            def(vec![arg("name", None, Value::Null)?])?,
            "The \"foo\" argument does not exist.",
        ),
        (
            params(vec![("--foo", Value::Null)]),
            def(vec![opt(
                "foo",
                Some("f"),
                Some(VALUE_REQUIRED),
                Value::Null,
            )?])?,
            "The \"--foo\" option requires a value.",
        ),
        (
            params(vec![("--foo", s("foo"))]),
            InputDefinition::default(),
            "The \"--foo\" option does not exist.",
        ),
        (
            params(vec![("-o", s("foo"))]),
            InputDefinition::default(),
            "The \"-o\" option does not exist.",
        ),
    ];
    for (parameters, definition, message) in cases {
        expect(Input::array(parameters, Some(definition)), invalid, message);
    }
    Ok(())
}

#[test]
fn testToString() -> R {
    let input = Input::array(
        params(vec![
            ("-f", Value::Null),
            ("-b", s("bar")),
            ("--foo", s("b a z")),
            ("--lala", Value::Null),
            ("test", s("Foo")),
            ("test2", s("A\nB'C")),
        ]),
        None,
    )?;
    assert_eq!(
        format!(
            "-f -b bar --foo={} --lala Foo {}",
            escapeshellarg("b a z"),
            escapeshellarg("A\nB'C")
        ),
        input.to_php_string()
    );
    let input = Input::array(
        params(vec![
            ("-b", list(&["bval_1", "bval_2"])),
            ("--f", list(&["fval_1", "fval_2"])),
        ]),
        None,
    )?;
    assert_eq!(
        "-b bval_1 -b bval_2 --f=fval_1 --f=fval_2",
        input.to_php_string()
    );
    let input = Input::array(params(vec![("array_arg", list(&["val_1", "val_2"]))]), None)?;
    assert_eq!("val_1 val_2", input.to_php_string());
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

#[test]
#[expect(clippy::too_many_lines, reason = "one function, as upstream's")]
fn testUnparse() -> R {
    let single = |option: Item,
                  input: PhpArray,
                  expected: Vec<&'static str>|
     -> Result<UnparseCase, ConsoleError> {
        Ok((Some(def(vec![option])?), input, None, expected))
    };
    let escape = |value: &str, expected: &'static str| -> Result<UnparseCase, ConsoleError> {
        Ok((
            Some(def(vec![opt(
                "opt",
                None,
                Some(VALUE_REQUIRED),
                Value::Null,
            )?])?),
            params(vec![("--opt", s(value))]),
            None,
            vec![expected],
        ))
    };
    let all_four = || {
        params(vec![
            ("requiredArgWithoutDefaultValue", s("argValue")),
            ("optionalArgWithDefaultValue", s("argDefaultValue")),
            ("--optWithoutDefaultValue", s("optValue")),
            ("--optWithDefaultValue", s("optDefaultValue")),
        ])
    };
    let two_options = || {
        params(vec![
            ("--optWithoutDefaultValue", s("optValue1")),
            ("--optWithDefaultValue", s("optValue2")),
        ])
    };
    let cases: Vec<(&str, UnparseCase)> = vec![
        (
            "empty input and empty definition",
            (
                Some(InputDefinition::default()),
                PhpArray::new(),
                None,
                vec![],
            ),
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
                PhpArray::new(),
                None,
                vec![],
            ),
        ),
        (
            "arguments & options: returns all passed options but ignore default values",
            (
                Some(complete_input_definition()?),
                params(vec![
                    ("requiredArgWithoutDefaultValue", s("argValue")),
                    ("--optWithoutDefaultValue", s("optValue")),
                ]),
                None,
                vec!["--optWithoutDefaultValue=optValue"],
            ),
        ),
        (
            "arguments & options; explicitly pass the default values: the default values are returned",
            (
                Some(complete_input_definition()?),
                all_four(),
                None,
                vec![
                    "--optWithoutDefaultValue=optValue",
                    "--optWithDefaultValue=optDefaultValue",
                ],
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
                params(vec![("requiredArgWithoutDefaultValue", s("argValue"))]),
                Some(vec!["requiredArgWithoutDefaultValue"]),
                vec![],
            ),
        ),
        (
            "arguments & options; non passed option: it is ignored",
            (
                Some(complete_input_definition()?),
                params(vec![("requiredArgWithoutDefaultValue", s("argValue"))]),
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
                vec!["--optWithDefaultValue=optValue2"],
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
            single(
                opt("opt", None, Some(VALUE_NONE), Value::Null)?,
                params(vec![("--opt", Value::Null)]),
                vec!["--opt"],
            )?,
        ),
        (
            "option without value by shortcut",
            single(
                opt("opt", Some("o"), Some(VALUE_NONE), Value::Null)?,
                params(vec![("-o", Value::Null)]),
                vec!["--opt"],
            )?,
        ),
        (
            "option with value required",
            single(
                opt("opt", None, Some(VALUE_REQUIRED), Value::Null)?,
                params(vec![("--opt", s("foo"))]),
                vec!["--opt=foo"],
            )?,
        ),
        (
            "option with non string value (bool)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED), Value::Null)?,
                params(vec![("--opt", Value::Bool(true))]),
                vec!["--opt=1"],
            )?,
        ),
        (
            "option with non string value (int)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED), Value::Null)?,
                params(vec![("--opt", Value::Int(20))]),
                vec!["--opt=20"],
            )?,
        ),
        (
            "option with non string value (float)",
            single(
                opt("opt", None, Some(VALUE_REQUIRED), Value::Null)?,
                params(vec![("--opt", Value::Float(5.3))]),
                vec!["--opt=5.3"],
            )?,
        ),
        (
            "option with non string value (array of strings)",
            single(
                opt(
                    "opt",
                    None,
                    Some(VALUE_REQUIRED | VALUE_IS_ARRAY),
                    Value::Null,
                )?,
                params(vec![("--opt", list(&["v1", "v2", "v3"]))]),
                vec!["--opt=v1", "--opt=v2", "--opt=v3"],
            )?,
        ),
        (
            "negatable option (positive)",
            single(
                opt("opt", None, Some(VALUE_NEGATABLE), Value::Null)?,
                params(vec![("--opt", Value::Null)]),
                vec!["--opt"],
            )?,
        ),
        (
            "negatable option (negative)",
            single(
                opt("opt", None, Some(VALUE_NEGATABLE), Value::Null)?,
                params(vec![("--no-opt", Value::Null)]),
                vec!["--no-opt"],
            )?,
        ),
        ("escape token; string token", escape("foo", "--opt=foo")?),
        (
            "escape token; escaped string token",
            escape("\"foo\"", "--opt=\"foo\"")?,
        ),
        (
            "escape token; escaped string token with both types of quotes",
            escape("\"o_id in('20')\"", "--opt=\"o_id in('20')\"")?,
        ),
        (
            "escape token; string token with spaces",
            escape("a b c d", "--opt=a b c d")?,
        ),
        (
            "escape token; string token with line return",
            escape("A\nB'C", "--opt=A\nB'C")?,
        ),
    ];
    for (name, (definition, parameters, parsed_options, expected)) in cases {
        let mut input = Input::array(parameters, None)?;
        if let Some(definition) = definition {
            input.bind(definition)?;
        }
        let actual = input.unparse(parsed_options.as_deref())?;
        assert_eq!(expected, actual, "{name}");
    }
    Ok(())
}
