//! Port of Symfony\Component\String\LazyString.php
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! A string computed on first use by a callback, then kept. PHP's
//! callables are values found by reflection; here the caller gives the
//! callback together with the name `getPrettyName()` would give it
//! ([`Callable`]), and a `[Closure, method]` lazy-callable is a factory of
//! an [`Invokable`] object ([`LazyString::from_lazy_callable`]). Upstream's
//! file has no `strict_types`, so the callback's value is coerced as a
//! `string` return type coerces it: null is `""`, scalars are cast
//! (floats with `precision = 14`), a Stringable is converted, and anything
//! else is the TypeError upstream rewrites. A callback that throws is
//! called again by the next cast, as upstream's closure is; a value that
//! fails the coercion fails again without a second call.
//!
//! Upstream: symfony/string LazyString.php

use std::{
    cell::RefCell,
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

use php_value::{Key, PhpArray, PhpString, Value, numeric::float_to_string};

/// `LazyString::class`.
pub const CLASS: &str = "Symfony\\Component\\String\\LazyString";

/// What a cast of a LazyString throws.
#[derive(Clone)]
pub enum Thrown {
    /// PHP's `\TypeError`: the rewritten return type error, a bad
    /// `fromCallable()` array, or a `fromStringable()` / `resolve()`
    /// argument.
    TypeError(String),
    /// PHP's `\Error` from calling an object (an undefined method).
    Error(String),
    /// What the callback (or a Stringable) threw, passed on unchanged.
    Callback(Arc<dyn std::error::Error + Send + Sync>),
    /// A cast of a LazyString from inside its own callback. PHP recurses
    /// until it runs out of stack; this port refuses.
    Recursion,
}

impl Thrown {
    /// Wraps what a callback throws.
    pub fn callback(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Callback(Arc::new(e))
    }

    /// PHP's `Call to undefined method Class::method()`.
    pub fn undefined_method(class: &str, method: &str) -> Self {
        Self::Error(format!("Call to undefined method {class}::{method}()"))
    }

    /// The PHP class of what is thrown, as `$e::class` gives it; a
    /// callback's own error is `None`.
    pub fn class(&self) -> Option<&'static str> {
        match self {
            Self::TypeError(_) => Some("TypeError"),
            Self::Error(_) | Self::Recursion => Some("Error"),
            Self::Callback(_) => None,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::TypeError(m) | Self::Error(m) => m.clone(),
            Self::Callback(e) => e.to_string(),
            Self::Recursion => "LazyString cast from inside its own callback".to_owned(),
        }
    }
}

impl fmt::Debug for Thrown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Callback(e) => write!(f, "Callback({e:?})"),
            other => write!(f, "{}({:?})", other.class().unwrap_or("?"), other.message()),
        }
    }
}

impl fmt::Display for Thrown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for Thrown {}

/// An object with `__toString()`.
pub trait Stringable: Send + Sync {
    fn to_php_string(&self) -> Result<PhpString, Thrown>;
}

/// What a callback returns.
#[derive(Clone)]
pub enum Returned {
    Value(Value),
    Stringable(Arc<dyn Stringable>),
}

impl fmt::Debug for Returned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(v) => write!(f, "{v:?}"),
            Self::Stringable(_) => f.write_str("Stringable"),
        }
    }
}

macro_rules! returned_from_value {
    ($($t:ty),*) => {$(
        impl From<$t> for Returned {
            fn from(value: $t) -> Self {
                Self::Value(value.into())
            }
        }
    )*};
}

returned_from_value!(Value, bool, i64, f64, &str, String, PhpString, PhpArray);

impl From<LazyString> for Returned {
    fn from(s: LazyString) -> Self {
        Self::Stringable(Arc::new(s))
    }
}

impl From<Arc<dyn Stringable>> for Returned {
    fn from(s: Arc<dyn Stringable>) -> Self {
        Self::Stringable(s)
    }
}

/// A callback and the arguments it is given.
pub type Callback = Box<dyn FnMut(&[Value]) -> Result<Returned, Thrown> + Send>;

/// A PHP callable: the function to run and the name `getPrettyName()`
/// gives it (a function name, `Class::method`, `Class::__invoke` for an
/// invokable object, or an anonymous closure's `{closure:...}` name).
pub struct Callable {
    name: String,
    call: Callback,
}

impl Callable {
    pub fn new(
        name: impl Into<String>,
        call: impl FnMut(&[Value]) -> Result<Returned, Thrown> + Send + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            call: Box::new(call),
        }
    }

    /// `[$object, 'method']`, `$object->method(...)` or `'Class::method'`.
    pub fn method(
        class: &str,
        method: &str,
        call: impl FnMut(&[Value]) -> Result<Returned, Thrown> + Send + 'static,
    ) -> Self {
        Self::new(format!("{class}::{method}"), call)
    }
}

