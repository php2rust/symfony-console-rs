//! Port of Symfony\Component\Console\Exception\*.php: the exception classes
//! a console error can be, each with its message.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Exception/ExceptionInterface.php Exception/InvalidArgumentException.php Exception/RuntimeException.php Exception/LogicException.php Exception/CommandNotFoundException.php Exception/NamespaceNotFoundException.php Exception/InvalidOptionException.php

/// A console exception: its class and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleError {
    /// `InvalidArgumentException` (extends `\InvalidArgumentException`).
    InvalidArgument(String),
    /// `InvalidOptionException` (extends `\InvalidArgumentException`).
    InvalidOption(String),
    /// `RuntimeException` (extends `\RuntimeException`).
    Runtime(String),
    /// `LogicException` (extends `\LogicException`).
    Logic(String),
    /// `CommandNotFoundException`, with the alternatives it suggests.
    CommandNotFound(String, Vec<String>),
    /// `NamespaceNotFoundException` (extends `CommandNotFoundException`).
    NamespaceNotFound(String, Vec<String>),
    /// An error from the command's own code, already formatted by its
    /// class name.
    Other { class: String, message: String },
}

impl ConsoleError {
    /// The PHP class, as `$e::class` gives it.
    pub fn class(&self) -> &str {
        match self {
            Self::InvalidArgument(_) => {
                "Symfony\\Component\\Console\\Exception\\InvalidArgumentException"
            }
            Self::InvalidOption(_) => {
                "Symfony\\Component\\Console\\Exception\\InvalidOptionException"
            }
            Self::Runtime(_) => "Symfony\\Component\\Console\\Exception\\RuntimeException",
            Self::Logic(_) => "Symfony\\Component\\Console\\Exception\\LogicException",
            Self::CommandNotFound(..) => {
                "Symfony\\Component\\Console\\Exception\\CommandNotFoundException"
            }
            Self::NamespaceNotFound(..) => {
                "Symfony\\Component\\Console\\Exception\\NamespaceNotFoundException"
            }
            Self::Other { class, .. } => class,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::InvalidArgument(m)
            | Self::InvalidOption(m)
            | Self::Runtime(m)
            | Self::Logic(m)
            | Self::CommandNotFound(m, _)
            | Self::NamespaceNotFound(m, _)
            | Self::Other { message: m, .. } => m,
        }
    }

    /// `instanceof \InvalidArgumentException`.
    pub fn is_invalid_argument(&self) -> bool {
        matches!(
            self,
            Self::InvalidArgument(_)
                | Self::InvalidOption(_)
                | Self::CommandNotFound(..)
                | Self::NamespaceNotFound(..)
        )
    }

    /// `instanceof \RuntimeException`.
    pub fn is_runtime(&self) -> bool {
        matches!(self, Self::Runtime(_))
    }

    /// `instanceof CommandNotFoundException`: its alternatives.
    pub fn alternatives(&self) -> Option<&[String]> {
        match self {
            Self::CommandNotFound(_, a) | Self::NamespaceNotFound(_, a) => Some(a),
            _ => None,
        }
    }

    /// `instanceof ExceptionInterface`: one of Console's own exceptions.
    pub fn is_console_exception(&self) -> bool {
        !matches!(self, Self::Other { .. })
    }
}

impl std::fmt::Display for ConsoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for ConsoleError {}

pub type Result<T, E = ConsoleError> = std::result::Result<T, E>;
