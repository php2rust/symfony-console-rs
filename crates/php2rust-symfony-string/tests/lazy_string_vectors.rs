//! `LazyString` where `LazyStringTest` leaves it open, against the answers
//! recorded from PHP (`tests/data/lazy.json`, `oracle/recorders/string_lazy.php`):
//! each kind of return value, the `TypeError` texts with each kind of
//! callable's name, retries and memoization, the `fromCallable()` array
//! check, `fromStringable()`, `resolve()`, `isStringable()`, JSON and
//! `serialize()`. Every recorded case is replayed, or listed in
//! [`NOT_EXPRESSIBLE`] with the reason.
//!
//! Upstream: none
#![allow(
    clippy::cast_possible_wrap,
    clippy::items_after_statements,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    reason = "test code ports PHPUnit tests and helpers as they are"
)]

use std::{
    collections::BTreeMap,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};

use php_value::{
    Key, PhpArray, PhpString, Value,
    constants::Registry,
    json::{JsonFlags, encode_string},
    object::{EnumCase, GenericObject, StdClass},
    serialize::{Classes, serialize, unserialize},
};
use serde_json::Value as Json;
use symfony_string::{
    LazyString,
    lazy_string::{CLASS, Callable, Invokable, Returned, Stringable, Thrown},
};

const DATA: &str = include_str!("data/lazy.json");

/// Recorded cases the typed API cannot build: PHP decides them from the
/// runtime type of a callable array, which a Rust caller cannot pass.
const NOT_EXPRESSIBLE: &[(&str, &str)] = &[
    (
        "lazy callable factory returning a string",
        "a factory returns an Invokable, not a class name",
    ),
    (
        "fromCallable three ints",
        "an array of three ints is not a Callable",
    ),
    (
        "fromCallable Closure and int",
        "a lazy-callable's method is a string",
    ),
];

/// A PHP exception a callback throws.
#[derive(Debug)]
struct PhpException {
    class: &'static str,
    message: &'static str,
}

impl fmt::Display for PhpException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for PhpException {}

fn throw(class: &'static str, message: &'static str) -> Thrown {
    Thrown::callback(PhpException { class, message })
}

/// `["string", s]` or `["throws", class, message]`, as recorded.
#[derive(Debug, PartialEq)]
enum Outcome {
    String(String),
    Throws(String, String),
}

fn outcome(result: Result<PhpString, Thrown>) -> Outcome {
    match result {
        Ok(s) => Outcome::String(s.to_string_lossy().into_owned()),
        Err(Thrown::Callback(e)) => match e.downcast_ref::<PhpException>() {
            Some(p) => Outcome::Throws(p.class.to_owned(), p.message.to_owned()),
            None => Outcome::Throws("?".to_owned(), e.to_string()),
        },
        Err(e) => Outcome::Throws(e.class().unwrap_or("?").to_owned(), e.message()),
    }
}

fn cast(s: &LazyString) -> Outcome {
    outcome(s.to_php_string())
}

