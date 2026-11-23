//! TableTest, TableStyleTest and TableCellStyleTest, with upstream's
//! names and assertions. The data providers are replayed from
//! tests/data/table_test.json, recorded on the oracle
//! (`scripts/record.sh console`); TableCell, TableSeparator and
//! TableCellStyle objects arrive with their options.
//!
//! Upstream: symfony/console Tests/Helper/TableTest.php Tests/Helper/TableStyleTest.php Tests/Helper/TableCellStyleTest.php
#![allow(non_snake_case)]

mod common;

use common::{rows, set_headers, text};
use php_value::{PhpArray, Value, vectors::decode};
use symfony_console::{
    ConsoleError,
    helper::{
        table::{Row, Table},
        table_cell::{Cell, CellOption, TableCell, TableCellStyle},
        table_style::TableStyle,
    },
    output::output::{StreamOutput, VERBOSITY_NORMAL},
};

type R = Result<(), ConsoleError>;

/// `getOutputStream($decorated)`.
fn output_stream(decorated: bool) -> StreamOutput<Vec<u8>> {
    StreamOutput::new(Vec::new(), false, VERBOSITY_NORMAL, Some(decorated), None)
}

/// `getOutputContent($output)`.
fn output_content(output: &mut StreamOutput<Vec<u8>>) -> String {
    String::from_utf8_lossy(output.get_stream()).into_owned()
}

fn colspan(value: &str, n: i64) -> Cell {
    Cell::Table(TableCell::with_options(value, &[("colspan", CellOption::Int(n))]).expect("cell"))
}

fn boxed() -> TableStyle {
    let mut style = TableStyle::new();
    style
        .set_horizontal_border_chars("─", None)
        .set_vertical_border_chars("│", None)
        .set_crossing_chars(
            "┼", "┌", "┬", "┐", "┤", "┘", "┴", "└", "├", None, None, None,
        );
    style
}

fn row(cells: &[&str]) -> Row {
    Row::cells(cells.iter().copied())
}

// Recorded providers

struct Provider {
    args: Vec<Value>,
    name: String,
}

fn provider(test: &str) -> Vec<Provider> {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("data/table_test.json")).expect("provider JSON");
    json[test]["rows"]
        .as_array()
        .expect("provider rows")
        .iter()
        .map(|row| Provider {
            name: row[0].as_str().unwrap_or_default().to_owned(),
            args: row[1]
                .as_array()
                .expect("arguments")
                .iter()
                .map(|arg| decode(arg).expect("typed value"))
                .collect(),
        })
        .collect()
}

fn render_rows(mode: &str) -> R {
    for p in provider("testRender") {
        let (headers, given, style, expected) =
            (&p.args[0], &p.args[1], text(&p.args[2]), text(&p.args[3]));
        let decorated = matches!(p.args.get(4), Some(Value::Bool(true)));
        let mut output = output_stream(decorated);
        {
            let mut table = Table::new(&mut output);
            set_headers(&mut table, headers);
            match mode {
                "set" => {
                    table.set_rows(rows(given));
                }
                "add" => {
                    table.add_rows(rows(given));
                }
                _ => {}
            }
            table.set_style(&style)?;
            if mode == "one" {
                for row in rows(given) {
                    table.add_row(row);
                }
            }
            table.render()?;
        }
        assert_eq!(expected, output_content(&mut output), "row {}", p.name);
    }
    Ok(())
}

#[test]
fn testRender() -> R {
    render_rows("set")
}

#[test]
fn testRenderAddRows() -> R {
    render_rows("add")
}

#[test]
fn testRenderAddRowsOneByOne() -> R {
    render_rows("one")
}

