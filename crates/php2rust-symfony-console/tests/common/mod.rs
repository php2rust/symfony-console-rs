//! Decoding of recorded tables (typed JSON from the oracle) into Table
//! rows and cells, shared by the Table tests.
//!
//! Upstream: none

#![allow(dead_code)]

use php_value::{Key, PhpArray, Value};
use symfony_console::helper::{
    table::{Arr, Row, Table},
    table_cell::{Cell, CellOption, TableCell, TableCellStyle},
};

pub fn text(value: &Value) -> String {
    value
        .as_string()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn property<'a>(properties: &'a PhpArray, name: &str) -> Option<&'a Value> {
    properties.get(&Key::from(name))
}

fn cell_style(value: &Value) -> Option<TableCellStyle> {
    let object = value.as_object()?;
    let properties = object.properties();
    let options: Vec<(String, Option<String>)> = properties
        .iter()
        .map(|(k, v)| {
            let key = match k {
                Key::Str(s) => s.to_string_lossy().into_owned(),
                Key::Int(i) => i.to_string(),
            };
            (key, if v.is_null() { None } else { Some(text(v)) })
        })
        .collect();
    let borrowed: Vec<(&str, Option<&str>)> = options
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_deref()))
        .collect();
    TableCellStyle::new(&borrowed).ok()
}

/// A recorded cell: a `TableCell` or `TableSeparator` object, or a value.
pub fn cell(value: &Value) -> Cell {
    let Some(object) = value.as_object() else {
        return Cell::Value(value.clone());
    };
    let properties = object.properties();
    let int = |name: &str| match property(&properties, name) {
        Some(Value::Int(i)) => *i,
        _ => 1,
    };
    let style = property(&properties, "style").and_then(cell_style);
    let options = vec![
        ("rowspan", CellOption::Int(int("rowspan"))),
        ("colspan", CellOption::Int(int("colspan"))),
        ("style", style.map_or(CellOption::Null, CellOption::Style)),
    ];
    let table_cell = if object.class().ends_with("TableSeparator") {
        TableCell::separator(&options)
    } else {
        TableCell::with_options(
            property(&properties, "value").map(text).unwrap_or_default(),
            &options,
        )
    };
    Cell::Table(table_cell.expect("recorded cell"))
}

fn is_separator(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.class().ends_with("TableSeparator"))
}

/// A recorded row: an array of cells (its keys kept) or a `TableSeparator`.
pub fn table_row(value: &Value) -> Row {
    match value.as_array() {
        Some(cells) => Row::Cells(Arr::keyed(
            cells
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        Key::Int(i) => *i,
                        Key::Str(_) => panic!("string keys are not supported"),
                    };
                    (key, cell(v))
                })
                .collect(),
        )),
        None if is_separator(value) => Row::separator(),
        None => panic!("a row must be an array or a TableSeparator"),
    }
}

pub fn rows(value: &Value) -> Vec<Row> {
    value
        .as_array()
        .map(|a| a.values().map(table_row).collect())
        .unwrap_or_default()
}

/// `setHeaders($headers)`: rows of headers when the first is an array.
pub fn set_headers(table: &mut Table<'_>, value: &Value) {
    let Some(headers) = value.as_array() else {
        return;
    };
    if headers
        .values()
        .next()
        .is_some_and(|h| h.as_array().is_some())
    {
        table.set_header_rows(headers.values().map(table_row).collect());
    } else {
        table.set_headers(headers.values().map(cell).collect::<Vec<_>>());
    }
}
