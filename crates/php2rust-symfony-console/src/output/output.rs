//! Port of Symfony\Component\Console\Output\Output.php,
//! OutputInterface.php, StreamOutput.php and BufferedOutput.php.
//!
//! `Output<S>` is the abstract class; `S` is its `doWrite()`: a buffer
//! (`BufferedOutput`) or a stream (`StreamOutput`). `PHP_EOL` is `"\n"`, as
//! on Linux. Whether a stream is a terminal is given by the caller
//! (`stream_isatty()`), since a Rust writer cannot tell.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Output/Output.php Output/OutputInterface.php Output/StreamOutput.php Output/BufferedOutput.php

use std::io::Write;

use crate::{
    env::{getenv, server},
    exception::Result,
    formatter::output_formatter::OutputFormatter,
    php::strip_tags,
};

pub const VERBOSITY_SILENT: i64 = 8;
pub const VERBOSITY_QUIET: i64 = 16;
pub const VERBOSITY_NORMAL: i64 = 32;
pub const VERBOSITY_VERBOSE: i64 = 64;
pub const VERBOSITY_VERY_VERBOSE: i64 = 128;
pub const VERBOSITY_DEBUG: i64 = 256;

pub const OUTPUT_NORMAL: i64 = 1;
pub const OUTPUT_RAW: i64 = 2;
pub const OUTPUT_PLAIN: i64 = 4;

/// `OutputInterface`.
pub trait OutputInterface {
    /// `write($messages, $newline, $options)`.
    fn write_messages(&mut self, messages: &[&str], newline: bool, options: i64) -> Result<()>;

    fn write(&mut self, message: &str) -> Result<()> {
        self.write_messages(&[message], false, OUTPUT_NORMAL)
    }

    fn writeln(&mut self, message: &str) -> Result<()> {
        self.write_messages(&[message], true, OUTPUT_NORMAL)
    }

    fn set_verbosity(&mut self, level: i64);
    fn get_verbosity(&self) -> i64;
    fn set_decorated(&mut self, decorated: bool);
    fn is_decorated(&self) -> bool;
    fn set_formatter(&mut self, formatter: OutputFormatter);
    fn get_formatter(&mut self) -> &mut OutputFormatter;

    fn is_silent(&self) -> bool {
        self.get_verbosity() == VERBOSITY_SILENT
    }
    fn is_quiet(&self) -> bool {
        self.get_verbosity() == VERBOSITY_QUIET
    }
    fn is_verbose(&self) -> bool {
        VERBOSITY_VERBOSE <= self.get_verbosity()
    }
    fn is_very_verbose(&self) -> bool {
        VERBOSITY_VERY_VERBOSE <= self.get_verbosity()
    }
    fn is_debug(&self) -> bool {
        VERBOSITY_DEBUG <= self.get_verbosity()
    }
}

/// `doWrite($message, $newline)`.
pub trait Sink {
    fn do_write(&mut self, message: &str, newline: bool);
}

/// `abstract class Output`.
#[derive(Debug)]
pub struct Output<S> {
    verbosity: i64,
    formatter: OutputFormatter,
    sink: S,
}

impl<S: Sink> Output<S> {
    /// `__construct($verbosity, $decorated, $formatter)`.
    pub fn with_sink(
        sink: S,
        verbosity: Option<i64>,
        decorated: bool,
        formatter: Option<OutputFormatter>,
    ) -> Self {
        let mut formatter = formatter.unwrap_or_default();
        formatter.set_decorated(decorated);
        Self {
            verbosity: verbosity.unwrap_or(VERBOSITY_NORMAL),
            formatter,
            sink,
        }
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }
}

impl<S: Sink> OutputInterface for Output<S> {
    fn write_messages(&mut self, messages: &[&str], newline: bool, options: i64) -> Result<()> {
        let types = OUTPUT_NORMAL | OUTPUT_RAW | OUTPUT_PLAIN;
        let kind = match types & options {
            0 => OUTPUT_NORMAL,
            t => t,
        };
        let verbosities = VERBOSITY_QUIET
            | VERBOSITY_NORMAL
            | VERBOSITY_VERBOSE
            | VERBOSITY_VERY_VERBOSE
            | VERBOSITY_DEBUG;
        let verbosity = match verbosities & options {
            0 => VERBOSITY_NORMAL,
            v => v,
        };
        if verbosity > self.verbosity {
            return Ok(());
        }
        for message in messages {
            // switch ($type): a combination of types matches no case and is
            // written unformatted.
            let message = match kind {
                OUTPUT_NORMAL => self.formatter.format(Some(message))?,
                OUTPUT_PLAIN => strip_tags(&self.formatter.format(Some(message))?),
                _ => (*message).to_owned(),
            };
            self.sink.do_write(&message, newline);
        }
        Ok(())
    }