/// The object a lazy-callable's factory returns.
pub trait Invokable: Send + Sync {
    /// `get_debug_type($object)`.
    fn debug_type(&self) -> String;

    /// `$object->$method(...$arguments)`; an undefined method is
    /// [`Thrown::undefined_method`].
    fn call(&self, method: &str, arguments: &[Value]) -> Result<Returned, Thrown>;
}

/// A lazy-callable's factory: `$callback[0]`.
pub type Factory = Box<dyn FnMut() -> Result<Arc<dyn Invokable>, Thrown> + Send>;

enum Target {
    Callable(Callable),
    Lazy {
        factory: Factory,
        method: String,
    },
    Object {
        object: Arc<dyn Invokable>,
        method: String,
    },
}

impl Target {
    /// `self::getPrettyName($callback)` once the callback has run.
    fn name(&self) -> String {
        match self {
            Self::Callable(c) => c.name.clone(),
            // Not reached: a lazy target becomes an object before it runs.
            Self::Lazy { method, .. } => format!("Closure::{method}"),
            Self::Object { object, method } => format!("{}::{method}", object.debug_type()),
        }
    }
}

enum State {
    Resolved(PhpString),
    Pending {
        target: Target,
        /// `$arguments`, until the callback has returned.
        arguments: Option<Vec<Value>>,
        /// The closure's `static $value`.
        value: Option<Returned>,
    },
}

/// `Symfony\Component\String\LazyString`. Clones are the same object.
#[derive(Clone)]
pub struct LazyString(Arc<Mutex<State>>);

impl fmt::Debug for LazyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.lock() {
            State::Resolved(s) => write!(f, "LazyString({s:?})"),
            State::Pending { .. } => f.write_str("LazyString(<pending>)"),
        }
    }
}

