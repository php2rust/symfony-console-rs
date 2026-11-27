//! `InputArgumentTest` and `InputOptionTest`, with upstream's names and
//! assertions. `expectUserDeprecationMessage` is checked with
//! `symfony_deprecation_contracts::collect`.
//!
//! Upstream: symfony/console Tests/Input/InputArgumentTest.php Tests/Input/InputOptionTest.php
#![allow(non_snake_case)]

use std::sync::Arc;

use php_value::{PhpArray, Value, object::GenericObject};
use symfony_console::{
    ConsoleError,
    completion::{completion_input::CompletionInput, suggestion::CompletionSuggestions},
    input::{
        SuggestedValues,
        input_argument::{IS_ARRAY, InputArgument, OPTIONAL, REQUIRED},
        input_option::{
            InputOption, Shortcut, VALUE_IS_ARRAY, VALUE_NEGATABLE, VALUE_NONE, VALUE_OPTIONAL,
            VALUE_REQUIRED,
        },
    },
};
use symfony_deprecation_contracts::collect;

type R = Result<(), ConsoleError>;

fn argument(
    name: &str,
    mode: Option<i64>,
    default: Value,
    suggested: SuggestedValues,
) -> Result<InputArgument, ConsoleError> {
    InputArgument::new(name, mode, "", default, suggested)
}

fn option(
    name: &str,
    shortcut: Shortcut<'_>,
    mode: Option<i64>,
    default: Value,
) -> Result<InputOption, ConsoleError> {
    InputOption::new(
        name,
        shortcut,
        mode,
        "",
        default,
        SuggestedValues::default(),
    )
}

fn list(values: &[i64]) -> Value {
    Value::Array(PhpArray::list(values.iter().map(|&v| Value::Int(v))))
}

fn date(text: &str) -> Value {
    Value::Object(Arc::new(GenericObject::new(
        "DateTimeImmutable",
        PhpArray::list([Value::string(text)]),
    )))
}

fn suggested_values(suggestions: &CompletionSuggestions) -> Vec<&str> {
    suggestions
        .get_value_suggestions()
        .iter()
        .map(symfony_console::completion::suggestion::Suggestion::get_value)
        .collect()
}

fn closure(value: Value) -> SuggestedValues {
    SuggestedValues::Closure(Arc::new(move |_| value.clone()))
}

