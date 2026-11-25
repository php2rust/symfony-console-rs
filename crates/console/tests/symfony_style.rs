//! `SymfonyStyleTest`, with upstream's names and assertions. The command
//! fixtures (Tests/Fixtures/Style/SymfonyStyle/command/command_*.php) are
//! closures written from them; their expected outputs are upstream's files,
//! copied to tests/Fixtures. `command_7` asks questions (`QuestionHelper`, M7),
//! so `testOutputs` stays pending and `outputs_without_questions` runs the
//! other 27 fixtures.
//!
//! Upstream: symfony/console Tests/Style/SymfonyStyleTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::PhpArray;
use symfony_console::{
    ConsoleError,
    command::command::{Code, Command, SUCCESS},
    env::putenv,
    helper::{
        table::Row,
        table_cell::{Cell, CellOption, TableCell},
    },
    input::input::Input,
    output::output::{OutputInterface, StreamOutput, VERBOSITY_NORMAL},
    style::symfony_style::{DefinitionItem, SymfonyStyle},
    tester::command_tester::{CommandTester, ExecuteOptions},
};

type R = Result<(), ConsoleError>;
type Fixture = fn(&mut Input, &mut dyn OutputInterface) -> Result<i64, ConsoleError>;

const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum";

/// `setUp()`: COLUMNS=121, a `sfstyle` command and its tester.
struct Columns;

impl Columns {
    fn set() -> Self {
        putenv("COLUMNS", Some("121"));
        Self
    }
}

impl Drop for Columns {
    fn drop(&mut self) {
        putenv("COLUMNS", None);
    }
}

fn run(code: Fixture, interactive: bool) -> Result<String, ConsoleError> {
    let mut command = Command::new(Some("sfstyle"), None)?;
    command.set_code(Code::closure(code));
    let mut tester = CommandTester::new(command);
    tester.execute(
        PhpArray::new(),
        ExecuteOptions {
            interactive: Some(interactive),
            decorated: Some(false),
            verbosity: None,
        },
    )?;
    tester.get_display(true)
}

fn colspan(value: &str, n: i64) -> Cell {
    Cell::Table(TableCell::with_options(value, &[("colspan", CellOption::Int(n))]).expect("cell"))
}

fn command_0(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.caution(&["Lorem ipsum dolor sit amet"])?;
    Ok(0)
}

fn command_1(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.title("Title")?;
    output.warning(&["Lorem ipsum dolor sit amet"])?;
    output.title("Title")?;
    Ok(0)
}

fn command_2(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.warning(&["Warning"])?;
    output.caution(&["Caution"])?;
    output.error(&["Error"])?;
    output.success(&["Success"])?;
    output.note(&["Note"])?;
    output.info(&["Info"])?;
    output.block(
        &["Custom block"],
        Some("CUSTOM"),
        Some("fg=white;bg=green"),
        "X ",
        true,
        true,
    )?;
    Ok(0)
}

fn command_3(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.title("First title")?;
    output.title("Second title")?;
    Ok(0)
}

/// `command_4.php`, and `command_4_with_iterators.php` (an `ArrayIterator` of
/// messages is the same list here).
fn command_4(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.write("Lorem ipsum dolor sit amet")?;
    output.title("First title")?;
    output.writeln("Lorem ipsum dolor sit amet")?;
    output.title("Second title")?;
    output.write("Lorem ipsum dolor sit amet")?;
    output.write("")?;
    output.title("Third title")?;
    //Ensure edge case by appending empty strings to history:
    output.write("Lorem ipsum dolor sit amet")?;
    output.write_all(&["", "", ""], false, 1)?;
    output.title("Fourth title")?;
    //Ensure have manual control over number of blank lines:
    output.writeln("Lorem ipsum dolor sit amet")?;
    output.writeln_all(&["", ""], 1)?; //Should append an extra blank line
    output.title("Fifth title")?;
    output.writeln("Lorem ipsum dolor sit amet")?;
    output.new_line(2)?; //Should append an extra blank line
    output.title("Fifth title")?;
    Ok(0)
}

fn command_5(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    let two = ["Lorem ipsum dolor sit amet", "consectetur adipiscing elit"];
    output.writeln("Lorem ipsum dolor sit amet")?;
    output.listing(&two)?;
    //Even using write:
    output.write("Lorem ipsum dolor sit amet")?;
    output.listing(&two)?;
    output.write("Lorem ipsum dolor sit amet")?;
    output.text(&two)?;
    output.new_line(1)?;
    output.write("Lorem ipsum dolor sit amet")?;
    output.comment(&two)?;
    Ok(0)
}

fn command_6(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.listing(&["Lorem ipsum dolor sit amet", "consectetur adipiscing elit"])?;
    output.success(&["Lorem ipsum dolor sit amet"])?;
    Ok(0)
}

