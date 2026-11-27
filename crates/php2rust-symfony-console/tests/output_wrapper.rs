//! `OutputWrapperTest`, with upstream's names, assertions and provider rows.
//!
//! Upstream: symfony/console Tests/Helper/OutputWrapperTest.php
#![allow(non_snake_case)]

use symfony_console::{ConsoleError, helper::output_wrapper::OutputWrapper};

const BASE_TEXT_WITH_UTF8_AND_URL: &str = "Árvíztűrőtükörfúrógép https://github.com/symfony/symfony Lorem ipsum <comment>dolor</comment> sit amet, consectetur adipiscing elit. Praesent vestibulum nulla quis urna maximus porttitor. Donec ullamcorper risus at <error>libero ornare</error> efficitur.";

fn text_provider() -> Vec<(&'static str, &'static str, i64, bool, &'static str)> {
    vec![
        (
            "Default URL cut",
            BASE_TEXT_WITH_UTF8_AND_URL,
            20,
            false,
            "Árvíztűrőtükörfúrógé\np https://github.com/symfony/symfony Lorem ipsum\n<comment>dolor</comment> sit amet,\nconsectetur\nadipiscing elit.\nPraesent vestibulum\nnulla quis urna\nmaximus porttitor.\nDonec ullamcorper\nrisus at <error>libero\nornare</error> efficitur.",
        ),
        (
            "Allow URL cut",
            BASE_TEXT_WITH_UTF8_AND_URL,
            20,
            true,
            "Árvíztűrőtükörfúrógé\np\nhttps://github.com/s\nymfony/symfony Lorem\nipsum <comment>dolor</comment> sit\namet, consectetur\nadipiscing elit.\nPraesent vestibulum\nnulla quis urna\nmaximus porttitor.\nDonec ullamcorper\nrisus at <error>libero\nornare</error> efficitur.",
        ),
    ]
}

#[test]
fn testBasicWrap() -> Result<(), ConsoleError> {
    for (name, text, width, allow_cut_urls, expected) in text_provider() {
        let wrapper = OutputWrapper::new(allow_cut_urls);
        assert_eq!(expected, wrapper.wrap(text, width, "\n")?, "{name}");
    }
    Ok(())
}