fn expect(
    result: Result<impl std::fmt::Debug, ConsoleError>,
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

// InputArgumentTest

#[test]
fn testConstructor() -> R {
    assert_eq!(
        "foo",
        InputArgument::simple("foo", None, "")?.get_name(),
        "__construct() takes a name as its first argument"
    );
    Ok(())
}

#[test]
fn testModes() -> R {
    assert!(!InputArgument::simple("foo", None, "")?.is_required());
    assert!(!InputArgument::simple("foo", None, "")?.is_required());
    assert!(!InputArgument::simple("foo", Some(OPTIONAL), "")?.is_required());
    assert!(InputArgument::simple("foo", Some(REQUIRED), "")?.is_required());
    Ok(())
}

#[test]
fn testInvalidModes() {
    expect(
        InputArgument::simple("foo", Some(-1), ""),
        invalid,
        "Argument mode \"-1\" is not valid.",
    );
}

#[test]
fn testAmbiguousRequirementSpecifierMode() {
    let (_, deprecations) = collect(|| InputArgument::simple("foo", Some(OPTIONAL | REQUIRED), ""));
    let messages: Vec<String> = deprecations
        .iter()
        .map(|d| String::from_utf8_lossy(&d.message).into_owned())
        .collect();
    assert!(
        messages.iter().any(|m| m == "Since symfony/console 8.1: Argument \"foo\" mode should specify either required or optional."),
        "{messages:?}"
    );
}

#[test]
fn testIsArray() -> R {
    assert!(InputArgument::simple("foo", Some(IS_ARRAY), "")?.is_array());
    assert!(InputArgument::simple("foo", Some(OPTIONAL | IS_ARRAY), "")?.is_array());
    assert!(!InputArgument::simple("foo", Some(OPTIONAL), "")?.is_array());
    Ok(())
}

#[test]
fn testGetDescription() -> R {
    assert_eq!(
        "Some description",
        InputArgument::simple("foo", None, "Some description")?.get_description()
    );
    Ok(())
}

#[test]
fn testGetDefault() -> R {
    let a = argument(
        "foo",
        Some(OPTIONAL),
        Value::string("default"),
        SuggestedValues::default(),
    )?;
    assert_eq!(&Value::string("default"), a.get_default());
    Ok(())
}

#[test]
fn testSetDefault() -> R {
    let mut a = argument(
        "foo",
        Some(OPTIONAL),
        Value::string("default"),
        SuggestedValues::default(),
    )?;
    a.set_default(Value::Null)?;
    assert!(
        a.get_default().is_null(),
        "->setDefault() can reset the default value by passing null"
    );
    a.set_default(Value::string("another"))?;
    assert_eq!(&Value::string("another"), a.get_default());
    let mut a = InputArgument::simple("foo", Some(OPTIONAL | IS_ARRAY), "")?;
    a.set_default(list(&[1, 2]))?;
    assert_eq!(&list(&[1, 2]), a.get_default());
    Ok(())
}

#[test]
fn testSetDefaultWithObject() -> R {
    let default = date("2024-01-01");
    let mut a = argument(
        "foo",
        Some(OPTIONAL),
        default.clone(),
        SuggestedValues::default(),
    )?;
    assert!(
        default.identical(a.get_default()),
        "->getDefault() returns the object default value"
    );
    let new_default = date("2024-06-01");
    a.set_default(new_default.clone())?;
    assert!(
        new_default.identical(a.get_default()),
        "->setDefault() changes the default value to an object"
    );
    Ok(())
}

#[test]
fn testSetDefaultWithRequiredArgument() -> R {
    let mut a = InputArgument::simple("foo", Some(REQUIRED), "")?;
    expect(
        a.set_default(Value::string("default")),
        logic,
        "Cannot set a default value except for InputArgument::OPTIONAL mode.",
    );
    Ok(())
}

#[test]
fn testSetDefaultWithRequiredArrayArgument() -> R {
    let mut a = InputArgument::simple("foo", Some(REQUIRED | IS_ARRAY), "")?;
    expect(
        a.set_default(list(&[])),
        logic,
        "Cannot set a default value except for InputArgument::OPTIONAL mode.",
    );
    Ok(())
}

#[test]
fn testSetDefaultWithArrayArgument() -> R {
    let mut a = InputArgument::simple("foo", Some(IS_ARRAY), "")?;
    expect(
        a.set_default(Value::string("default")),
        logic,
        "A default value for an array argument must be an array.",
    );
    Ok(())
}

#[test]
fn testCompleteArray() -> R {
    let a = argument(
        "foo",
        None,
        Value::Null,
        SuggestedValues::list(["foo", "bar"]),
    )?;
    assert!(a.has_completion());
    let mut suggestions = CompletionSuggestions::default();
    a.complete(&CompletionInput::from_tokens(vec![], 0), &mut suggestions)?;
    assert_eq!(vec!["foo", "bar"], suggested_values(&suggestions));
    Ok(())
}

#[test]
fn testCompleteClosure() -> R {
    let values = Value::Array(PhpArray::list([Value::string("foo"), Value::string("bar")]));
    let a = argument("foo", None, Value::Null, closure(values))?;
    assert!(a.has_completion());
    let mut suggestions = CompletionSuggestions::default();
    a.complete(&CompletionInput::from_tokens(vec![], 0), &mut suggestions)?;
    assert_eq!(vec!["foo", "bar"], suggested_values(&suggestions));
    Ok(())
}

#[test]
fn testCompleteClosureReturnIncorrectType() -> R {
    let a = argument(
        "foo",
        Some(OPTIONAL),
        Value::Null,
        closure(Value::string("invalid")),
    )?;
    expect(
        a.complete(
            &CompletionInput::from_tokens(vec![], 0),
            &mut CompletionSuggestions::default(),
        ),
        logic,
        "Closure for argument \"foo\" must return an array. Got \"string\".",
    );
    Ok(())
}

// InputOptionTest

#[test]
fn testConstructor_InputOption() -> R {
    assert_eq!(
        "foo",
        InputOption::simple("foo", None, None, "")?.get_name()
    );
    assert_eq!(
        "foo",
        InputOption::simple("--foo", None, None, "")?.get_name(),
        "__construct() removes the leading -- of the option name"
    );
    Ok(())
}

#[test]
fn testArrayModeWithoutValue() {
    expect(
        InputOption::simple("foo", Some("f"), Some(VALUE_IS_ARRAY), ""),
        invalid,
        "Impossible to have an option mode VALUE_IS_ARRAY if the option does not accept a value.",
    );
}

#[test]
fn testBooleanWithRequired() {
    expect(
        InputOption::simple("foo", Some("f"), Some(VALUE_REQUIRED | VALUE_NEGATABLE), ""),
        invalid,
        "Impossible to have an option mode VALUE_NEGATABLE if the option also accepts a value.",
    );
}

#[test]
fn testBooleanWithOptional() {
    expect(
        InputOption::simple("foo", Some("f"), Some(VALUE_OPTIONAL | VALUE_NEGATABLE), ""),
        invalid,
        "Impossible to have an option mode VALUE_NEGATABLE if the option also accepts a value.",
    );
}

#[test]
fn testShortcut() -> R {
    let s = |shortcut: Shortcut<'_>| -> Result<Option<String>, ConsoleError> {
        Ok(option("foo", shortcut, None, Value::Null)?
            .get_shortcut()
            .map(str::to_owned))
    };
    assert_eq!(Some("f".to_owned()), s(Shortcut::One("f"))?);
    assert_eq!(Some("f|ff|fff".to_owned()), s(Shortcut::One("-f|-ff|fff"))?);
    assert_eq!(
        Some("f|ff|fff".to_owned()),
        s(Shortcut::Many(&["f", "ff", "-fff"]))?
    );
    assert_eq!(None, s(Shortcut::None)?);
    assert_eq!(None, s(Shortcut::One(""))?);
    assert_eq!(None, s(Shortcut::Many(&[]))?);
    assert_eq!(
        Some("f|fff".to_owned()),
        s(Shortcut::Many(&["f", "", "fff"]))?
    );
    assert_eq!(Some("f|fff".to_owned()), s(Shortcut::One("f||fff"))?);
    assert_eq!(Some("0".to_owned()), s(Shortcut::One("0"))?);
    assert_eq!(Some("0|z".to_owned()), s(Shortcut::Many(&["0", "z"]))?);
    assert_eq!(Some("0|z".to_owned()), s(Shortcut::One("0|z"))?);
    Ok(())
}

