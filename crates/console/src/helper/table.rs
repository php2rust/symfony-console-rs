//! Port of Symfony\Component\Console\Helper\Table.php and
//! TableRows.php.
//!
//! The algorithm is upstream's, step by step. PHP's row and cell arrays
//! are `Arr`s: ordered maps with integer keys, so `isset()`, `$a[] =`,
//! `array_splice()` (which renumbers) and `array_replace_recursive()` keep
//! their meaning. A row is cells, a `TableSeparator`, or the divider that
//! `render()` puts between the headers and the rows (`$divider === $row`).
//!
//! The style registry (`setStyleDefinition()`) is per thread, as the
//! crate's other PHP statics are. Not ported (M7, with ConsoleSectionOutput):
//! `appendRow()` re-rendering into a section; it refuses every output, as
//! PHP refuses one that is not a section.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/Table.php Helper/TableRows.php

use std::{cell::RefCell, collections::HashMap};

use fancy_regex::Regex;

use super::{
    helper::Helper,
    table_cell::{Cell, CellOption, TableCell},
    table_style::{PadType, TableStyle},
};
use crate::{
    exception::{ConsoleError, Result},
    formatter::output_formatter::OutputFormatter,
    output::output::OutputInterface,
    php::{http_build_query, mb_str_split, mb_substr, sprintf_s, str_pad},
};

/// A PHP array with integer keys, in insertion order.
#[derive(Debug, Clone, PartialEq)]
pub struct Arr<T>(Vec<(i64, T)>);

impl<T> Default for Arr<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: Clone> Arr<T> {
    /// A list: keys 0, 1, 2...
    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    pub fn list(values: Vec<T>) -> Self {
        Self(
            values
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i as i64, v))
                .collect(),
        )
    }

    /// Entries with the given keys, in order.
    pub fn keyed(entries: Vec<(i64, T)>) -> Self {
        Self(entries)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: i64) -> Option<&T> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    fn get_mut(&mut self, key: i64) -> Option<&mut T> {
        self.0.iter_mut().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: i64) -> bool {
        self.get(key).is_some()
    }

    /// `$a[$key] = $value`: an existing key keeps its place.
    pub fn set(&mut self, key: i64, value: T) {
        match self.get_mut(key) {
            Some(slot) => *slot = value,
            None => self.0.push((key, value)),
        }
    }

    /// `$a[] = $value`: the next key is one more than the largest.
    pub fn push(&mut self, value: T) {
        let key = self.0.iter().map(|(k, _)| k + 1).max().unwrap_or(0).max(0);
        self.0.push((key, value));
    }

    /// `array_splice($a, $offset, 0, [$value])`: inserted at a position,
    /// then the keys renumbered.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn splice_insert(&mut self, offset: i64, value: T) {
        let at = (offset.max(0) as usize).min(self.0.len());
        self.0.insert(at, (0, value));
        for (i, entry) in self.0.iter_mut().enumerate() {
            entry.0 = i as i64;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, &T)> {
        self.0.iter().map(|(k, v)| (*k, v))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.0.iter().map(|(_, v)| v)
    }

    fn keys(&self) -> Vec<i64> {
        self.0.iter().map(|(k, _)| *k).collect()
    }
}

/// A row's cells.
pub type Cells = Arr<Cell>;

/// A table row.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Cells(Cells),
    /// A `TableSeparator` given as a row.
    Separator(TableCell),
    /// `render()`'s divider between the headers and the rows.
    Divider,
}

impl Row {
    /// A row of cells (keys 0, 1, 2...).
    pub fn cells<C: Into<Cell>>(cells: impl IntoIterator<Item = C>) -> Self {
        Self::Cells(Arr::list(cells.into_iter().map(Into::into).collect()))
    }

    /// `new TableSeparator()` as a row.
    pub fn separator() -> Self {
        Self::Separator(TableCell::new("").into_separator())
    }

    /// `instanceof TableSeparator`.
    fn is_separator(&self) -> bool {
        !matches!(self, Self::Cells(_))
    }
}