#[test]
fn testRenderMultiByte() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["■■"])
        .set_rows(vec![Row::cells([1234])])
        .set_style("default")?
        .render()?;
    assert_eq!(
        "+------+\n| ■■   |\n+------+\n| 1234 |\n+------+\n",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testTableCellWithNumericIntValue() -> R {
    let mut output = output_stream(false);
    // new TableCell(12345): the int is coerced to "12345"
    Table::new(&mut output)
        .set_rows(vec![Row::cells([TableCell::new(12345.to_string())])])
        .render()?;
    assert_eq!(
        "+-------+\n| 12345 |\n+-------+\n",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testTableCellWithNumericFloatValue() -> R {
    let mut output = output_stream(false);
    // new TableCell(12345.01): the float is coerced to "12345.01"
    let value = php_value::numeric::float_to_string(12345.01);
    Table::new(&mut output)
        .set_rows(vec![Row::cells([TableCell::new(value)])])
        .render()?;
    assert_eq!(
        "+----------+\n| 12345.01 |\n+----------+\n",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testStyle() -> R {
    let mut style = TableStyle::new();
    style
        .set_horizontal_border_chars(".", None)
        .set_vertical_border_chars(".", None)
        .set_default_crossing_char(".");
    Table::set_style_definition("dotfull", style);

    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["Foo"])
        .set_rows(vec![row(&["Bar"])])
        .set_style("dotfull")?
        .render()?;
    assert_eq!(
        ".......\n. Foo .\n.......\n. Bar .\n.......\n",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testRowSeparator() -> R {
    let mut output = output_stream(false);
    {
        let mut table = Table::new(&mut output);
        table.set_headers(["Foo"]).set_rows(vec![
            row(&["Bar1"]),
            Row::separator(),
            row(&["Bar2"]),
            Row::separator(),
            row(&["Bar3"]),
        ]);
        table.render()?;
        let ret: *const Table<'_> = table.add_row(Row::separator());
        assert!(
            std::ptr::eq(ret, &table),
            "fluent interface on addRow() with a single TableSeparator() works"
        );
    }
    assert_eq!(
        "+------+\n| Foo  |\n+------+\n| Bar1 |\n+------+\n| Bar2 |\n+------+\n| Bar3 |\n+------+\n",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testRenderMultiCalls() -> R {
    let mut output = output_stream(false);
    {
        let mut table = Table::new(&mut output);
        table.set_rows(vec![Row::cells([colspan("foo", 2)])]);
        table.render()?;
        table.render()?;
        table.render()?;
    }
    assert_eq!(
        "+--------+\n| foo    |\n+--------+\n+--------+\n| foo    |\n+--------+\n+--------+\n| foo    |\n+--------+\n",
        output_content(&mut output)
    );
    Ok(())
}

fn books_with_price() -> Vec<Row> {
    vec![
        row(&["99921-58-10-7", "Divine Comedy", "Dante Alighieri", "9.95"]),
        row(&[
            "9971-5-0210-0",
            "A Tale of Two Cities",
            "Charles Dickens",
            "139.25",
        ]),
    ]
}

fn pad_left() -> Result<TableStyle, ConsoleError> {
    let mut style = TableStyle::new();
    style.set_pad_type(0)?; // STR_PAD_LEFT
    Ok(style)
}

#[test]
fn testColumnStyle() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["ISBN", "Title", "Author", "Price"])
        .set_rows(books_with_price())
        .set_column_table_style(3, pad_left()?)
        .render()?;
    assert_eq!(
        "+---------------+----------------------+-----------------+--------+
| ISBN          | Title                | Author          |  Price |
+---------------+----------------------+-----------------+--------+
| 99921-58-10-7 | Divine Comedy        | Dante Alighieri |   9.95 |
| 9971-5-0210-0 | A Tale of Two Cities | Charles Dickens | 139.25 |
+---------------+----------------------+-----------------+--------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testThrowsWhenTheCellInAnArray() {
    let mut output = output_stream(false);
    let error = Table::new(&mut output)
        .set_headers(["ISBN", "Title", "Author", "Price"])
        .set_rows(vec![Row::cells([
            Cell::from("99921-58-10-7"),
            Cell::Value(Value::Array(PhpArray::new())),
            Cell::from("Dante Alighieri"),
            Cell::from("9.95"),
        ])])
        .render()
        .expect_err("InvalidArgumentException");
    assert!(matches!(error, ConsoleError::InvalidArgument(_)));
    assert_eq!(
        "A cell must be a TableCell, a scalar or an object implementing \"__toString()\", \"array\" given.",
        error.message()
    );
}

const COLUMN_WIDTH_EXPECTED: &str =
    "+-----------------+----------------------+-----------------+------------+
| ISBN            | Title                | Author          |      Price |
+-----------------+----------------------+-----------------+------------+
| 99921-58-10-7   | Divine Comedy        | Dante Alighieri |       9.95 |
| 9971-5-0210-0   | A Tale of Two Cities | Charles Dickens |     139.25 |
+-----------------+----------------------+-----------------+------------+
";

#[test]
fn testColumnWidth() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["ISBN", "Title", "Author", "Price"])
        .set_rows(books_with_price())
        .set_column_width(0, 15)
        .set_column_width(3, 10)
        .set_column_table_style(3, pad_left()?)
        .render()?;
    assert_eq!(COLUMN_WIDTH_EXPECTED, output_content(&mut output));
    Ok(())
}

#[test]
fn testColumnWidths() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["ISBN", "Title", "Author", "Price"])
        .set_rows(books_with_price())
        .set_column_widths(&[(0, 15), (1, 0), (2, -1), (3, 10)])
        .set_column_table_style(3, pad_left()?)
        .render()?;
    assert_eq!(COLUMN_WIDTH_EXPECTED, output_content(&mut output));
    Ok(())
}

#[test]
fn testColumnWidthIsKeptWhenARowSpansColumns() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["ISBN", "Author"])
        .set_rows(vec![
            row(&["9971-5-0210-0", "Dante Alighieri"]),
            Row::cells([colspan("span", 2)]),
        ])
        .set_column_width(0, 20)
        .render()?;
    assert_eq!(
        "+----------------------+-----------------+
| ISBN                 | Author          |
+----------------------+-----------------+
| 9971-5-0210-0        | Dante Alighieri |
| span                                   |
+----------------------------------------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testColumnMinWidthDoesNotInflateSiblingColumns() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["A", "B"])
        .set_rows(vec![
            row(&["x", "y"]),
            Row::cells([colspan("short span", 2)]),
        ])
        .set_column_width(0, 20)
        .render()?;
    assert_eq!(
        "+----------------------+-------+
| A                    | B     |
+----------------------+-------+
| x                    | y     |
| short span                   |
+------------------------------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testColumnWidthComputedForASpanningRowIsNotReusedByTheNextOne() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_headers(["H0", "H1"])
        .set_rows(vec![
            row(&["value0", "value1"]),
            Row::cells([colspan("zzzzz", 2)]),
            Row::cells([colspan("qq", 2)]),
        ])
        .set_column_max_width(1, 6)
        .render()?;
    assert_eq!(
        "+--------+--------+
| H0     | H1     |
+--------+--------+
| value0 | value1 |
| zzzzz           |
| qq              |
+-----------------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testAppendRowWithoutSectionOutput() {
    let mut output = output_stream(false);
    let error = Table::new(&mut output)
        .append_row(row(&[
            "9971-5-0210-0",
            "A Tale of Two Cities",
            "Charles Dickens",
            "139.25",
        ]))
        .map(|_| ())
        .expect_err("RuntimeException");
    assert!(matches!(error, ConsoleError::Runtime(_)));
    assert_eq!(
        "Output should be an instance of \"Symfony\\Component\\Console\\Output\\ConsoleSectionOutput\" when calling \"Symfony\\Component\\Console\\Helper\\Table::appendRow\".",
        error.message()
    );
}

#[test]
fn testIsNotDefinedStyleException() {
    let mut output = output_stream(false);
    let error = Table::new(&mut output)
        .set_style("absent")
        .map(|_| ())
        .expect_err("InvalidArgumentException");
    assert!(matches!(error, ConsoleError::InvalidArgument(_)));
    assert_eq!("Style \"absent\" is not defined.", error.message());
}

#[test]
fn testGetStyleDefinition() {
    let error = Table::get_style_definition("absent").expect_err("InvalidArgumentException");
    assert!(matches!(error, ConsoleError::InvalidArgument(_)));
    assert_eq!("Style \"absent\" is not defined.", error.message());
}

#[test]
fn testSetTitle() -> R {
    for p in provider("testSetTitle") {
        let header_title = (!p.args[0].is_null()).then(|| text(&p.args[0]));
        let footer_title = (!p.args[1].is_null()).then(|| text(&p.args[1]));
        let mut output = output_stream(false);
        Table::new(&mut output)
            .set_header_title(header_title.as_deref())
            .set_footer_title(footer_title.as_deref())
            .set_headers(["ISBN", "Title", "Author"])
            .set_rows(vec![
                row(&["99921-58-10-7", "Divine Comedy", "Dante Alighieri"]),
                row(&["9971-5-0210-0", "A Tale of Two Cities", "Charles Dickens"]),
                row(&["960-425-059-0", "The Lord of the Rings", "J. R. R. Tolkien"]),
                row(&[
                    "80-902734-1-6",
                    "And Then There Were None",
                    "Agatha Christie",
                ]),
            ])
            .set_style(&text(&p.args[2]))?
            .render()?;
        assert_eq!(
            text(&p.args[3]),
            output_content(&mut output),
            "row {}",
            p.name
        );
    }
    Ok(())
}

#[test]
fn testSetTitleWithoutHeaders() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_header_title(Some("Reproducer"))
        .set_rows(vec![
            row(&["Value", "123-456"]),
            row(&["Some other value", "789-0"]),
        ])
        .render()?;
    assert_eq!(
        "+-------- Reproducer --------+
| Value            | 123-456 |
| Some other value | 789-0   |
+------------------+---------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testColumnMaxWidths() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_rows(vec![row(&[
            "Divine Comedy",
            "A Tale of Two Cities",
            "The Lord of the Rings",
            "And Then There Were None",
        ])])
        .set_column_max_width(1, 5)
        .set_column_max_width(2, 10)
        .set_column_max_width(3, 15)
        .render()?;
    assert_eq!(
        "+---------------+-------+----------+----------------+
| Divine Comedy | A     | The Lord | And Then There |
|               | Tale  | of the   | Were None      |
|               | of    | Rings    |                |
|               | Two   |          |                |
|               | Citie |          |                |
|               | s     |          |                |
+---------------+-------+----------+----------------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testColumnMaxWidthsHeaders() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_header_rows(vec![row(&[
            "Publication",
            "Very long header with a lot of information",
        ])])
        .set_rows(vec![row(&[
            "1954",
            "The Lord of the Rings, by J.R.R. Tolkien",
        ])])
        .set_column_max_width(1, 30)
        .render()?;
    assert_eq!(
        "+-------------+--------------------------------+
| Publication | Very long header with a lot of |
|             | information                    |
+-------------+--------------------------------+
| 1954        | The Lord of the Rings, by      |
|             | J.R.R. Tolkien                 |
+-------------+--------------------------------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testColumnMaxWidthsWithTrailingBackslash() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_column_max_width(0, 5)
        .set_rows(vec![row(&["1234\\6"])])
        .render()?;
    assert_eq!(
        "+-------+\n| 1234\\ |\n| 6     |\n+-------+\n",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testBoxedStyleWithColspan() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_table_style(boxed())
        .set_headers(["ISBN", "Title", "Author"])
        .set_rows(vec![
            row(&["99921-58-10-7", "Divine Comedy", "Dante Alighieri"]),
            Row::separator(),
            Row::cells([colspan("This value spans 3 columns.", 3)]),
        ])
        .render()?;
    assert_eq!(
        "┌───────────────┬───────────────┬─────────────────┐
│ ISBN          │ Title         │ Author          │
├───────────────┼───────────────┼─────────────────┤
│ 99921-58-10-7 │ Divine Comedy │ Dante Alighieri │
├───────────────┴───────────────┴─────────────────┤
│ This value spans 3 columns.                     │
└─────────────────────────────────────────────────┘
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testRenderHorizontal() -> R {
    for p in provider("testRenderHorizontal") {
        let mut output = output_stream(false);
        {
            let mut table = Table::new(&mut output);
            set_headers(&mut table, &p.args[0]);
            table.set_rows(rows(&p.args[1])).set_horizontal(true);
            table.render()?;
        }
        assert_eq!(
            text(&p.args[2]),
            output_content(&mut output),
            "row {}",
            p.name
        );
    }
    Ok(())
}

#[test]
fn testWithColspanAndMaxWith() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_column_max_width(0, 15)
        .set_column_max_width(1, 15)
        .set_column_max_width(2, 15)
        .set_rows(vec![
            Row::cells([colspan("Lorem ipsum dolor sit amet, <fg=white;bg=green>consectetur</> adipiscing elit, <fg=white;bg=red>sed</> do <fg=white;bg=red>eiusmod</> tempor", 3)]),
            Row::separator(),
            Row::cells([colspan("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor", 3)]),
            Row::separator(),
            Row::cells([colspan("Lorem ipsum <fg=white;bg=red>dolor</> sit amet, consectetur ", 2), Cell::from("hello world")]),
            Row::separator(),
            Row::cells([Cell::from("hello <fg=white;bg=green>world</>"), colspan("Lorem ipsum dolor sit amet, <fg=white;bg=green>consectetur</> adipiscing elit", 2)]),
            Row::separator(),
            Row::cells([Cell::from("hello "), colspan("world", 1), Cell::from("Lorem ipsum dolor sit amet, consectetur")]),
            Row::separator(),
            Row::cells([Cell::from("Symfony "), colspan("Test", 1), Cell::from("Lorem <fg=white;bg=green>ipsum</> dolor sit amet, consectetur")]),
        ])
        .render()?;
    assert_eq!(
        "+-----------------------------------------------------+
| Lorem ipsum dolor sit amet, consectetur adipiscing  |
| elit, sed do eiusmod tempor                         |
+-----------------------------------------------------+
| Lorem ipsum dolor sit amet, consectetur adipiscing  |
| elit, sed do eiusmod tempor                         |
+-----------------------------------+-----------------+
| Lorem ipsum dolor sit amet, conse | hello world     |
| ctetur                            |                 |
+-----------------+-----------------+-----------------+
| hello world     | Lorem ipsum dolor sit amet, conse |
|                 | ctetur adipiscing elit            |
+-----------------+-----------------+-----------------+
| hello           | world           | Lorem ipsum     |
|                 |                 | dolor sit amet, |
|                 |                 | consectetur     |
+-----------------+-----------------+-----------------+
| Symfony         | Test            | Lorem ipsum dol |
|                 |                 | or sit amet,    |
|                 |                 | consectetur     |
+-----------------+-----------------+-----------------+
",
        output_content(&mut output)
    );
    Ok(())
}

#[test]
fn testVerticalRender() -> R {
    for p in provider("testVerticalRender") {
        let style = p.args.get(3).map_or_else(|| "default".to_owned(), text);
        let header_title = p.args.get(4).map(text).unwrap_or_default();
        let footer_title = p.args.get(5).map(text).unwrap_or_default();
        let mut output = output_stream(false);
        {
            let mut table = Table::new(&mut output);
            set_headers(&mut table, &p.args[1]);
            table.set_rows(rows(&p.args[2])).set_vertical(true);
            table.set_style(&style)?;
            if !header_title.is_empty() {
                table.set_header_title(Some(&header_title));
            }
            if !footer_title.is_empty() {
                table.set_footer_title(Some(&footer_title));
            }
            table.render()?;
        }
        assert_eq!(
            text(&p.args[0]),
            output_content(&mut output),
            "row {}",
            p.name
        );
    }
    Ok(())
}

#[test]
fn testWithHyperlinkAndMaxWidth() -> R {
    let mut output = output_stream(true);
    Table::new(&mut output)
        .set_rows(vec![row(&["<href=Lorem>Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor</>"])])
        .set_column_max_width(0, 20)
        .render()?;
    assert_eq!(
        "+----------------------+
| \x1b]8;;Lorem\x1b\\Lorem ipsum dolor\x1b]8;;\x1b\\    |
| \x1b]8;;Lorem\x1b\\sit amet,\x1b]8;;\x1b\\            |
| \x1b]8;;Lorem\x1b\\consectetur\x1b]8;;\x1b\\          |
| \x1b]8;;Lorem\x1b\\adipiscing elit, sed\x1b]8;;\x1b\\ |
| \x1b]8;;Lorem\x1b\\do eiusmod tempor\x1b]8;;\x1b\\    |
+----------------------+
",
        output_content(&mut output)
    );
    Ok(())
}

