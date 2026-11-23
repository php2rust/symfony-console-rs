//! Port of Symfony\Component\Console\Style\SymfonyStyle.php, with
//! OutputStyle.php (the output it decorates) and StyleInterface.php.
//!
//! A SymfonyStyle borrows its command's input and output. Like upstream
//! it keeps the last two bytes it wrote in a TrimmedBufferOutput, to know
//! how many blank lines to put before a block or text. Tables are written
//! to the decorated output directly, as upstream's are.
//!
//! Not ported (M7): the questions (`ask()`, `askHidden()`, `confirm()`,
//! `choice()`, `askFile()`, `askQuestion()`), progress bars, trees, and
//! sections of a ConsoleOutput. `getErrorStyle()` returns a style on the
//! same output, since no output here has a separate error output.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Style/SymfonyStyle.php Style/OutputStyle.php Style/StyleInterface.php

use crate::{
    exception::Result,
    formatter::output_formatter::OutputFormatter,
    helper::{
        helper::Helper,
        output_wrapper::OutputWrapper,
        table::{Row, Table},
        table_cell::{Cell, CellOption, TableCell},
    },
    input::input::Input,
    output::{
        output::{OUTPUT_NORMAL, OutputInterface},
        trimmed_buffer_output::TrimmedBufferOutput,
    },
    php::sprintf_s,
    terminal::Terminal,
};

pub const MAX_LINE_LENGTH: i64 = 120;

/// One entry of `definitionList()`.
#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionItem {
    /// A title spanning both columns (`'A title'`).
    Title(String),
    /// `['key' => 'value']`.
    Pair(Cell, Cell),
    /// `new TableSeparator()`.
    Separator,
}

/// `SymfonyStyle`.
pub struct SymfonyStyle<'a> {
    input: &'a Input,
    output: &'a mut dyn OutputInterface,
    line_length: i64,
    buffered_output: TrimmedBufferOutput,
}

/// `$style` truthiness: "" and "0" are false.
fn truthy(s: Option<&str>) -> bool {
    s.is_some_and(|s| !s.is_empty() && s != "0")
}

impl<'a> SymfonyStyle<'a> {
    /// `new SymfonyStyle($input, $output)`.
    pub fn new(input: &'a Input, output: &'a mut dyn OutputInterface) -> Result<Self> {
        // DIRECTORY_SEPARATOR is '/', as on Linux: two bytes are kept
        let formatter = output.get_formatter().clone();
        let buffered_output =
            TrimmedBufferOutput::new(2, Some(output.get_verbosity()), false, Some(formatter))?;
        let width = match Terminal.get_width() {
            0 => MAX_LINE_LENGTH,
            width => width,
        };
        Ok(Self {
            input,
            output,
            line_length: width.min(MAX_LINE_LENGTH),
            buffered_output,
        })
    }

    /// The input the style was made with.
    pub fn input(&self) -> &Input {
        self.input
    }

    /// `block($messages, $type, $style, $prefix, $padding, $escape)`.
    pub fn block(
        &mut self,
        messages: &[&str],
        kind: Option<&str>,
        style: Option<&str>,
        prefix: &str,
        padding: bool,
        escape: bool,
    ) -> Result<()> {
        self.auto_prepend_block()?;
        let lines = self.create_block(messages, kind, style, prefix, padding, escape)?;
        self.writeln_all(&lines, OUTPUT_NORMAL)?;
        self.new_line(1)
    }

    /// `title($message)`.
    pub fn title(&mut self, message: &str) -> Result<()> {
        self.heading(message, "=")
    }

    /// `section($message)`.
    pub fn section(&mut self, message: &str) -> Result<()> {
        self.heading(message, "-")
    }

    fn heading(&mut self, message: &str, underline: &str) -> Result<()> {
        self.auto_prepend_block()?;
        let width = Helper::width(&Helper::remove_decoration(
            self.output.get_formatter(),
            message,
        )?);
        self.writeln_all(
            &[
                format!(
                    "<comment>{}</>",
                    OutputFormatter::escape_trailing_backslash(message)
                ),
                format!("<comment>{}</>", underline.repeat(width)),
            ],
            OUTPUT_NORMAL,
        )?;
        self.new_line(1)
    }

    /// `listing($elements)`.
    pub fn listing(&mut self, elements: &[&str]) -> Result<()> {
        self.auto_prepend_text()?;
        let elements: Vec<String> = elements.iter().map(|e| format!(" * {e}")).collect();
        self.writeln_all(&elements, OUTPUT_NORMAL)?;
        self.new_line(1)
    }

    /// `text($message)`.
    pub fn text(&mut self, messages: &[&str]) -> Result<()> {
        self.auto_prepend_text()?;
        for message in messages {
            self.writeln_all(&[format!(" {message}")], OUTPUT_NORMAL)?;
        }
        Ok(())
    }