impl TableCell {
    fn into_separator(self) -> Self {
        // TableSeparator::__construct([]) cannot fail
        Self::separator(&[]).unwrap_or(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Orientation {
    Default,
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Separator {
    Top,
    TopBottom,
    Mid,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Border {
    Outside,
    Inside,
}

thread_local! {
    static STYLES: RefCell<Option<HashMap<String, TableStyle>>> = const { RefCell::new(None) };
}

fn with_styles<R>(f: impl FnOnce(&mut HashMap<String, TableStyle>) -> R) -> R {
    STYLES.with(|styles| f(styles.borrow_mut().get_or_insert_with(init_styles)))
}

/// `Table`.
pub struct Table<'a> {
    output: &'a mut dyn OutputInterface,
    header_title: Option<String>,
    footer_title: Option<String>,
    headers: Vec<Row>,
    rows: Arr<Row>,
    effective_column_widths: Vec<i64>,
    number_of_columns: i64,
    style: TableStyle,
    column_styles: HashMap<i64, TableStyle>,
    column_widths: HashMap<i64, i64>,
    configured_column_widths: HashMap<i64, i64>,
    column_max_widths: HashMap<i64, i64>,
    rendered: bool,
    orientation: Orientation,
}

impl<'a> Table<'a> {
    /// `new Table($output)`.
    pub fn new(output: &'a mut dyn OutputInterface) -> Self {
        Self {
            output,
            header_title: None,
            footer_title: None,
            headers: Vec::new(),
            rows: Arr::default(),
            effective_column_widths: Vec::new(),
            number_of_columns: 0,
            style: with_styles(|s| s.get("default").cloned()).unwrap_or_default(),
            column_styles: HashMap::new(),
            column_widths: HashMap::new(),
            configured_column_widths: HashMap::new(),
            column_max_widths: HashMap::new(),
            rendered: false,
            orientation: Orientation::Default,
        }
    }

    /// `Table::setStyleDefinition($name, $style)`.
    pub fn set_style_definition(name: &str, style: TableStyle) {
        with_styles(|s| s.insert(name.to_owned(), style));
    }

    /// `Table::getStyleDefinition($name)`.
    pub fn get_style_definition(name: &str) -> Result<TableStyle> {
        with_styles(|s| s.get(name).cloned()).ok_or_else(|| {
            ConsoleError::InvalidArgument(format!("Style \"{name}\" is not defined."))
        })
    }

    /// `setStyle($name)` with a style name.
    pub fn set_style(&mut self, name: &str) -> Result<&mut Self> {
        self.style = Self::get_style_definition(name)?;
        Ok(self)
    }

    /// `setStyle($style)` with a `TableStyle`.
    pub fn set_table_style(&mut self, style: TableStyle) -> &mut Self {
        self.style = style;
        self
    }

    /// `getStyle()`.
    pub fn get_style(&self) -> &TableStyle {
        &self.style
    }

    /// `setColumnStyle($columnIndex, $name)` with a style name.
    pub fn set_column_style(&mut self, column_index: i64, name: &str) -> Result<&mut Self> {
        let style = Self::get_style_definition(name)?;
        self.column_styles.insert(column_index, style);
        Ok(self)
    }

    /// `setColumnStyle($columnIndex, $style)` with a `TableStyle`.
    pub fn set_column_table_style(&mut self, column_index: i64, style: TableStyle) -> &mut Self {
        self.column_styles.insert(column_index, style);
        self
    }

    /// `getColumnStyle($columnIndex)`.
    pub fn get_column_style(&self, column_index: i64) -> &TableStyle {
        self.column_styles.get(&column_index).unwrap_or(&self.style)
    }

    /// `setColumnWidth($columnIndex, $width)`.
    pub fn set_column_width(&mut self, column_index: i64, width: i64) -> &mut Self {
        self.column_widths.insert(column_index, width);
        self.configured_column_widths.insert(column_index, width);
        self
    }

    /// `setColumnWidths($widths)`.
    pub fn set_column_widths(&mut self, widths: &[(i64, i64)]) -> &mut Self {
        self.column_widths.clear();
        self.configured_column_widths.clear();
        for (index, width) in widths {
            self.set_column_width(*index, *width);
        }
        self
    }

    /// `setColumnMaxWidth($columnIndex, $width)`. The formatter here is
    /// always wrappable.
    pub fn set_column_max_width(&mut self, column_index: i64, width: i64) -> &mut Self {
        self.column_max_widths.insert(column_index, width);
        self
    }

    /// `setHeaders($headers)` with one row of headers. An empty list sets
    /// no headers.
    pub fn set_headers<C: Into<Cell>>(
        &mut self,
        headers: impl IntoIterator<Item = C>,
    ) -> &mut Self {
        let headers: Vec<Cell> = headers.into_iter().map(Into::into).collect();
        self.headers = if headers.is_empty() {
            Vec::new()
        } else {
            vec![Row::Cells(Arr::list(headers))]
        };
        self
    }

    /// `setHeaders($headers)` with rows of headers (each an array).
    pub fn set_header_rows(&mut self, headers: Vec<Row>) -> &mut Self {
        self.headers = headers;
        self
    }

    /// `setRows($rows)`.
    pub fn set_rows(&mut self, rows: Vec<Row>) -> &mut Self {
        self.rows = Arr::default();
        self.add_rows(rows)
    }

    /// `addRows($rows)`.
    pub fn add_rows(&mut self, rows: Vec<Row>) -> &mut Self {
        for row in rows {
            self.add_row(row);
        }
        self
    }

    /// `addRow($row)`: a row of cells is renumbered (`array_values()`).
    pub fn add_row(&mut self, row: Row) -> &mut Self {
        let row = match row {
            Row::Cells(cells) => Row::Cells(Arr::list(cells.values().cloned().collect())),
            other => other,
        };
        self.rows.push(row);
        self
    }

    /// `appendRow($row)`: needs a `ConsoleSectionOutput`, which is not
    /// ported, so it is always refused as PHP refuses another output.
    pub fn append_row(&mut self, _row: Row) -> Result<&mut Self> {
        Err(ConsoleError::Runtime(
            "Output should be an instance of \"Symfony\\Component\\Console\\Output\\ConsoleSectionOutput\" when calling \"Symfony\\Component\\Console\\Helper\\Table::appendRow\".".to_owned(),
        ))
    }

    /// `setRow($column, $row)`: kept as given.
    pub fn set_row(&mut self, column: i64, row: Row) -> &mut Self {
        self.rows.set(column, row);
        self
    }

    /// `setHeaderTitle($title)`.
    pub fn set_header_title(&mut self, title: Option<&str>) -> &mut Self {
        self.header_title = title.map(str::to_owned);
        self
    }

    /// `setFooterTitle($title)`.
    pub fn set_footer_title(&mut self, title: Option<&str>) -> &mut Self {
        self.footer_title = title.map(str::to_owned);
        self
    }

    /// `setHorizontal($horizontal)`.
    pub fn set_horizontal(&mut self, horizontal: bool) -> &mut Self {
        self.orientation = if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Default
        };
        self
    }

    /// `setVertical($vertical)`.
    pub fn set_vertical(&mut self, vertical: bool) -> &mut Self {
        self.orientation = if vertical {
            Orientation::Vertical
        } else {
            Orientation::Default
        };
        self
    }

    fn formatter(&mut self) -> &mut OutputFormatter {
        self.output.get_formatter()
    }

    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn width_of(&mut self, text: &str) -> Result<i64> {
        Ok(Helper::width(&Helper::remove_decoration(self.formatter(), text)?) as i64)
    }

    /// `render()`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    #[expect(clippy::too_many_lines, reason = "one function, as upstream's")]
    pub fn render(&mut self) -> Result<()> {
        let horizontal = self.orientation == Orientation::Horizontal;
        let vertical = self.orientation == Orientation::Vertical;
        let first_headers = match self.headers.first() {
            Some(Row::Cells(cells)) => cells.clone(),
            _ => Arr::default(),
        };

        let mut rows: Arr<Row> = Arr::default();
        if horizontal {
            for (i, header) in first_headers.iter() {
                let mut row = Arr::list(vec![header.clone()]);
                for source in self.rows.values() {
                    let Row::Cells(source) = source else {
                        continue;
                    };
                    match source.get(i) {
                        Some(cell) if !cell.is_null() => row.push(cell.clone()),
                        _ if is_cell_with_colspan(header) => {
                            // Noop, there is a "title"
                        }
                        _ => row.push(Cell::null()),
                    }
                }
                rows.set(i, Row::Cells(row));
            }
        } else if vertical {
            let mut max_header_length = 0;
            for header in first_headers.values() {
                let header = header.to_php_string()?;
                max_header_length = max_header_length.max(self.width_of(&header)?);
            }
            let divider = Cell::Table(TableCell::new("").into_separator());

            for source in self.rows.values().cloned().collect::<Vec<_>>() {
                let Row::Cells(source) = source else {
                    continue;
                };
                if !rows.is_empty() {
                    rows.push(Row::Cells(Arr::list(vec![divider.clone()])));
                }
                let contains_colspan = source.values().any(is_cell_with_colspan);
                let max_rows = first_headers.len().max(source.len()) as i64;
                for i in 0..max_rows {
                    let cell = match source.get(i) {
                        Some(cell) => cell.to_php_string()?,
                        None => String::new(),
                    };
                    let eol = if cell.contains("\r\n") { "\r\n" } else { "\n" };
                    for (idx, part) in cell.split(eol).enumerate() {
                        if !first_headers.is_empty() && !contains_colspan {
                            let header = match first_headers.get(i) {
                                Some(h) => h.to_php_string()?,
                                None => String::new(),
                            };
                            let line = if idx == 0 {
                                let pad = (max_header_length - self.width_of(&header)?).max(0);
                                format!("<comment>{}{header}</>: {part}", " ".repeat(pad as usize))
                            } else {
                                format!("{}  {part}", " ".repeat(max_header_length.max(0) as usize))
                            };
                            rows.push(Row::Cells(Arr::list(vec![Cell::from(line)])));
                        } else if !cell.is_empty() {
                            rows.push(Row::Cells(Arr::list(vec![Cell::from(part)])));
                        }
                    }
                }
            }
        } else {
            let mut merged: Vec<Row> = self.headers.clone();
            merged.push(Row::Divider);
            merged.extend(self.rows.values().cloned());
            rows = Arr::list(merged);
        }

        self.calculate_number_of_columns(&rows);

        let row_groups = self.build_table_rows(rows)?;
        self.calculate_columns_width(&row_groups)?;

        let mut is_header = !horizontal;
        let mut is_first_row = horizontal;
        let mut has_title = self.header_title.as_deref().is_some_and(truthy);
        let mut previous_row: Option<Cells> = None;
        let mut pending_separators = 0;
        let headers_present = self
            .headers
            .iter()
            .any(|h| !matches!(h, Row::Cells(c) if c.is_empty()));

        for row_group in &row_groups {
            let mut is_header_separator_rendered = false;

            for row in row_group {
                let cells = match row {
                    Row::Divider => {
                        is_header = false;
                        is_first_row = true;
                        continue;
                    }
                    Row::Separator(_) => {
                        // defer rendering until the row below is known, to align crossings with it
                        pending_separators += 1;
                        continue;
                    }
                    Row::Cells(cells) if cells.is_empty() => continue,
                    Row::Cells(cells) => cells,
                };

                while pending_separators > 0 {
                    self.render_row_separator(
                        Separator::Mid,
                        None,
                        None,
                        previous_row.as_ref(),
                        Some(cells),
                    )?;
                    pending_separators -= 1;
                }

                if is_header && !is_header_separator_rendered && self.style.display_outside_border()
                {
                    let title = if has_title {
                        self.header_title.clone()
                    } else {
                        None
                    };
                    let format = has_title.then(|| self.style.get_header_title_format().to_owned());
                    self.render_row_separator(
                        Separator::Top,
                        title.as_deref(),
                        format.as_deref(),
                        None,
                        Some(cells),
                    )?;
                    has_title = false;
                    is_header_separator_rendered = true;
                }

                if is_first_row {
                    let kind = if horizontal || !headers_present {
                        Separator::Top
                    } else {
                        Separator::TopBottom
                    };
                    let title = if has_title {
                        self.header_title.clone()
                    } else {
                        None
                    };
                    let format = has_title.then(|| self.style.get_header_title_format().to_owned());
                    self.render_row_separator(
                        kind,
                        title.as_deref(),
                        format.as_deref(),
                        previous_row.as_ref(),
                        Some(cells),
                    )?;
                    is_first_row = false;
                    has_title = false;
                }

                if vertical {
                    is_header = false;
                    is_first_row = false;
                }

                if horizontal {
                    let row_format = self.style.get_cell_row_format().to_owned();
                    let header_format = self.style.get_cell_header_format().to_owned();
                    self.render_row(cells, &row_format, Some(&header_format))?;
                } else {
                    let format = if is_header {
                        self.style.get_cell_header_format().to_owned()
                    } else {
                        self.style.get_cell_row_format().to_owned()
                    };
                    self.render_row(cells, &format, None)?;
                }

                previous_row = Some(cells.clone());
            }
        }

        while pending_separators > 0 {
            self.render_row_separator(Separator::Mid, None, None, previous_row.as_ref(), None)?;
            pending_separators -= 1;
        }

        if self.style.display_outside_border() {
            let title = self.footer_title.clone();
            let format = self.style.get_footer_title_format().to_owned();
            self.render_row_separator(
                Separator::Bottom,
                title.as_deref(),
                Some(&format),
                previous_row.as_ref(),
                None,
            )?;
        }

        self.cleanup();
        self.rendered = true;
        Ok(())
    }

    /// `renderRowSeparator($type, $title, $titleFormat, $rowAbove, $rowBelow)`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn render_row_separator(
        &mut self,
        kind: Separator,
        title: Option<&str>,
        title_format: Option<&str>,
        row_above: Option<&Cells>,
        row_below: Option<&Cells>,
    ) -> Result<()> {
        let count = self.number_of_columns;
        if count == 0 {
            return Ok(());
        }

        let borders: Vec<String> = self
            .style
            .get_border_chars()
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        if !truthy(&borders[0]) && !truthy(&borders[2]) && !truthy(self.style.get_crossing_char()) {
            return Ok(());
        }

        let crossings: Vec<String> = self
            .style
            .get_crossing_chars()
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let (horizontal, left_char, mid_char, right_char) = match kind {
            Separator::Mid => (&borders[2], &crossings[8], &crossings[0], &crossings[4]),
            Separator::Top => (&borders[0], &crossings[1], &crossings[2], &crossings[3]),
            Separator::TopBottom => (&borders[0], &crossings[9], &crossings[10], &crossings[11]),
            Separator::Bottom => (&borders[0], &crossings[7], &crossings[6], &crossings[5]),
        };

        // only draw a crossing where a vertical border meets the separator; a cell
        // spanning several columns has no border on the columns it covers
        let has_line_above = kind != Separator::Top;
        let has_line_below = kind != Separator::Bottom;
        let columns_above = row_above.map(|r| self.get_row_columns(r));
        let columns_below = row_below.map(|r| self.get_row_columns(r));

        let mut markup = left_char.clone();
        for column in 0..count {
            let width = self
                .effective_column_widths
                .get(column as usize)
                .copied()
                .unwrap_or(0);
            markup.push_str(&horizontal.repeat(width.max(0) as usize));
            if column == count - 1 {
                markup.push_str(right_char);
                continue;
            }
            let border_above = has_line_above
                && columns_above
                    .as_ref()
                    .is_none_or(|c| c.contains(&(column + 1)));
            let border_below = has_line_below
                && columns_below
                    .as_ref()
                    .is_none_or(|c| c.contains(&(column + 1)));
            // the one-sided junctions reuse the top/bottom crossing chars; for the "box-double"
            // style these are double-horizontal (╤/╧) while an inner separator is single (─), as
            // that style has no single-line ┬/┴ to fall back to
            markup.push_str(match (border_above, border_below) {
                (true, true) => mid_char,
                (false, true) => &crossings[2],
                (true, false) => &crossings[6],
                (false, false) => horizontal,
            });
        }

        if let Some(title) = title {
            let title_format = title_format.unwrap_or("%s");
            let mut formatted_title = sprintf_s(title_format, &[title])?;
            let mut title_length = self.width_of(&formatted_title)?;
            let markup_length = Helper::width(&markup) as i64;
            let limit = markup_length - 4;
            if title_length > limit {
                title_length = limit;
                let format_length = self.width_of(&sprintf_s(title_format, &[""])?)?;
                let cut = Helper::substr(title, 0, Some(limit - format_length - 3));
                formatted_title = sprintf_s(title_format, &[&format!("{cut}...")])?;
            }

            let title_start = (markup_length - title_length).div_euclid(2).max(0) as usize;
            let after = (title_start as i64 + title_length).max(0) as usize;
            markup = format!(
                "{}{formatted_title}{}",
                mb_substr(&markup, 0, Some(title_start)),
                mb_substr(&markup, after, None)
            );
        }

        let line = sprintf_s(self.style.get_border_format(), &[&markup])?;
        self.output.writeln(&line)
    }

    /// `renderColumnSeparator($type)`.
    fn render_column_separator(&self, kind: Border) -> Result<String> {
        let borders = self.style.get_border_chars();
        let border = match kind {
            Border::Outside => borders[1],
            Border::Inside => borders[3],
        };
        sprintf_s(self.style.get_border_format(), &[border])
    }

    /// `renderRow($row, $cellFormat, $firstCellFormat)`.
    fn render_row(
        &mut self,
        row: &Cells,
        cell_format: &str,
        first_cell_format: Option<&str>,
    ) -> Result<()> {
        let mut row_content = self.render_column_separator(Border::Outside)?;
        let columns = self.get_row_columns(row);
        let last = columns.last().copied();
        for column in &columns {
            let format = match first_cell_format {
                Some(first) if truthy(first) && *column == 0 => first,
                _ => cell_format,
            };
            row_content.push_str(&self.render_cell(row, *column, format)?);
            row_content.push_str(&self.render_column_separator(if last == Some(*column) {
                Border::Outside
            } else {
                Border::Inside
            })?);
        }
        self.output.writeln(&row_content)
    }

    /// `renderCell($row, $column, $cellFormat)`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn render_cell(&mut self, row: &Cells, column: i64, cell_format: &str) -> Result<String> {
        let cell = row
            .get(column)
            .filter(|c| !c.is_null())
            .cloned()
            .unwrap_or_else(|| Cell::from(""));
        let text = cell.to_php_string()?;
        let effective = |c: i64| {
            self.effective_column_widths
                .get(c as usize)
                .copied()
                .unwrap_or(0)
        };
        let mut width = effective(column);
        if let Some(table_cell) = cell.as_table_cell()
            && table_cell.get_colspan() > 1
        {
            // add the width of the following columns(numbers of colspan).
            let separator_width = self.get_column_separator_width()?;
            for next_column in (column + 1)..(column + table_cell.get_colspan()) {
                width += separator_width + effective(next_column);
            }
        }

        // str_pad won't work properly with multi-byte strings, we need to fix the padding
        width +=
            text.len() as i64 - Helper::width(&text) as i64 - text.matches('\0').count() as i64;
        let style = self.get_column_style(column).clone();

        if cell.is_separator() {
            let fill = style.get_border_chars()[2].repeat(width.max(0) as usize);
            return sprintf_s(style.get_border_format(), &[&fill]);
        }

        let undecorated = Helper::remove_decoration(self.formatter(), &text)?;
        width += Helper::length(&text) as i64 - Helper::length(&undecorated) as i64;
        let mut content = sprintf_s(style.get_cell_row_content_format(), &[&text])?;

        let mut cell_format = cell_format.to_owned();
        let mut pad_type = style.get_pad_type();
        if let Some(cell_style) = cell.as_table_cell().and_then(TableCell::get_style) {
            if !styled_by_tag(&text)? {
                cell_format = if let Some(format) = cell_style.get_cell_format() {
                    format.to_owned()
                } else {
                    let tag = http_build_query(&cell_style.get_tag_options(), ";");
                    format!("<{tag}>%s</>")
                };

                if content.contains("</>") {
                    content = content.replace("</>", "");
                    width -= 3;
                }
                if content.contains("<fg=default;bg=default>") {
                    content = content.replace("<fg=default;bg=default>", "");
                    width -= "<fg=default;bg=default>".len() as i64;
                }
            }

            pad_type = cell_style.get_pad_by_align();
        }

        let padded = str_pad(&content, width, style.get_padding_char(), pad_type);
        sprintf_s(&cell_format, &[&padded])
    }

    /// `calculateNumberOfColumns($rows)`.
    fn calculate_number_of_columns(&mut self, rows: &Arr<Row>) {
        self.number_of_columns = rows
            .values()
            .filter_map(|row| match row {
                Row::Cells(cells) => Some(number_of_columns(cells)),
                _ => None,
            })
            .fold(0, i64::max);
    }

    /// `buildTableRows($rows)`: each row with the extra lines its
    /// multi-line cells add.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    #[expect(clippy::too_many_lines, reason = "one function, as upstream's")]
    fn build_table_rows(&mut self, rows: Arr<Row>) -> Result<Vec<Vec<Row>>> {
        let mut rows = rows;
        let mut unmerged_rows: Arr<Arr<Cells>> = Arr::default();
        let mut row_key = 0i64;
        while (row_key as usize) < rows.len() {
            rows = self.fill_next_rows(rows, row_key)?;

            // Remove any new line breaks and replace it with a new line
            let Some(Row::Cells(snapshot)) = rows.get(row_key).cloned() else {
                row_key += 1;
                continue;
            };
            for (column, original) in snapshot.iter() {
                let colspan = original.colspan();

                let mut min_wrapped_width = 0;
                let mut width_applied: Vec<(bool, i64)> = Vec::new(); // (is max, column)
                let length_column_border = self.get_column_separator_width()?
                    + Helper::width(self.style.get_cell_row_content_format()) as i64
                    - 2;
                for i in column..column + colspan {
                    if let Some(max) = self.column_max_widths.get(&i) {
                        min_wrapped_width += max;
                        width_applied.push((true, i));
                    } else if self.column_widths.get(&i).copied().unwrap_or(0) > 0 && colspan > 1 {
                        min_wrapped_width += self.column_widths.get(&i).copied().unwrap_or(0);
                        width_applied.push((false, i));
                    }
                }
                if width_applied.len() == 1 {
                    if colspan > 1 {
                        min_wrapped_width *= colspan; // previous logic
                    }
                } else if width_applied.len() > 1 {
                    min_wrapped_width += (width_applied.len() as i64 - 1) * length_column_border;
                }

                let mut cell = original.clone();
                let text = cell.to_php_string()?;
                let mut cell_width = self.width_of(&text)?;
                if min_wrapped_width != 0 && cell_width > min_wrapped_width {
                    let wrapped = self
                        .formatter()
                        .format_and_wrap(Some(&text), min_wrapped_width.max(0) as usize)?;
                    cell = Cell::from(wrapped);
                }
                // update minimal columnWidths for spanned columns
                if colspan > 1 && min_wrapped_width > 0 {
                    let mut processed: Vec<i64> = Vec::new();
                    cell_width = cell_width.min(min_wrapped_width);
                    for (is_max, item_column) in &width_applied {
                        let max = self
                            .column_max_widths
                            .get(item_column)
                            .copied()
                            .unwrap_or(0);
                        let configured = self
                            .configured_column_widths
                            .get(item_column)
                            .copied()
                            .unwrap_or(0);
                        if *is_max && cell_width >= max {
                            self.column_widths.insert(*item_column, max);
                            processed.push(*item_column);
                            cell_width -= max + length_column_border;
                        } else if !*is_max && configured > 0 {
                            // this column already covers part of the cell, so share only the rest
                            processed.push(*item_column);
                            cell_width -= configured + length_column_border;
                        }
                    }
                    for i in column..column + colspan {
                        if processed.contains(&i) {
                            continue;
                        }
                        // a width set through setColumnWidth() is a minimum, unlike one computed for an earlier row
                        let configured =
                            self.configured_column_widths.get(&i).copied().unwrap_or(0);
                        self.column_widths
                            .insert(i, configured.max(cell_width + length_column_border));
                    }
                }

                let text = cell.to_php_string()?;
                if !text.contains('\n') {
                    continue;
                }
                let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
                let escaped = text
                    .split(eol)
                    .map(OutputFormatter::escape_trailing_backslash)
                    .collect::<Vec<_>>()
                    .join(eol);
                let cell = match cell.as_table_cell() {
                    Some(table_cell) => {
                        Cell::Table(colspan_cell(&escaped, table_cell.get_colspan())?)
                    }
                    None => Cell::from(escaped),
                };
                let text = cell.to_php_string()?;
                let replaced = text.replace(eol, &format!("<fg=default;bg=default></>{eol}"));
                for (line_key, line) in replaced.split(eol).enumerate() {
                    let line_key = line_key as i64;
                    let line = if colspan > 1 {
                        Cell::Table(colspan_cell(line, colspan)?)
                    } else {
                        Cell::from(line)
                    };
                    if line_key == 0 {
                        if let Some(Row::Cells(cells)) = rows.get_mut(row_key) {
                            cells.set(column, line);
                        }
                    } else {
                        let exists = unmerged_rows
                            .get(row_key)
                            .is_some_and(|lines| lines.contains_key(line_key));
                        if !exists {
                            let copy = copy_row(&rows, row_key)?;
                            if !unmerged_rows.contains_key(row_key) {
                                unmerged_rows.set(row_key, Arr::default());
                            }
                            if let Some(lines) = unmerged_rows.get_mut(row_key) {
                                lines.set(line_key, copy);
                            }
                        }
                        if let Some(lines) = unmerged_rows.get_mut(row_key)
                            && let Some(copy) = lines.get_mut(line_key)
                        {
                            copy.set(column, line);
                        }
                    }
                }
            }
            row_key += 1;
        }

        // TableRows: each row and its extra lines
        let mut groups = Vec::with_capacity(rows.len());
        for (row_key, row) in rows.iter() {
            let mut group = vec![fill_cells_row(row)];
            if let Some(lines) = unmerged_rows.get(row_key) {
                for line in lines.values() {
                    group.push(Row::Cells(fill_cells(line)));
                }
            }
            groups.push(group);
        }
        Ok(groups)
    }

    /// `fillNextRows($rows, $line)`: the cells a rowspan adds to the rows
    /// below.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    #[expect(clippy::too_many_lines, reason = "one function, as upstream's")]
    fn fill_next_rows(&self, rows: Arr<Row>, line: i64) -> Result<Arr<Row>> {
        let mut rows = rows;
        let mut unmerged_rows: Arr<Cells> = Arr::default();
        let Some(Row::Cells(snapshot)) = rows.get(line).cloned() else {
            return Ok(rows);
        };
        for (column, cell) in snapshot.iter() {
            cell.check()?;
            let Some(table_cell) = cell.as_table_cell().filter(|c| c.get_rowspan() > 1) else {
                continue;
            };
            let mut nb_lines = table_cell.get_rowspan() - 1;
            let mut lines: Arr<String> = Arr::list(vec![table_cell.value().to_owned()]);
            let text = table_cell.value();
            let options = |colspan: i64| -> Vec<(&str, CellOption)> {
                vec![
                    ("colspan", CellOption::Int(colspan)),
                    (
                        "style",
                        table_cell
                            .get_style()
                            .cloned()
                            .map_or(CellOption::Null, CellOption::Style),
                    ),
                ]
            };
            if text.contains('\n') {
                let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
                let replaced = text.replace(eol, &format!("<fg=default;bg=default>{eol}</>"));
                lines = Arr::list(replaced.split(eol).map(str::to_owned).collect());
                nb_lines = if lines.len() as i64 > nb_lines {
                    text.matches(eol).count() as i64
                } else {
                    nb_lines
                };

                let first = lines.get(0).cloned().unwrap_or_default();
                if let Some(Row::Cells(cells)) = rows.get_mut(line) {
                    cells.set(
                        column,
                        Cell::Table(TableCell::with_options(
                            first,
                            &options(table_cell.get_colspan()),
                        )?),
                    );
                }
                lines.0.retain(|(k, _)| *k != 0);
            }

            // create a two dimensional array (rowspan x colspan)
            let mut replaced: Arr<Cells> = Arr::default();
            for key in line + 1..line + 1 + nb_lines.max(0) {
                replaced.set(key, unmerged_rows.get(key).cloned().unwrap_or_default());
            }
            for (key, value) in unmerged_rows.iter() {
                if !replaced.contains_key(key) {
                    replaced.set(key, value.clone());
                }
            }
            unmerged_rows = replaced;
            for unmerged_row_key in unmerged_rows.keys() {
                let value = lines
                    .get(unmerged_row_key - line)
                    .cloned()
                    .unwrap_or_default();
                let new_cell = TableCell::with_options(value, &options(table_cell.get_colspan()))?;
                if let Some(unmerged_row) = unmerged_rows.get_mut(unmerged_row_key) {
                    unmerged_row.set(column, Cell::Table(new_cell));
                }
                if nb_lines == unmerged_row_key - line {
                    break;
                }
            }
        }

        for (unmerged_row_key, unmerged_row) in unmerged_rows.iter() {
            // we need to know if $unmergedRow will be merged or inserted into $rows
            let merge = match rows.get(unmerged_row_key) {
                Some(Row::Cells(target)) => {
                    number_of_columns(target) + number_of_columns(unmerged_row)
                        <= self.number_of_columns
                }
                _ => false,
            };
            if merge {
                if let Some(Row::Cells(target)) = rows.get_mut(unmerged_row_key) {
                    for (cell_key, cell) in unmerged_row.iter() {
                        // insert cell into row at cellKey position
                        target.splice_insert(cell_key, cell.clone());
                    }
                }
            } else {
                let mut row = copy_row(&rows, unmerged_row_key - 1)?;
                let separator_above =
                    matches!(rows.get(unmerged_row_key - 1), Some(r) if r.is_separator());
                for (column, cell) in unmerged_row.iter() {
                    if cell.truthy() {
                        if separator_above {
                            return Err(ConsoleError::Other {
                                class: "Error".to_owned(),
                                message: "Cannot use object of type Symfony\\Component\\Console\\Helper\\TableSeparator as array".to_owned(),
                            });
                        }
                        row.set(column, cell.clone());
                    }
                }
                rows.splice_insert(unmerged_row_key, Row::Cells(row));
            }
        }

        Ok(rows)
    }

    /// `getRowColumns($row)`: the columns a row's cells start, those a
    /// colspan covers left out.
    fn get_row_columns(&self, row: &Cells) -> Vec<i64> {
        // range(0, $n - 1): with no column, PHP's range(0, -1) is [0, -1]
        let mut columns: Vec<i64> = if self.number_of_columns > 0 {
            (0..self.number_of_columns).collect()
        } else {
            vec![0, -1]
        };
        for (cell_key, cell) in row.iter() {
            if let Some(table_cell) = cell.as_table_cell()
                && table_cell.get_colspan() > 1
            {
                // exclude grouped columns.
                let covered = cell_key + 1..=cell_key + table_cell.get_colspan() - 1;
                columns.retain(|c| !covered.contains(c));
            }
        }
        columns
    }

    /// `calculateColumnsWidth($groups)`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn calculate_columns_width(&mut self, groups: &[Vec<Row>]) -> Result<()> {
        let n = self.number_of_columns.max(0) as usize;
        // Initialize max widths for all columns
        let mut max_column_widths = vec![0i64; n];

        // Single pass through all rows to calculate all column widths at once
        for group in groups {
            for row in group {
                let Row::Cells(row) = row else {
                    continue;
                };
                let mut row = row.clone();

                // Process TableCell colspan splitting once per row
                for (i, cell) in row.clone().iter() {
                    if let Some(table_cell) = cell.as_table_cell()
                        && table_cell.get_colspan() > 1
                    {
                        let text_content =
                            Helper::remove_decoration(self.formatter(), table_cell.value())?;
                        let text_length = Helper::width(&text_content) as i64;
                        if text_length > 0 {
                            let chunk = (text_length + table_cell.get_colspan() - 1)
                                / table_cell.get_colspan();
                            for (position, content) in mb_str_split(&text_content, chunk as usize)
                                .into_iter()
                                .enumerate()
                            {
                                row.set(i + position as i64, Cell::from(content));
                            }
                        }
                    }
                }

                // Calculate width for each column in this row
                for (column, max) in max_column_widths.iter_mut().enumerate() {
                    let cell_width = self.get_cell_width(&row, column as i64)?;
                    if cell_width > *max {
                        *max = cell_width;
                    }
                }
            }
        }

        let cell_content_format_width =
            Helper::width(self.style.get_cell_row_content_format()) as i64 - 2;
        self.effective_column_widths = max_column_widths
            .iter()
            .map(|w| w + cell_content_format_width)
            .collect();
        Ok(())
    }

