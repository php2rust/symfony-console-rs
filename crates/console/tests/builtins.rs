//! Replays tests/data/builtins.json, recorded from PHP 8.4.27 with
//! symfony/console v8.1.9 (`scripts/record.sh console`,
//! `oracle/recorders/console_builtins.php`): `stripcslashes()`, `StringInput`'s
//! tokens, `levenshtein()`, `strnatcasecmp()`, `<=>` of two strings,
//! `OutputWrapper::wrap()`, and
//! `Application::find()` / `findNamespace()` on a fixed set of commands.
//!
//! Upstream: none

use std::cmp::Ordering;

use serde_json::Value as Json;
use symfony_console::{
    ConsoleError,
    application::Application,
    command::command::Command,
    env::putenv,
    helper::output_wrapper::OutputWrapper,
    input::string_input::{stripcslashes, tokenize},
    php::{compare_strings, levenshtein, strnatcasecmp},
};

type Failures = Vec<String>;

fn data() -> Result<Json, serde_json::Error> {
    serde_json::from_str(include_str!("data/builtins.json"))
}

fn rows<'a>(data: &'a Json, key: &str) -> &'a [Json] {
    data[key].as_array().map_or(&[], Vec::as_slice)
}

fn text(value: &Json) -> &str {
    value.as_str().unwrap_or_default()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn check(name: &str, total: usize, failures: &Failures) {
    assert!(
        failures.is_empty(),
        "{name}: {} of {total} differ:\n{}",
        failures.len(),
        failures[..failures.len().min(20)].join("\n")
    );
    assert!(total > 0, "{name}: no recorded cases");
}

fn sign(o: Ordering) -> i64 {
    match o {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[test]
fn stripcslashes_matches_php() -> Result<(), serde_json::Error> {
    let data = data()?;
    let cases = rows(&data, "stripcslashes");
    let failures: Failures = cases
        .iter()
        .filter_map(|c| {
            let ours = hex(&stripcslashes(text(&c[0]).as_bytes()));
            (ours != text(&c[1])).then(|| format!("{:?}: {ours}, PHP {}", c[0], c[1]))
        })
        .collect();
    check("stripcslashes", cases.len(), &failures);
    Ok(())
}

#[test]
fn string_input_tokens_match_php() -> Result<(), serde_json::Error> {
    let data = data()?;
    let cases = rows(&data, "string_input");
    let failures: Failures = cases
        .iter()
        .filter_map(|c| {
            let input = text(&c[0]);
            let ours = match tokenize(input.as_bytes()) {
                Ok(tokens) => Json::from(tokens.iter().map(|t| hex(t)).collect::<Vec<_>>()),
                Err(e) => Json::from(vec![e.class().to_owned(), e.message().to_owned()]),
            };
            (ours != c[1]).then(|| format!("{input:?}: {ours}, PHP {}", c[1]))
        })
        .collect();
    check("StringInput", cases.len(), &failures);
    Ok(())
}

#[test]
fn levenshtein_matches_php() -> Result<(), serde_json::Error> {
    let data = data()?;
    let cases = rows(&data, "levenshtein");
    let failures: Failures = cases
        .iter()
        .filter_map(|c| {
            let ours = levenshtein(text(&c[0]).as_bytes(), text(&c[1]).as_bytes()) as u64;
            (Some(ours) != c[2].as_u64()).then(|| format!("{c}: {ours}"))
        })
        .collect();
    check("levenshtein", cases.len(), &failures);
    Ok(())
}

#[test]
fn strnatcasecmp_matches_php() -> Result<(), serde_json::Error> {
    let data = data()?;
    let cases = rows(&data, "strnatcasecmp");
    let failures: Failures = cases
        .iter()
        .filter_map(|c| {
            let ours = sign(strnatcasecmp(
                text(&c[0]).as_bytes(),
                text(&c[1]).as_bytes(),
            ));
            (Some(ours) != c[2].as_i64()).then(|| format!("{c}: {ours}"))
        })
        .collect();
    check("strnatcasecmp", cases.len(), &failures);
    Ok(())
}

#[test]
fn string_comparison_matches_php() -> Result<(), serde_json::Error> {
    let data = data()?;
    let cases = rows(&data, "compare");
    let failures: Failures = cases
        .iter()
        .filter_map(|c| {
            let ours = sign(compare_strings(
                text(&c[0]).as_bytes(),
                text(&c[1]).as_bytes(),
            ));
            (Some(ours) != c[2].as_i64()).then(|| format!("{c}: {ours}"))
        })
        .collect();
    check("<=>", cases.len(), &failures);
    Ok(())
}

#[test]
fn wrap_matches_php() -> Result<(), Box<dyn std::error::Error>> {
    let data = data()?;
    let cases = rows(&data, "wrap");
    let mut failures = Failures::new();
    for c in cases {
        let (text_in, width, cut) = (
            text(&c[0]),
            c[1].as_i64().unwrap_or(0),
            c[2].as_bool().unwrap_or(false),
        );
        let ours = OutputWrapper::new(cut).wrap(
            text_in, width, "
",
        )?;
        if ours != text(&c[3]) {
            failures.push(format!(
                "wrap({text_in:?}, {width}, {cut}):
  ours {ours:?}
  PHP  {:?}",
                text(&c[3])
            ));
        }
    }
    check("wrap", cases.len(), &failures);
    Ok(())
}

fn outcome(result: Result<String, ConsoleError>) -> Json {
    match result {
        Ok(name) => Json::from(vec![Json::from("ok"), Json::from(name)]),
        Err(e) => Json::from(vec![
            Json::from(e.class()),
            Json::from(e.message()),
            Json::from(e.alternatives().unwrap_or_default().to_vec()),
        ]),
    }
}

fn integers_as_strings(value: &Json) -> Json {
    match value {
        Json::Number(n) if n.is_i64() => Json::from(n.to_string()),
        Json::Array(items) => Json::from(items.iter().map(integers_as_strings).collect::<Vec<_>>()),
        other => other.clone(),
    }
}

#[test]
fn find_matches_php() -> Result<(), Box<dyn std::error::Error>> {
    putenv("COLUMNS", Some("120"));
    let data = data()?;
    let mut application = Application::default();
    for c in rows(&data, "commands") {
        let mut command = Command::new(Some(text(&c[0])), None)?;
        let aliases: Vec<&str> = c[1]
            .as_array()
            .map_or(Vec::new(), |a| a.iter().map(text).collect());
        command.set_aliases(aliases)?;
        command.set_description(text(&c[2]));
        command.set_hidden(c[3].as_bool().unwrap_or_default());
        application.add_command(command)?;
    }

    let mut total = 0;
    let mut failures = Failures::new();
    for (key, namespace) in [("find", false), ("find_namespace", true)] {
        for c in rows(&data, key) {
            total += 1;
            let name = text(&c[0]);
            let ours = if namespace {
                outcome(application.find_namespace(name))
            } else {
                outcome(
                    application
                        .find(name)
                        .map(|command| command.lock().get_name().unwrap_or_default().to_owned()),
                )
            };
            // A numeric command name is an integer key in PHP's array, so
            // getAlternatives() gives it as an int; here it stays a string
            // (crate README, Differences).
            if ours != integers_as_strings(&c[1]) {
                failures.push(format!("{key}({name:?}):\n  ours {ours}\n  PHP  {}", c[1]));
            }
        }
    }
    putenv("COLUMNS", None);
    check("find", total, &failures);
    Ok(())
}
