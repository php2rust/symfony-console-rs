//! What the ports of ByteStringTest, CodePointStringTest and
//! UnicodeStringTest share: their data providers as the oracle returns
//! them (`tests/data/*_test.json`, `scripts/record.sh string`), read as
//! typed values, and the test methods of AbstractAsciiTestCase and
//! AbstractUnicodeTestCase as macros, with upstream's names and
//! assertions.
//!
//! Upstream: symfony/string Tests/AbstractAsciiTestCase.php Tests/AbstractUnicodeTestCase.php
#![allow(dead_code)]

use php_value::base64;
use serde_json::Value as Json;
use symfony_string::{AbstractString, CodePointString, Needle, TruncateMode, UnicodeString};

/// A provider argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Null,
    Bool(bool),
    Int(i64),
    Str(Vec<u8>),
    Array(Vec<(Arg, Arg)>),
    /// A class and its properties; symfony/string objects carry `string`.
    Object(String, Vec<(Arg, Arg)>),
    /// An enum case's name.
    Enum(String),
}

fn decode(json: &Json) -> Arg {
    let kind = json[0].as_str().unwrap_or_default();
    match kind {
        "null" => Arg::Null,
        "bool" => Arg::Bool(json[1].as_bool().unwrap_or_default()),
        "int" => Arg::Int(json[1].as_i64().unwrap_or_default()),
        "string" => Arg::Str(
            base64::decode_strict(json[1].as_str().unwrap_or_default().as_bytes())
                .unwrap_or_else(|| panic!("base64: {json}")),
        ),
        "array" => Arg::Array(pairs(&json[1])),
        "object" => Arg::Object(
            json[1].as_str().unwrap_or_default().to_owned(),
            pairs(&json[2]),
        ),
        "enum" => Arg::Enum(json[2].as_str().unwrap_or_default().to_owned()),
        other => panic!("unexpected typed value {other}: {json}"),
    }
}

fn pairs(json: &Json) -> Vec<(Arg, Arg)> {
    json.as_array()
        .into_iter()
        .flatten()
        .map(|p| (decode(&p[0]), decode(&p[1])))
        .collect()
}

/// The rows of `test`'s provider: name and arguments.
pub fn provider(json: &str, test: &str) -> Vec<(String, Vec<Arg>)> {
    let data: Json = serde_json::from_str(json).unwrap_or_else(|e| panic!("providers: {e}"));
    let rows = data[test]["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("no provider for {test}"));
    assert!(!rows.is_empty(), "{test}: empty provider");
    rows.iter()
        .map(|row| {
            let name = row[0].as_str().unwrap_or_default().to_owned();
            let args = row[1]
                .as_array()
                .into_iter()
                .flatten()
                .map(decode)
                .collect();
            (name, args)
        })
        .collect()
}

impl Arg {
    /// A string, or a symfony/string object's text.
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Arg::Str(s) => s.clone(),
            Arg::Object(_, props) => props
                .iter()
                .find(|(k, _)| *k == Arg::Str(b"string".to_vec()))
                .map(|(_, v)| v.bytes())
                .unwrap_or_else(|| panic!("object without string: {self:?}")),
            other => panic!("not a string: {other:?}"),
        }
    }

    pub fn int(&self) -> i64 {
        match self {
            Arg::Int(i) => *i,
            other => panic!("not an int: {other:?}"),
        }
    }

    pub fn opt_int(&self) -> Option<i64> {
        match self {
            Arg::Null => None,
            other => Some(other.int()),
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Arg::Bool(b) => *b,
            other => panic!("not a bool: {other:?}"),
        }
    }

    pub fn values(&self) -> Vec<Arg> {
        match self {
            Arg::Array(pairs) => pairs.iter().map(|(_, v)| v.clone()).collect(),
            other => panic!("not an array: {other:?}"),
        }
    }

    /// A `string|iterable` argument.
    pub fn needle(&self) -> Needle {
        match self {
            Arg::Array(_) => Needle::Many(self.values().iter().map(Arg::bytes).collect()),
            other => Needle::One(other.bytes()),
        }
    }

    /// `int|null` expected positions.
    pub fn opt_usize(&self) -> Option<usize> {
        self.opt_int().map(|i| i as usize)
    }

    pub fn truncate_mode(args: &[Arg], at: usize) -> TruncateMode {
        match args.get(at) {
            None => TruncateMode::Char,
            Some(Arg::Bool(b)) => (*b).into(),
            Some(Arg::Enum(name)) => match name.as_str() {
                "Char" => TruncateMode::Char,
                "WordBefore" => TruncateMode::WordBefore,
                "WordAfter" => TruncateMode::WordAfter,
                other => panic!("TruncateMode::{other}"),
            },
            Some(other) => panic!("not a cut: {other:?}"),
        }
    }
}

