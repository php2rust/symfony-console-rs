//! `InputDefinitionTest`, with upstream's names and assertions. PHP's arrays
//! keyed by name compare as the ordered lists of their items.
//!
//! Upstream: symfony/console Tests/Input/InputDefinitionTest.php
#![allow(non_snake_case)]

use php_value::{Key, PhpArray, Value};
use symfony_console::{
    ConsoleError,
    input::{
        SuggestedValues,
        input_argument::{IS_ARRAY, InputArgument, OPTIONAL, REQUIRED},
        input_definition::{InputDefinition, Item},
        input_option::{
            InputOption, Shortcut, VALUE_IS_ARRAY, VALUE_NEGATABLE, VALUE_NONE, VALUE_OPTIONAL,
            VALUE_REQUIRED,
        },
    },
};

type R = Result<(), ConsoleError>;

struct Arguments {
    foo: InputArgument,
    bar: InputArgument,
    foo1: InputArgument,
    foo2: InputArgument,
}

fn initialize_arguments() -> Result<Arguments, ConsoleError> {
    Ok(Arguments {
        foo: InputArgument::simple("foo", None, "")?,
        bar: InputArgument::simple("bar", None, "")?,
        foo1: InputArgument::simple("foo", None, "")?,
        foo2: InputArgument::simple("foo2", Some(REQUIRED), "")?,
    })
}

struct Options {
    foo: InputOption,
    bar: InputOption,
    foo1: InputOption,
    foo2: InputOption,
    multi: InputOption,
}

fn initialize_options() -> Result<Options, ConsoleError> {
    Ok(Options {
        foo: InputOption::simple("foo", Some("f"), None, "")?,
        bar: InputOption::simple("bar", Some("b"), None, "")?,
        foo1: InputOption::simple("fooBis", Some("f"), None, "")?,
        foo2: InputOption::simple("foo", Some("p"), None, "")?,
        multi: InputOption::simple("multi", Some("m|mm|mmm"), None, "")?,
    })
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

fn logic(e: &ConsoleError) -> bool {
    matches!(e, ConsoleError::Logic(_))
}

fn invalid(e: &ConsoleError) -> bool {
    e.is_invalid_argument()
}

#[test]
fn testConstructorArguments() -> R {
    let a = initialize_arguments()?;
    assert_eq!(InputDefinition::default().get_arguments(), []);
    let definition = InputDefinition::new(vec![a.foo.clone().into(), a.bar.clone().into()])?;
    assert_eq!(&[a.foo, a.bar], definition.get_arguments());
    Ok(())
}

#[test]
fn testConstructorOptions() -> R {
    let o = initialize_options()?;
    assert_eq!(InputDefinition::default().get_options(), []);
    let definition = InputDefinition::new(vec![o.foo.clone().into(), o.bar.clone().into()])?;
    assert_eq!(&[o.foo, o.bar], definition.get_options());
    Ok(())
}

#[test]
fn testSetArguments() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.set_arguments(vec![a.foo.clone()])?;
    assert_eq!(
        &[a.foo],
        definition.get_arguments(),
        "->setArguments() sets the array of InputArgument objects"
    );
    definition.set_arguments(vec![a.bar.clone()])?;
    assert_eq!(
        &[a.bar],
        definition.get_arguments(),
        "->setArguments() clears all InputArgument objects"
    );
    Ok(())
}

#[test]
fn testAddArguments() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_arguments(vec![a.foo.clone()])?;
    assert_eq!(std::slice::from_ref(&a.foo), definition.get_arguments());
    definition.add_arguments(vec![a.bar.clone()])?;
    assert_eq!(&[a.foo, a.bar], definition.get_arguments());
    Ok(())
}

#[test]
fn testAddArgument() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_argument(a.foo.clone())?;
    assert_eq!(std::slice::from_ref(&a.foo), definition.get_arguments());
    definition.add_argument(a.bar.clone())?;
    assert_eq!(&[a.foo, a.bar], definition.get_arguments());
    Ok(())
}

