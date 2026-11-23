//! HelperSetTest and FormatterHelperTest, with upstream's names and
//! assertions. The mock helpers are a test helper that records its
//! `setHelperSet()` calls.
//!
//! Upstream: symfony/console Tests/Helper/HelperSetTest.php Tests/Helper/FormatterHelperTest.php
#![allow(non_snake_case)]

use std::sync::{Arc, Mutex};

use symfony_console::helper::{
    formatter_helper::FormatterHelper,
    helper_set::{HelperInterface, HelperSet},
};

/// `getGenericMockHelper($name, $helperset)`.
struct MockHelper {
    name: String,
    /// The sets `setHelperSet()` was called with, by address.
    calls: Mutex<Vec<usize>>,
}

impl HelperInterface for MockHelper {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn set_helper_set(&self, helper_set: &HelperSet) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(std::ptr::from_ref(helper_set) as usize);
        }
    }
}

fn mock(name: &str) -> Arc<MockHelper> {
    Arc::new(MockHelper {
        name: name.to_owned(),
        calls: Mutex::new(Vec::new()),
    })
}

/// `->expects($this->once())->method('setHelperSet')->with($this->equalTo($helperset))`.
fn assert_set_once(helper: &MockHelper, helper_set: &HelperSet) {
    let calls = helper.calls.lock().map(|c| c.clone()).unwrap_or_default();
    assert_eq!(vec![std::ptr::from_ref(helper_set) as usize], calls);
}