    /// `comment($message)`.
    pub fn comment(&mut self, messages: &[&str]) -> Result<()> {
        self.block(
            messages,
            None,
            None,
            "<fg=default;bg=default> // </>",
            false,
            false,
        )
    }

    /// `success($message)`.
    pub fn success(&mut self, messages: &[&str]) -> Result<()> {
        self.block(
            messages,
            Some("OK"),
            Some("fg=black;bg=green"),
            " ",
            true,
            true,
        )
    }

    /// `error($message)`.
    pub fn error(&mut self, messages: &[&str]) -> Result<()> {
        self.block(
            messages,
            Some("ERROR"),
            Some("fg=white;bg=red"),
            " ",
            true,
            true,
        )
    }

    /// `warning($message)`.
    pub fn warning(&mut self, messages: &[&str]) -> Result<()> {
        self.block(
            messages,
            Some("WARNING"),
            Some("fg=black;bg=yellow"),
            " ",
            true,
            true,
        )
    }

    /// `note($message)`.
    pub fn note(&mut self, messages: &[&str]) -> Result<()> {
        self.block(
            messages,
            Some("NOTE"),
            Some("fg=yellow"),
            " ! ",
            false,
            true,
        )
    }

    /// `info($message)`.
    pub fn info(&mut self, messages: &[&str]) -> Result<()> {
        self.block(messages, Some("INFO"), Some("fg=green"), " ", true, true)
    }

    /// `caution($message)`.
    pub fn caution(&mut self, messages: &[&str]) -> Result<()> {
        self.block(
            messages,
            Some("CAUTION"),
            Some("fg=white;bg=red"),
            " ! ",
            true,
            true,
        )
    }

    /// `outlineBlock($messages, $type, $style, $padding, $escape)`.
    pub fn outline_block(
        &mut self,
        messages: &[&str],
        kind: Option<&str>,
        style: Option<&str>,
        padding: bool,
        escape: bool,
    ) -> Result<()> {
        self.auto_prepend_block()?;
        let lines = self.create_outline_block(messages, kind, style, padding, escape)?;
        self.writeln_all(&lines, OUTPUT_NORMAL)?;
        self.new_line(1)
    }

    pub fn outline_success(&mut self, messages: &[&str]) -> Result<()> {
        self.outline_block(messages, Some("✅ Success"), Some("fg=green"), true, true)
    }

    pub fn outline_error(&mut self, messages: &[&str]) -> Result<()> {
        self.outline_block(messages, Some("❌ Error"), Some("fg=red"), true, true)
    }

    pub fn outline_warning(&mut self, messages: &[&str]) -> Result<()> {
        self.outline_block(messages, Some("⚠️ Warning"), Some("fg=yellow"), true, true)
    }

    pub fn outline_note(&mut self, messages: &[&str]) -> Result<()> {
        self.outline_block(messages, Some("📝 Note"), Some("fg=blue"), true, true)
    }

    pub fn outline_info(&mut self, messages: &[&str]) -> Result<()> {
        self.outline_block(messages, Some("ℹ️ Info"), Some("fg=green"), true, true)
    }

    pub fn outline_caution(&mut self, messages: &[&str]) -> Result<()> {
        self.outline_block(messages, Some("🚨 Caution"), Some("fg=red"), true, true)
    }

    /// `table($headers, $rows)` with one row of headers.
    pub fn table(&mut self, headers: Vec<Cell>, rows: Vec<Row>) -> Result<()> {
        self.create_table()?
            .set_headers(headers)
            .set_rows(rows)
            .render()?;
        self.new_line(1)
    }

    /// `table($headers, $rows)` with rows of headers.
    pub fn table_with_header_rows(&mut self, headers: Vec<Row>, rows: Vec<Row>) -> Result<()> {
        self.create_table()?
            .set_header_rows(headers)
            .set_rows(rows)
            .render()?;
        self.new_line(1)
    }

    /// `horizontalTable($headers, $rows)`.
    pub fn horizontal_table(&mut self, headers: Vec<Cell>, rows: Vec<Row>) -> Result<()> {
        self.create_table()?
            .set_horizontal(true)
            .set_headers(headers)
            .set_rows(rows)
            .render()?;
        self.new_line(1)
    }

    /// `definitionList(...$list)`.
    pub fn definition_list(&mut self, list: &[DefinitionItem]) -> Result<()> {
        let mut headers = Vec::with_capacity(list.len());
        let mut row = Vec::with_capacity(list.len());
        for value in list {
            match value {
                DefinitionItem::Separator => {
                    let separator = Cell::Table(TableCell::separator(&[])?);
                    headers.push(separator.clone());
                    row.push(separator);
                }
                DefinitionItem::Title(title) => {
                    headers.push(Cell::Table(TableCell::with_options(
                        title.as_str(),
                        &[("colspan", CellOption::Int(2))],
                    )?));
                    row.push(Cell::null());
                }
                DefinitionItem::Pair(key, value) => {
                    headers.push(key.clone());
                    row.push(value.clone());
                }
            }
        }
        self.horizontal_table(headers, vec![Row::cells(row)])
    }