#[test]
fn testArgumentsMustHaveDifferentNames() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_argument(a.foo)?;
    expect(
        definition.add_argument(a.foo1),
        logic,
        "An argument with name \"foo\" already exists.",
    );
    Ok(())
}

#[test]
fn testArrayArgumentHasToBeLast() -> R {
    let mut definition = InputDefinition::default();
    definition.add_argument(InputArgument::simple("fooarray", Some(IS_ARRAY), "")?)?;
    expect(
        definition.add_argument(InputArgument::simple("anotherbar", None, "")?),
        logic,
        "Cannot add a required argument \"anotherbar\" after an array argument \"fooarray\".",
    );
    Ok(())
}

#[test]
fn testRequiredArgumentCannotFollowAnOptionalOne() -> R {
    let a = initialize_arguments()?;
    let message = format!(
        "Cannot add a required argument \"{}\" after an optional one \"{}\".",
        a.foo2.get_name(),
        a.foo.get_name()
    );
    let mut definition = InputDefinition::default();
    definition.add_argument(a.foo)?;
    expect(definition.add_argument(a.foo2), logic, &message);
    Ok(())
}

#[test]
fn testGetArgument() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_arguments(vec![a.foo.clone()])?;
    assert_eq!(&a.foo, definition.get_argument("foo")?);
    Ok(())
}

#[test]
fn testGetInvalidArgument() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_arguments(vec![a.foo])?;
    expect(
        definition.get_argument("bar"),
        invalid,
        "The \"bar\" argument does not exist.",
    );
    Ok(())
}

#[test]
fn testHasArgument() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_arguments(vec![a.foo])?;
    assert!(definition.has_argument("foo"));
    assert!(!definition.has_argument("bar"));
    Ok(())
}

#[test]
fn testGetArgumentRequiredCount() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_argument(a.foo2)?;
    assert_eq!(1, definition.get_argument_required_count());
    definition.add_argument(a.foo)?;
    assert_eq!(1, definition.get_argument_required_count());
    Ok(())
}

#[test]
fn testGetArgumentCount() -> R {
    let a = initialize_arguments()?;
    let mut definition = InputDefinition::default();
    definition.add_argument(a.foo2)?;
    assert_eq!(1, definition.get_argument_count());
    definition.add_argument(a.foo)?;
    assert_eq!(2, definition.get_argument_count());
    Ok(())
}

fn ints(values: &[i64]) -> Value {
    Value::Array(PhpArray::list(values.iter().map(|&v| Value::Int(v))))
}

fn map(entries: Vec<(&str, Value)>) -> PhpArray {
    entries
        .into_iter()
        .map(|(k, v)| (Key::from(k), v))
        .collect()
}

#[test]
fn testGetArgumentDefaults() -> R {
    let definition = InputDefinition::new(vec![
        InputArgument::simple("foo1", Some(OPTIONAL), "")?.into(),
        InputArgument::new(
            "foo2",
            Some(OPTIONAL),
            "",
            Value::string("default"),
            SuggestedValues::default(),
        )?
        .into(),
        InputArgument::simple("foo3", Some(OPTIONAL | IS_ARRAY), "")?.into(),
    ])?;
    assert_eq!(
        map(vec![
            ("foo1", Value::Null),
            ("foo2", Value::string("default")),
            ("foo3", ints(&[]))
        ]),
        definition.get_argument_defaults()
    );
    let definition = InputDefinition::new(vec![
        InputArgument::new(
            "foo4",
            Some(OPTIONAL | IS_ARRAY),
            "",
            ints(&[1, 2]),
            SuggestedValues::default(),
        )?
        .into(),
    ])?;
    assert_eq!(
        map(vec![("foo4", ints(&[1, 2]))]),
        definition.get_argument_defaults()
    );
    Ok(())
}

#[test]
fn testSetOptions() -> R {
    let o = initialize_options()?;
    let mut definition = InputDefinition::new(vec![o.foo.clone().into()])?;
    assert_eq!(std::slice::from_ref(&o.foo), definition.get_options());
    definition.set_options(vec![o.bar.clone()])?;
    assert_eq!(&[o.bar], definition.get_options());
    Ok(())
}

