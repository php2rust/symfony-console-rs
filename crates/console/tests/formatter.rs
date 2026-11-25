//! `OutputFormatterTest`, `OutputFormatterStyleTest` and
//! `OutputFormatterStyleStackTest`, with upstream's names, assertions and
//! provider rows (transcribed: they are literals).
//!
//! Upstream: symfony/console Tests/Formatter/OutputFormatterTest.php Tests/Formatter/OutputFormatterStyleTest.php Tests/Formatter/OutputFormatterStyleStackTest.php
#![allow(non_snake_case)]

use symfony_console::{
    ConsoleError,
    env::{getenv, putenv, server, set_server},
    formatter::{
        output_formatter::OutputFormatter, output_formatter_style::OutputFormatterStyle,
        output_formatter_style_stack::OutputFormatterStyleStack,
    },
};

type R = Result<(), ConsoleError>;

fn style(fg: Option<&str>, bg: Option<&str>, options: &[&str]) -> OutputFormatterStyle {
    OutputFormatterStyle::new(fg, bg, options).unwrap_or_else(|e| panic!("{e}"))
}

fn f(formatter: &mut OutputFormatter, message: &str) -> String {
    formatter
        .format(Some(message))
        .unwrap_or_else(|e| panic!("{e}"))
}

// OutputFormatterTest

#[test]
fn testEmptyTag() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!("foo<>bar", f(&mut formatter, "foo<>bar"));
}

#[test]
fn testLGCharEscaping() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!("foo<bar", f(&mut formatter, "foo\\<bar"));
    assert_eq!("foo << bar", f(&mut formatter, "foo << bar"));
    assert_eq!("foo << bar \\", f(&mut formatter, "foo << bar \\"));
    assert_eq!(
        "foo << \x1b[32mbar \\ baz\x1b[39m \\",
        f(&mut formatter, "foo << <info>bar \\ baz</info> \\")
    );
    assert_eq!(
        "<info>some info</info>",
        f(&mut formatter, "\\<info>some info\\</info>")
    );
    assert_eq!(
        "\\<info\\>some info\\</info\\>",
        OutputFormatter::escape("<info>some info</info>")
    );
    // every < and > gets escaped, backslashes are left as they are
    assert_eq!(
        "foo \\< bar \\\\< baz \\\\\\< foo \\> bar \\\\> baz \\\\\\> \\x",
        OutputFormatter::escape("foo < bar \\< baz \\\\< foo > bar \\> baz \\\\> \\x")
    );
    assert_eq!(
        "\x1b[33mSymfony\\Component\\Console does work very well!\x1b[39m",
        f(
            &mut formatter,
            "<comment>Symfony\\Component\\Console does work very well!</comment>"
        )
    );
}

#[test]
fn testFormatRestoresEscapedText() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "foo \\<bar",
        f(&mut formatter, &OutputFormatter::escape("foo \\<bar"))
    );
    assert_eq!(
        "foo \\>bar",
        f(&mut formatter, &OutputFormatter::escape("foo \\>bar"))
    );
    assert_eq!(
        "foo \\\\<bar>",
        f(&mut formatter, &OutputFormatter::escape("foo \\\\<bar>"))
    );
    assert_eq!(
        "foo \\<info>bar\\</info>",
        f(
            &mut formatter,
            &OutputFormatter::escape("foo \\<info>bar\\</info>")
        )
    );
    assert_eq!(
        "foo \\",
        f(&mut formatter, &OutputFormatter::escape("foo \\"))
    );
}

#[test]
fn testBundledStyles() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert!(formatter.has_style("error"));
    assert!(formatter.has_style("info"));
    assert!(formatter.has_style("comment"));
    assert!(formatter.has_style("question"));
    assert_eq!(
        "\x1b[37;41msome error\x1b[39;49m",
        f(&mut formatter, "<error>some error</error>")
    );
    assert_eq!(
        "\x1b[32msome info\x1b[39m",
        f(&mut formatter, "<info>some info</info>")
    );
    assert_eq!(
        "\x1b[33msome comment\x1b[39m",
        f(&mut formatter, "<comment>some comment</comment>")
    );
    assert_eq!(
        "\x1b[30;46msome question\x1b[39;49m",
        f(&mut formatter, "<question>some question</question>")
    );
}