    /// `writeln($messages, $type)`: each message to the output and to the
    /// buffer.
    pub fn writeln_all<S: AsRef<str>>(&mut self, messages: &[S], kind: i64) -> Result<()> {
        for message in messages {
            self.output
                .write_messages(&[message.as_ref()], true, kind)?;
            self.buffered_output
                .write_messages(&[message.as_ref()], true, kind)?;
        }
        Ok(())
    }

    /// `write($messages, $newline, $type)`.
    pub fn write_all<S: AsRef<str>>(
        &mut self,
        messages: &[S],
        newline: bool,
        kind: i64,
    ) -> Result<()> {
        for message in messages {
            self.output
                .write_messages(&[message.as_ref()], newline, kind)?;
            self.buffered_output
                .write_messages(&[message.as_ref()], newline, kind)?;
        }
        Ok(())
    }

    /// `newLine($count)`.
    pub fn new_line(&mut self, count: usize) -> Result<()> {
        // OutputStyle::newLine()
        self.output
            .write_messages(&["\n".repeat(count).as_str()], false, OUTPUT_NORMAL)?;
        self.buffered_output
            .write_messages(&["\n".repeat(count).as_str()], false, OUTPUT_NORMAL)
    }

    /// `getErrorStyle()`: a new style on the same output (none here has a
    /// separate error output).
    pub fn get_error_style(self) -> Result<SymfonyStyle<'a>> {
        SymfonyStyle::new(self.input, self.output)
    }

    /// `createTable()`: a Table on the decorated output, in the
    /// symfony-style-guide style with `<info>` headers.
    pub fn create_table(&mut self) -> Result<Table<'_>> {
        let mut style = Table::get_style_definition("symfony-style-guide")?;
        style.set_cell_header_format("<info>%s</info>");
        let mut table = Table::new(&mut *self.output);
        table.set_table_style(style);
        Ok(table)
    }

    /// `autoPrependBlock()`.
    fn auto_prepend_block(&mut self) -> Result<()> {
        let fetched = self.buffered_output.fetch();
        let chars = &fetched[fetched.len().saturating_sub(2)..];
        if chars.is_empty() {
            // empty history, so we should start with a new line.
            return self.new_line(1);
        }
        // Prepend new line for each non LF chars (This means no blank line was output before)
        let newlines = chars.iter().filter(|&&b| b == b'\n').count();
        self.new_line(2 - newlines)
    }

    /// `autoPrependText()`.
    fn auto_prepend_text(&mut self) -> Result<()> {
        let fetched = self.buffered_output.fetch();
        // Prepend new line if last char isn't EOL:
        let truthy = !fetched.is_empty() && fetched.as_slice() != b"0";
        if truthy && !fetched.ends_with(b"\n") {
            self.new_line(1)?;
        }
        Ok(())
    }

    fn width_without_decoration(&mut self, text: &str) -> Result<i64> {
        Ok(Helper::width(&Helper::remove_decoration(
            self.output.get_formatter(),
            text,
        )?) as i64)
    }

    /// `createBlock($messages, $type, $style, $prefix, $padding, $escape)`.
    fn create_block(
        &mut self,
        messages: &[&str],
        kind: Option<&str>,
        style: Option<&str>,
        prefix: &str,
        padding: bool,
        escape: bool,
    ) -> Result<Vec<String>> {
        let mut indent_length = 0;
        let prefix_length = self.width_without_decoration(prefix)?;
        let mut lines: Vec<String> = Vec::new();
        let kind = kind.map(|k| format!("[{k}] "));
        let mut line_indentation = String::new();
        if let Some(kind) = &kind {
            indent_length = Helper::width(kind) as i64;
            line_indentation = " ".repeat(indent_length as usize);
        }

        // wrap and add newlines for each element
        let output_wrapper = OutputWrapper::new(false);
        for (key, message) in messages.iter().enumerate() {
            let mut message = if escape {
                OutputFormatter::escape(message)
            } else {
                (*message).to_owned()
            };
            message = message.replace("\r\n", "\n");
            let wrapped = output_wrapper.wrap(
                &message,
                self.line_length - prefix_length - indent_length,
                "\n",
            )?;
            lines.extend(wrapped.split('\n').map(str::to_owned));

            if messages.len() > 1 && key < messages.len() - 1 {
                lines.push(String::new());
            }
        }

        let mut first_line_index = 0;
        if padding && self.output.is_decorated() {
            first_line_index = 1;
            lines.insert(0, String::new());
            lines.push(String::new());
        }

        let decorated = self.output.is_decorated();
        for (i, line) in lines.iter_mut().enumerate() {
            if let Some(kind) = &kind {
                *line = if first_line_index == i {
                    format!("{kind}{line}")
                } else {
                    format!("{line_indentation}{line}")
                };
            }

            *line = format!("{prefix}{line}");
            let undecorated = Helper::remove_decoration(self.output.get_formatter(), line)?;
            let padding_length = (self.line_length - Helper::width(&undecorated) as i64).max(0);

            // ECH paints the trailing cells with the current background and CUF moves the cursor past
            // them without writing characters, so most terminals trim them when copying the selection.
            if truthy(style) && decorated && padding_length > 0 {
                line.push_str(&format!("\x1b[{padding_length}X\x1b[{padding_length}C"));
            } else {
                line.push_str(&" ".repeat(padding_length as usize));
            }

            if let Some(style) = style.filter(|s| truthy(Some(s))) {
                *line = format!("<{style}>{line}</>");
            }
        }

        Ok(lines)
    }

    /// `createOutlineBlock($messages, $type, $style, $padding, $escape)`.
    fn create_outline_block(
        &mut self,
        messages: &[&str],
        kind: Option<&str>,
        style: Option<&str>,
        padding: bool,
        escape: bool,
    ) -> Result<Vec<String>> {
        // Line format: ' │ '(3) + content($contentWidth) + ' │'(2) = lineLength
        let content_width = self.line_length - 5;

        let mut lines: Vec<String> = Vec::new();
        let output_wrapper = OutputWrapper::new(false);
        for (key, message) in messages.iter().enumerate() {
            let mut message = if escape {
                OutputFormatter::escape(message)
            } else {
                (*message).to_owned()
            };
            message = message.replace("\r\n", "\n");
            let wrapped = output_wrapper.wrap(&message, content_width, "\n")?;
            lines.extend(wrapped.split('\n').map(str::to_owned));

            if messages.len() > 1 && key < messages.len() - 1 {
                lines.push(String::new());
            }
        }

        if padding {
            lines.insert(0, String::new());
            lines.push(String::new());
        }

        let styled = |line: String| match style.filter(|s| truthy(Some(s))) {
            Some(style) => sprintf_s("<%s>%s</>", &[style, &line]),
            None => Ok(line),
        };

        let mut result = Vec::with_capacity(lines.len() + 2);

        // Top border: ' ┌─ Type ────┐' or ' ┌────┐' when no type
        let top = match kind {
            Some(kind) => format!(
                " ┌─ {kind} {}┐",
                "─".repeat((self.line_length - 6 - Helper::width(kind) as i64).max(0) as usize)
            ),
            None => format!(" ┌{}┐", "─".repeat((self.line_length - 3).max(0) as usize)),
        };
        result.push(styled(top)?);

        for line in lines {
            let line_content_width = self.width_without_decoration(&line)?;
            let padded = format!(
                "{line}{}",
                " ".repeat((content_width - line_content_width).max(0) as usize)
            );
            result.push(match style.filter(|s| truthy(Some(s))) {
                Some(style) => format!("<{style}> │ </>{padded}<{style}> │</>"),
                None => format!(" │ {padded} │"),
            });
        }

        let border_dashes = "─".repeat((self.line_length - 3).max(0) as usize);
        result.push(styled(format!(" └{border_dashes}┘"))?);

        Ok(result)
    }
}

impl OutputInterface for SymfonyStyle<'_> {
    fn write_messages(&mut self, messages: &[&str], newline: bool, options: i64) -> Result<()> {
        self.write_all(messages, newline, options)
    }

    fn set_verbosity(&mut self, level: i64) {
        self.output.set_verbosity(level);
    }

    fn get_verbosity(&self) -> i64 {
        self.output.get_verbosity()
    }

    fn set_decorated(&mut self, decorated: bool) {
        self.output.set_decorated(decorated);
    }

    fn is_decorated(&self) -> bool {
        self.output.is_decorated()
    }

    fn set_formatter(&mut self, formatter: OutputFormatter) {
        self.output.set_formatter(formatter);
    }

    fn get_formatter(&mut self) -> &mut OutputFormatter {
        self.output.get_formatter()
    }

    fn is_silent(&self) -> bool {
        self.output.is_silent()
    }

    fn is_quiet(&self) -> bool {
        self.output.is_quiet()
    }

    fn is_verbose(&self) -> bool {
        self.output.is_verbose()
    }

    fn is_very_verbose(&self) -> bool {
        self.output.is_very_verbose()
    }

    fn is_debug(&self) -> bool {
        self.output.is_debug()
    }
}