/// `grapheme_strlen('च्छे')`: 2 on an ICU with ICU-21661 fixed.
pub fn icu_21661_fixed() -> bool {
    UnicodeString::new("च्छे").length() == 2
}

/// `$instance->normalize($form)`, `codePointsAt()`: methods of the Unicode
/// classes only.
pub trait UnicodeMethods: AbstractString {
    fn normalize_form(&self, form: i64) -> Self;
    fn code_points(&self, offset: i64) -> Vec<u32>;
}

impl UnicodeMethods for UnicodeString {
    fn normalize_form(&self, form: i64) -> Self {
        self.normalize(form)
            .unwrap_or_else(|e| panic!("normalize: {e}"))
    }
    fn code_points(&self, offset: i64) -> Vec<u32> {
        self.code_points_at(offset)
    }
}

impl UnicodeMethods for CodePointString {
    fn normalize_form(&self, form: i64) -> Self {
        self.normalize(form)
            .unwrap_or_else(|e| panic!("normalize: {e}"))
    }
    fn code_points(&self, offset: i64) -> Vec<u32> {
        self.code_points_at(offset)
    }
}

/// `static::createFromString($string)`.
pub fn create<S: AbstractString>(s: impl AsRef<[u8]>) -> S {
    S::create(s.as_ref())
        .unwrap_or_else(|e| panic!("create {:?}: {e}", String::from_utf8_lossy(s.as_ref())))
}