    /// `getColumnSeparatorWidth()`.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "PHP's int and float conversions, offsets and lengths, as upstream"
    )]
    fn get_column_separator_width(&self) -> Result<i64> {
        let border = sprintf_s(
            self.style.get_border_format(),
            &[self.style.get_border_chars()[3]],
        )?;
        Ok(Helper::width(&border) as i64)
    }

    /// `getCellWidth($row, $column, $formatter)`.
    fn get_cell_width(&mut self, row: &Cells, column: i64) -> Result<i64> {
        let mut cell_width = 0;
        if let Some(cell) = row.get(column).filter(|c| !c.is_null()) {
            let text = cell.to_php_string()?;
            cell_width = self.width_of(&text)?;
        }
        let column_width = self.column_widths.get(&column).copied().unwrap_or(0);
        cell_width = cell_width.max(column_width);
        Ok(match self.column_max_widths.get(&column) {
            Some(max) => (*max).min(cell_width),
            None => cell_width,
        })
    }

    /// `cleanup()`.
    fn cleanup(&mut self) {
        self.effective_column_widths.clear();
        self.number_of_columns = 0;
    }
}

/// `$cell instanceof TableCell && $cell->getColspan() >= 2`.
fn is_cell_with_colspan(cell: &Cell) -> bool {
    cell.as_table_cell().is_some_and(|c| c.get_colspan() >= 2)
}