thread_local! {
    /// The LazyStrings this thread is casting, to refuse a recursive cast
    /// rather than wait on its own lock.
    static CASTING: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

struct Casting(usize);

impl Casting {
    fn enter(id: usize) -> Option<Self> {
        CASTING.with(|c| {
            let mut c = c.borrow_mut();
            if c.contains(&id) {
                None
            } else {
                c.push(id);
                Some(Self(id))
            }
        })
    }
}

impl Drop for Casting {
    fn drop(&mut self) {
        CASTING.with(|c| c.borrow_mut().retain(|&id| id != self.0));
    }
}

/// `$value ?? ''` through a `string` return type: the string, or the type
/// PHP's TypeError names.
fn coerce(value: Option<&Returned>) -> Result<Result<PhpString, String>, Thrown> {
    Ok(match value {
        None | Some(Returned::Value(Value::Null)) => Ok(PhpString::default()),
        Some(Returned::Value(Value::Float(f))) => Ok(PhpString::from(float_to_string(*f))),
        Some(Returned::Value(v @ (Value::Array(_) | Value::Object(_)))) => Err(v.debug_type()),
        Some(Returned::Value(v)) => Ok(v.to_php_string().unwrap_or_default()),
        Some(Returned::Stringable(s)) => Ok(s.to_php_string()?),
    })
}

fn argument_type_error(method: &str, given: &Value) -> Thrown {
    Thrown::TypeError(format!(
        "{CLASS}::{method}(): Argument #1 ($value) must be of type Stringable|string|int|float|bool, {} given",
        given.debug_type()
    ))
}

impl LazyString {
    fn new(state: State) -> Self {
        Self(Arc::new(Mutex::new(state)))
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A callback that panicked left the state as it was before the call.
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// `LazyString::fromCallable($callback, ...$arguments)`.
    pub fn from_callable(callback: Callable, arguments: Vec<Value>) -> Self {
        Self::new(State::Pending {
            target: Target::Callable(callback),
            arguments: Some(arguments),
            value: None,
        })
    }

    /// `LazyString::fromCallable([$factory, $method], ...$arguments)`: the
    /// factory runs on the first cast, then `$object->$method()` (or
    /// `__invoke()` without a method) is called. The object is kept even
    /// if that call throws.
    pub fn from_lazy_callable(
        factory: impl FnMut() -> Result<Arc<dyn Invokable>, Thrown> + Send + 'static,
        method: Option<&str>,
        arguments: Vec<Value>,
    ) -> Self {
        Self::new(State::Pending {
            target: Target::Lazy {
                factory: Box::new(factory),
                method: method.unwrap_or("__invoke").to_owned(),
            },
            arguments: Some(arguments),
            value: None,
        })
    }

    /// The check `fromCallable()` makes of an array that is not callable
    /// (`$is_callable` false): it must start with a Closure or have more
    /// than two items, else this TypeError. Rust callers build a
    /// [`Callable`] or a lazy-callable instead; this keeps the check for
    /// arrays that arrive as values.
    pub fn check_callable_array(items: &PhpArray, is_callable: bool) -> Result<(), Thrown> {
        let first_is_closure = matches!(
            items.get(&Key::Int(0)),
            Some(Value::Object(o)) if o.class() == "Closure"
        );
        if is_callable || first_is_closure || items.len() > 2 {
            return Ok(());
        }
        let types: Vec<String> = items.iter().map(|(_, v)| v.debug_type()).collect();
        Err(Thrown::TypeError(format!(
            "Argument 1 passed to \"{CLASS}::fromCallable()\" must be a callable or a [Closure, method] lazy-callable, \"[{}]\" given.",
            types.join(", ")
        )))
    }

    /// `LazyString::fromStringable($value)` for a scalar: cast now. Null,
    /// arrays and objects are PHP's TypeError (without its ", called in"
    /// suffix).
    pub fn from_stringable_value(value: &Value) -> Result<Self, Thrown> {
        match value {
            Value::Null | Value::Array(_) | Value::Object(_) => {
                Err(argument_type_error("fromStringable", value))
            }
            other => Ok(Self::new(State::Resolved(Self::scalar(other)))),
        }
    }

    /// `LazyString::fromStringable($object)`: `__toString()` runs on the
    /// first cast.
    pub fn from_stringable(value: Arc<dyn Stringable>) -> Self {
        Self::from_callable(
            Callable::new("__toString", move |_| {
                Ok(Returned::Value(Value::String(value.to_php_string()?)))
            }),
            Vec::new(),
        )
    }

    fn scalar(value: &Value) -> PhpString {
        match value {
            Value::Float(f) => PhpString::from(float_to_string(*f)),
            other => other.to_php_string().unwrap_or_default(),
        }
    }

    /// `LazyString::isStringable($value)`: strings and scalars. No
    /// php-value object has `__toString()`; a Stringable is one by type.
    pub fn is_stringable(value: &Value) -> bool {
        matches!(
            value,
            Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_)
        )
    }

    /// `LazyString::resolve($value)` for a value.
    pub fn resolve(value: &Value) -> Result<PhpString, Thrown> {
        match value {
            Value::Null | Value::Array(_) | Value::Object(_) => {
                Err(argument_type_error("resolve", value))
            }
            other => Ok(Self::scalar(other)),
        }
    }

    /// `LazyString::resolve($stringable)`.
    pub fn resolve_stringable(value: &dyn Stringable) -> Result<PhpString, Thrown> {
        value.to_php_string()
    }

    /// `(string) $lazyString`.
    pub fn to_php_string(&self) -> Result<PhpString, Thrown> {
        let id = Arc::as_ptr(&self.0) as *const () as usize;
        let Some(_casting) = Casting::enter(id) else {
            return Err(Thrown::Recursion);
        };
        let mut state = self.lock();
        let (target, arguments, value) = match &mut *state {
            State::Resolved(s) => return Ok(s.clone()),
            State::Pending {
                target,
                arguments,
                value,
            } => (target, arguments, value),
        };
        if let Some(args) = arguments {
            let returned = match target {
                Target::Callable(c) => (c.call)(args)?,
                Target::Object { object, method } => object.call(method, args)?,
                // `$callback[0] = $callback[0]()`: kept even if the call throws.
                Target::Lazy { factory, method } => {
                    let object = factory()?;
                    let method = std::mem::take(method);
                    let returned = object.call(&method, args);
                    *target = Target::Object { object, method };
                    returned?
                }
            };
            *value = Some(returned);
            *arguments = None;
        }
        match coerce(value.as_ref())? {
            Ok(s) => {
                *state = State::Resolved(s.clone());
                Ok(s)
            }
            Err(type_name) => Err(Thrown::TypeError(format!(
                "Return value of {}() passed to {CLASS}::fromCallable() must be of the type string, {type_name} returned.",
                target.name()
            ))),
        }
    }

    /// `jsonSerialize()`: the string.
    pub fn json_serialize(&self) -> Result<PhpString, Thrown> {
        self.to_php_string()
    }

    /// `__serialize()`: `['value' => (string) $this]`.
    pub fn serialize_data(&self) -> Result<PhpArray, Thrown> {
        let s = self.to_php_string()?;
        let mut data = PhpArray::new();
        data.insert(Key::from("value"), Value::String(s));
        Ok(data)
    }

    /// The object `unserialize()` restores from `__serialize()`'s data
    /// (the class has no `__unserialize()`, so PHP sets the property);
    /// `None` without a string `value`.
    pub fn from_serialized(data: &PhpArray) -> Option<Self> {
        match data.get(&Key::from("value")) {
            Some(Value::String(s)) => Some(Self::new(State::Resolved(s.clone()))),
            _ => None,
        }
    }
}

impl Stringable for LazyString {
    fn to_php_string(&self) -> Result<PhpString, Thrown> {
        LazyString::to_php_string(self)
    }
}
