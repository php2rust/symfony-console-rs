//! Replays tests/data/table_vectors.json: 1,500 generated tables rendered
//! by PHP 8.4.27 with symfony/console v8.1.9
//! (oracle/recorders/console_table.php, `scripts/record.sh console`).
//!
//! Upstream: none

mod common;

use common::{rows, set_headers, text};
use php_value::{Key, PhpArray, Value, vectors::decode};
use symfony_console::{
    ConsoleError,
    helper::table::Table,
    output::output::{StreamOutput, VERBOSITY_NORMAL},
};

fn field<'a>(case: &'a PhpArray, name: &str) -> &'a Value {
    case.get(&Key::from(name)).unwrap_or(&Value::Null)
}

fn int_pairs(value: &Value) -> Vec<(i64, i64)> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|(k, v)| match (k, v) {
                    (Key::Int(k), Value::Int(v)) => Some((*k, *v)),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn render(case: &PhpArray) -> Result<String, ConsoleError> {
    let decorated = matches!(field(case, "decorated"), Value::Bool(true));
    let mut output = StreamOutput::new(Vec::new(), false, VERBOSITY_NORMAL, Some(decorated), None);
    {
        let mut table = Table::new(&mut output);
        set_headers(&mut table, field(case, "headers"));
        table.set_rows(rows(field(case, "rows")));
        table.set_style(&text(field(case, "style")))?;
        match text(field(case, "orientation")).as_str() {
            "horizontal" => {
                table.set_horizontal(true);
            }
            "vertical" => {
                table.set_vertical(true);
            }
            _ => {}
        }
        for (column, width) in int_pairs(field(case, "widths")) {
            table.set_column_width(column, width);
        }
        for (column, width) in int_pairs(field(case, "max_widths")) {
            table.set_column_max_width(column, width);
        }
        let title = |name: &str| match field(case, name) {
            Value::Null => None,
            v => Some(text(v)),
        };
        let (header, footer) = (title("header_title"), title("footer_title"));
        table.set_header_title(header.as_deref());
        table.set_footer_title(footer.as_deref());
        table.render()?;
    }
    Ok(String::from_utf8_lossy(output.get_stream()).into_owned())
}

#[test]
fn table_renders_match_php() -> Result<(), php_value::vectors::Error> {
    let json: serde_json::Value = serde_json::from_str(include_str!("data/table_vectors.json"))?;
    let cases = json.as_array().ok_or("cases")?;
    let mut failures = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        let input = decode(&case["case"])?;
        let input = input.as_array().ok_or("case array")?;
        let result = &case["result"];
        let expected = match result[0].as_str() {
            Some("output") => Ok(String::from_utf8(php_value::vectors::base64(
                result[1].as_str().unwrap_or_default(),
            )?)?),
            _ => Err((
                result[1].as_str().unwrap_or_default().to_owned(),
                result[2].as_str().unwrap_or_default().to_owned(),
            )),
        };
        let actual = render(input).map_err(|e| (e.class().to_owned(), e.message().to_owned()));
        if actual != expected {
            failures.push(format!("case {n}:\n  ours {actual:?}\n  PHP  {expected:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(8)].join("\n")
    );
    Ok(())
}