/// A string's truthiness: "" and "0" are false.
fn truthy(s: &str) -> bool {
    !s.is_empty() && s != "0"
}

fn colspan_cell(value: &str, colspan: i64) -> Result<TableCell> {
    TableCell::with_options(value, &[("colspan", CellOption::Int(colspan))])
}

/// `getNumberOfColumns($row)`.
#[expect(
    clippy::cast_possible_wrap,
    reason = "PHP's int and float conversions, offsets and lengths, as upstream"
)]
fn number_of_columns(row: &Cells) -> i64 {
    row.len() as i64
        + row
            .values()
            .map(|c| c.as_table_cell().map_or(0, |t| t.get_colspan() - 1))
            .sum::<i64>()
}

/// `copyRow($rows, $line)`: the row's cells emptied, colspans kept.
fn copy_row(rows: &Arr<Row>, line: i64) -> Result<Cells> {
    let Some(Row::Cells(row)) = rows.get(line) else {
        // a separator row has no cells to copy; a missing one is null,
        // which PHP turns into an empty array when a cell is set
        return Ok(Arr::default());
    };
    let mut copy = Arr::default();
    for (key, value) in row.iter() {
        let empty = match value.as_table_cell() {
            Some(cell) => Cell::Table(colspan_cell("", cell.get_colspan())?),
            None => Cell::from(""),
        };
        copy.set(key, empty);
    }
    Ok(copy)
}