/// The recorder's `Text`, a Stringable.
struct Text(&'static str);

impl Stringable for Text {
    fn to_php_string(&self) -> Result<PhpString, Thrown> {
        Ok(PhpString::from(self.0))
    }
}

/// The recorder's `Named` class.
struct Named;

impl Invokable for Named {
    fn debug_type(&self) -> String {
        "Named".to_owned()
    }

    fn call(&self, method: &str, _: &[Value]) -> Result<Returned, Thrown> {
        match method {
            "method" | "__invoke" => Ok(Returned::from(PhpArray::new())),
            "text" => Ok(Returned::from("text")),
            other => Err(Thrown::undefined_method("Named", other)),
        }
    }
}

/// The recorder's `Invokable` class.
struct Counter(AtomicI64);

impl Invokable for Counter {
    fn debug_type(&self) -> String {
        "Invokable".to_owned()
    }

    fn call(&self, method: &str, args: &[Value]) -> Result<Returned, Thrown> {
        match method {
            "__invoke" => Ok(Returned::from(format!(
                "invoked {}",
                self.0.fetch_add(1, Ordering::SeqCst) + 1
            ))),
            "other" => {
                let a = args
                    .first()
                    .and_then(Value::to_php_string)
                    .unwrap_or_else(|| PhpString::from("x"));
                Ok(Returned::from(format!("other {}", a.to_string_lossy())))
            }
            other => Err(Thrown::undefined_method("Invokable", other)),
        }
    }
}

/// The anonymous class whose `__invoke()` throws once.
struct FailsOnce(AtomicI64);

impl Invokable for FailsOnce {
    fn debug_type(&self) -> String {
        "class@anonymous".to_owned()
    }

    fn call(&self, _: &str, _: &[Value]) -> Result<Returned, Thrown> {
        let n = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        if n == 1 {
            return Err(throw("RuntimeException", "invoke failed"));
        }
        Ok(Returned::from(format!("invoke {n}")))
    }
}

fn returning(value: Returned) -> LazyString {
    // the recorder's `static fn () => $value` inside `$case`'s closure
    let name = "{closure:{closure:FILE:98}:98}";
    LazyString::from_callable(Callable::new(name, move |_| Ok(value.clone())), Vec::new())
}

fn object(class: &str) -> Value {
    Value::Object(Arc::new(GenericObject::new(class, PhpArray::new())))
}

fn suit() -> Value {
    Value::Object(EnumCase::new("Suit", "Hearts", None).object())
}

fn list(values: Vec<Value>) -> PhpArray {
    let mut array = PhpArray::new();
    for (i, v) in values.into_iter().enumerate() {
        array.insert(Key::Int(i as i64), v);
    }
    array
}

/// The Rust side of every case, in the recorder's order.
fn replay() -> Vec<(String, Outcome)> {
    let mut out: Vec<(String, Outcome)> = Vec::new();
    let mut case = |name: &str, outcome: Outcome| out.push((name.to_owned(), outcome));

    let returns: Vec<(&str, Returned)> = vec![
        ("int", Returned::from(42i64)),
        ("negative int", Returned::from(-7i64)),
        ("int max", Returned::from(i64::MAX)),
        ("float", Returned::from(123.456)),
        ("float 0.1+0.2", Returned::from(0.1 + 0.2)),
        ("float 1e100", Returned::from(1e100)),
        ("float 1e15", Returned::from(1e15)),
        ("float 1/3", Returned::from(1.0 / 3.0)),
        ("float 1e14", Returned::from(1e14)),
        ("float -1.5e-7", Returned::from(-1.5e-7)),
        ("float -0.0", Returned::from(-0.0)),
        ("float 1.0", Returned::from(1.0)),
        ("float INF", Returned::from(f64::INFINITY)),
        ("float NAN", Returned::from(f64::NAN)),
        ("true", Returned::from(true)),
        ("false", Returned::from(false)),
        ("null", Returned::from(Value::Null)),
        ("empty string", Returned::from("")),
        ("string", Returned::from("abc")),
        ("array", Returned::from(PhpArray::new())),
        (
            "list",
            Returned::from(list(vec![Value::from(1), Value::from(2)])),
        ),
        (
            "stdClass",
            Returned::from(StdClass::object(PhpArray::new())),
        ),
        (
            "Stringable",
            Returned::Stringable(Arc::new(Text("stringable"))),
        ),
        ("enum", Returned::from(suit())),
        (
            "LazyString",
            Returned::from(LazyString::from_callable(
                Callable::new("{closure}", |_| Ok(Returned::from("inner"))),
                Vec::new(),
            )),
        ),
    ];
    for (name, value) in returns {
        case(&format!("returns {name}"), cast(&returning(value)));
    }

    let array = |_: &[Value]| Ok(Returned::from(PhpArray::new()));
    let callable = |name: &str| {
        cast(&LazyString::from_callable(
            Callable::new(name, array),
            Vec::new(),
        ))
    };
    case("string callable returning array", callable("returns_array"));
    case(
        "static method string returning string",
        cast(&LazyString::from_callable(
            Callable::method("Named", "make", |_| Ok(Returned::from("made"))),
            Vec::new(),
        )),
    );
    case("array callable returning array", callable("Named::method"));
    case(
        "static array callable",
        cast(&LazyString::from_callable(
            Callable::method("Named", "make", |_| Ok(Returned::from("made"))),
            Vec::new(),
        )),
    );
    case(
        "first-class method callable returning array",
        callable("Named::method"),
    );
    case(
        "first-class function callable returning array",
        callable("returns_array"),
    );
    case(
        "invokable object returning array",
        callable("Named::__invoke"),
    );
    let named = || Ok(Arc::new(Named) as Arc<dyn Invokable>);
    case(
        "lazy callable returning array",
        cast(&LazyString::from_lazy_callable(named, None, Vec::new())),
    );
    case(
        "lazy callable with method returning array",
        cast(&LazyString::from_lazy_callable(
            named,
            Some("method"),
            Vec::new(),
        )),
    );
    case(
        "lazy callable with method returning string",
        cast(&LazyString::from_lazy_callable(
            named,
            Some("text"),
            Vec::new(),
        )),
    );
    let counter = || Ok(Arc::new(Counter(AtomicI64::new(0))) as Arc<dyn Invokable>);
    case(
        "lazy callable invokable",
        cast(&LazyString::from_lazy_callable(counter, None, Vec::new())),
    );
    case(
        "lazy callable with arguments",
        cast(&LazyString::from_lazy_callable(
            counter,
            Some("other"),
            vec![Value::from("arg")],
        )),
    );
    case(
        "lazy callable with an undefined method",
        cast(&LazyString::from_lazy_callable(
            counter,
            Some("missing"),
            Vec::new(),
        )),
    );
    case(
        "callable with arguments",
        cast(&LazyString::from_callable(
            Callable::new("sprintf", |args: &[Value]| {
                let parts: Vec<String> = args[1..]
                    .iter()
                    .filter_map(Value::to_php_string)
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect();
                Ok(Returned::from(parts.join("-")))
            }),
            vec![Value::from("%s-%s"), Value::from("a"), Value::from("b")],
        )),
    );
    case(
        "callable with arguments returning array",
        cast(&LazyString::from_callable(
            Callable::new("str_split", array),
            vec![Value::from("ab")],
        )),
    );
    case(
        "Closure::fromCallable of a function returning array",
        callable("returns_array"),
    );
    case(
        "exception from the callback",
        cast(&LazyString::from_callable(
            Callable::new("{closure}", |_| Err(throw("RuntimeException", "boom"))),
            Vec::new(),
        )),
    );
    case(
        "TypeError from the callback",
        cast(&LazyString::from_callable(
            Callable::new("{closure}", |_| Err(Thrown::TypeError("own".to_owned()))),
            Vec::new(),
        )),
    );

    let closure = object("Closure");
    for (name, items) in [
        ("empty array", vec![]),
        ("string list", vec![Value::from("foo")]),
        ("ints", vec![Value::from(1), Value::from(2)]),
        (
            "object and method",
            vec![StdClass::object(PhpArray::new()), Value::from("x")],
        ),
        ("null", vec![Value::Null]),
        (
            "nested array",
            vec![Value::from(list(vec![Value::from(1)])), Value::from("x")],
        ),
        ("bool and float", vec![Value::from(true), Value::from(1.5)]),
    ] {
        let check = LazyString::check_callable_array(&list(items), false);
        let Err(e) = check else {
            panic!("fromCallable {name}: accepted");
        };
        case(&format!("fromCallable {name}"), outcome(Err(e)));
    }
    // A Closure first, or more than two items, passes the check.
    assert!(LazyString::check_callable_array(&list(vec![closure.clone()]), false).is_ok());
    assert!(LazyString::check_callable_array(&list(vec![Value::from(1); 3]), false).is_ok());

    let count = Arc::new(AtomicI64::new(0));
    let c = count.clone();
    let memo = LazyString::from_callable(
        Callable::new("{closure}", move |_| {
            Ok(Returned::from(format!(
                "call {}",
                c.fetch_add(1, Ordering::SeqCst) + 1
            )))
        }),
        Vec::new(),
    );
    case("memoized first", cast(&memo));
    case("memoized second", cast(&memo));
    let tries = Arc::new(AtomicI64::new(0));
    let t = tries.clone();
    let retry = LazyString::from_callable(
        Callable::new("{closure}", move |_| {
            let n = t.fetch_add(1, Ordering::SeqCst) + 1;
            if n == 1 {
                return Err(throw("RuntimeException", "first try"));
            }
            Ok(Returned::from(format!("try {n}")))
        }),
        Vec::new(),
    );
    case("retry first", cast(&retry));
    case("retry second", cast(&retry));
    case("retry third", cast(&retry));
    let calls = Arc::new(AtomicI64::new(0));
    let k = calls.clone();
    let bad = LazyString::from_callable(
        Callable::new("{closure:FILE:154}", move |_| {
            k.fetch_add(1, Ordering::SeqCst);
            Ok(Returned::from(PhpArray::new()))
        }),
        Vec::new(),
    );
    case("type error first", cast(&bad));
    case("type error second", cast(&bad));
    case(
        "type error callback calls",
        Outcome::String(calls.load(Ordering::SeqCst).to_string()),
    );
    let invokable: Arc<dyn Invokable> = Arc::new(Counter(AtomicI64::new(0)));
    let lazy = LazyString::from_lazy_callable(move || Ok(invokable.clone()), None, Vec::new());
    case("lazy factory memoized first", cast(&lazy));
    case("lazy factory memoized second", cast(&lazy));
    let factory_calls = Arc::new(AtomicI64::new(0));
    let f = factory_calls.clone();
    let throwing = LazyString::from_lazy_callable(
        move || {
            f.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(FailsOnce(AtomicI64::new(0))) as Arc<dyn Invokable>)
        },
        None,
        Vec::new(),
    );
    case("lazy factory, invoke throws first", cast(&throwing));
    case("lazy factory, invoke throws second", cast(&throwing));
    case(
        "lazy factory calls",
        Outcome::String(factory_calls.load(Ordering::SeqCst).to_string()),
    );

    enum Input {
        Value(Value),
        Stringable(Arc<dyn Stringable>),
    }
    let inputs = vec![
        ("string", Input::Value(Value::from("abc"))),
        ("int", Input::Value(Value::from(123))),
        ("float", Input::Value(Value::from(123.456))),
        ("true", Input::Value(Value::from(true))),
        ("false", Input::Value(Value::from(false))),
        ("float 1e100", Input::Value(Value::from(1e100))),
        ("float 0.1+0.2", Input::Value(Value::from(0.1 + 0.2))),
        ("float 1/3", Input::Value(Value::from(1.0 / 3.0))),
        ("float 1e14", Input::Value(Value::from(1e14))),
        ("float 0.00001", Input::Value(Value::from(0.00001))),
        ("float -0.0", Input::Value(Value::from(-0.0))),
        ("Stringable", Input::Stringable(Arc::new(Text("text")))),
        (
            "LazyString",
            Input::Stringable(Arc::new(LazyString::from_callable(
                Callable::new("{closure}", |_| Ok(Returned::from("lazy"))),
                Vec::new(),
            ))),
        ),
        ("null", Input::Value(Value::Null)),
        ("array", Input::Value(Value::from(PhpArray::new()))),
        ("stdClass", Input::Value(StdClass::object(PhpArray::new()))),
        ("enum", Input::Value(suit())),
    ];
    for (name, input) in inputs {
        let (from, resolved) = match &input {
            Input::Value(v) => (
                LazyString::from_stringable_value(v).and_then(|s| s.to_php_string()),
                LazyString::resolve(v),
            ),
            Input::Stringable(s) => (
                LazyString::from_stringable(s.clone()).to_php_string(),
                LazyString::resolve_stringable(s.as_ref()),
            ),
        };
        case(&format!("fromStringable {name}"), outcome(from));
        case(&format!("resolve {name}"), outcome(resolved));
    }
    out
}