#[test]
fn testSetOptionsClearsOptions() -> R {
    let o = initialize_options()?;
    let mut definition = InputDefinition::new(vec![o.foo.into()])?;
    definition.set_options(vec![o.bar])?;
    expect(
        definition.get_option_for_shortcut("f"),
        invalid,
        "The \"-f\" option does not exist.",
    );
    Ok(())
}

#[test]
fn testAddOptions() -> R {
    let o = initialize_options()?;
    let mut definition = InputDefinition::new(vec![o.foo.clone().into()])?;
    assert_eq!(std::slice::from_ref(&o.foo), definition.get_options());
    definition.add_options(vec![o.bar.clone()])?;
    assert_eq!(&[o.foo, o.bar], definition.get_options());
    Ok(())
}

#[test]
fn testAddOption() -> R {
    let o = initialize_options()?;
    let mut definition = InputDefinition::default();
    definition.add_option(o.foo.clone())?;
    assert_eq!(std::slice::from_ref(&o.foo), definition.get_options());
    definition.add_option(o.bar.clone())?;
    assert_eq!(&[o.foo, o.bar], definition.get_options());
    Ok(())
}

#[test]
fn testAddDuplicateOption() -> R {
    let o = initialize_options()?;
    let mut definition = InputDefinition::default();
    definition.add_option(o.foo)?;
    expect(
        definition.add_option(o.foo2),
        logic,
        "An option named \"foo\" already exists.",
    );
    Ok(())
}

#[test]
fn testAddDuplicateNegatedOption() -> R {
    let mut definition = InputDefinition::default();
    definition.add_option(InputOption::simple("no-foo", None, None, "")?)?;
    expect(
        definition.add_option(InputOption::simple("foo", None, Some(VALUE_NEGATABLE), "")?),
        logic,
        "An option named \"no-foo\" already exists.",
    );
    Ok(())
}

#[test]
fn testAddDuplicateNegatedReverseOption() -> R {
    let mut definition = InputDefinition::default();
    definition.add_option(InputOption::simple("foo", None, Some(VALUE_NEGATABLE), "")?)?;
    expect(
        definition.add_option(InputOption::simple("no-foo", None, None, "")?),
        logic,
        "An option named \"no-foo\" already exists.",
    );
    Ok(())
}

#[test]
fn testAddDuplicateShortcutOption() -> R {
    let o = initialize_options()?;
    let mut definition = InputDefinition::default();
    definition.add_option(o.foo)?;
    expect(
        definition.add_option(o.foo1),
        logic,
        "An option with shortcut \"f\" already exists.",
    );
    Ok(())
}

#[test]
fn testGetOption() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.foo.clone().into()])?;
    assert_eq!(&o.foo, definition.get_option("foo")?);
    Ok(())
}

#[test]
fn testGetInvalidOption() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.foo.into()])?;
    expect(
        definition.get_option("bar"),
        invalid,
        "The \"--bar\" option does not exist.",
    );
    Ok(())
}

#[test]
fn testHasOption() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.foo.into()])?;
    assert!(definition.has_option("foo"));
    assert!(!definition.has_option("bar"));
    Ok(())
}

#[test]
fn testHasShortcut() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.foo.into()])?;
    assert!(definition.has_shortcut("f"));
    assert!(!definition.has_shortcut("b"));
    Ok(())
}

#[test]
fn testGetOptionForShortcut() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.foo.clone().into()])?;
    assert_eq!(&o.foo, definition.get_option_for_shortcut("f")?);
    Ok(())
}

#[test]
fn testGetOptionForMultiShortcut() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.multi.clone().into()])?;
    assert_eq!(&o.multi, definition.get_option_for_shortcut("m")?);
    assert_eq!(&o.multi, definition.get_option_for_shortcut("mmm")?);
    Ok(())
}

#[test]
fn testGetOptionForInvalidShortcut() -> R {
    let o = initialize_options()?;
    let definition = InputDefinition::new(vec![o.foo.into()])?;
    expect(
        definition.get_option_for_shortcut("l"),
        invalid,
        "The \"-l\" option does not exist.",
    );
    Ok(())
}

