//! What Console reads from PHP's environment: `getenv()`, `putenv()` and
//! `$_SERVER` entries, for the current thread.
//!
//! PHP reads the process environment. Changing it is `unsafe` in Rust, so
//! `putenv()` writes an overlay that `getenv()` reads first, as the Dotenv
//! crate does; it is per thread, so tests that set variables do not see
//! each other's. `$_SERVER` holds what PHP's CLI puts there: the process
//! environment, plus entries a caller sets.
//!
//! Upstream: none

use std::{cell::RefCell, collections::HashMap};

#[derive(Default)]
struct Overlay {
    /// `putenv('NAME=value')` is `Some(value)`, `putenv('NAME')` is `None`.
    env: HashMap<String, Option<String>>,
    server: HashMap<String, Option<String>>,
    /// `$_SERVER['argv']` when a caller set it.
    argv: Option<Vec<String>>,
}

thread_local! {
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay::default());
}

/// `getenv($name)`: `None` is PHP's `false`.
pub fn getenv(name: &str) -> Option<String> {
    OVERLAY
        .with(|o| o.borrow().env.get(name).cloned())
        .unwrap_or_else(|| std::env::var(name).ok())
}

/// `putenv("$name=$value")`, or `putenv($name)` with `None` to unset.
pub fn putenv(name: &str, value: Option<&str>) {
    OVERLAY.with(|o| {
        o.borrow_mut()
            .env
            .insert(name.to_owned(), value.map(str::to_owned));
    });
}

/// `$_SERVER[$name] ?? null` for a string entry.
pub fn server(name: &str) -> Option<String> {
    OVERLAY
        .with(|o| o.borrow().server.get(name).cloned())
        .unwrap_or_else(|| std::env::var(name).ok())
}

/// `$_SERVER[$name] = $value`, or `unset($_SERVER[$name])` with `None`.
pub fn set_server(name: &str, value: Option<&str>) {
    OVERLAY.with(|o| {
        o.borrow_mut()
            .server
            .insert(name.to_owned(), value.map(str::to_owned));
    });
}

/// `$_SERVER['argv']`: the process's arguments, unless a caller set them.
/// An argument that is not UTF-8 is returned as the error.
pub fn server_argv() -> std::result::Result<Vec<String>, std::ffi::OsString> {
    if let Some(argv) = OVERLAY.with(|o| o.borrow().argv.clone()) {
        return Ok(argv);
    }
    std::env::args_os().map(|arg| arg.into_string()).collect()
}

/// `$_SERVER['argv'] = $argv`; `None` reads the process's arguments again.
pub fn set_server_argv(argv: Option<Vec<String>>) {
    OVERLAY.with(|o| o.borrow_mut().argv = argv);
}
