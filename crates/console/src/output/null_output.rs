//! Port of Symfony\Component\Console\Output\NullOutput.php.
//!
//! `getFormatter()` returns a `NullOutputFormatter` in PHP; here it is an
//! undecorated `OutputFormatter` that nothing writes through (the
//! `OutputInterface` trait returns the concrete formatter).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Output/NullOutput.php

use super::output::{OutputInterface, VERBOSITY_SILENT};
use crate::{exception::Result, formatter::output_formatter::OutputFormatter};

/// `NullOutput`: suppresses all output.
#[derive(Debug, Default)]
pub struct NullOutput {
    formatter: OutputFormatter,
}

impl NullOutput {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OutputInterface for NullOutput {
    fn write_messages(&mut self, _: &[&str], _: bool, _: i64) -> Result<()> {
        Ok(())
    }
    fn set_verbosity(&mut self, _: i64) {}
    fn get_verbosity(&self) -> i64 {
        VERBOSITY_SILENT
    }
    fn set_decorated(&mut self, _: bool) {}
    fn is_decorated(&self) -> bool {
        false
    }
    fn set_formatter(&mut self, _: OutputFormatter) {}
    fn get_formatter(&mut self) -> &mut OutputFormatter {
        &mut self.formatter
    }
    fn is_silent(&self) -> bool {
        true
    }
    fn is_quiet(&self) -> bool {
        false
    }
    fn is_verbose(&self) -> bool {
        false
    }
    fn is_very_verbose(&self) -> bool {
        false
    }
    fn is_debug(&self) -> bool {
        false
    }
}