fn hello_world(horizontal: bool) -> Result<String, ConsoleError> {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_table_style(boxed())
        .set_header_title(Some("Title"))
        .set_headers(["Hello", "World"])
        .set_rows(vec![Row::cells([1, 2]), Row::cells([3, 4])])
        .set_horizontal(horizontal)
        .render()?;
    Ok(output_content(&mut output))
}

#[test]
fn testGithubIssue52101HorizontalTrue() -> R {
    assert_eq!(
        "┌──── Title ┬───┐\n│ Hello │ 1 │ 3 │\n│ World │ 2 │ 4 │\n└───────┴───┴───┘\n",
        hello_world(true)?
    );
    Ok(())
}

#[test]
fn testGithubIssue52101HorizontalFalse() -> R {
    assert_eq!(
        "┌──── Title ────┐\n│ Hello │ World │\n├───────┼───────┤\n│ 1     │ 2     │\n│ 3     │ 4     │\n└───────┴───────┘\n",
        hello_world(false)?
    );
    Ok(())
}

#[test]
fn testGithubIssue60038WidthOfCellWithEmoji() -> R {
    let mut output = output_stream(false);
    Table::new(&mut output)
        .set_header_title(Some("Test Title"))
        .set_headers(["Title", "Author"])
        .set_rows(vec![
            row(&["🎭 💫 ☯ Divine Comedy", "Dante Alighieri"]),
            // the snowflake in text style (e2 9d 84 ef b8 8e) has a variant selector
            row(&["❄︎❄︎❄︎ snowflake in text style ❄︎❄︎❄︎", ""]),
            row(&[
                "And a very long line to show difference in previous lines",
                "",
            ]),
        ])
        .render()?;
    assert_eq!(
        "+-------------------------------- Test Title ---------------+-----------------+
| Title                                                     | Author          |
+-----------------------------------------------------------+-----------------+
| 🎭 💫 ☯ Divine Comedy                                     | Dante Alighieri |
| ❄︎❄︎❄︎ snowflake in text style ❄︎❄︎❄︎                           |                 |
| And a very long line to show difference in previous lines |                 |
+-----------------------------------------------------------+-----------------+
",
        output_content(&mut output)
    );
    Ok(())
}

// TableStyleTest

#[test]
fn testSetPadTypeWithInvalidType() {
    let error = TableStyle::new()
        .set_pad_type(31)
        .map(|_| ())
        .expect_err("InvalidArgumentException");
    assert!(error.is_invalid_argument());
    assert_eq!(
        "Invalid padding type. Expected one of (STR_PAD_LEFT, STR_PAD_RIGHT, STR_PAD_BOTH).",
        error.message()
    );
}

// TableCellStyleTest

#[test]
fn testCreateTableCellStyle() -> R {
    let table_cell_style = TableCellStyle::new(&[("fg", Some("red"))])?;
    assert_eq!(
        Some("red"),
        table_cell_style
            .get_options()
            .iter()
            .find(|(k, _)| *k == "fg")
            .and_then(|(_, v)| *v)
    );
    let error = TableCellStyle::new(&[("wrong_key", None)]).expect_err("InvalidArgumentException");
    assert!(matches!(error, ConsoleError::InvalidArgument(_)));
    Ok(())
}
