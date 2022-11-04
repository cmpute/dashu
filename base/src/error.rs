//! Error types.

use core::fmt::{self, Display, Formatter};

/// Number out of bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionError {
    /// The number is not in the representation range
    OutOfBounds,
    /// The conversion will cause a loss of precision
    LossOfPrecision
}

impl Display for ConversionError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ConversionError::OutOfBounds => f.write_str("number out of bounds"),
            ConversionError::LossOfPrecision => f.write_str("number can't be converted losslessly"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConversionError {}

/// Error parsing a number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// No digits in the string.
    NoDigits,
    /// Invalid digit for a given radix.
    InvalidDigit,
    /// The radix is not supported.
    UnsupportedRadix,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ParseError::NoDigits => f.write_str("no digits"),
            ParseError::InvalidDigit => f.write_str("invalid digit"),
            ParseError::UnsupportedRadix => f.write_str("unsupported radix"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}