/// PHP's `TypeError` for a user function's argument ends with ", called in
/// FILE on line N", which a Rust caller has no file or line for.
fn without_called_in(message: &str) -> &str {
    match message.rfind(", called in FILE on line ") {
        Some(at) => &message[..at],
        None => message,
    }
}

fn recorded(json: &Json) -> Outcome {
    let text = |i: usize| json[i].as_str().unwrap_or_default().to_owned();
    match json[0].as_str() {
        Some("string") => Outcome::String(text(1)),
        Some("throws") => Outcome::Throws(text(1), without_called_in(&text(2)).to_owned()),
        other => panic!("unexpected outcome {other:?}"),
    }
}

#[test]
fn cases_match_php() {
    let data: Json = serde_json::from_str(DATA).unwrap_or_else(|e| panic!("data: {e}"));
    let rust: BTreeMap<String, Outcome> = replay().into_iter().collect();
    let cases = data["cases"]
        .as_array()
        .unwrap_or_else(|| panic!("no cases"));
    let mut replayed = 0;
    for case in cases {
        let name = case[0].as_str().unwrap_or_default();
        let expected = recorded(&case[1]);
        if NOT_EXPRESSIBLE.iter().any(|(n, _)| *n == name) {
            assert!(!rust.contains_key(name), "{name} is listed but replayed");
            continue;
        }
        let got = rust
            .get(name)
            .unwrap_or_else(|| panic!("recorded case {name:?} is not replayed"));
        assert_eq!(&expected, got, "{name}");
        replayed += 1;
    }
    assert_eq!(replayed, rust.len(), "a replayed case was not recorded");
    assert_eq!(replayed + NOT_EXPRESSIBLE.len(), cases.len());
}