#[test]
fn testNestedStyles() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "\x1b[37;41msome \x1b[39;49m\x1b[32msome info\x1b[39m\x1b[37;41m error\x1b[39;49m",
        f(
            &mut formatter,
            "<error>some <info>some info</info> error</error>"
        )
    );
}

#[test]
fn testAdjacentStyles() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "\x1b[37;41msome error\x1b[39;49m\x1b[32msome info\x1b[39m",
        f(
            &mut formatter,
            "<error>some error</error><info>some info</info>"
        )
    );
}

#[test]
fn testStyleMatchingNotGreedy() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "(\x1b[32m>=2.0,<2.3\x1b[39m)",
        f(&mut formatter, "(<info>>=2.0,<2.3</info>)")
    );
}

#[test]
fn testStyleEscaping() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    let message = format!(
        "(<info>{}</info>)",
        OutputFormatter::escape("z>=2.0,<\\<<a2.3\\")
    );
    assert_eq!(
        "(\x1b[32mz>=2.0,<\\<<a2.3\\\x1b[39m)",
        f(&mut formatter, &message)
    );
    let message = format!(
        "<info>{}</info>",
        OutputFormatter::escape("<error>some error</error>")
    );
    assert_eq!(
        "\x1b[32m<error>some error</error>\x1b[39m",
        f(&mut formatter, &message)
    );
}

#[test]
fn testDeepNestedStyles() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "\x1b[37;41merror\x1b[39;49m\x1b[32minfo\x1b[39m\x1b[33mcomment\x1b[39m\x1b[37;41merror\x1b[39;49m",
        f(
            &mut formatter,
            "<error>error<info>info<comment>comment</info>error</error>"
        )
    );
}

#[test]
fn testNewStyle() -> R {
    let mut formatter = OutputFormatter::new(true, vec![]);
    let test = style(Some("blue"), Some("white"), &[]);
    formatter.set_style("test", test.clone());
    assert_eq!(&test, formatter.get_style("test")?);
    assert_ne!(&test, formatter.get_style("info")?);
    formatter.set_style("b", style(Some("blue"), Some("white"), &[]));
    assert_eq!(
        "\x1b[34;47msome \x1b[39;49m\x1b[34;47mcustom\x1b[39;49m\x1b[34;47m msg\x1b[39;49m",
        f(&mut formatter, "<test>some <b>custom</b> msg</test>")
    );
    Ok(())
}

#[test]
fn testRedefineStyle() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    formatter.set_style("info", style(Some("blue"), Some("white"), &[]));
    assert_eq!(
        "\x1b[34;47msome custom msg\x1b[39;49m",
        f(&mut formatter, "<info>some custom msg</info>")
    );
}

#[test]
fn testInlineStyle() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "\x1b[34;41msome text\x1b[39;49m",
        f(&mut formatter, "<fg=blue;bg=red>some text</>")
    );
    assert_eq!(
        "\x1b[34;41msome text\x1b[39;49m",
        f(&mut formatter, "<fg=blue;bg=red>some text</fg=blue;bg=red>")
    );
}

