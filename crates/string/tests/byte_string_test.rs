//! Port of symfony/string's `ByteStringTest`: `AbstractAsciiTestCase`'s tests
//! on `ByteString` with `ByteStringTest`'s providers (tests/data/
//! `byte_string_test.json`), and `fromRandom()`'s tests.
//!
//! Upstream: symfony/string Tests/ByteStringTest.php
#![allow(non_snake_case)]

mod common;

ascii_tests!(
    symfony_string::ByteString,
    include_str!("data/byte_string_test.json"),
    normalize: |_: &S, _: i64| -> S { unreachable!("ByteString has no normalize()") }
);

#[test]
fn testCreateFromString() {
    run_testCreateFromString();
}

#[test]
fn testCreateFromEmptyString() {
    run_testCreateFromEmptyString();
}

#[test]
fn testBytesAt() {
    run_testBytesAt();
}

#[test]
fn testContainsAny() {
    run_testContainsAny();
}

#[test]
fn testContainsAnyIgnoreCase() {
    run_testContainsAnyIgnoreCase();
}

#[test]
fn testUnwrap() {
    run_testUnwrap();
}

#[test]
fn testWordwrap() {
    run_testWordwrap();
}

#[test]
fn testWrap() {
    run_testWrap();
}

#[test]
fn testLength() {
    run_testLength();
}

#[test]
fn testIndexOf() {
    run_testIndexOf();
}

#[test]
fn testIndexOfIgnoreCase() {
    run_testIndexOfIgnoreCase();
}

#[test]
fn testIndexOfLast() {
    run_testIndexOfLast();
}

#[test]
fn testIndexOfLastIgnoreCase() {
    run_testIndexOfLastIgnoreCase();
}

#[test]
fn testSplit() {
    run_testSplit();
}

#[test]
fn testInvalidChunkLength() {
    run_testInvalidChunkLength();
}

#[test]
fn testChunk() {
    run_testChunk();
}

#[test]
fn testLower() {
    run_testLower();
}

#[test]
fn testUpper() {
    run_testUpper();
}

#[test]
fn testTitle() {
    run_testTitle();
}

#[test]
fn testSlice() {
    run_testSlice();
}

#[test]
fn testSplice() {
    run_testSplice();
}

#[test]
fn testAppend() {
    run_testAppend();
}

#[test]
fn testPrepend() {
    run_testPrepend();
}

#[test]
fn testTrim() {
    run_testTrim();
}

#[test]
fn testTrimPrefix() {
    run_testTrimPrefix();
}

#[test]
fn testTrimStart() {
    run_testTrimStart();
}

#[test]
fn testTrimSuffix() {
    run_testTrimSuffix();
}

#[test]
fn testTrimEnd() {
    run_testTrimEnd();
}

#[test]
fn testBeforeAfter() {
    run_testBeforeAfter();
}

#[test]
fn testBeforeAfterIgnoreCase() {
    run_testBeforeAfterIgnoreCase();
}

#[test]
fn testBeforeAfterLast() {
    run_testBeforeAfterLast();
}

#[test]
fn testBeforeAfterLastIgnoreCase() {
    run_testBeforeAfterLastIgnoreCase();
}

#[test]
fn testFolded() {
    run_testFolded();
}

#[test]
fn testReplace() {
    run_testReplace();
}

#[test]
fn testReplaceMatches() {
    run_testReplaceMatches();
}

#[test]
fn testReplaceIgnoreCase() {
    run_testReplaceIgnoreCase();
}

#[test]
fn testCamel() {
    run_testCamel();
}

#[test]
fn testSnake() {
    run_testSnake();
}

#[test]
fn testKebab() {
    run_testKebab();
}

#[test]
fn testPascal() {
    run_testPascal();
}

#[test]
fn testStartsWith() {
    run_testStartsWith();
}

#[test]
fn testStartsWithIgnoreCase() {
    run_testStartsWithIgnoreCase();
}