#[test]
fn is_stringable_matches_php() {
    let data: Json = serde_json::from_str(DATA).unwrap_or_else(|e| panic!("data: {e}"));
    for row in data["isStringable"].as_array().into_iter().flatten() {
        let name = row[0].as_str().unwrap_or_default();
        let expected = row[1].as_bool().unwrap_or_default();
        let value = match name {
            "string" => Value::from("a"),
            "empty string" => Value::from(""),
            "int" => Value::from(0),
            "float" => Value::from(0.0),
            "true" => Value::from(true),
            "false" => Value::from(false),
            "null" => Value::Null,
            "array" => Value::from(PhpArray::new()),
            "stdClass" => StdClass::object(PhpArray::new()),
            "enum" => suit(),
            "Closure" => object("Closure"),
            // Stringable by type; php-value has no resources.
            "Stringable" | "LazyString" => {
                assert!(expected, "{name}");
                continue;
            }
            "resource" => {
                assert!(!expected, "{name}");
                continue;
            }
            other => panic!("unexpected isStringable case {other}"),
        };
        assert_eq!(expected, LazyString::is_stringable(&value), "{name}");
    }
}

fn text(name: &str) -> &'static str {
    match name {
        "abc" => "abc",
        "unicode" => "é/\"",
        "empty" => "",
        other => panic!("unexpected case {other}"),
    }
}