/// `fillCells($row)` of a row (a separator stays as it is).
fn fill_cells_row(row: &Row) -> Row {
    match row {
        Row::Cells(cells) => Row::Cells(fill_cells(cells)),
        other => other.clone(),
    }
}

/// `fillCells($row)`: an empty cell after a colspan cell for each column
/// it covers.
fn fill_cells(row: &Cells) -> Cells {
    let mut new_row = Vec::with_capacity(row.len());
    for (_, cell) in row.iter() {
        new_row.push(cell.clone());
        let colspan = cell.colspan();
        if cell.as_table_cell().is_some() && colspan > 1 {
            for _ in 1..colspan {
                new_row.push(Cell::from(""));
            }
        }
    }
    if new_row.is_empty() {
        row.clone()
    } else {
        Arr::list(new_row)
    }
}

/// `preg_match('/^<(\w+|(\w+=[\w,]+;?)*)>.+<\/(\w+|(\w+=\w+;?)*)?>$/', $cell)`.
fn styled_by_tag(cell: &str) -> Result<bool> {
    thread_local! {
        static PATTERN: Option<Regex> = Regex::new(
            r"^<([A-Za-z0-9_]+|([A-Za-z0-9_]+=[A-Za-z0-9_,]+;?)*)>.+</([A-Za-z0-9_]+|([A-Za-z0-9_]+=[A-Za-z0-9_]+;?)*)?>\n?\z",
        )
        .ok();
    }
    PATTERN.with(|p| match p {
        Some(regex) => regex
            .is_match(cell)
            .map_err(|e| ConsoleError::Runtime(format!("Table: {e}"))),
        None => Err(ConsoleError::Logic(
            "Table: invalid cell style pattern".to_owned(),
        )),
    })
}