fn command_8(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    let headers = vec![
        Row::cells([colspan("Main table title", 3)]),
        Row::cells(["ISBN", "Title", "Author"]),
    ];
    let rows = vec![
        Row::cells([
            Cell::from("978-0521567817"),
            Cell::from("De Monarchia"),
            Cell::Table(TableCell::with_options(
                "Dante Alighieri\nspans multiple rows",
                &[("rowspan", CellOption::Int(2))],
            )?),
        ]),
        Row::cells(["978-0804169127", "Divine Comedy"]),
    ];
    output.table_with_header_rows(headers, rows)?;
    Ok(0)
}

fn command_9(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.block(
        &["Custom block", "Second custom block line"],
        Some("CUSTOM"),
        Some("fg=white;bg=green"),
        "X ",
        true,
        true,
    )?;
    Ok(0)
}

fn command_10(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.block(
        &[LOREM],
        Some("CUSTOM"),
        Some("fg=white;bg=green"),
        "X ",
        true,
        true,
    )?;
    Ok(0)
}

fn command_11(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let word = "Lopadotemachoselachogaleokranioleipsanodrimhypotrimmatosilphioparaomelitokatakechymenokichlepikossyphophattoperisteralektryonoptekephalliokigklopeleiolagoiosiraiobaphetraganopterygon";
    let mut sf_style = SymfonyStyle::new(input, output)?;
    sf_style.block(
        &[word],
        Some("CUSTOM"),
        Some("fg=white;bg=blue"),
        " § ",
        false,
        true,
    )?;
    Ok(0)
}

fn command_12(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.comment(&[LOREM])?;
    Ok(0)
}

fn command_13(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.set_decorated(true);
    output.comment(&["Árvíztűrőtükörfúrógép 🎼 Lorem ipsum dolor sit <comment>💕 amet, consectetur adipisicing elit, sed do eiusmod tempor incididu labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.</comment> Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum"])?;
    Ok(0)
}

fn command_14(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.block(&[LOREM], None, None, "$ ", true, true)?;
    Ok(0)
}

fn command_15(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.block(&[LOREM], Some("TEST"), None, " ", false, true)?;
    Ok(0)
}

fn command_16(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.set_decorated(true);
    // success($message, 'TEST'): the extra argument is ignored
    output.success(&[LOREM])?;
    Ok(0)
}

fn command_17(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.title("Title ending with \\")?;
    output.section("Section ending with \\")?;
    Ok(0)
}

fn command_18(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.definition_list(&[
        DefinitionItem::Pair(Cell::from("foo"), Cell::from("bar")),
        DefinitionItem::Separator,
        DefinitionItem::Title("this is a title".to_owned()),
        DefinitionItem::Separator,
        DefinitionItem::Pair(Cell::from("foo2"), Cell::from("bar2")),
    ])?;
    Ok(0)
}

fn command_19(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.horizontal_table(
        ["a", "b", "c", "d"].map(Cell::from).to_vec(),
        vec![
            Row::cells([1, 2, 3]),
            Row::cells([4, 5]),
            Row::cells([7, 8, 9]),
        ],
    )?;
    Ok(0)
}

fn command_20(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.set_decorated(true);
    output.write("<question>do you want <comment>something</>")?;
    output.writeln("?</>")?;
    Ok(0)
}

fn command_21(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.success(&["Lorem ipsum dolor sit amet"])?;
    output.success(&["Lorem ipsum dolor sit amet with one emoji 🎉"])?;
    output.success(&["Lorem ipsum dolor sit amet with so many of them 🌾🌾🌾🌾🌾"])?;
    Ok(0)
}

fn command_22(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.set_decorated(true);
    output.block(
        &["Árvíztűrőtükörfúrógép Lorem ipsum dolor sit <comment>amet, consectetur adipisicing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.</comment> Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum"],
        Some("★"), // UTF-8 star!
        None,
        "<fg=default;bg=default> ║ </>", // UTF-8 double line!
        false,
        false,
    )?;
    Ok(0)
}

fn command_23(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.text(&["Hello"])?;
    Ok(0)
}

fn command_24(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    let message = ["Lorem ipsum dolor sit amet"];
    output.outline_success(&message)?;
    output.outline_error(&message)?;
    output.outline_warning(&message)?;
    output.outline_note(&message)?;
    output.outline_info(&message)?;
    output.outline_caution(&message)?;
    Ok(0)
}

fn command_25(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.outline_block(
        &["Deployment finished in 3.2s"],
        Some("Deploy"),
        Some("fg=cyan"),
        true,
        true,
    )?;
    output.outline_block(
        &["No title, just a plain bordered box."],
        None,
        None,
        true,
        true,
    )?;
    Ok(0)
}

fn command_26(input: &mut Input, output: &mut dyn OutputInterface) -> Result<i64, ConsoleError> {
    let mut output = SymfonyStyle::new(input, output)?;
    output.outline_block(
        &["First message.", "Second message.", "Third message."],
        Some("Build"),
        Some("fg=blue"),
        true,
        true,
    )?;
    output.outline_success(&["This is a very long message that should wrap across multiple lines because it exceeds the maximum line length of the terminal."])?;
    Ok(0)
}

