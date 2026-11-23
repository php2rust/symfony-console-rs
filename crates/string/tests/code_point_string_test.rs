//! Port of symfony/string's CodePointStringTest: AbstractAsciiTestCase's
//! and AbstractUnicodeTestCase's tests on CodePointString with
//! CodePointStringTest's providers (tests/data/code_point_string_test.json).
//!
//! Upstream: symfony/string Tests/CodePointStringTest.php
#![allow(non_snake_case)]

mod common;

ascii_tests!(
    symfony_string::CodePointString,
    include_str!("data/code_point_string_test.json"),
    normalize: |s: &S, form: i64| -> S { common::UnicodeMethods::normalize_form(s, form) }
);
unicode_tests!();

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

#[test]
fn testCreateFromStringWithInvalidUtf8Input() {
    run_testCreateFromStringWithInvalidUtf8Input();
}

#[test]
fn testCodePointsAt() {
    run_testCodePointsAt();
}

#[test]
fn testTrimWithInvalidUtf8CharList() {
    run_testTrimWithInvalidUtf8CharList();
}

#[test]
fn testTrimStartWithInvalidUtf8CharList() {
    run_testTrimStartWithInvalidUtf8CharList();
}

#[test]
fn testTrimEndWithInvalidUtf8CharList() {
    run_testTrimEndWithInvalidUtf8CharList();
}

#[test]
fn testAppendInvalidUtf8String() {
    run_testAppendInvalidUtf8String();
}

#[test]
fn testPrependInvalidUtf8String() {
    run_testPrependInvalidUtf8String();
}

#[test]
fn testReplaceWithInvalidUtf8Pattern() {
    run_testReplaceWithInvalidUtf8Pattern();
}

#[test]
fn testReplaceWithInvalidUtf8PatternReplacement() {
    run_testReplaceWithInvalidUtf8PatternReplacement();
}

#[test]
fn testSpliceMultibyte() {
    run_testSpliceMultibyte();
}
