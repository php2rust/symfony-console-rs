//! OutputTest and StreamOutputTest, with upstream's names and assertions.
//!
//! Upstream: symfony/console Tests/Output/OutputTest.php Tests/Output/StreamOutputTest.php
#![allow(non_snake_case)]

use std::io::{self, Write};

use symfony_console::{
    ConsoleError,
    formatter::output_formatter_style::OutputFormatterStyle,
    output::output::{
        OUTPUT_PLAIN, OUTPUT_RAW, Output, OutputInterface, Sink, StreamOutput, VERBOSITY_DEBUG,
        VERBOSITY_NORMAL, VERBOSITY_QUIET, VERBOSITY_SILENT, VERBOSITY_VERBOSE,
        VERBOSITY_VERY_VERBOSE,
    },
};

type R = Result<(), ConsoleError>;

/// The test's `TestOutput extends Output`.
#[derive(Default)]
struct TestSink {
    output: String,
}

impl Sink for TestSink {
    fn do_write(&mut self, message: &str, newline: bool) {
        self.output.push_str(message);
        if newline {
            self.output.push('\n');
        }
    }
}

fn test_output(verbosity: Option<i64>, decorated: bool) -> Output<TestSink> {
    Output::with_sink(TestSink::default(), verbosity, decorated, None)
}

fn output_of(output: &Output<TestSink>) -> &str {
    &output.sink().output
}

// OutputTest

#[test]
fn testConstructor() {
    let output = test_output(Some(VERBOSITY_QUIET), true);
    assert_eq!(
        VERBOSITY_QUIET,
        output.get_verbosity(),
        "__construct() takes the verbosity as its first argument"
    );
    assert!(
        output.is_decorated(),
        "__construct() takes the decorated flag as its second argument"
    );
}

#[test]
fn testSetIsDecorated() {
    let mut output = test_output(None, false);
    output.set_decorated(true);
    assert!(
        output.is_decorated(),
        "setDecorated() sets the decorated flag"
    );
}

#[test]
fn testSetGetVerbosity() {
    let mut output = test_output(None, false);
    output.set_verbosity(VERBOSITY_QUIET);
    assert_eq!(
        VERBOSITY_QUIET,
        output.get_verbosity(),
        "->setVerbosity() sets the verbosity"
    );
    let flags = |o: &Output<TestSink>| {
        (
            o.is_quiet(),
            o.is_verbose(),
            o.is_very_verbose(),
            o.is_debug(),
        )
    };
    assert_eq!((true, false, false, false), flags(&output));
    output.set_verbosity(VERBOSITY_NORMAL);
    assert_eq!((false, false, false, false), flags(&output));
    output.set_verbosity(VERBOSITY_VERBOSE);
    assert_eq!((false, true, false, false), flags(&output));
    output.set_verbosity(VERBOSITY_VERY_VERBOSE);
    assert_eq!((false, true, true, false), flags(&output));
    output.set_verbosity(VERBOSITY_DEBUG);
    assert_eq!((false, true, true, true), flags(&output));
}

#[test]
fn testWriteWithVerbosityQuiet() -> R {
    let mut output = test_output(Some(VERBOSITY_QUIET), false);
    output.writeln("foo")?;
    assert_eq!(
        "",
        output_of(&output),
        "->writeln() outputs nothing if verbosity is set to VERBOSITY_QUIET"
    );
    Ok(())
}

#[test]
fn testWriteAnArrayOfMessages() -> R {
    let mut output = test_output(None, false);
    output.write_messages(&["foo", "bar"], true, 0)?;
    assert_eq!(
        "foo\nbar\n",
        output_of(&output),
        "->writeln() can take an array of messages to output"
    );
    Ok(())
}

#[test]
fn testWriteAnIterableOfMessages() -> R {
    let mut output = test_output(None, false);
    let generated: Vec<&str> = ["foo", "bar"].into_iter().collect();
    output.write_messages(&generated, true, 0)?;
    assert_eq!(
        "foo\nbar\n",
        output_of(&output),
        "->writeln() can take an iterable of messages to output"
    );
    Ok(())
}

#[test]
fn testWriteRawMessage() -> R {
    for (message, kind, expected) in [
        ("<info>foo</info>", OUTPUT_RAW, "<info>foo</info>\n"),
        ("<info>foo</info>", OUTPUT_PLAIN, "foo\n"),
    ] {
        let mut output = test_output(None, false);
        output.write_messages(&[message], true, kind)?;
        assert_eq!(expected, output_of(&output));
    }
    Ok(())
}

