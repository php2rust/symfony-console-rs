//! Port of symfony/string's `LazyStringTest`. PHP's closures are Rust
//! closures given with the name `getPrettyName()` would find; the
//! anonymous class of `testLazyCallable` is an [`Invokable`]; `\Exception`
//! (a Stringable) is a test type with PHP's `__toString()` layout.
//!
//! Upstream: symfony/string Tests/LazyStringTest.php
#![allow(non_snake_case)]
#![allow(
    clippy::items_after_statements,
    reason = "test code ports PHPUnit tests and helpers as they are"
)]

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use php_value::{PhpArray, PhpString, Value, object::StdClass};
use symfony_string::{
    LazyString,
    lazy_string::{Callable, Invokable, Returned, Stringable, Thrown},
};

/// `new \Exception($message)` as a Stringable: `(string) $e` is
/// "Exception: message in <file:line>" and the trace.
struct Exception(&'static str);

impl Stringable for Exception {
    fn to_php_string(&self) -> Result<PhpString, Thrown> {
        Ok(PhpString::from(format!(
            "Exception: {} in LazyStringTest.php:77\nStack trace:\n#0 {{main}}",
            self.0
        )))
    }
}

fn string(s: &LazyString) -> String {
    match s.to_php_string() {
        Ok(s) => s.to_string_lossy().into_owned(),
        Err(e) => panic!("{e:?}"),
    }
}

#[test]
fn testLazyString() {
    let count = Arc::new(AtomicI64::new(0));
    let counter = count.clone();
    let s = LazyString::from_callable(
        Callable::new("{closure:LazyStringTest.php:23}", move |_| {
            Ok(Returned::from(counter.fetch_add(1, Ordering::SeqCst) + 1))
        }),
        Vec::new(),
    );

    assert_eq!(0, count.load(Ordering::SeqCst));
    assert_eq!("1", string(&s));
    assert_eq!(1, count.load(Ordering::SeqCst));
}

#[test]
fn testReturnTypeError() {
    let s = LazyString::from_callable(
        Callable::new("{closure:LazyStringTest.php:35}", |_| {
            Ok(Returned::from(PhpArray::new()))
        }),
        Vec::new(),
    );

    let Err(Thrown::TypeError(message)) = s.to_php_string() else {
        panic!("expected a TypeError");
    };
    // '{^Return value of .*\{closure.*\}\(\) passed to '.preg_quote(LazyString::class).'::fromCallable\(\) must be of the type string, array returned\.$}'
    let rest = message
        .strip_prefix("Return value of ")
        .unwrap_or_else(|| panic!("{message}"));
    let at = rest.find("{closure").unwrap_or_else(|| panic!("{message}"));
    assert!(
        rest[at..].ends_with(
            "}() passed to Symfony\\Component\\String\\LazyString::fromCallable() must be of the type string, array returned."
        ),
        "{message}"
    );
}

/// The anonymous class: `__invoke()` increments the test's counter.
struct Counter(Arc<AtomicI64>);

impl Invokable for Counter {
    fn debug_type(&self) -> String {
        "class@anonymous".to_owned()
    }

    fn call(&self, method: &str, _: &[Value]) -> Result<Returned, Thrown> {
        match method {
            "__invoke" => Ok(Returned::from(self.0.fetch_add(1, Ordering::SeqCst) + 1)),
            other => Err(Thrown::undefined_method("class@anonymous", other)),
        }
    }
}

#[test]
fn testLazyCallable() {
    let count = Arc::new(AtomicI64::new(0));
    let counter = count.clone();
    let s = LazyString::from_lazy_callable(
        move || Ok(Arc::new(Counter(counter.clone())) as Arc<dyn Invokable>),
        None,
        Vec::new(),
    );

    assert_eq!(0, count.load(Ordering::SeqCst));
    assert_eq!("1", string(&s));
    assert_eq!(1, count.load(Ordering::SeqCst));
    assert_eq!("1", string(&s)); // ensure the value is memoized
    assert_eq!(1, count.load(Ordering::SeqCst));
}

#[test]
fn testFromStringable() {
    let from = |v: Value| LazyString::from_stringable_value(&v).unwrap_or_else(|e| panic!("{e}"));
    // assertInstanceOf(LazyString::class, ...): the return type
    let _: LazyString = from(Value::from("abc"));
    assert_eq!("abc", string(&from(Value::from("abc"))));
    assert_eq!("1", string(&from(Value::from(true))));
    assert_eq!("", string(&from(Value::from(false))));
    assert_eq!("123", string(&from(Value::from(123))));
    assert_eq!("123.456", string(&from(Value::from(123.456))));
    assert!(string(&LazyString::from_stringable(Arc::new(Exception("hello")))).contains("hello"));
}

#[test]
fn testResolve() {
    let resolve = |v: Value| {
        LazyString::resolve(&v)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_string_lossy()
            .into_owned()
    };
    assert_eq!("abc", resolve(Value::from("abc")));
    assert_eq!("1", resolve(Value::from(true)));
    assert_eq!("", resolve(Value::from(false)));
    assert_eq!("123", resolve(Value::from(123)));
    assert_eq!("123.456", resolve(Value::from(123.456)));
    let exception = LazyString::resolve_stringable(&Exception("hello")).unwrap_or_default();
    assert!(exception.to_string_lossy().contains("hello"));
}

#[test]
fn testIsStringable() {
    assert!(LazyString::is_stringable(&Value::from("abc")));
    assert!(LazyString::is_stringable(&Value::from(true)));
    assert!(LazyString::is_stringable(&Value::from(false)));
    assert!(LazyString::is_stringable(&Value::from(123)));
    assert!(LazyString::is_stringable(&Value::from(123.456)));
    // new \Exception('hello'): a Stringable is one by its type.
    fn stringable(_: &dyn Stringable) -> bool {
        true
    }
    assert!(stringable(&Exception("hello")));
}

#[test]
fn testIsNotStringable() {
    assert!(!LazyString::is_stringable(&Value::Null));
    assert!(!LazyString::is_stringable(&Value::from(PhpArray::new())));
    // \STDIN: php-value has no resources.
    assert!(!LazyString::is_stringable(&StdClass::object(
        PhpArray::new()
    )));
}
