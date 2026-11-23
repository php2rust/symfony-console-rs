//! Port of Symfony\Component\Console\Helper\TableCell.php,
//! TableCellStyle.php and TableSeparator.php, and the cell values a
//! Table row holds.
//!
//! PHP passes options as arrays and checks their names; here they are
//! `(name, value)` pairs, checked the same way, with upstream's messages.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/TableCell.php Helper/TableCellStyle.php Helper/TableSeparator.php

use php_value::{Value, numeric::float_to_string};

use super::table_style::PadType;
use crate::exception::{ConsoleError, Result};

/// A TableCell option's value.
#[derive(Debug, Clone, PartialEq)]
pub enum CellOption {
    /// `rowspan` or `colspan`, as PHP's `(int)` of the value given.
    Int(i64),
    Style(TableCellStyle),
    Null,
}

/// `TableCell` (and `TableSeparator`, which is a TableCell).
#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    value: String,
    rowspan: i64,
    colspan: i64,
    style: Option<Box<TableCellStyle>>,
    separator: bool,
}

impl TableCell {
    /// `new TableCell($value)`.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            rowspan: 1,
            colspan: 1,
            style: None,
            separator: false,
        }
    }

    /// `new TableCell($value, $options)`.
    pub fn with_options(value: impl Into<String>, options: &[(&str, CellOption)]) -> Result<Self> {
        let mut cell = Self::new(value);
        cell.apply(options, "TableCell")?;
        Ok(cell)
    }

    /// `new TableSeparator($options)`.
    pub fn separator(options: &[(&str, CellOption)]) -> Result<Self> {
        let mut cell = Self::new("");
        cell.separator = true;
        cell.apply(options, "TableCell")?;
        Ok(cell)
    }

    fn apply(&mut self, options: &[(&str, CellOption)], class: &str) -> Result<()> {
        let unknown: Vec<&str> = options
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| !matches!(*name, "rowspan" | "colspan" | "style"))
            .collect();
        if !unknown.is_empty() {
            return Err(ConsoleError::InvalidArgument(format!(
                "The {class} does not support the following options: '{}'.",
                unknown.join("', '")
            )));
        }
        for (name, value) in options {
            match (*name, value) {
                ("style", CellOption::Style(style)) => self.style = Some(Box::new(style.clone())),
                ("style", CellOption::Null) => self.style = None,
                ("style", CellOption::Int(_)) => {
                    return Err(ConsoleError::InvalidArgument(
                        "The style option must be an instance of \"TableCellStyle\".".to_owned(),
                    ));
                }
                ("rowspan", v) => self.rowspan = option_int(v),
                (_, v) => self.colspan = option_int(v),
            }
        }
        Ok(())
    }

    /// `instanceof TableSeparator`.
    pub fn is_separator(&self) -> bool {
        self.separator
    }

    /// `__toString()`.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// `getColspan()`.
    pub fn get_colspan(&self) -> i64 {
        self.colspan
    }

    /// `getRowspan()`.
    pub fn get_rowspan(&self) -> i64 {
        self.rowspan
    }

    /// `getStyle()`.
    pub fn get_style(&self) -> Option<&TableCellStyle> {
        self.style.as_deref()
    }
}

/// `(int) $option`.
fn option_int(value: &CellOption) -> i64 {
    match value {
        CellOption::Int(i) => *i,
        CellOption::Null => 0,
        CellOption::Style(_) => 1,
    }
}

/// `TableCellStyle`. Its option values are strings or null.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellStyle {
    fg: Option<String>,
    bg: Option<String>,
    options: Option<String>,
    align: String,
    cell_format: Option<String>,
}

impl Default for TableCellStyle {
    fn default() -> Self {
        Self {
            fg: Some("default".to_owned()),
            bg: Some("default".to_owned()),
            options: None,
            align: TableCellStyle::DEFAULT_ALIGN.to_owned(),
            cell_format: None,
        }
    }
}

impl TableCellStyle {
    pub const DEFAULT_ALIGN: &'static str = "left";