/// `provideInlineStyleOptionsCases`. `createStyleFromString()` is private:
/// whether it returns a style shows in `format()`, which keeps an unknown
/// tag as text and applies a known one.
#[test]
fn testInlineStyleOptions() {
    let cases: [(&str, Option<&str>, Option<&str>, bool); 9] = [
        ("<unknown=_unknown_>", None, None, false),
        ("<unknown=_unknown_;a=1;b>", None, None, false),
        (
            "<fg=green;>",
            Some("\x1b[32m[test]\x1b[39m"),
            Some("[test]"),
            false,
        ),
        (
            "<fg=green;bg=blue;>",
            Some("\x1b[32;44ma\x1b[39;49m"),
            Some("a"),
            false,
        ),
        (
            "<fg=green;options=bold>",
            Some("\x1b[32;1mb\x1b[39;22m"),
            Some("b"),
            false,
        ),
        (
            "<fg=green;options=reverse;>",
            Some("\x1b[32;7m<a>\x1b[39;27m"),
            Some("<a>"),
            false,
        ),
        (
            "<fg=green;options=bold,underscore>",
            Some("\x1b[32;1;4mz\x1b[39;22;24m"),
            Some("z"),
            false,
        ),
        (
            "<fg=green;options=bold,underscore,reverse;>",
            Some("\x1b[32;1;4;7md\x1b[39;22;24;27m"),
            Some("d"),
            false,
        ),
        (
            "<fg=#00ff00;bg=#00f>",
            Some("\x1b[38;2;0;255;0;48;2;0;0;255m[test]\x1b[39;49m"),
            Some("[test]"),
            true,
        ),
    ];
    for (tag, expected, input, truecolor) in cases {
        if truecolor && getenv("COLORTERM").as_deref() != Some("truecolor") {
            // markTestSkipped('The terminal does not support true colors.')
            continue;
        }
        let style_string = &tag[1..tag.len() - 1];
        let mut formatter = OutputFormatter::new(true, vec![]);
        let input = input.unwrap_or_default();
        match expected {
            None => {
                let expected = format!("{tag}{input}</{style_string}>");
                assert_eq!(expected, f(&mut formatter, &expected), "{tag}");
            }
            Some(expected) => {
                assert_eq!(
                    expected,
                    f(&mut formatter, &format!("{tag}{input}</>")),
                    "{tag}"
                );
                assert_eq!(
                    expected,
                    f(&mut formatter, &format!("{tag}{input}</{style_string}>")),
                    "{tag}"
                );
            }
        }
    }
}

#[test]
fn testNonStyleTag() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "\x1b[32msome \x1b[39m\x1b[32m<tag>\x1b[39m\x1b[32m \x1b[39m\x1b[32m<setting=value>\x1b[39m\x1b[32m styled \x1b[39m\x1b[32m<p>\x1b[39m\x1b[32msingle-char tag\x1b[39m\x1b[32m</p>\x1b[39m",
        f(
            &mut formatter,
            "<info>some <tag> <setting=value> styled <p>single-char tag</p></info>"
        )
    );
}

#[test]
fn testFormatLongString() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    let long = "\\".repeat(14000);
    assert_eq!(
        format!("\x1b[37;41msome error\x1b[39;49m{long}"),
        f(&mut formatter, &format!("<error>some error</error>{long}"))
    );
}

/// The test's `TableCell` is a `Stringable`; PHP's `format()` takes its
/// `__toString()`, as a Rust caller does.
#[test]
fn testFormatToStringObject() {
    let mut formatter = OutputFormatter::new(false, vec![]);
    let table_cell = "<info>some info</info>";
    assert_eq!("some info", f(&mut formatter, table_cell));
}

#[test]
fn testFormatterHasStyles() {
    let formatter = OutputFormatter::new(false, vec![]);
    assert!(formatter.has_style("error"));
    assert!(formatter.has_style("info"));
    assert!(formatter.has_style("comment"));
    assert!(formatter.has_style("question"));
}

/// `provideDecoratedAndNonDecoratedOutput`.
fn decorated_and_non_decorated_output() -> Vec<(&'static str, &'static str, &'static str, bool)> {
    vec![
        (
            "<error>some error</error>",
            "some error",
            "\x1b[37;41msome error\x1b[39;49m",
            false,
        ),
        (
            "<info>some info</info>",
            "some info",
            "\x1b[32msome info\x1b[39m",
            false,
        ),
        (
            "<comment>some comment</comment>",
            "some comment",
            "\x1b[33msome comment\x1b[39m",
            false,
        ),
        (
            "<question>some question</question>",
            "some question",
            "\x1b[30;46msome question\x1b[39;49m",
            false,
        ),
        (
            "<fg=red>some text with inline style</>",
            "some text with inline style",
            "\x1b[31msome text with inline style\x1b[39m",
            false,
        ),
        (
            "<href=idea://open/?file=/path/SomeFile.php&line=12>some URL</>",
            "some URL",
            "\x1b]8;;idea://open/?file=/path/SomeFile.php&line=12\x1b\\some URL\x1b]8;;\x1b\\",
            false,
        ),
        (
            "<href=https://example.com/\\<woohoo\\>>some URL with \\<woohoo\\></>",
            "some URL with <woohoo>",
            "\x1b]8;;https://example.com/<woohoo>\x1b\\some URL with <woohoo>\x1b]8;;\x1b\\",
            false,
        ),
        (
            "<href=idea://open/?file=/path/SomeFile.php&line=12>some URL</>",
            "some URL",
            "some URL",
            true,
        ),
    ]
}