    fn set_verbosity(&mut self, level: i64) {
        self.verbosity = level;
    }
    fn get_verbosity(&self) -> i64 {
        self.verbosity
    }
    fn set_decorated(&mut self, decorated: bool) {
        self.formatter.set_decorated(decorated);
    }
    fn is_decorated(&self) -> bool {
        self.formatter.is_decorated()
    }
    fn set_formatter(&mut self, formatter: OutputFormatter) {
        self.formatter = formatter;
    }
    fn get_formatter(&mut self) -> &mut OutputFormatter {
        &mut self.formatter
    }
}

/// `BufferedOutput`'s buffer.
#[derive(Debug, Default)]
pub struct Buffer(String);

impl Sink for Buffer {
    fn do_write(&mut self, message: &str, newline: bool) {
        self.0.push_str(message);
        if newline {
            self.0.push('\n');
        }
    }
}

pub type BufferedOutput = Output<Buffer>;

impl BufferedOutput {
    /// `new BufferedOutput($verbosity, $decorated, $formatter)`.
    pub fn new(
        verbosity: Option<i64>,
        decorated: bool,
        formatter: Option<OutputFormatter>,
    ) -> Self {
        Self::with_sink(Buffer::default(), verbosity, decorated, formatter)
    }

    /// `fetch()`: the buffer, emptied.
    pub fn fetch(&mut self) -> String {
        std::mem::take(&mut self.sink.0)
    }
}

/// `StreamOutput`'s stream, and whether it is a terminal.
#[derive(Debug)]
pub struct Stream<W> {
    stream: W,
    is_tty: bool,
}

impl<W: Write> Sink for Stream<W> {
    fn do_write(&mut self, message: &str, newline: bool) {
        // @fwrite(): a failed write is silent.
        let _ = self.stream.write_all(message.as_bytes());
        if newline {
            let _ = self.stream.write_all(b"\n");
        }
        let _ = self.stream.flush();
    }
}

pub type StreamOutput<W> = Output<Stream<W>>;

impl<W: Write> StreamOutput<W> {
    /// `new StreamOutput($stream, $verbosity, $decorated, $formatter)`;
    /// `is_tty` is `stream_isatty($stream)`.
    pub fn new(
        stream: W,
        is_tty: bool,
        verbosity: i64,
        decorated: Option<bool>,
        formatter: Option<OutputFormatter>,
    ) -> Self {
        let decorated = decorated.unwrap_or_else(|| has_color_support(is_tty));
        Self::with_sink(
            Stream { stream, is_tty },
            Some(verbosity),
            decorated,
            formatter,
        )
    }

    pub fn get_stream(&mut self) -> &mut W {
        &mut self.sink.stream
    }

    pub fn is_tty(&self) -> bool {
        self.sink.is_tty
    }
}

/// `hasColorSupport()`, as on Linux.
pub fn has_color_support(is_tty: bool) -> bool {
    let first_char = |name: &str| {
        server(name)
            .or_else(|| getenv(name))
            .is_some_and(|v| !v.is_empty())
    };
    // Follow https://no-color.org/
    if first_char("NO_COLOR") {
        return false;
    }
    // Follow https://force-color.org/
    if first_char("FORCE_COLOR") {
        return true;
    }
    let msystem = getenv("MSYSTEM").unwrap_or_default().to_uppercase();
    if !is_tty && !matches!(msystem.as_str(), "MINGW32" | "MINGW64") {
        return false;
    }
    if getenv("TERM_PROGRAM").as_deref() == Some("Hyper")
        || getenv("COLORTERM").is_some()
        || getenv("ANSICON").is_some()
        || getenv("ConEmuANSI").as_deref() == Some("ON")
    {
        return true;
    }
    let term = getenv("TERM").unwrap_or_default();
    if term == "dumb" {
        return false;
    }
    supports_color_term(&term)
}

/// `/^((screen|xterm|vt100|vt220|putty|rxvt|ansi|cygwin|linux).*)|(.*-256(color)?(-bce)?)$/`.
fn supports_color_term(term: &str) -> bool {
    const PREFIXES: [&str; 9] = [
        "screen", "xterm", "vt100", "vt220", "putty", "rxvt", "ansi", "cygwin", "linux",
    ];
    if PREFIXES.iter().any(|p| term.starts_with(p)) {
        return true;
    }
    // `$` also matches before a final newline.
    let term = term.strip_suffix('\n').unwrap_or(term);
    ["-256", "-256color", "-256-bce", "-256color-bce"]
        .iter()
        .any(|s| term.ends_with(s))
}
