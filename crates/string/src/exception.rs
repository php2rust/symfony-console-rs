//! Port of Symfony\Component\String\Exception\InvalidArgumentException.php
//! and Exception\RuntimeException.php, both behind ExceptionInterface.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/string Exception/InvalidArgumentException.php Exception/RuntimeException.php Exception/ExceptionInterface.php

use std::fmt;

/// `Symfony\Component\String\Exception\InvalidArgumentException`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidArgumentException(pub String);

impl fmt::Display for InvalidArgumentException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidArgumentException {}

/// `Symfony\Component\String\Exception\RuntimeException`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeException(pub String);

impl fmt::Display for RuntimeException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RuntimeException {}

/// What a symfony/string method throws (`ExceptionInterface`), plus what
/// this port refuses rather than approximates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidArgument(InvalidArgumentException),
    Runtime(RuntimeException),
    /// A PHP `\Error` (`ValueError`, `TypeError`) from a native function
    /// upstream calls without catching it, e.g. `str_pad()` with an empty
    /// pad on a ByteString.
    Php(String),
    /// Input PHP accepts that this port does not reproduce (a pattern
    /// feature php-pcre does not translate, an ICU transliterator).
    Unsupported(String),
}

impl Error {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidArgument(InvalidArgumentException(message.into()))
    }

    pub(crate) fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(RuntimeException(message.into()))
    }

    /// `new InvalidArgumentException('Invalid UTF-8 string.')`.
    pub(crate) fn invalid_utf8() -> Self {
        Self::invalid("Invalid UTF-8 string.")
    }

    /// The exception's message.
    pub fn message(&self) -> &str {
        match self {
            Self::InvalidArgument(e) => &e.0,
            Self::Runtime(e) => &e.0,
            Self::Php(m) | Self::Unsupported(m) => m,
        }
    }

    pub fn is_invalid_argument(&self) -> bool {
        matches!(self, Self::InvalidArgument(_))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Error {}

impl From<InvalidArgumentException> for Error {
    fn from(e: InvalidArgumentException) -> Self {
        Self::InvalidArgument(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