/// `initStyles()`.
fn init_styles() -> HashMap<String, TableStyle> {
    let mut markdown = TableStyle::new();
    markdown
        .set_default_crossing_char("|")
        .set_display_outside_border(false);

    let mut borderless = TableStyle::new();
    borderless
        .set_horizontal_border_chars("=", None)
        .set_vertical_border_chars(" ", None)
        .set_default_crossing_char(" ");

    let mut compact = TableStyle::new();
    compact
        .set_horizontal_border_chars("", None)
        .set_vertical_border_chars("", None)
        .set_default_crossing_char("")
        .set_cell_row_content_format("%s ");

    let mut style_guide = TableStyle::new();
    style_guide
        .set_horizontal_border_chars("-", None)
        .set_vertical_border_chars(" ", None)
        .set_default_crossing_char(" ")
        .set_cell_header_format("%s");

    let mut boxed = TableStyle::new();
    boxed
        .set_horizontal_border_chars("─", None)
        .set_vertical_border_chars("│", None)
        .set_crossing_chars(
            "┼", "┌", "┬", "┐", "┤", "┘", "┴", "└", "├", None, None, None,
        );

    let mut box_double = TableStyle::new();
    box_double
        .set_horizontal_border_chars("═", Some("─"))
        .set_vertical_border_chars("║", Some("│"))
        .set_crossing_chars(
            "┼",
            "╔",
            "╤",
            "╗",
            "╢",
            "╝",
            "╧",
            "╚",
            "╟",
            Some("╠"),
            Some("╪"),
            Some("╣"),
        );

    HashMap::from([
        ("default".to_owned(), TableStyle::new()),
        ("markdown".to_owned(), markdown),
        ("borderless".to_owned(), borderless),
        ("compact".to_owned(), compact),
        ("symfony-style-guide".to_owned(), style_guide),
        ("box".to_owned(), boxed),
        ("box-double".to_owned(), box_double),
    ])
}

/// `STR_PAD_*` as PHP constants, for callers that hold them as ints.
pub fn pad_type(value: i64) -> Option<PadType> {
    PadType::from_value(value)
}
