//! `GithubActionReporterTest`, with upstream's names, assertions and
//! provider rows.
//!
//! Upstream: symfony/console Tests/CI/GithubActionReporterTest.php
#![allow(non_snake_case)]

use symfony_console::{
    ConsoleError,
    ci::github_action_reporter::GithubActionReporter,
    env::{getenv, putenv},
    output::output::BufferedOutput,
};

#[test]
fn testIsGithubActionEnvironment() {
    let prev = getenv("GITHUB_ACTIONS");
    putenv("GITHUB_ACTIONS", None);

    assert!(!GithubActionReporter::is_github_action_environment());
    putenv("GITHUB_ACTIONS", Some("1"));
    assert!(GithubActionReporter::is_github_action_environment());

    putenv("GITHUB_ACTIONS", prev.as_deref().filter(|p| !p.is_empty()));
}

type Annotation = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<i64>,
    Option<i64>,
    &'static str,
);

fn annotations_format_provider() -> Vec<Annotation> {
    vec![
        (
            "warning",
            "warning",
            "A warning",
            None,
            None,
            None,
            "::warning::A warning",
        ),
        (
            "error",
            "error",
            "An error",
            None,
            None,
            None,
            "::error::An error",
        ),
        (
            "debug",
            "debug",
            "A debug log",
            None,
            None,
            None,
            "::debug::A debug log",
        ),
        (
            "with message to escape",
            "debug",
            "There are 100% chances\nfor this to be escaped properly\rRight?",
            None,
            None,
            None,
            "::debug::There are 100%25 chances%0Afor this to be escaped properly%0DRight?",
        ),
        (
            "with meta",
            "warning",
            "A warning",
            Some("foo/bar.php"),
            Some(2),
            Some(4),
            "::warning file=foo/bar.php,line=2,col=4::A warning",
        ),
        (
            "with file property to escape",
            "warning",
            "A warning",
            Some("foo,bar:baz%quz.php"),
            Some(2),
            Some(4),
            "::warning file=foo%2Cbar%3Abaz%25quz.php,line=2,col=4::A warning",
        ),
        (
            "without file ignores col & line",
            "warning",
            "A warning",
            None,
            Some(2),
            Some(4),
            "::warning::A warning",
        ),
    ]
}

#[test]
fn testAnnotationsFormat() -> Result<(), ConsoleError> {
    for (name, kind, message, file, line, col, expected) in annotations_format_provider() {
        let mut buffer = BufferedOutput::new(None, false, None);
        {
            let mut reporter = GithubActionReporter::new(&mut buffer);
            match kind {
                "warning" => reporter.warning(message, file, line, col)?,
                "error" => reporter.error(message, file, line, col)?,
                _ => reporter.debug(message, file, line, col)?,
            }
        }
        assert_eq!(format!("{expected}\n"), buffer.fetch(), "{name}");
    }
    Ok(())
}
