//! The parts of symfony/console that the Yaml and Dotenv commands use
//! (PLAN.md, M1-B). See README.md for the contract.
//!
//! Upstream: none

#![forbid(unsafe_code)]

pub mod application;
pub mod attribute;
pub mod ci;
pub mod color;
pub mod command;
pub mod completion;
pub mod env;
pub mod exception;
pub mod formatter;
pub mod helper;
pub mod input;
pub mod output;
pub mod php;
pub mod style;
pub mod terminal;
pub mod tester;

pub use exception::{ConsoleError, Result};