#[test]
fn testModes_InputOption() -> R {
    let flags = |o: InputOption| {
        (
            o.accept_value(),
            o.is_value_required(),
            o.is_value_optional(),
        )
    };
    assert_eq!(
        (false, false, false),
        flags(InputOption::simple("foo", Some("f"), None, "")?)
    );
    assert_eq!(
        (false, false, false),
        flags(InputOption::simple("foo", Some("f"), None, "")?)
    );
    assert_eq!(
        (false, false, false),
        flags(InputOption::simple("foo", Some("f"), Some(VALUE_NONE), "")?)
    );
    assert_eq!(
        (true, true, false),
        flags(InputOption::simple(
            "foo",
            Some("f"),
            Some(VALUE_REQUIRED),
            ""
        )?)
    );
    assert_eq!(
        (true, false, true),
        flags(InputOption::simple(
            "foo",
            Some("f"),
            Some(VALUE_OPTIONAL),
            ""
        )?)
    );
    Ok(())
}

#[test]
fn testInvalidModes_InputOption() {
    expect(
        InputOption::simple("foo", Some("f"), Some(-1), ""),
        invalid,
        "Option mode \"-1\" is not valid.",
    );
}

#[test]
fn testCombiningInvalidModesIsDeprecated() {
    for mode in [
        VALUE_NONE | VALUE_REQUIRED,
        VALUE_NONE | VALUE_OPTIONAL,
        VALUE_REQUIRED | VALUE_OPTIONAL,
        VALUE_NONE | VALUE_REQUIRED | VALUE_OPTIONAL,
    ] {
        let (_, deprecations) = collect(|| InputOption::simple("foo", Some("f"), Some(mode), ""));
        let messages: Vec<String> = deprecations
            .iter()
            .map(|d| String::from_utf8_lossy(&d.message).into_owned())
            .collect();
        assert!(
            messages.iter().any(|m| m == "Since symfony/console 8.1: Option \"foo\" mode should be either none, required or optional."),
            "{mode}: {messages:?}"
        );
    }
}