#[test]
fn json_and_serialize_match_php() {
    let data: Json = serde_json::from_str(DATA).unwrap_or_else(|e| panic!("data: {e}"));
    for row in data["json"].as_array().into_iter().flatten() {
        let name = row[0].as_str().unwrap_or_default();
        let t = text(name);
        let s = LazyString::from_callable(
            Callable::new("{closure}", move |_| Ok(Returned::from(t))),
            Vec::new(),
        );
        let encoded = s
            .json_serialize()
            .ok()
            .and_then(|s| encode_string(s.as_bytes(), JsonFlags::default()));
        assert_eq!(row[1].as_str(), encoded.as_deref(), "{name}");
        let listed = encoded.map(|e| format!("[{e}]"));
        assert_eq!(row[2].as_str(), listed.as_deref(), "{name}");
    }
    for row in data["serialize"].as_array().into_iter().flatten() {
        let name = row[0].as_str().unwrap_or_default();
        let t = text(name);
        let s = LazyString::from_callable(
            Callable::new("{closure}", move |_| Ok(Returned::from(t))),
            Vec::new(),
        );
        let fields = s.serialize_data().unwrap_or_else(|e| panic!("{e}"));
        let serialized = serialize(&Value::Object(Arc::new(GenericObject::new(CLASS, fields))));
        assert_eq!(
            row[1].as_str().unwrap_or_default().as_bytes(),
            serialized.as_bytes(),
            "{name}"
        );
        let mut classes = Classes::new();
        classes.allow(CLASS);
        let Some(Value::Object(o)) =
            unserialize(serialized.as_bytes(), &classes, &Registry::default())
        else {
            panic!("{name}: unserialize failed");
        };
        assert_eq!(CLASS, o.class());
        let restored = LazyString::from_serialized(&o.properties())
            .unwrap_or_else(|| panic!("{name}: no value"));
        assert_eq!(
            Some(row[2].as_str().unwrap_or_default()),
            restored
                .to_php_string()
                .ok()
                .map(|s| s.to_string_lossy().into_owned())
                .as_deref(),
            "{name}"
        );
        assert_eq!(Some(true), row[3].as_bool());
    }
}

/// A cast from inside the callback is refused, where PHP recurses until
/// it runs out of stack.
#[test]
fn recursive_cast_is_refused() {
    let slot: Arc<std::sync::Mutex<Option<LazyString>>> = Arc::default();
    let inner = slot.clone();
    let s = LazyString::from_callable(
        Callable::new("{closure}", move |_| {
            let me = inner.lock().ok().and_then(|g| g.clone());
            match me {
                Some(me) => Ok(Returned::from(me.to_php_string()?)),
                None => Ok(Returned::from("none")),
            }
        }),
        Vec::new(),
    );
    if let Ok(mut g) = slot.lock() {
        *g = Some(s.clone());
    }
    assert!(matches!(s.to_php_string(), Err(Thrown::Recursion)));
    // and nothing is kept: the next cast refuses again rather than hang
    assert!(matches!(s.to_php_string(), Err(Thrown::Recursion)));
}