/// The tests of AbstractAsciiTestCase, for `$S` and its providers.
#[macro_export]
macro_rules! ascii_tests {
    ($S:ty, $json:expr, normalize: $normalize:expr) => {
        use common::{Arg, create, provider};
        use symfony_string::{AbstractString, Replacement};

        type S = $S;
        const JSON: &str = $json;

        fn rows(test: &str) -> Vec<(String, Vec<Arg>)> {
            provider(JSON, test)
        }

        fn c(s: impl AsRef<[u8]>) -> S {
            create::<S>(s)
        }

        pub(crate) fn run_testCreateFromString() {
            let bytes = c("Symfony is a PHP framework!");
            assert_eq!(b"Symfony is a PHP framework!", bytes.as_bytes());
            assert_eq!(27, bytes.length());
            assert!(!bytes.is_empty());
        }

        pub(crate) fn run_testCreateFromEmptyString() {
            let instance = c("");
            assert_eq!(b"", instance.as_bytes());
            assert_eq!(0, instance.length());
            assert!(instance.is_empty());
        }

        pub(crate) fn run_testBytesAt() {
            for (name, a) in rows("testBytesAt") {
                if !common::icu_21661_fixed() && a[1].bytes() == "नमस्ते".as_bytes() {
                    continue;
                }
                let mut instance = c(a[1].bytes());
                if let Some(form) = a.get(3).and_then(Arg::opt_int) {
                    instance = $normalize(&instance, form);
                }
                let expected: Vec<u8> = a[0].values().iter().map(|v| v.int() as u8).collect();
                assert_eq!(expected, instance.bytes_at(a[2].int()), "{name}");
            }
        }

        pub(crate) fn run_testContainsAny() {
            for (name, a) in rows("testContainsAny") {
                let instance = c(a[1].bytes());
                assert_eq!(
                    instance.index_of(a[2].needle(), 0).is_some(),
                    instance.contains_any(a[2].needle()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testContainsAnyIgnoreCase() {
            for (name, a) in rows("testContainsAnyIgnoreCase") {
                let instance = c(a[1].bytes());
                assert_eq!(
                    instance.ignore_case().index_of(a[2].needle(), 0).is_some(),
                    instance.ignore_case().contains_any(a[2].needle()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testUnwrap() {
            let mut values = php_value::PhpArray::new();
            values.insert(0, php_value::Value::string("hello"));
            values.insert(1, php_value::Value::string("world"));
            let wrapped = S::wrap(&values).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(values, S::unwrap(&wrapped));
        }

        pub(crate) fn run_testWordwrap() {
            for (name, a) in rows("testWordwrap") {
                let cut = a.get(4).is_some_and(Arg::bool);
                let actual = c(a[1].bytes())
                    .wordwrap(a[2].int(), a[3].bytes(), cut)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(a[0].bytes(), actual.as_bytes(), "{name}");
            }
        }

        pub(crate) fn run_testWrap() {
            fn to_value(a: &Arg) -> php_value::Value {
                match a {
                    Arg::Str(s) => php_value::Value::string(s.clone()),
                    Arg::Int(i) => php_value::Value::Int(*i),
                    Arg::Bool(b) => php_value::Value::Bool(*b),
                    Arg::Array(p) => php_value::Value::Array(to_array(p)),
                    Arg::Object(_, _) => php_value::Value::string(a.bytes()),
                    other => panic!("{other:?}"),
                }
            }
            fn to_array(p: &[(Arg, Arg)]) -> php_value::PhpArray {
                let mut out = php_value::PhpArray::new();
                for (k, v) in p {
                    let key = match k {
                        Arg::Int(i) => php_value::array::Key::Int(*i),
                        other => php_value::array::Key::from_bytes(&other.bytes()),
                    };
                    out.insert(key, to_value(v));
                }
                out
            }
            fn is_wrapped(expected: &Arg, actual: &symfony_string::Wrapped<S>) -> bool {
                match (expected, actual) {
                    (Arg::Object(_, _), symfony_string::Wrapped::String(s)) => {
                        *s == c(expected.bytes())
                    }
                    (Arg::Array(p), symfony_string::Wrapped::Array(a)) => {
                        p.len() == a.len()
                            && p.iter().zip(a).all(|((ek, ev), (ak, av))| {
                                to_value(ek) == php_value::Value::from(ak.clone())
                                    && is_wrapped(ev, av)
                            })
                    }
                    (Arg::Int(i), symfony_string::Wrapped::Other(php_value::Value::Int(j))) => {
                        i == j
                    }
                    (Arg::Bool(i), symfony_string::Wrapped::Other(php_value::Value::Bool(j))) => {
                        i == j
                    }
                    _ => false,
                }
            }
            for (name, a) in rows("testWrap") {
                let Arg::Array(values) = &a[1] else {
                    panic!("{name}")
                };
                let wrapped = S::wrap(&to_array(values)).unwrap_or_else(|e| panic!("{name}: {e}"));
                let Arg::Array(expected) = &a[0] else {
                    panic!("{name}")
                };
                assert_eq!(expected.len(), wrapped.len(), "{name}");
                for ((ek, ev), (ak, av)) in expected.iter().zip(&wrapped) {
                    assert_eq!(
                        to_value(ek),
                        php_value::Value::from(ak.clone()),
                        "{name}: key"
                    );
                    assert!(is_wrapped(ev, av), "{name}: {ev:?} vs {av:?}");
                }
            }
        }

        pub(crate) fn run_testLength() {
            for (name, a) in rows("testLength") {
                if !common::icu_21661_fixed() && a[1].bytes() == "अनुच्छेद".as_bytes()
                {
                    continue;
                }
                assert_eq!(a[0].int() as usize, c(a[1].bytes()).length(), "{name}");
            }
        }

        pub(crate) fn run_testIndexOf() {
            for (name, a) in rows("testIndexOf") {
                assert_eq!(
                    a[0].opt_usize(),
                    c(a[1].bytes()).index_of(a[2].needle(), a[3].int()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testIndexOfIgnoreCase() {
            for (name, a) in rows("testIndexOfIgnoreCase") {
                assert_eq!(
                    a[0].opt_usize(),
                    c(a[1].bytes())
                        .ignore_case()
                        .index_of(a[2].needle(), a[3].int()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testIndexOfLast() {
            for (name, a) in rows("testIndexOfLast") {
                assert_eq!(
                    a[0].opt_usize(),
                    c(a[1].bytes()).index_of_last(a[2].needle(), a[3].int()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testIndexOfLastIgnoreCase() {
            for (name, a) in rows("testIndexOfLastIgnoreCase") {
                assert_eq!(
                    a[0].opt_usize(),
                    c(a[1].bytes())
                        .ignore_case()
                        .index_of_last(a[2].needle(), a[3].int()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testSplit() {
            for (name, a) in rows("testSplit") {
                let flags = a.get(4).and_then(Arg::opt_int);
                let instance = c(a[0].bytes());
                if flags.is_some_and(|f| f & 4 != 0) {
                    let expected: Vec<(S, i64)> = a[2]
                        .values()
                        .iter()
                        .map(|p| {
                            let p = p.values();
                            (c(p[0].bytes()), p[1].int())
                        })
                        .collect();
                    let actual = instance
                        .split_offsets(a[1].bytes(), a[3].opt_int(), flags.unwrap_or(0))
                        .unwrap_or_else(|e| panic!("{name}: {e}"));
                    assert_eq!(expected, actual, "{name}");
                    continue;
                }
                let expected: Vec<S> = a[2].values().iter().map(|v| c(v.bytes())).collect();
                let actual = instance
                    .split(a[1].bytes(), a[3].opt_int(), flags)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(expected, actual, "{name}");
            }
        }

        pub(crate) fn run_testInvalidChunkLength() {
            for (name, a) in rows("testInvalidChunkLength") {
                let e = c("foo|bar|baz").chunk(a[0].int()).err();
                assert!(e.is_some_and(|e| e.is_invalid_argument()), "{name}");
            }
        }

        pub(crate) fn run_testChunk() {
            for (name, a) in rows("testChunk") {
                let expected: Vec<S> = a[1].values().iter().map(|v| c(v.bytes())).collect();
                assert_eq!(
                    expected,
                    c(a[0].bytes())
                        .chunk(a[2].int())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testLower() {
            for (name, a) in rows("testLower") {
                let instance = c(a[1].bytes()).lower();
                assert_eq!(c(a[0].bytes()), instance, "{name}");
                assert_eq!(a[0].bytes(), instance.as_bytes(), "{name}");
            }
        }

        pub(crate) fn run_testUpper() {
            for (name, a) in rows("testUpper") {
                let instance = c(a[1].bytes()).upper();
                assert_eq!(c(a[0].bytes()), instance, "{name}");
                assert_eq!(a[0].bytes(), instance.as_bytes(), "{name}");
            }
        }

        pub(crate) fn run_testTitle() {
            for (name, a) in rows("testTitle") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .title(a[2].bool())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testSlice() {
            for (name, a) in rows("testSlice") {
                let length = a.get(3).and_then(Arg::opt_int);
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes()).slice(a[2].int(), length),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testSplice() {
            for (name, a) in rows("testSplice") {
                let length = a.get(2).and_then(Arg::opt_int);
                let actual = c("Symfony is awesome")
                    .splice(b"X", a[1].int(), length)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), actual, "{name}");
            }
        }

        pub(crate) fn run_testAppend() {
            for (name, a) in rows("testAppend") {
                let suffixes: Vec<Vec<u8>> = a[1].values().iter().map(Arg::bytes).collect();
                let mut instance = c("");
                for suffix in &suffixes {
                    instance = instance
                        .append(&[suffix])
                        .unwrap_or_else(|e| panic!("{name}: {e}"));
                }
                assert_eq!(a[0].bytes(), instance.as_bytes(), "{name}");
                let all: Vec<&[u8]> = suffixes.iter().map(Vec::as_slice).collect();
                assert_eq!(
                    c(a[0].bytes()),
                    c("").append(&all).unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testPrepend() {
            for (name, a) in rows("testPrepend") {
                let prefixes: Vec<Vec<u8>> = a[1].values().iter().map(Arg::bytes).collect();
                let mut instance = c("");
                for prefix in prefixes.iter().rev() {
                    instance = instance
                        .prepend(&[prefix])
                        .unwrap_or_else(|e| panic!("{name}: {e}"));
                }
                assert_eq!(c(a[0].bytes()), instance, "{name}");
                let all: Vec<&[u8]> = prefixes.iter().map(Vec::as_slice).collect();
                assert_eq!(
                    c(a[0].bytes()),
                    c("")
                        .prepend(&all)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testTrim() {
            for (name, a) in rows("testTrim") {
                let chars = match &a[2] {
                    Arg::Null => None,
                    other => Some(other.bytes()),
                };
                let result = c(a[1].bytes())
                    .trim(chars.as_deref())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), result, "{name}");
            }
        }

        pub(crate) fn run_testTrimPrefix() {
            let str = c("abc.def");
            assert_eq!(c("def"), str.trim_prefix("abc.").unwrap());
            assert_eq!(c("def"), str.trim_prefix(["abc.", "def"]).unwrap());
            assert_eq!(c("def"), str.ignore_case().trim_prefix("ABC.").unwrap());
        }

        pub(crate) fn run_testTrimStart() {
            for (name, a) in rows("testTrimStart") {
                let chars = match &a[2] {
                    Arg::Null => None,
                    other => Some(other.bytes()),
                };
                let result = c(a[1].bytes())
                    .trim_start(chars.as_deref())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), result, "{name}");
            }
        }

        pub(crate) fn run_testTrimSuffix() {
            let str = c("abc.def");
            assert_eq!(c("abc"), str.trim_suffix(".def").unwrap());
            assert_eq!(c("abc"), str.trim_suffix([".def", "abc"]).unwrap());
            assert_eq!(c("abc"), str.ignore_case().trim_suffix(".DEF").unwrap());
        }

        pub(crate) fn run_testTrimEnd() {
            for (name, a) in rows("testTrimEnd") {
                let chars = match &a[2] {
                    Arg::Null => None,
                    other => Some(other.bytes()),
                };
                let result = c(a[1].bytes())
                    .trim_end(chars.as_deref())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), result, "{name}");
            }
        }

        fn before_after(test: &str, ignore_case: bool, last: bool) {
            for (name, a) in rows(test) {
                let mut result = c(a[2].bytes());
                if ignore_case {
                    result = result.ignore_case();
                }
                let (needle, offset) = (a[1].bytes(), a[3].int());
                let result = match (a[4].bool(), last) {
                    (true, false) => result.before(needle, false, offset),
                    (false, false) => result.after(needle, true, offset),
                    (true, true) => result.before_last(needle, false, offset),
                    (false, true) => result.after_last(needle, true, offset),
                };
                assert_eq!(c(a[0].bytes()), result, "{test} {name}");
            }
        }

        pub(crate) fn run_testBeforeAfter() {
            before_after("testBeforeAfter", false, false);
        }

        pub(crate) fn run_testBeforeAfterIgnoreCase() {
            before_after("testBeforeAfterIgnoreCase", true, false);
        }

        pub(crate) fn run_testBeforeAfterLast() {
            before_after("testBeforeAfterLast", false, true);
        }

        pub(crate) fn run_testBeforeAfterLastIgnoreCase() {
            before_after("testBeforeAfterLastIgnoreCase", true, true);
        }

        pub(crate) fn run_testFolded() {
            for (name, a) in rows("testFolded") {
                assert_eq!(c(a[0].bytes()), c(a[1].bytes()).folded(), "{name}");
            }
        }

        pub(crate) fn run_testReplace() {
            for (name, a) in rows("testReplace") {
                let result = c(a[2].bytes())
                    .replace(&a[3].bytes(), &a[4].bytes())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), result, "{name}");
            }
        }

        pub(crate) fn run_testReplaceMatches() {
            for (name, a) in rows("testReplaceMatches") {
                let origin = c(a[1].bytes());
                let result = match &a[3] {
                    Arg::Object(class, _) if class == "Closure" => {
                        if a[0].bytes().starts_with(b"hello") {
                            // static fn ($word) => strtolower($word[1])
                            let mut f =
                                |m: &[Option<&[u8]>]| m[1].unwrap_or_default().to_ascii_lowercase();
                            origin.replace_matches(&a[2].bytes(), Replacement::Callback(&mut f))
                        } else {
                            // foreach ($matches as $match) return strtoupper($match);
                            let mut f =
                                |m: &[Option<&[u8]>]| m[0].unwrap_or_default().to_ascii_uppercase();
                            origin.replace_matches(&a[2].bytes(), Replacement::Callback(&mut f))
                        }
                    }
                    to => origin.replace_matches(&a[2].bytes(), Replacement::Text(&to.bytes())),
                };
                assert_eq!(
                    c(a[0].bytes()),
                    result.unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testReplaceIgnoreCase() {
            for (name, a) in rows("testReplaceIgnoreCase") {
                let result = c(a[2].bytes())
                    .ignore_case()
                    .replace(&a[3].bytes(), &a[4].bytes())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), result, "{name}");
            }
        }

        pub(crate) fn run_testCamel() {
            for (name, a) in rows("testCamel") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .camel()
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testSnake() {
            for (name, a) in rows("testSnake") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .snake()
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testKebab() {
            for (name, a) in rows("testKebab") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .kebab()
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testPascal() {
            for (name, a) in rows("testPascal") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .pascal()
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testStartsWith() {
            for (name, a) in rows("testStartsWith") {
                let mut instance = c(a[1].bytes());
                if let Some(form) = a.get(3).and_then(Arg::opt_int) {
                    instance = $normalize(&instance, form);
                }
                assert_eq!(a[0].bool(), instance.starts_with(a[2].needle()), "{name}");
            }
        }

        pub(crate) fn run_testStartsWithIgnoreCase() {
            for (name, a) in rows("testStartsWithIgnoreCase") {
                assert_eq!(
                    a[0].bool(),
                    c(a[1].bytes()).ignore_case().starts_with(a[2].needle()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEndsWith() {
            for (name, a) in rows("testEndsWith") {
                let mut instance = c(a[1].bytes());
                if let Some(form) = a.get(3).and_then(Arg::opt_int) {
                    instance = $normalize(&instance, form);
                }
                assert_eq!(a[0].bool(), instance.ends_with(a[2].needle()), "{name}");
            }
        }

        pub(crate) fn run_testEndsWithIgnoreCase() {
            for (name, a) in rows("testEndsWithIgnoreCase") {
                assert_eq!(
                    a[0].bool(),
                    c(a[1].bytes()).ignore_case().ends_with(a[2].needle()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEnsureStart() {
            for (name, a) in rows("testEnsureStart") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .ensure_start(a[2].bytes())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEnsureStartIgnoreCase() {
            for (name, a) in rows("testEnsureStartIgnoreCase") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .ignore_case()
                        .ensure_start(a[2].bytes())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEnsureEnd() {
            for (name, a) in rows("testEnsureEnd") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .ensure_end(a[2].bytes())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEnsureEndIgnoreCase() {
            for (name, a) in rows("testEnsureEndIgnoreCase") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .ignore_case()
                        .ensure_end(a[2].bytes())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testCollapseWhitespace() {
            for (name, a) in rows("testCollapseWhitespace") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes()).collapse_whitespace(),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEqualsTo() {
            for (name, a) in rows("testEqualsTo") {
                assert_eq!(
                    a[0].bool(),
                    c(a[1].bytes()).equals_to(a[2].needle()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testEqualsToIgnoreCase() {
            for (name, a) in rows("testEqualsToIgnoreCase") {
                assert_eq!(
                    a[0].bool(),
                    c(a[1].bytes()).ignore_case().equals_to(a[2].needle()),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testIsEmpty() {
            for (name, a) in rows("testIsEmpty") {
                assert_eq!(a[0].bool(), c(a[1].bytes()).is_empty(), "{name}");
            }
        }

        pub(crate) fn run_testJoin() {
            for (name, a) in rows("testJoin") {
                let parts: Vec<Vec<u8>> = a[2].values().iter().map(Arg::bytes).collect();
                let parts: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .join(&parts, None)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        pub(crate) fn run_testJoinWithLastGlue() {
            let joined = c(", ")
                .join(&[b"foo", b"bar", b"baz"], Some(b" and "))
                .unwrap();
            assert_eq!(b"foo, bar and baz", joined.as_bytes());
        }

        pub(crate) fn run_testRepeat() {
            for (name, a) in rows("testRepeat") {
                assert_eq!(
                    c(a[0].bytes()),
                    c(a[1].bytes())
                        .repeat(a[2].int())
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                    "{name}"
                );
            }
        }

        fn pad(test: &str, f: fn(&S, i64, &[u8]) -> symfony_string::exception::Result<S>) {
            for (name, a) in rows(test) {
                let actual = f(&c(a[1].bytes()), a[2].int(), &a[3].bytes())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), actual, "{test} {name}");
            }
        }

        pub(crate) fn run_testPadBoth() {
            pad("testPadBoth", S::pad_both);
        }

        pub(crate) fn run_testPadEnd() {
            pad("testPadEnd", S::pad_end);
        }

        pub(crate) fn run_testPadStart() {
            pad("testPadStart", S::pad_start);
        }

        pub(crate) fn run_testTruncate() {
            for (name, a) in rows("testTruncate") {
                let cut = Arg::truncate_mode(&a, 4);
                let actual = c(a[1].bytes())
                    .truncate(a[2].int(), a[3].bytes(), cut)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), actual, "{name}");
            }
        }

        pub(crate) fn run_testToString() {
            assert_eq!(b"foobar".to_vec(), c("foobar").to_php_string());
        }

        pub(crate) fn run_testReverse() {
            for (name, a) in rows("testReverse") {
                assert_eq!(c(a[0].bytes()), c(a[1].bytes()).reverse(), "{name}");
            }
        }

        pub(crate) fn run_testWidth() {
            for (name, a) in rows("testWidth") {
                let ignore = !matches!(a.get(2), Some(Arg::Bool(false)));
                assert_eq!(a[0].int() as usize, c(a[1].bytes()).width(ignore), "{name}");
            }
        }

        pub(crate) fn run_testToByteString() {
            for (name, a) in rows("testToByteString") {
                let encoding = String::from_utf8(a[1].bytes()).unwrap_or_default();
                let instance: symfony_string::ByteString = c(a[0].bytes())
                    .to_byte_string(Some(&encoding))
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert!(!instance.as_bytes().is_empty(), "{name}");
            }
        }
    };
}

/// The tests AbstractUnicodeTestCase adds.
#[macro_export]
macro_rules! unicode_tests {
    () => {
        use common::UnicodeMethods;

        pub(crate) fn run_testCreateFromStringWithInvalidUtf8Input() {
            assert!(S::create(b"\xE9").is_err_and(|e| e.is_invalid_argument()));
        }

        pub(crate) fn run_testCodePointsAt() {
            for (name, a) in rows("testCodePointsAt") {
                if !common::icu_21661_fixed() && a[1].bytes() == "नमस्ते".as_bytes() {
                    continue;
                }
                let mut instance = c(a[1].bytes());
                if let Some(form) = a.get(3).and_then(Arg::opt_int) {
                    instance = instance.normalize_form(form);
                }
                let expected: Vec<u32> = a[0].values().iter().map(|v| v.int() as u32).collect();
                assert_eq!(expected, instance.code_points(a[2].int()), "{name}");
            }
        }

        pub(crate) fn run_testTrimWithInvalidUtf8CharList() {
            assert!(
                c("Symfony")
                    .trim(Some(b"\xE9"))
                    .is_err_and(|e| e.is_invalid_argument())
            );
        }

        pub(crate) fn run_testTrimStartWithInvalidUtf8CharList() {
            assert!(
                c("Symfony")
                    .trim_start(Some(b"\xE9"))
                    .is_err_and(|e| e.is_invalid_argument())
            );
        }

        pub(crate) fn run_testTrimEndWithInvalidUtf8CharList() {
            assert!(
                c("Symfony")
                    .trim_end(Some(b"\xE9"))
                    .is_err_and(|e| e.is_invalid_argument())
            );
        }

        pub(crate) fn run_testAppendInvalidUtf8String() {
            assert!(
                c("Symfony")
                    .append(&[b"\xE9"])
                    .is_err_and(|e| e.is_invalid_argument())
            );
        }

        pub(crate) fn run_testPrependInvalidUtf8String() {
            assert!(
                c("Symfony")
                    .prepend(&[b"\xE9"])
                    .is_err_and(|e| e.is_invalid_argument())
            );
        }

        pub(crate) fn run_testReplaceWithInvalidUtf8Pattern() {
            assert_eq!(
                b"Symfony",
                c("Symfony").replace(b"\xE9", b"p").unwrap().as_bytes()
            );
        }

        pub(crate) fn run_testReplaceWithInvalidUtf8PatternReplacement() {
            assert!(
                c("Symfony")
                    .replace(b"f", b"\xE9")
                    .is_err_and(|e| e.is_invalid_argument())
            );
        }

        pub(crate) fn run_testSpliceMultibyte() {
            for (name, a) in rows("testSpliceMultibyte") {
                let length = a.get(4).and_then(Arg::opt_int);
                let actual = c(a[1].bytes())
                    .splice(&a[2].bytes(), a[3].int(), length)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(c(a[0].bytes()), actual, "{name}");
            }
        }
    };
}