#[test]
fn testNotDecoratedFormatterOnJediTermEmulator() {
    for (input, non_decorated, decorated, jedi) in decorated_and_non_decorated_output() {
        let previous = getenv("TERMINAL_EMULATOR");
        putenv(
            "TERMINAL_EMULATOR",
            Some(if jedi {
                "JetBrains-JediTerm"
            } else {
                "Unknown"
            }),
        );
        assert_eq!(
            decorated,
            f(&mut OutputFormatter::new(true, vec![]), input),
            "{input}"
        );
        assert_eq!(
            non_decorated,
            f(&mut OutputFormatter::new(false, vec![]), input),
            "{input}"
        );
        putenv(
            "TERMINAL_EMULATOR",
            previous.as_deref().filter(|p| !p.is_empty()),
        );
    }
}

#[test]
fn testNotDecoratedFormatterOnIDEALikeEnvironment() {
    for (input, non_decorated, decorated, idea) in decorated_and_non_decorated_output() {
        let previous = server("IDEA_INITIAL_DIRECTORY");
        if idea {
            set_server("IDEA_INITIAL_DIRECTORY", Some(env!("CARGO_MANIFEST_DIR")));
        } else if previous.is_some() {
            set_server("IDEA_INITIAL_DIRECTORY", None);
        }
        assert_eq!(
            decorated,
            f(&mut OutputFormatter::new(true, vec![]), input),
            "{input}"
        );
        assert_eq!(
            non_decorated,
            f(&mut OutputFormatter::new(false, vec![]), input),
            "{input}"
        );
        set_server("IDEA_INITIAL_DIRECTORY", previous.as_deref());
    }
}

#[test]
fn testContentWithLineBreaks() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    assert_eq!(
        "\x1b[32m\nsome text\x1b[39m",
        f(&mut formatter, "<info>\nsome text</info>")
    );
    assert_eq!(
        "\x1b[32msome text\n\x1b[39m",
        f(&mut formatter, "<info>some text\n</info>")
    );
    assert_eq!(
        "\x1b[32m\nsome text\n\x1b[39m",
        f(&mut formatter, "<info>\nsome text\n</info>")
    );
    assert_eq!(
        "\x1b[32m\nsome text\nmore text\n\x1b[39m",
        f(&mut formatter, "<info>\nsome text\nmore text\n</info>")
    );
}