    /// `new TableCellStyle($options)`.
    pub fn new(options: &[(&str, Option<&str>)]) -> Result<Self> {
        let unknown: Vec<&str> = options
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| !matches!(*name, "fg" | "bg" | "options" | "align" | "cellFormat"))
            .collect();
        if !unknown.is_empty() {
            return Err(ConsoleError::InvalidArgument(format!(
                "The TableCellStyle does not support the following options: '{}'.",
                unknown.join("', '")
            )));
        }
        // isset($options['align']): a null align is not checked
        if let Some((_, Some(align))) = options.iter().find(|(n, _)| *n == "align")
            && !matches!(*align, "left" | "center" | "right")
        {
            return Err(ConsoleError::InvalidArgument(
                "Wrong align value. Value must be following: 'left', 'center', 'right'.".to_owned(),
            ));
        }
        let mut style = Self::default();
        for (name, value) in options {
            let value = value.map(str::to_owned);
            match *name {
                "fg" => style.fg = value,
                "bg" => style.bg = value,
                "options" => style.options = value,
                "align" => style.align = value.unwrap_or_default(),
                _ => style.cell_format = value,
            }
        }
        Ok(style)
    }

    /// `getOptions()`, in upstream's key order.
    pub fn get_options(&self) -> Vec<(&'static str, Option<&str>)> {
        vec![
            ("fg", self.fg.as_deref()),
            ("bg", self.bg.as_deref()),
            ("options", self.options.as_deref()),
            ("align", Some(self.align.as_str())),
            ("cellFormat", self.cell_format.as_deref()),
        ]
    }

    /// `getTagOptions()`: fg, bg and options, when set.
    pub fn get_tag_options(&self) -> Vec<(&'static str, &str)> {
        [
            ("fg", self.fg.as_deref()),
            ("bg", self.bg.as_deref()),
            ("options", self.options.as_deref()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect()
    }

    /// `getPadByAlign()`. A null align (allowed by the constructor) is
    /// not in the map; PHP then pads with `null`, which is `STR_PAD_LEFT`.
    pub fn get_pad_by_align(&self) -> PadType {
        match self.align.as_str() {
            "center" => PadType::Both,
            "right" | "" => PadType::Left,
            _ => PadType::Right,
        }
    }

    /// `getCellFormat()`.
    pub fn get_cell_format(&self) -> Option<&str> {
        self.cell_format.as_deref()
    }
}

/// A cell of a Table row: a scalar, null or another value as PHP holds
/// it, or a TableCell (a TableSeparator included).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Value(Value),
    Table(TableCell),
}

impl Cell {
    /// `null`.
    pub fn null() -> Self {
        Self::Value(Value::Null)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Value(Value::Null))
    }

    /// `$cell instanceof TableCell`.
    pub fn as_table_cell(&self) -> Option<&TableCell> {
        match self {
            Self::Table(cell) => Some(cell),
            Self::Value(_) => None,
        }
    }

    /// `$cell instanceof TableSeparator`.
    pub fn is_separator(&self) -> bool {
        self.as_table_cell().is_some_and(TableCell::is_separator)
    }

    /// The colspan of a TableCell, else 1.
    pub fn colspan(&self) -> i64 {
        self.as_table_cell().map_or(1, TableCell::get_colspan)
    }

    /// `(string) $cell`. An array or a non-Stringable object has no
    /// string form (PHP warns or throws); `fillNextRows()` refuses such a
    /// cell before it is used.
    pub fn to_php_string(&self) -> Result<String> {
        match self {
            Self::Table(cell) => Ok(cell.value.clone()),
            Self::Value(value) => match value {
                Value::Null | Value::Bool(false) => Ok(String::new()),
                Value::Bool(true) => Ok("1".to_owned()),
                Value::Int(i) => Ok(i.to_string()),
                Value::Float(f) => Ok(float_to_string(*f)),
                Value::String(s) => Ok(s.to_string_lossy().into_owned()),
                other => Err(not_a_cell(other)),
            },
        }
    }

    /// `if ($cell)`: a TableCell is an object, so always true.
    pub fn truthy(&self) -> bool {
        match self {
            Self::Table(_) => true,
            Self::Value(value) => match value {
                Value::Null | Value::Bool(false) => false,
                Value::Int(i) => *i != 0,
                Value::Float(f) => *f != 0.0,
                Value::String(s) => !(s.is_empty() || s.as_bytes() == b"0"),
                Value::Array(a) => !a.is_empty(),
                _ => true,
            },
        }
    }

    /// Whether `fillNextRows()` accepts the cell: null, a TableCell, a
    /// scalar or a Stringable.
    pub fn check(&self) -> Result<()> {
        match self {
            Self::Value(v @ (Value::Array(_) | Value::Object(_))) => Err(not_a_cell(v)),
            _ => Ok(()),
        }
    }
}

fn not_a_cell(value: &Value) -> ConsoleError {
    ConsoleError::InvalidArgument(format!(
        "A cell must be a TableCell, a scalar or an object implementing \"__toString()\", \"{}\" given.",
        value.debug_type()
    ))
}

impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        Self::Value(Value::string(s))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Self::Value(Value::string(s))
    }
}

impl From<i64> for Cell {
    fn from(i: i64) -> Self {
        Self::Value(Value::Int(i))
    }
}

impl From<TableCell> for Cell {
    fn from(cell: TableCell) -> Self {
        Self::Table(cell)
    }
}

impl From<Value> for Cell {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}
