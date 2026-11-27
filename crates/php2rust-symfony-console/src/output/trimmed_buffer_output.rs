//! Port of Symfony\Component\Console\Output\TrimmedBufferOutput.php: a
//! buffered output that keeps only its last bytes.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Output/TrimmedBufferOutput.php

use super::output::{Output, Sink};
use crate::{
    exception::{ConsoleError, Result},
    formatter::output_formatter::OutputFormatter,
};

/// The buffer: its last `max_length` bytes.
#[derive(Debug, Default)]
pub struct TrimmedBuffer {
    max_length: usize,
    buffer: Vec<u8>,
}

impl Sink for TrimmedBuffer {
    /// `doWrite($message, $newline)`: `substr($buffer, -$maxLength)`.
    fn do_write(&mut self, message: &str, newline: bool) {
        self.buffer.extend_from_slice(message.as_bytes());
        if newline {
            self.buffer.push(b'\n');
        }
        let excess = self.buffer.len().saturating_sub(self.max_length);
        self.buffer.drain(..excess);
    }
}

pub type TrimmedBufferOutput = Output<TrimmedBuffer>;

impl TrimmedBufferOutput {
    /// `new TrimmedBufferOutput($maxLength, $verbosity, $decorated, $formatter)`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    pub fn new(
        max_length: i64,
        verbosity: Option<i64>,
        decorated: bool,
        formatter: Option<OutputFormatter>,
    ) -> Result<Self> {
        if max_length <= 0 {
            return Err(ConsoleError::InvalidArgument(format!(
                "\"Symfony\\Component\\Console\\Output\\TrimmedBufferOutput::__construct()\" expects a strictly positive maxLength. Got {max_length}."
            )));
        }
        let sink = TrimmedBuffer {
            max_length: max_length as usize,
            buffer: Vec::new(),
        };
        Ok(Self::with_sink(sink, verbosity, decorated, formatter))
    }

    /// `fetch()`: the buffer, emptied. Its bytes may end in the middle of
    /// a character.
    pub fn fetch(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.sink_mut().buffer)
    }
}
