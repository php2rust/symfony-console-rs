//! Port of symfony/string's FunctionsTest: `u()`, `b()` and `s()`, on
//! upstream's provider rows (PHP's `null` input is Rust's `""`).
//!
//! Upstream: symfony/string Tests/FunctionsTest.php
#![allow(non_snake_case)]

use symfony_string::{ByteString, Str, UnicodeString, b, s, u};

#[test]
fn testS() {
    let rows: [(Str, &[u8]); 6] = [
        (Str::Unicode(UnicodeString::new("")), b""),
        (Str::Unicode(UnicodeString::new("")), b""),
        (Str::Unicode(UnicodeString::new("foo")), b"foo"),
        (
            Str::Unicode(UnicodeString::new("अनुच्छेद")),
            "अनुच्छेद".as_bytes(),
        ),
        (Str::Byte(ByteString::new(b"b\x80ar")), b"b\x80ar"),
        (Str::Byte(ByteString::new(b"\xfe\xff")), b"\xfe\xff"),
    ];
    for (expected, input) in rows {
        assert_eq!(expected, s(input));
    }
}

#[test]
fn testU() {
    for (expected, input) in [("", ""), ("", ""), ("foo", "foo"), ("अनुच्छेद", "अनुच्छेद")]
    {
        assert_eq!(UnicodeString::new(expected), u(input));
    }
}

#[test]
fn testB() {
    let rows: [(&[u8], &[u8]); 4] = [
        (b"", b""),
        (b"", b""),
        (b"b\x80ar", b"b\x80ar"),
        (b"\xfe\xff", b"\xfe\xff"),
    ];
    for (expected, input) in rows {
        assert_eq!(ByteString::new(expected), b(input));
    }
}