fn w(formatter: &mut OutputFormatter, message: &str, width: usize) -> String {
    formatter
        .format_and_wrap(Some(message), width)
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn testFormatAndWrap() {
    let mut formatter = OutputFormatter::new(true, vec![]);
    let x = &mut formatter;
    assert_eq!(
        "fo\no\x1b[37;41mb\x1b[39;49m\n\x1b[37;41mar\x1b[39;49m\nba\nz",
        w(x, "foo<error>bar</error> baz", 2)
    );
    assert_eq!(
        "pr\ne \x1b[37;41m\x1b[39;49m\n\x1b[37;41mfo\x1b[39;49m\n\x1b[37;41mo\x1b[39;49m\n\x1b[37;41mba\x1b[39;49m\n\x1b[37;41mr\x1b[39;49m\n\x1b[37;41mba\x1b[39;49m\n\x1b[37;41mz\x1b[39;49m \npo\nst",
        w(x, "pre <error>foo bar baz</error> post", 2)
    );
    assert_eq!(
        "pre\x1b[37;41m\x1b[39;49m\n\x1b[37;41mfoo\x1b[39;49m\n\x1b[37;41mbar\x1b[39;49m\n\x1b[37;41mbaz\x1b[39;49m\npos\nt",
        w(x, "pre <error>foo bar baz</error> post", 3)
    );
    assert_eq!(
        "pre \x1b[37;41m\x1b[39;49m\n\x1b[37;41mfoo\x1b[39;49m\n\x1b[37;41mbar\x1b[39;49m\n\x1b[37;41mbaz\x1b[39;49m \npost",
        w(x, "pre <error>foo bar baz</error> post", 4)
    );
    assert_eq!(
        "pre \x1b[37;41mf\x1b[39;49m\n\x1b[37;41moo\x1b[39;49m\n\x1b[37;41mbar\x1b[39;49m\n\x1b[37;41mbaz\x1b[39;49m p\nost",
        w(x, "pre <error>foo bar baz</error> post", 5)
    );
    assert_eq!(
        "Lore\nm \x1b[37;41mip\x1b[39;49m\n\x1b[37;41msum\x1b[39;49m \ndolo\nr \x1b[32msi\x1b[39m\n\x1b[32mt\x1b[39m am\net",
        w(
            x,
            "Lorem <error>ipsum</error> dolor <info>sit</info> amet",
            4
        )
    );
    assert_eq!(
        "Lorem \x1b[37;41mip\x1b[39;49m\n\x1b[37;41msum\x1b[39;49m dolo\nr \x1b[32msit\x1b[39m am\net",
        w(
            x,
            "Lorem <error>ipsum</error> dolor <info>sit</info> amet",
            8
        )
    );
    assert_eq!(
        "Lorem \x1b[37;41mipsum\x1b[39;49m dolor \x1b[32m\x1b[39m\n\x1b[32msit\x1b[39m, \x1b[37;41mamet\x1b[39;49m et \x1b[32mlauda\x1b[39m\n\x1b[32mntium\x1b[39m architecto",
        w(
            x,
            "Lorem <error>ipsum</error> dolor <info>sit</info>, <error>amet</error> et <info>laudantium</info> architecto",
            18
        )
    );
    assert_eq!(
        "\x1b[37;41mnon-empty-array\x1b[39;49m\x1b[37;41m<mixed, mixed>\x1b[39;49m given.\n🪪  argument.type",
        w(
            x,
            "<error>non-empty-array<mixed, mixed></error> given.\n🪪  argument.type",
            38
        )
    );
    assert_eq!(
        "Usuário <strong>{{user_name}}</strong> não é válid\no.",
        w(
            x,
            "Usuário <strong>{{user_name}}</strong> não é válido.",
            50
        )
    );
    assert_eq!(
        "foo\x1b[37;41mb\x1b[39;49m\n\x1b[37;41mar\x1b[39;49mbaz",
        w(x, "foo<error>b\nar</error>baz", 7)
    );
    assert_eq!(
        "foo\x1b[37;41mbar\x1b[39;49mbaz\nnewline",
        w(x, "foo<error>bar</error>baz\nnewline", 11)
    );
    assert_eq!(
        "foobarbaz\n\x1b[37;41mnewline\x1b[39;49m",
        w(x, "foobarbaz\n<error>newline</error>", 11)
    );
    assert_eq!(
        "foobar\x1b[37;41mbaz\x1b[39;49m\n\x1b[37;41mnewline\x1b[39;49m",
        w(x, "foobar<error>baz\nnewline</error>", 11)
    );
    assert_eq!(
        "foobar\x1b[37;41mbazne\x1b[39;49m\n\x1b[37;41mwline\x1b[39;49m",
        w(x, "foobar<error>bazne\nwline</error>", 11)
    );
    assert_eq!(
        "foobar\x1b[37;41mbazne\x1b[39;49m\n\x1b[37;41mw\x1b[39;49m\n\x1b[37;41mline\x1b[39;49m",
        w(x, "foobar<error>baznew\nline</error>", 11)
    );
    assert_eq!("\x1b[37;41m👩‍🌾\x1b[39;49m", w(x, "<error>👩‍🌾</error>", 1));

    let mut formatter = OutputFormatter::new(false, vec![]);
    let x = &mut formatter;
    assert_eq!("fo\nob\nar\nba\nz", w(x, "foo<error>bar</error> baz", 2));
    assert_eq!(
        "pr\ne \nfo\no\nba\nr\nba\nz \npo\nst",
        w(x, "pre <error>foo bar baz</error> post", 2)
    );
    assert_eq!(
        "pre\nfoo\nbar\nbaz\npos\nt",
        w(x, "pre <error>foo bar baz</error> post", 3)
    );
    assert_eq!(
        "pre \nfoo\nbar\nbaz \npost",
        w(x, "pre <error>foo bar baz</error> post", 4)
    );
    assert_eq!(
        "pre f\noo\nbar\nbaz p\nost",
        w(x, "pre <error>foo bar baz</error> post", 5)
    );
    assert_eq!(
        "Â rèälly\nlöng tîtlè\nthät cöüld\nnèêd\nmúltîplê\nlínès",
        w(x, "Â rèälly löng tîtlè thät cöüld nèêd múltîplê línès", 10)
    );
    assert_eq!(
        "Â rèälly\nlöng tîtlè\nthät cöüld\nnèêd\nmúltîplê\n línès",
        w(
            x,
            "Â rèälly löng tîtlè thät cöüld nèêd múltîplê\n línès",
            10
        )
    );
    assert_eq!(
        "",
        x.format_and_wrap(None, 5).unwrap_or_else(|e| panic!("{e}"))
    );
    assert_eq!(
        "non-empty-array<mixed, mixed> given.\n🪪  argument.type",
        w(
            x,
            "<error>non-empty-array<mixed, mixed></error> given.\n🪪  argument.type",
            38
        )
    );
    assert_eq!(
        "Usuário <strong>{{user_name}}</strong> não é válid\no.",
        w(
            x,
            "Usuário <strong>{{user_name}}</strong> não é válido.",
            50
        )
    );
    assert_eq!("foob\narbaz", w(x, "foo<error>b\nar</error>baz", 7));
    assert_eq!(
        "foobarbaz\nnewline",
        w(x, "foo<error>bar</error>baz\nnewline", 11)
    );
    assert_eq!(
        "foobarbaz\nnewline",
        w(x, "foobarbaz\n<error>newline</error>", 11)
    );
    assert_eq!(
        "foobarbaz\nnewline",
        w(x, "foobar<error>baz\nnewline</error>", 11)
    );
    assert_eq!(
        "foobarbazne\nwline",
        w(x, "foobar<error>bazne\nwline</error>", 11)
    );
    assert_eq!(
        "foobarbazne\nw\nline",
        w(x, "foobar<error>baznew\nline</error>", 11)
    );
    assert_eq!("👩‍🌾", w(x, "👩‍🌾", 1));
}

#[test]
fn testFormatKeepsMultiByteContentUnchanged() {
    let mut formatter = OutputFormatter::default();
    assert_eq!("e\u{0301}x", f(&mut formatter, "e<info>\u{0301}x</info>"));
    assert_eq!(
        "a\u{0301} e\u{0301}",
        f(&mut formatter, "a\u{0301} <info>e\u{0301}</info>")
    );
    assert_eq!(
        "não é válido 日本語 😀",
        f(
            &mut formatter,
            "não <info>é válido</info> 日本語 <comment>😀</comment>"
        )
    );
}

// OutputFormatterStyleTest

#[test]
fn testConstructor() {
    assert_eq!(
        "\x1b[32;40;1;4mfoo\x1b[39;49;22;24m",
        style(Some("green"), Some("black"), &["bold", "underscore"]).apply("foo")
    );
    assert_eq!(
        "\x1b[31;5mfoo\x1b[39;25m",
        style(Some("red"), None, &["blink"]).apply("foo")
    );
    assert_eq!(
        "\x1b[47mfoo\x1b[49m",
        style(None, Some("white"), &[]).apply("foo")
    );
}

#[test]
fn testForeground() -> R {
    let mut s = OutputFormatterStyle::empty();
    s.set_foreground(Some("black"))?;
    assert_eq!("\x1b[30mfoo\x1b[39m", s.apply("foo"));
    s.set_foreground(Some("blue"))?;
    assert_eq!("\x1b[34mfoo\x1b[39m", s.apply("foo"));
    s.set_foreground(Some("default"))?;
    assert_eq!("\x1b[39mfoo\x1b[39m", s.apply("foo"));
    assert!(
        s.set_foreground(Some("undefined-color"))
            .is_err_and(|e| e.is_invalid_argument())
    );
    Ok(())
}

#[test]
fn testBackground() -> R {
    let mut s = OutputFormatterStyle::empty();
    s.set_background(Some("black"))?;
    assert_eq!("\x1b[40mfoo\x1b[49m", s.apply("foo"));
    s.set_background(Some("yellow"))?;
    assert_eq!("\x1b[43mfoo\x1b[49m", s.apply("foo"));
    s.set_background(Some("default"))?;
    assert_eq!("\x1b[49mfoo\x1b[49m", s.apply("foo"));
    assert!(
        s.set_background(Some("undefined-color"))
            .is_err_and(|e| e.is_invalid_argument())
    );
    Ok(())
}

#[test]
fn testOptions() -> R {
    let mut s = OutputFormatterStyle::empty();
    s.set_options(&["reverse", "conceal"])?;
    assert_eq!("\x1b[7;8mfoo\x1b[27;28m", s.apply("foo"));
    s.set_option("bold")?;
    assert_eq!("\x1b[7;8;1mfoo\x1b[27;28;22m", s.apply("foo"));
    s.unset_option("reverse")?;
    assert_eq!("\x1b[8;1mfoo\x1b[28;22m", s.apply("foo"));
    s.set_option("bold")?;
    assert_eq!("\x1b[8;1mfoo\x1b[28;22m", s.apply("foo"));
    s.set_options(&["bold"])?;
    assert_eq!("\x1b[1mfoo\x1b[22m", s.apply("foo"));
    match s.set_option("foo") {
        Ok(()) => panic!(
            "->setOption() throws an \\InvalidArgumentException when the option does not exist in the available options"
        ),
        Err(e) => {
            assert!(e.is_invalid_argument());
            assert!(e.message().contains("Invalid option specified: \"foo\""));
        }
    }
    Ok(())
}

#[test]
fn testHref() {
    let previous = getenv("TERMINAL_EMULATOR");
    putenv("TERMINAL_EMULATOR", None);
    let mut s = OutputFormatterStyle::empty();
    s.set_href("idea://open/?file=/path/SomeFile.php&line=12");
    assert_eq!(
        "\x1b]8;;idea://open/?file=/path/SomeFile.php&line=12\x1b\\some URL\x1b]8;;\x1b\\",
        s.apply("some URL")
    );
    putenv(
        "TERMINAL_EMULATOR",
        previous.as_deref().filter(|p| !p.is_empty()),
    );
}

// OutputFormatterStyleStackTest

#[test]
fn testPush() {
    let mut stack = OutputFormatterStyleStack::default();
    stack.push(style(Some("white"), Some("black"), &[]));
    let s2 = style(Some("yellow"), Some("blue"), &[]);
    stack.push(s2.clone());
    assert_eq!(&s2, stack.get_current());
    let s3 = style(Some("green"), Some("red"), &[]);
    stack.push(s3.clone());
    assert_eq!(&s3, stack.get_current());
}

#[test]
fn testPop() -> R {
    let mut stack = OutputFormatterStyleStack::default();
    let s1 = style(Some("white"), Some("black"), &[]);
    let s2 = style(Some("yellow"), Some("blue"), &[]);
    stack.push(s1.clone());
    stack.push(s2.clone());
    assert_eq!(s2, stack.pop(None)?);
    assert_eq!(s1, stack.pop(None)?);
    Ok(())
}

#[test]
fn testPopEmpty() -> R {
    let mut stack = OutputFormatterStyleStack::default();
    assert_eq!(OutputFormatterStyle::empty(), stack.pop(None)?);
    Ok(())
}

#[test]
fn testPopNotLast() -> R {
    let mut stack = OutputFormatterStyleStack::default();
    let s1 = style(Some("white"), Some("black"), &[]);
    let s2 = style(Some("yellow"), Some("blue"), &[]);
    stack.push(s1.clone());
    stack.push(s2.clone());
    stack.push(style(Some("green"), Some("red"), &[]));
    assert_eq!(s2, stack.pop(Some(&s2))?);
    assert_eq!(s1, stack.pop(None)?);
    Ok(())
}

#[test]
fn testInvalidPop() {
    let mut stack = OutputFormatterStyleStack::default();
    stack.push(style(Some("white"), Some("black"), &[]));
    assert!(
        stack
            .pop(Some(&style(Some("yellow"), Some("blue"), &[])))
            .is_err_and(|e| e.is_invalid_argument())
    );
}