/// `inputCommandToOutputFilesProvider()` without `command_7`.
fn fixtures() -> Vec<(&'static str, Fixture, &'static str)> {
    macro_rules! out {
        ($n:literal) => {
            include_str!(concat!(
                "Fixtures/Style/SymfonyStyle/output/output_",
                $n,
                ".txt"
            ))
        };
    }
    vec![
        ("command_0", command_0, out!("0")),
        ("command_1", command_1, out!("1")),
        ("command_10", command_10, out!("10")),
        ("command_11", command_11, out!("11")),
        ("command_12", command_12, out!("12")),
        ("command_13", command_13, out!("13")),
        ("command_14", command_14, out!("14")),
        ("command_15", command_15, out!("15")),
        ("command_16", command_16, out!("16")),
        ("command_17", command_17, out!("17")),
        ("command_18", command_18, out!("18")),
        ("command_19", command_19, out!("19")),
        ("command_2", command_2, out!("2")),
        ("command_20", command_20, out!("20")),
        ("command_21", command_21, out!("21")),
        ("command_22", command_22, out!("22")),
        ("command_23", command_23, out!("23")),
        ("command_24", command_24, out!("24")),
        ("command_25", command_25, out!("25")),
        ("command_26", command_26, out!("26")),
        ("command_3", command_3, out!("3")),
        ("command_4", command_4, out!("4")),
        (
            "command_4_with_iterators",
            command_4,
            out!("4_with_iterators"),
        ),
        ("command_5", command_5, out!("5")),
        ("command_6", command_6, out!("6")),
        ("command_8", command_8, out!("8")),
        ("command_9", command_9, out!("9")),
    ]
}

/// `testOutputs` on every fixture but `command_7` (questions, M7).
#[test]
fn outputs_without_questions() -> R {
    let _columns = Columns::set();
    for (name, code, expected) in fixtures() {
        assert_eq!(expected, run(code, false)?, "{name}");
    }
    Ok(())
}

#[test]
fn testBlockWithWindowsLineEndings() -> R {
    let _columns = Columns::set();
    let display = run(
        |input, output| {
            let mut io = SymfonyStyle::new(input, output)?;
            io.block(
                &["First line.\r\nSecond line."],
                Some("INFO"),
                Some("fg=white;bg=blue"),
                " ",
                true,
                true,
            )?;
            Ok(SUCCESS)
        },
        false,
    )?;
    assert!(!display.contains('\r'));
    assert!(display.contains("First line."));
    assert!(display.contains("Second line."));
    Ok(())
}

fn styled_block(decorated: bool, style: Option<&str>) -> Result<String, ConsoleError> {
    let mut output = StreamOutput::new(Vec::new(), false, VERBOSITY_NORMAL, Some(decorated), None);
    let input = Input::array(PhpArray::new(), None)?;
    {
        let mut io = SymfonyStyle::new(&input, &mut output)?;
        io.block(&["msg"], None, style, " ", false, true)?;
    }
    Ok(String::from_utf8_lossy(output.get_stream()).into_owned())
}

#[test]
fn testStyledBlockUsesEchPaddingWhenDecorated() -> R {
    let display = styled_block(true, Some("fg=white;bg=blue"))?;
    // /\e\[\d+X\e\[\d+C/
    let ech = display.split("\x1b[").any(|part| {
        let digits = part.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && part[digits..].starts_with('X')
    });
    assert!(ech && display.contains("X\x1b["), "{display:?}");
    Ok(())
}

#[test]
fn testStyledBlockKeepsSpacePaddingWhenNotDecorated() -> R {
    assert!(!styled_block(false, Some("fg=white;bg=blue"))?.contains("\x1b["));
    Ok(())
}

#[test]
fn testUnstyledBlockKeepsSpacePaddingEvenWhenDecorated() -> R {
    assert!(!styled_block(true, None)?.contains("\x1b["));
    Ok(())
}

#[test]
fn testCreateTableWithoutConsoleOutput() -> R {
    let mut output = StreamOutput::new(Vec::new(), false, VERBOSITY_NORMAL, Some(false), None);
    let input = Input::array(PhpArray::new(), None)?;
    let mut style = SymfonyStyle::new(&input, &mut output)?;
    let error = style
        .create_table()?
        .append_row(Row::cells(["row"]))
        .map(|_| ())
        .expect_err("RuntimeException");
    assert!(matches!(error, ConsoleError::Runtime(_)));
    assert!(error.message().contains(
        "Output should be an instance of \"Symfony\\Component\\Console\\Output\\ConsoleSectionOutput\""
    ));
    Ok(())
}

#[test]
fn testGetErrorStyleUsesTheCurrentOutputIfNoErrorOutputIsAvailable() -> R {
    let mut output = StreamOutput::new(Vec::new(), false, VERBOSITY_NORMAL, Some(false), None);
    let input = Input::array(PhpArray::new(), None)?;
    let style = SymfonyStyle::new(&input, &mut output)?;
    let mut error_style = style.get_error_style()?;
    error_style.write("x")?;
    drop(error_style);
    assert_eq!(b"x", output.get_stream().as_slice());
    Ok(())
}