#[test]
fn testGetOptionDefaults() -> R {
    let o = |name: &str, mode: i64, default: Value| -> Result<Item, ConsoleError> {
        Ok(InputOption::new(
            name,
            Shortcut::None,
            Some(mode),
            "",
            default,
            SuggestedValues::default(),
        )?
        .into())
    };
    let definition = InputDefinition::new(vec![
        o("foo1", VALUE_NONE, Value::Null)?,
        o("foo2", VALUE_REQUIRED, Value::Null)?,
        o("foo3", VALUE_REQUIRED, Value::string("default"))?,
        o("foo4", VALUE_OPTIONAL, Value::Null)?,
        o("foo5", VALUE_OPTIONAL, Value::string("default"))?,
        o("foo6", VALUE_OPTIONAL | VALUE_IS_ARRAY, Value::Null)?,
        o("foo7", VALUE_OPTIONAL | VALUE_IS_ARRAY, ints(&[1, 2]))?,
    ])?;
    let defaults = map(vec![
        ("foo1", Value::Bool(false)),
        ("foo2", Value::Null),
        ("foo3", Value::string("default")),
        ("foo4", Value::Null),
        ("foo5", Value::string("default")),
        ("foo6", ints(&[])),
        ("foo7", ints(&[1, 2])),
    ]);
    assert_eq!(defaults, definition.get_option_defaults());
    Ok(())
}

#[test]
fn testGetSynopsis() -> R {
    let option =
        |name: &str, shortcut: Option<&str>, mode: Option<i64>| -> Result<Item, ConsoleError> {
            Ok(InputOption::simple(name, shortcut, mode, "")?.into())
        };
    let argument = |name: &str, mode: Option<i64>| -> Result<Item, ConsoleError> {
        Ok(InputArgument::simple(name, mode, "")?.into())
    };
    let cases: Vec<(Vec<Item>, &str, &str)> = vec![
        (
            vec![option("foo", None, None)?],
            "[--foo]",
            "puts optional options in square brackets",
        ),
        (
            vec![option("foo", Some("f"), None)?],
            "[-f|--foo]",
            "separates shortcut with a pipe",
        ),
        (
            vec![option("foo", Some("f"), Some(VALUE_REQUIRED))?],
            "[-f|--foo FOO]",
            "uses shortcut as value placeholder",
        ),
        (
            vec![option("foo", Some("f"), Some(VALUE_OPTIONAL))?],
            "[-f|--foo [FOO]]",
            "puts optional values in square brackets",
        ),
        (
            vec![argument("foo", Some(REQUIRED))?],
            "<foo>",
            "puts arguments in angle brackets",
        ),
        (
            vec![argument("foo", None)?],
            "[<foo>]",
            "puts optional arguments in square brackets",
        ),
        (
            vec![argument("foo", None)?, argument("bar", None)?],
            "[<foo> [<bar>]]",
            "chains optional arguments inside brackets",
        ),
        (
            vec![argument("foo", Some(IS_ARRAY))?],
            "[<foo>...]",
            "uses an ellipsis for array arguments",
        ),
        (
            vec![argument("foo", Some(REQUIRED | IS_ARRAY))?],
            "<foo>...",
            "uses an ellipsis for required array arguments",
        ),
        (
            vec![option("foo", None, None)?, argument("foo", Some(REQUIRED))?],
            "[--foo] [--] <foo>",
            "puts [--] between options and arguments",
        ),
    ];
    for (items, expected, message) in cases {
        assert_eq!(
            expected,
            InputDefinition::new(items)?.get_synopsis(false),
            "->getSynopsis() {message}"
        );
    }
    Ok(())
}

#[test]
fn testGetShortSynopsis() -> R {
    let definition = InputDefinition::new(vec![
        InputOption::simple("foo", None, None, "")?.into(),
        InputOption::simple("bar", None, None, "")?.into(),
        InputArgument::simple("cat", None, "")?.into(),
    ])?;
    assert_eq!("[options] [--] [<cat>]", definition.get_synopsis(true));
    Ok(())
}