fn same(a: &Arc<dyn HelperInterface>, b: &Arc<MockHelper>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

#[test]
fn testConstructor() {
    let mock_helper = mock("fake_helper");
    let helperset = HelperSet::new(vec![(
        Some("fake_helper_alias".to_owned()),
        mock_helper.clone(),
    )]);

    assert!(
        helperset
            .get("fake_helper_alias")
            .is_ok_and(|h| same(&h, &mock_helper)),
        "__construct sets given helper to helpers"
    );
    assert!(
        helperset.has("fake_helper_alias"),
        "__construct sets helper alias for given helper"
    );
}

#[test]
fn testSet() {
    let mut helperset = HelperSet::new(vec![]);
    let helper = mock("fake_helper");
    helperset.set(helper.clone(), None);
    assert!(
        helperset.has("fake_helper"),
        "->set() adds helper to helpers"
    );
    assert_set_once(&helper, &helperset);

    let mut helperset = HelperSet::new(vec![]);
    let (one, two) = (mock("fake_helper_01"), mock("fake_helper_02"));
    helperset.set(one.clone(), None);
    helperset.set(two.clone(), None);
    assert!(
        helperset.has("fake_helper_01"),
        "->set() will set multiple helpers on consecutive calls"
    );
    assert!(
        helperset.has("fake_helper_02"),
        "->set() will set multiple helpers on consecutive calls"
    );
    assert_set_once(&one, &helperset);
    assert_set_once(&two, &helperset);

    let mut helperset = HelperSet::new(vec![]);
    let helper = mock("fake_helper");
    helperset.set(helper.clone(), Some("fake_helper_alias"));
    assert!(
        helperset.has("fake_helper"),
        "->set() adds helper alias when set"
    );
    assert!(
        helperset.has("fake_helper_alias"),
        "->set() adds helper alias when set"
    );
    assert_set_once(&helper, &helperset);
}

#[test]
fn testHas() {
    let helperset = HelperSet::new(vec![(
        Some("fake_helper_alias".to_owned()),
        mock("fake_helper"),
    )]);
    assert!(helperset.has("fake_helper"), "->has() finds set helper");
    assert!(
        helperset.has("fake_helper_alias"),
        "->has() finds set helper by alias"
    );
}

#[test]
fn testGet() {
    let helper_01 = mock("fake_helper_01");
    let helper_02 = mock("fake_helper_02");
    let helperset = HelperSet::new(vec![
        (Some("fake_helper_01_alias".to_owned()), helper_01.clone()),
        (Some("fake_helper_02_alias".to_owned()), helper_02.clone()),
    ]);
    for (name, helper) in [
        ("fake_helper_01", &helper_01),
        ("fake_helper_01_alias", &helper_01),
        ("fake_helper_02", &helper_02),
        ("fake_helper_02_alias", &helper_02),
    ] {
        assert!(
            helperset.get(name).is_ok_and(|h| same(&h, helper)),
            "->get() returns correct helper by name or alias ({name})"
        );
    }

    let helperset = HelperSet::new(vec![]);
    match helperset.get("foo") {
        Ok(_) => panic!("->get() throws InvalidArgumentException when helper not found"),
        Err(e) => {
            assert!(
                e.is_invalid_argument(),
                "->get() throws InvalidArgumentException when helper not found"
            );
            assert!(
                e.is_console_exception(),
                "->get() throws domain specific exception when helper not found"
            );
            assert!(
                e.message().contains("The helper \"foo\" is not defined."),
                "->get() throws InvalidArgumentException when helper not found"
            );
        }
    }
}

#[test]
fn testIteration() {
    let mut helperset = HelperSet::new(vec![]);
    helperset.set(mock("fake_helper_01"), None);
    helperset.set(mock("fake_helper_02"), None);

    let helpers = ["fake_helper_01", "fake_helper_02"];
    let names: Vec<&str> = helperset.iter().map(|(_, h)| h.get_name()).collect();
    assert_eq!(helpers.to_vec(), names);
}

// FormatterHelperTest

#[test]
fn testFormatSection() {
    let formatter = FormatterHelper::new();
    assert_eq!(
        "<info>[cli]</info> Some text to display",
        formatter.format_section("cli", "Some text to display", "info"),
        "::formatSection() formats a message in a section"
    );
}

#[test]
fn testFormatBlock() {
    let formatter = FormatterHelper::new();
    assert_eq!(
        "<error> Some text to display </error>",
        formatter.format_block(&["Some text to display"], "error", false),
    );
    assert_eq!(
        "<error> Some text to display </error>\n<error> foo bar              </error>",
        formatter.format_block(&["Some text to display", "foo bar"], "error", false),
    );
    assert_eq!(
        "<error>                        </error>\n<error>  Some text to display  </error>\n<error>                        </error>",
        formatter.format_block(&["Some text to display"], "error", true),
    );
}

#[test]
fn testFormatBlockWithDiacriticLetters() {
    let formatter = FormatterHelper::new();
    assert_eq!(
        "<error>                       </error>\n<error>  Du texte à afficher  </error>\n<error>                       </error>",
        formatter.format_block(&["Du texte à afficher"], "error", true),
    );
}

#[test]
fn testFormatBlockWithDoubleWidthDiacriticLetters() {
    let formatter = FormatterHelper::new();
    assert_eq!(
        "<error>                    </error>\n<error>  表示するテキスト  </error>\n<error>                    </error>",
        formatter.format_block(&["表示するテキスト"], "error", true),
    );
}

#[test]
fn testFormatBlockLGEscaping() {
    let formatter = FormatterHelper::new();
    assert_eq!(
        "<error>                              </error>\n<error>  \\<info\\>some info\\</info\\>  </error>\n<error>                              </error>",
        formatter.format_block(&["<info>some info</info>"], "error", true),
        "::formatBlock() escapes '<' chars"
    );
}

#[test]
fn testTruncatingWithShorterLengthThanMessageWithSuffix() {
    let formatter = FormatterHelper::new();
    let message = "testing truncate";
    assert_eq!("test...", formatter.truncate(message, 4, "..."));
    assert_eq!("testing truncat...", formatter.truncate(message, 15, "..."));
    assert_eq!(
        "testing truncate...",
        formatter.truncate(message, 16, "...")
    );
    assert_eq!(
        "zażółć gęślą...",
        formatter.truncate("zażółć gęślą jaźń", 12, "...")
    );
}

#[test]
fn testTruncatingMessageWithCustomSuffix() {
    let formatter = FormatterHelper::new();
    assert_eq!("test!", formatter.truncate("testing truncate", 4, "!"));
}

#[test]
fn testTruncatingWithLongerLengthThanMessageWithSuffix() {
    let formatter = FormatterHelper::new();
    assert_eq!("test", formatter.truncate("test", 10, "..."));
}

#[test]
fn testTruncatingWithNegativeLength() {
    let formatter = FormatterHelper::new();
    let message = "testing truncate";
    assert_eq!("testing tru...", formatter.truncate(message, -5, "..."));
    assert_eq!("...", formatter.truncate(message, -100, "..."));
}