#[test]
fn testWriteWithDecorationTurnedOff() -> R {
    let mut output = test_output(None, false);
    output.set_decorated(false);
    output.writeln("<info>foo</info>")?;
    assert_eq!(
        "foo\n",
        output_of(&output),
        "->writeln() strips decoration tags if decoration is set to false"
    );
    Ok(())
}

#[test]
fn testWriteDecoratedMessage() -> R {
    let foo_style = OutputFormatterStyle::new(Some("yellow"), Some("red"), &["blink"])?;
    let mut output = test_output(None, false);
    output.get_formatter().set_style("FOO", foo_style);
    output.set_decorated(true);
    output.writeln("<foo>foo</foo>")?;
    assert_eq!(
        "\x1b[33;41;5mfoo\x1b[39;49;25m\n",
        output_of(&output),
        "->writeln() decorates the output"
    );
    Ok(())
}

#[test]
fn testWriteWithInvalidStyle() -> R {
    let mut output = test_output(None, false);
    output.sink_mut().output.clear();
    output.write("<bar>foo</bar>")?;
    assert_eq!(
        "<bar>foo</bar>",
        output_of(&output),
        "->write() do nothing when a style does not exist"
    );
    output.sink_mut().output.clear();
    output.writeln("<bar>foo</bar>")?;
    assert_eq!(
        "<bar>foo</bar>\n",
        output_of(&output),
        "->writeln() do nothing when a style does not exist"
    );
    Ok(())
}

#[test]
fn testWriteWithVerbosityOption() -> R {
    for (verbosity, expected, msg) in [
        (
            VERBOSITY_SILENT,
            "",
            "->write() in SILENT mode never outputs",
        ),
        (
            VERBOSITY_QUIET,
            "2",
            "->write() in QUIET mode only outputs when an explicit QUIET verbosity is passed",
        ),
        (
            VERBOSITY_NORMAL,
            "123",
            "->write() in NORMAL mode outputs anything below an explicit VERBOSE verbosity",
        ),
        (
            VERBOSITY_VERBOSE,
            "1234",
            "->write() in VERBOSE mode outputs anything below an explicit VERY_VERBOSE verbosity",
        ),
        (
            VERBOSITY_VERY_VERBOSE,
            "12345",
            "->write() in VERY_VERBOSE mode outputs anything below an explicit DEBUG verbosity",
        ),
        (
            VERBOSITY_DEBUG,
            "123456",
            "->write() in DEBUG mode outputs everything",
        ),
    ] {
        let mut output = test_output(None, false);
        output.set_verbosity(verbosity);
        output.sink_mut().output.clear();
        output.write_messages(&["1"], false, 0)?;
        output.write_messages(&["2"], false, VERBOSITY_QUIET)?;
        output.write_messages(&["3"], false, VERBOSITY_NORMAL)?;
        output.write_messages(&["4"], false, VERBOSITY_VERBOSE)?;
        output.write_messages(&["5"], false, VERBOSITY_VERY_VERBOSE)?;
        output.write_messages(&["6"], false, VERBOSITY_DEBUG)?;
        assert_eq!(expected, output_of(&output), "{msg}");
    }
    Ok(())
}

// StreamOutputTest. `php://memory` is a `Vec<u8>`, not a terminal.

#[test]
fn testConstructor_StreamOutput() {
    let output = StreamOutput::new(Vec::new(), false, VERBOSITY_QUIET, Some(true), None);
    assert_eq!(
        VERBOSITY_QUIET,
        output.get_verbosity(),
        "__construct() takes the verbosity as its first argument"
    );
    assert!(
        output.is_decorated(),
        "__construct() takes the decorated flag as its second argument"
    );
}

#[test]
fn testGetStream() {
    let mut output = StreamOutput::new(b"stream".to_vec(), false, VERBOSITY_NORMAL, None, None);
    assert_eq!(
        b"stream",
        output.get_stream().as_slice(),
        "->getStream() returns the current stream"
    );
}

#[test]
fn testDoWrite() -> R {
    let mut output = StreamOutput::new(Vec::new(), false, VERBOSITY_NORMAL, None, None);
    output.writeln("foo")?;
    assert_eq!(
        b"foo\n",
        output.get_stream().as_slice(),
        "->doWrite() writes to the stream"
    );
    Ok(())
}

/// A stream opened read-only: every write fails, silently (`@fwrite`).
struct ReadOnly(Vec<u8>);

impl Write for ReadOnly {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn testDoWriteOnFailure() {
    let output = StreamOutput::new(ReadOnly(Vec::new()), false, VERBOSITY_NORMAL, None, None);
    let mut output = output;
    let _ = output.writeln("foo");
    assert_eq!(b"", output.get_stream().0.as_slice());
}
