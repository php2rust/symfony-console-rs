//! Port of Symfony\Component\Console\CI\GithubActionReporter.php:
//! errors, warnings and debug logs in GitHub Actions' annotation format.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console CI/GithubActionReporter.php

use crate::{env::getenv, exception::Result, output::output::OutputInterface};

/// `ESCAPED_DATA`: `strtr()` with single-character keys.
fn escape_data(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '%' => out.push_str("%25"),
            '\r' => out.push_str("%0D"),
            '\n' => out.push_str("%0A"),
            c => out.push(c),
        }
    }
    out
}

/// `ESCAPED_PROPERTIES`.
fn escape_property(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

/// `GithubActionReporter`.
pub struct GithubActionReporter<'a> {
    output: &'a mut dyn OutputInterface,
}

impl<'a> GithubActionReporter<'a> {
    pub fn new(output: &'a mut dyn OutputInterface) -> Self {
        Self { output }
    }

    /// `isGithubActionEnvironment()`: `getenv('GITHUB_ACTIONS')` is set.
    pub fn is_github_action_environment() -> bool {
        getenv("GITHUB_ACTIONS").is_some()
    }

    /// `error($message, $file, $line, $col)`.
    pub fn error(
        &mut self,
        message: &str,
        file: Option<&str>,
        line: Option<i64>,
        col: Option<i64>,
    ) -> Result<()> {
        self.log("error", message, file, line, col)
    }

    /// `warning($message, $file, $line, $col)`.
    pub fn warning(
        &mut self,
        message: &str,
        file: Option<&str>,
        line: Option<i64>,
        col: Option<i64>,
    ) -> Result<()> {
        self.log("warning", message, file, line, col)
    }

    /// `debug($message, $file, $line, $col)`.
    pub fn debug(
        &mut self,
        message: &str,
        file: Option<&str>,
        line: Option<i64>,
        col: Option<i64>,
    ) -> Result<()> {
        self.log("debug", message, file, line, col)
    }

    fn log(
        &mut self,
        kind: &str,
        message: &str,
        file: Option<&str>,
        line: Option<i64>,
        col: Option<i64>,
    ) -> Result<()> {
        // Some values must be encoded.
        let message = escape_data(message);

        // !$file: null, "" and "0"
        let Some(file) = file.filter(|f| !f.is_empty() && *f != "0") else {
            // No file provided, output the message solely:
            return self.output.writeln(&format!("::{kind}::{message}"));
        };

        self.output.writeln(&format!(
            "::{kind} file={},line={},col={}::{message}",
            escape_property(file),
            escape_property(&line.unwrap_or(1).to_string()),
            escape_property(&col.unwrap_or(0).to_string()),
        ))
    }
}