#[test]
fn testEmptyNameIsInvalid() {
    assert!(InputOption::simple("", None, None, "").is_err_and(|e| e.is_invalid_argument()));
}

#[test]
fn testDoubleDashNameIsInvalid() {
    assert!(InputOption::simple("--", None, None, "").is_err_and(|e| e.is_invalid_argument()));
}

#[test]
fn testSingleDashOptionIsInvalid() {
    assert!(
        InputOption::simple("foo", Some("-"), None, "").is_err_and(|e| e.is_invalid_argument())
    );
}

#[test]
fn testIsArray_InputOption() -> R {
    assert!(
        InputOption::simple("foo", None, Some(VALUE_OPTIONAL | VALUE_IS_ARRAY), "")?.is_array()
    );
    assert!(!InputOption::simple("foo", None, Some(VALUE_NONE), "")?.is_array());
    Ok(())
}

#[test]
fn testGetDescription_InputOption() -> R {
    assert_eq!(
        "Some description",
        InputOption::simple("foo", Some("f"), None, "Some description")?.get_description()
    );
    Ok(())
}

#[test]
fn testGetDefault_InputOption() -> R {
    assert_eq!(
        &Value::string("default"),
        option(
            "foo",
            Shortcut::None,
            Some(VALUE_OPTIONAL),
            Value::string("default")
        )?
        .get_default()
    );
    assert_eq!(
        &Value::string("default"),
        option(
            "foo",
            Shortcut::None,
            Some(VALUE_REQUIRED),
            Value::string("default")
        )?
        .get_default()
    );
    assert!(
        option("foo", Shortcut::None, Some(VALUE_REQUIRED), Value::Null)?
            .get_default()
            .is_null()
    );
    assert_eq!(
        &list(&[]),
        option(
            "foo",
            Shortcut::None,
            Some(VALUE_OPTIONAL | VALUE_IS_ARRAY),
            Value::Null
        )?
        .get_default()
    );
    assert_eq!(
        &Value::Bool(false),
        option("foo", Shortcut::None, Some(VALUE_NONE), Value::Null)?.get_default()
    );
    Ok(())
}

#[test]
fn testSetDefault_InputOption() -> R {
    let mut o = option(
        "foo",
        Shortcut::None,
        Some(VALUE_REQUIRED),
        Value::string("default"),
    )?;
    o.set_default(Value::Null)?;
    assert!(o.get_default().is_null());
    o.set_default(Value::string("another"))?;
    assert_eq!(&Value::string("another"), o.get_default());
    let mut o = option(
        "foo",
        Shortcut::None,
        Some(VALUE_REQUIRED | VALUE_IS_ARRAY),
        Value::Null,
    )?;
    o.set_default(list(&[1, 2]))?;
    assert_eq!(&list(&[1, 2]), o.get_default());
    let mut o = option(
        "foo",
        Shortcut::None,
        Some(VALUE_NONE | VALUE_NEGATABLE),
        Value::Null,
    )?;
    o.set_default(Value::Bool(true))?;
    assert_eq!(&Value::Bool(true), o.get_default());
    let mut o = option("foo", Shortcut::None, Some(VALUE_NEGATABLE), Value::Null)?;
    o.set_default(Value::Bool(false))?;
    assert_eq!(&Value::Bool(false), o.get_default());
    Ok(())
}

#[test]
fn testDefaultValueWithNegatableMode() -> R {
    let mut o = InputOption::simple("foo", Some("f"), Some(VALUE_NEGATABLE), "")?;
    expect(
        o.set_default(Value::string("default")),
        logic,
        "A default value for a negatable option must be a boolean or null.",
    );
    Ok(())
}