#[test]
fn testEndsWith() {
    run_testEndsWith();
}

#[test]
fn testEndsWithIgnoreCase() {
    run_testEndsWithIgnoreCase();
}

#[test]
fn testEnsureStart() {
    run_testEnsureStart();
}

#[test]
fn testEnsureStartIgnoreCase() {
    run_testEnsureStartIgnoreCase();
}

#[test]
fn testEnsureEnd() {
    run_testEnsureEnd();
}

#[test]
fn testEnsureEndIgnoreCase() {
    run_testEnsureEndIgnoreCase();
}

#[test]
fn testCollapseWhitespace() {
    run_testCollapseWhitespace();
}

#[test]
fn testEqualsTo() {
    run_testEqualsTo();
}

#[test]
fn testEqualsToIgnoreCase() {
    run_testEqualsToIgnoreCase();
}

#[test]
fn testIsEmpty() {
    run_testIsEmpty();
}

#[test]
fn testJoin() {
    run_testJoin();
}

#[test]
fn testJoinWithLastGlue() {
    run_testJoinWithLastGlue();
}

#[test]
fn testRepeat() {
    run_testRepeat();
}

#[test]
fn testPadBoth() {
    run_testPadBoth();
}

#[test]
fn testPadEnd() {
    run_testPadEnd();
}

#[test]
fn testPadStart() {
    run_testPadStart();
}

#[test]
fn testTruncate() {
    run_testTruncate();
}

#[test]
fn testToString() {
    run_testToString();
}

#[test]
fn testReverse() {
    run_testReverse();
}

#[test]
fn testWidth() {
    run_testWidth();
}

#[test]
fn testToByteString() {
    run_testToByteString();
}

use symfony_string::ByteString;

/// A deterministic byte source for `from_random_with()` (`from_random()`
/// reads the operating system).
fn source() -> impl FnMut(&mut [u8]) {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    move |buf: &mut [u8]| {
        for b in buf {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *b = state as u8;
        }
    }
}

#[test]
fn testFromRandom() {
    let random = ByteString::from_random(32, None).unwrap();
    assert_eq!(32, random.length());
    let alphabet = ByteString::new("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz");
    for char in random.chunk(1).unwrap() {
        assert!(alphabet.index_of(&char, 0).is_some());
    }
}

#[test]
fn testFromRandomWithSpecificChars() {
    let random = ByteString::from_random(32, Some(b"abc")).unwrap();
    assert_eq!(32, random.length());
    for char in random.chunk(1).unwrap() {
        assert!(ByteString::new("abc").index_of(&char, 0).is_some());
    }
}

/// Not upstream: `from_random_with()` draws only from the caller's source,
/// so the same source gives the same string.
#[test]
fn from_random_with_uses_the_callers_source() {
    let a = ByteString::from_random_with(32, Some(b"abc"), &mut source()).unwrap();
    let b = ByteString::from_random_with(32, Some(b"abc"), &mut source()).unwrap();
    assert_eq!(32, a.length());
    assert_eq!(a.as_bytes(), b.as_bytes());
}

#[test]
fn testFromRandoWithZeroLength() {
    let e = ByteString::from_random(0, None).unwrap_err();
    assert!(e.is_invalid_argument());
    assert_eq!(
        "A strictly positive length is expected, \"0\" given.",
        e.message()
    );
}

#[test]
fn testFromRandomThrowsForNegativeLength() {
    let e = ByteString::from_random(-1, None).unwrap_err();
    assert!(e.is_invalid_argument());
    assert_eq!(
        "A strictly positive length is expected, \"-1\" given.",
        e.message()
    );
}

#[test]
fn testFromRandomAlphabetMin() {
    let e = ByteString::from_random(32, Some(b"a")).unwrap_err();
    assert!(e.is_invalid_argument());
    assert_eq!(
        "The length of the alphabet must in the [2^1, 2^56] range.",
        e.message()
    );
}