#[test]
fn testSetDefaultWithObject_InputOption() -> R {
    let default = date("2024-01-01");
    let mut o = option("foo", Shortcut::None, Some(VALUE_REQUIRED), default.clone())?;
    assert!(default.identical(o.get_default()));
    let new_default = date("2024-06-01");
    o.set_default(new_default.clone())?;
    assert!(new_default.identical(o.get_default()));
    Ok(())
}

#[test]
fn testDefaultValueWithValueNoneMode() -> R {
    let mut o = InputOption::simple("foo", Some("f"), Some(VALUE_NONE), "")?;
    expect(
        o.set_default(Value::string("default")),
        logic,
        "Cannot set a default value when using InputOption::VALUE_NONE mode.",
    );
    Ok(())
}

#[test]
fn testDefaultValueWithIsArrayMode() -> R {
    let mut o = InputOption::simple("foo", Some("f"), Some(VALUE_OPTIONAL | VALUE_IS_ARRAY), "")?;
    expect(
        o.set_default(Value::string("default")),
        logic,
        "A default value for an array option must be an array.",
    );
    Ok(())
}

#[test]
fn testEquals() -> R {
    let o = InputOption::simple("foo", Some("f"), None, "Some description")?;
    assert!(o.equals(&InputOption::simple(
        "foo",
        Some("f"),
        None,
        "Alternative description"
    )?));
    let o = InputOption::simple("foo", Some("f"), Some(VALUE_OPTIONAL), "Some description")?;
    let o2 = InputOption::new(
        "foo",
        Shortcut::One("f"),
        Some(VALUE_OPTIONAL),
        "Some description",
        Value::Bool(true),
        SuggestedValues::default(),
    )?;
    assert!(!o.equals(&o2));
    let o = InputOption::simple("foo", Some("f"), None, "Some description")?;
    assert!(!o.equals(&InputOption::simple(
        "bar",
        Some("f"),
        None,
        "Some description"
    )?));
    assert!(!o.equals(&InputOption::simple(
        "foo",
        Some(""),
        None,
        "Some description"
    )?));
    assert!(!o.equals(&InputOption::simple(
        "foo",
        Some("f"),
        Some(VALUE_OPTIONAL),
        "Some description"
    )?));
    Ok(())
}

#[test]
fn testSuggestedValuesErrorIfNoValue() {
    expect(
        InputOption::new(
            "foo",
            Shortcut::None,
            Some(VALUE_NONE),
            "",
            Value::Null,
            SuggestedValues::list(["foo"]),
        ),
        logic,
        "Cannot set suggested values if the option does not accept a value.",
    );
}

#[test]
fn testCompleteArray_InputOption() -> R {
    let o = InputOption::new(
        "foo",
        Shortcut::None,
        Some(VALUE_OPTIONAL),
        "",
        Value::Null,
        SuggestedValues::list(["foo", "bar"]),
    )?;
    assert!(o.has_completion());
    let mut suggestions = CompletionSuggestions::default();
    o.complete(&CompletionInput::from_tokens(vec![], 0), &mut suggestions)?;
    assert_eq!(vec!["foo", "bar"], suggested_values(&suggestions));
    Ok(())
}

#[test]
fn testCompleteClosure_InputOption() -> R {
    let values = Value::Array(PhpArray::list([Value::string("foo"), Value::string("bar")]));
    let o = InputOption::new(
        "foo",
        Shortcut::None,
        Some(VALUE_OPTIONAL),
        "",
        Value::Null,
        closure(values),
    )?;
    assert!(o.has_completion());
    let mut suggestions = CompletionSuggestions::default();
    o.complete(&CompletionInput::from_tokens(vec![], 0), &mut suggestions)?;
    assert_eq!(vec!["foo", "bar"], suggested_values(&suggestions));
    Ok(())
}

#[test]
fn testCompleteClosureReturnIncorrectType_InputOption() -> R {
    let o = InputOption::new(
        "foo",
        Shortcut::None,
        Some(VALUE_OPTIONAL),
        "",
        Value::Null,
        closure(Value::string("invalid")),
    )?;
    expect(
        o.complete(
            &CompletionInput::from_tokens(vec![], 0),
            &mut CompletionSuggestions::default(),
        ),
        logic,
        "Closure for option \"foo\" must return an array. Got \"string\".",
    );
    Ok(())
}
